//! 采购服务模块（po = purchase order）
//!
//! 由原 `services/purchase_order_service.rs`（1752 行）按业务子领域拆分而来。
//! 子模块：
//! - `order`    采购订单（核心 CRUD / 生命周期）
//! - `contract` 采购合同（审批工作流：提交、审批、拒绝）
//! - `receipt`  采购收货（含库存联动）
//! - `price`    采购价格（采购建议、预算占用）
//! - `return_rs`采购退货（占位模块，待后续扩展）
//!
//! 兼容说明：原 `crate::services::purchase_order_service::*` 路径需要由上层
//! `services/mod.rs` 通过 `pub use super::po::*;` 重新导出以保持向后兼容。

use chrono::NaiveDate;
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use validator::Validate;

pub mod contract;
pub mod order;
pub mod price;
pub mod receipt;
pub mod return_rs;

// =====================================================
// 请求 DTO（与原 purchase_order_service.rs 末尾保持一致）
// =====================================================

/// 创建采购订单请求
#[derive(Debug, Validate, Deserialize)]
pub struct CreatePurchaseOrderRequest {
    /// 供应商 ID
    #[validate(range(min = 1, message = "供应商ID必须大于0"))]
    pub supplier_id: i32,

    /// 订单日期
    pub order_date: NaiveDate,

    /// 预计交货日期
    pub expected_delivery_date: Option<NaiveDate>,

    /// 仓库 ID
    pub warehouse_id: Option<i32>,

    /// 部门 ID
    pub department_id: Option<i32>,

    /// 币种
    #[validate(length(max = 10, message = "币种长度不能超过10个字符"))]
    pub currency: Option<String>,

    /// 汇率
    pub exchange_rate: Option<Decimal>,

    /// 付款条件
    #[validate(length(max = 200, message = "付款条件长度不能超过200个字符"))]
    pub payment_terms: Option<String>,

    /// 运输条款
    #[validate(length(max = 200, message = "运输条款长度不能超过200个字符"))]
    pub shipping_terms: Option<String>,

    /// 备注
    #[validate(length(max = 1000, message = "备注长度不能超过1000个字符"))]
    pub notes: Option<String>,

    /// 附件 URL 列表
    pub attachment_urls: Option<Vec<String>>,

    /// 订单明细
    #[validate(length(min = 1, message = "订单至少需要一行明细"))]
    pub items: Option<Vec<CreateOrderItemRequest>>,
}

/// 更新采购订单请求
#[derive(Debug, Default, Deserialize)]
pub struct UpdatePurchaseOrderRequest {
    pub supplier_id: Option<i32>,
    pub order_date: Option<NaiveDate>,
    pub expected_delivery_date: Option<NaiveDate>,
    pub warehouse_id: Option<i32>,
    pub department_id: Option<i32>,
    pub currency: Option<String>,
    pub exchange_rate: Option<Decimal>,
    pub payment_terms: Option<String>,
    pub shipping_terms: Option<String>,
    pub notes: Option<String>,
    pub attachment_urls: Option<Vec<String>>,
}

/// 创建订单明细请求
#[derive(Debug, Clone, Validate, Deserialize, Serialize)]
pub struct CreateOrderItemRequest {
    /// 行号
    pub line_no: Option<i32>,

    /// 物料 ID
    pub material_id: Option<i32>,

    /// 物料编码
    #[validate(length(max = 50, message = "物料编码长度不能超过50个字符"))]
    pub material_code: Option<String>,

    /// 物料名称
    #[validate(length(max = 200, message = "物料名称长度不能超过200个字符"))]
    pub material_name: Option<String>,

    /// 规格型号
    #[validate(length(max = 500, message = "规格型号长度不能超过500个字符"))]
    pub specification: Option<String>,

    /// 批次号
    #[validate(length(max = 50, message = "批次号长度不能超过50个字符"))]
    pub batch_no: Option<String>,

    /// 色号
    #[validate(length(max = 50, message = "色号长度不能超过50个字符"))]
    pub color_code: Option<String>,

    /// 缸号
    #[validate(length(max = 50, message = "缸号长度不能超过50个字符"))]
    pub lot_no: Option<String>,

    /// 等级
    #[validate(length(max = 20, message = "等级长度不能超过20个字符"))]
    pub grade: Option<String>,

    /// 克重
    pub gram_weight: Option<Decimal>,

    /// 幅宽
    pub width: Option<Decimal>,

    /// 单价
    pub unit_price: Option<Decimal>,

    /// 币种
    #[validate(length(max = 10, message = "币种长度不能超过10个字符"))]
    pub currency: Option<String>,

    /// 订购数量（主单位）
    pub quantity_ordered: Option<Decimal>,

    /// 主单位
    #[validate(length(max = 20, message = "主单位长度不能超过20个字符"))]
    pub unit_master: Option<String>,

    /// 辅助单位
    #[validate(length(max = 20, message = "辅助单位长度不能超过20个字符"))]
    pub unit_alt: Option<String>,

    /// 换算系数
    pub conversion_factor: Option<Decimal>,

    /// 订购数量（辅助单位）
    pub quantity_alt_ordered: Option<Decimal>,

    /// 税率
    pub tax_rate: Option<Decimal>,

    /// 折扣百分比
    pub discount_percent: Option<Decimal>,

    /// 交货日期
    pub delivery_date: Option<NaiveDate>,

    /// 仓库 ID
    pub warehouse_id: Option<i32>,

    /// 备注
    pub notes: Option<String>,
}

/// 更新订单明细请求
#[derive(Debug, Default, Deserialize)]
pub struct UpdateOrderItemRequest {
    pub line_no: Option<i32>,
    pub material_id: Option<i32>,
    pub material_code: Option<String>,
    pub material_name: Option<String>,
    pub specification: Option<String>,
    pub batch_no: Option<String>,
    pub color_code: Option<String>,
    pub lot_no: Option<String>,
    pub grade: Option<String>,
    pub gram_weight: Option<Decimal>,
    pub width: Option<Decimal>,
    pub unit_price: Option<Decimal>,
    pub quantity_ordered: Option<Decimal>,
    pub tax_rate: Option<Decimal>,
    pub delivery_date: Option<NaiveDate>,
    pub notes: Option<String>,
}

// =====================================================
// 统一对外导出（兼容旧路径 + 子模块直接访问）
// =====================================================

#[allow(unused_imports)]
pub use order::{PurchaseOrderDto, PurchaseOrderItemDto, PurchaseOrderService};

/// 重新导出 Arc<DatabaseConnection> 给子模块共用类型
pub type DbConn = Arc<sea_orm::DatabaseConnection>;
