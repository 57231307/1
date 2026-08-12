//! 凭证管理 Service（facade，批次 488 D10-2a 拆分）
//!
//! 本文件为 facade 入口，仅保留 `VoucherService` struct + `new` 构造函数 + DTOs + 类型定义 + 单元测试。
//! 业务实现已按职责拆分到 `voucher_ops/` 子模块（与 `voucher_service` 同为 `crate::services` 下兄弟模块）：
//! - `voucher_ops::crud`：CRUD 与状态校验（11 方法，原 L124-350 + L480-522 + L1258-1288）
//! - `voucher_ops::workflow`：工作流状态机 submit/review/post（5 方法，原 L523-717）
//! - `voucher_ops::balance`：科目余额更新（12 方法 + BalanceUpdateContext，原 L88-97 + L720-1045）
//! - `voucher_ops::assist`：辅助核算记录写入（11 方法 + AssistRecordContext，原 L98-113 + L1052-1255）
//!
//! 设计要点（与拆分前一致）：
//! - 凭证状态机：draft → submitted → reviewed → posted（不可逆）
//! - 状态变更加 lock_exclusive 串行化并发
//! - post 内部调用 balance::update_account_balances 回写科目余额
//! - post 内部调用 assist::write_assist_accounting_records_txn 写入辅助核算记录
//! - 期末余额按会计制度计算（借方科目：期初借+本期借-本期贷；贷方科目反之）
//! - 辅助核算五维 ID：BATCH:{}|COLOR:{}|DYE_LOT:{}|GRADE:{}|WORKSHOP:{}
//!
//! 拆分兼容性：
//! - 外部 handler 通过 `crate::services::voucher_service::VoucherService::new` 调用，路径不变
//! - 外部 handler 通过 `crate::services::voucher_service::{CreateVoucherRequest, UpdateVoucherRequest, VoucherItemRequest, VoucherQueryParams, VoucherTypeDefinition}` 引用，路径不变
//! - `db` 字段使用 `pub(crate)` 可见性，voucher_ops 子模块的 impl 块可直接访问
//! - impl 块分散在 voucher_ops 子模块，Rust 允许同一 crate 多文件多 impl 块
//! - `update_account_balances` / `write_assist_accounting_records_txn` 使用 `pub(crate)` 可见性，允许 workflow::post 跨 impl 块调用

// 凭证管理 Service
//
// 凭证业务逻辑层（核心）

use sea_orm::DatabaseConnection;
use std::sync::Arc;

use crate::models::{voucher, voucher_item};
use rust_decimal::Decimal;

// 批次 102 v6 P3-1 修复：状态字符串常量化，引用 crate::models::status::voucher

/// 创建凭证请求
#[derive(Debug, Clone)]
pub struct CreateVoucherRequest {
    pub voucher_type: String,
    pub voucher_date: chrono::NaiveDate,
    pub source_type: Option<String>,
    pub source_module: Option<String>,
    pub source_bill_id: Option<i32>,
    pub source_bill_no: Option<String>,
    pub batch_no: Option<String>,
    pub color_no: Option<String>,
    pub items: Vec<VoucherItemRequest>,
}

/// 凭证分录请求
#[derive(Debug, Clone)]
pub struct VoucherItemRequest {
    pub line_no: Option<i32>,
    pub subject_code: Option<String>,
    pub subject_name: Option<String>,
    pub debit: Decimal,
    pub credit: Decimal,
    pub summary: Option<String>,
    pub assist_customer_id: Option<i32>,
    pub assist_supplier_id: Option<i32>,
    pub assist_department_id: Option<i32>,
    pub assist_employee_id: Option<i32>,
    pub assist_project_id: Option<i32>,
    pub assist_batch_id: Option<i32>,
    pub assist_color_no_id: Option<i32>,
    pub assist_dye_lot_id: Option<i32>,
    pub assist_grade: Option<String>,
    pub assist_workshop_id: Option<i32>,
    pub quantity_meters: Option<Decimal>,
    pub quantity_kg: Option<Decimal>,
    pub unit_price: Option<Decimal>,
}

/// 更新凭证请求
#[derive(Debug, Clone)]
pub struct UpdateVoucherRequest {
    pub voucher_type: Option<String>,
    pub voucher_date: Option<chrono::NaiveDate>,
    pub items: Option<Vec<VoucherItemRequest>>,
}

/// 凭证查询参数
#[derive(Debug, Clone)]
pub struct VoucherQueryParams {
    pub voucher_type: Option<String>,
    pub status: Option<String>,
    pub start_date: Option<chrono::NaiveDate>,
    pub end_date: Option<chrono::NaiveDate>,
    pub batch_no: Option<String>,
    pub color_no: Option<String>,
    pub page: Option<u64>,
    pub page_size: Option<u64>,
}

/// 凭证 Service（struct 定义保留在 facade，impl 块按职责分散到 `voucher_ops/` 子模块。）
pub struct VoucherService {
    /// 数据库连接句柄（`pub(crate)` 可见性：voucher_ops 兄弟模块的 impl 块需直接访问此字段。）
    pub(crate) db: Arc<DatabaseConnection>,
}

impl VoucherService {
    /// 创建凭证服务实例
    pub fn new(db: Arc<DatabaseConnection>) -> Self {
        Self { db }
    }
}

/// 凭证类型定义（v11 批次 155 P2-C：静态配置化，避免 handler 硬编码）
#[derive(Debug, Clone, serde::Serialize)]
pub struct VoucherTypeDefinition {
    pub code: &'static str,
    pub name: &'static str,
}

impl VoucherTypeDefinition {
    pub fn new(code: &'static str, name: &'static str) -> Self {
        Self { code, name }
    }
}

/// 凭证详情（包含分录）
// v11 批次 148 P2-A：移除失效的 dead_code 标注（get_by_id 方法返回 VoucherDetail，被 voucher_handler::get_voucher 真实调用）
#[derive(Debug, Clone)]
pub struct VoucherDetail {
    pub voucher: voucher::Model,
    pub items: Vec<voucher_item::Model>,
}
