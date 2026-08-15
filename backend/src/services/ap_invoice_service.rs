//! 应付单 Service（facade）
//!
//! 应付单服务层，负责应付单的核心业务逻辑
//! 包含应付单自动生成、手工创建、审核、核销等全流程管理
//!
//! 批次 490 D10-4b 拆分：本文件作为 facade，保留 ApInvoiceService struct + new 构造函数
//! + ApInvoiceListQuery 查询参数 + 校验纯函数 + 单号生成宏 + 单元测试。
//! impl 块迁移至 `ap_invoice_ops` 子模块（receipt / crud / report），
//! DTOs 迁移至 `ap_invoice_ops::types`，通过 db 字段 pub(crate) 让 ops 访问，
//! 外部引用路径（crate::services::ap_invoice_service::ApInvoiceService 等）保持不变。

use chrono::NaiveDate;
use rust_decimal::Decimal;
use sea_orm::DatabaseConnection;
use std::sync::Arc;

// 重新导出 DTOs（迁移至 ap_invoice_ops::types），保持外部引用路径不变
// 外部仍可通过 crate::services::ap_invoice_service::{CreateApInvoiceRequest, ...} 访问
pub use crate::services::ap_invoice_ops::types::{
    AgingAnalysisItem, ApInvoiceStatistics, BalanceSummary, CreateApInvoiceRequest, StatusStatItem,
    UpdateApInvoiceRequest,
};

// 批次 102 v6 P3-2 修复：状态字符串常量化，引用 crate::models::status

/// 默认本位币汇率（CNY 本位币 = 1.0）
/// 历史缺陷（P0-1，2026-06-25 综合审计）：自动生成 AP 发票时曾误用；`Decimal::new(1, 2)` = 0.01，导致下游按汇率换算本位币金额被缩小 100 倍。；抽取为常量并在单元测试中断言其值，避免再次被改错。；注意：`Decimal::new` 不是 const fn，不能用于 const 初始化；使用 rust_decimal 提供的 const 关联常量 `Decimal::ONE`（= 1.0）。
pub const DEFAULT_BASE_CURRENCY_EXCHANGE_RATE: Decimal = Decimal::ONE;

/// 应付单服务
pub struct ApInvoiceService {
    pub(crate) db: Arc<DatabaseConnection>,
}

/// 应付单列表查询参数（service 层，page/page_size 已解析为非 Option）
#[derive(Debug, Clone)]
pub struct ApInvoiceListQuery {
    pub supplier_id: Option<i32>,
    pub invoice_status: Option<String>,
    pub invoice_type: Option<String>,
    pub start_date: Option<NaiveDate>,
    pub end_date: Option<NaiveDate>,
    pub page: u64,
    pub page_size: u64,
}

impl ApInvoiceService {
    /// 创建服务实例
    pub fn new(db: Arc<DatabaseConnection>) -> Self {
        Self { db }
    }

    // 生成应付单号
    // 格式：AP + 年月日 + 三位序号（AP20260315001）
    crate::impl_generate_no!(
        generate_invoice_no,
        "API",
        crate::models::ap_invoice::Entity,
        crate::models::ap_invoice::Column::InvoiceNo
    );
}

// =====================================================
// DTO 校验函数（TS-S-5 安全加固）
// =====================================================
//
// 校验纯函数保留在 facade（与 DTOs 的 #[validate] 引用耦合），
// 通过 pub(crate) 让 ap_invoice_ops::types 的 DTOs 全路径引用
//（crate::services::ap_invoice_service::validate_*），与 crate::utils::validator 用法一致。

/// 校验 Decimal 为正数
pub fn validate_positive_decimal(value: &Decimal) -> Result<(), validator::ValidationError> {
    if *value <= Decimal::ZERO {
        return Err(validator::ValidationError::new("金额必须为正数"));
    }
    Ok(())
}

/// 校验 Decimal 为非负数
pub fn validate_non_negative_decimal(value: &Decimal) -> Result<(), validator::ValidationError> {
    if *value < Decimal::ZERO {
        return Err(validator::ValidationError::new("金额不能为负数"));
    }
    Ok(())
}

/// 校验汇率合法：必须大于 0 且不等于 P0-1 历史缺陷值 0.01
pub fn validate_exchange_rate(value: &Decimal) -> Result<(), validator::ValidationError> {
    if *value <= Decimal::ZERO {
        return Err(validator::ValidationError::new("汇率必须大于0"));
    }
    // P0-1 防护：拒绝 0.01 汇率（历史缺陷值）
    if *value == Decimal::new(1, 2) {
        return Err(validator::ValidationError::new(
            "汇率不能为0.01（P0-1历史缺陷值，本位币汇率应为1.0）",
        ));
    }
    Ok(())
}
