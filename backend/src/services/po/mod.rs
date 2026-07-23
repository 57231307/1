//! 采购服务模块（po = purchase order）
//!
//! 由原 `services/purchase_order_service.rs`（1752 行）按业务子领域拆分而来。
//! 子模块：
//! - `order`    采购订单（核心 CRUD / 生命周期）
//! - `contract` 采购合同（审批工作流：提交、审批、拒绝）
//! - `receipt`  采购收货（含库存联动）
//! - `price`    采购价格（采购建议、预算占用）
//! - `purchase_return`采购退货（占位模块，待后续扩展）
//!
//! 兼容说明：原 `crate::services::po::order::*` 路径需要由上层
//! `services/mod.rs` 通过 `pub use super::po::*;` 重新导出以保持向后兼容。

use chrono::NaiveDate;
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use validator::Validate;

pub mod contract;
pub mod order;
pub mod order_ops;
pub mod price;
pub mod receipt;

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

    /// 单价
    pub unit_price: Option<Decimal>,

    /// 订购数量（主单位）
    pub quantity_ordered: Option<Decimal>,

    /// 订购数量（辅助单位）
    pub quantity_alt_ordered: Option<Decimal>,

    /// 税率
    pub tax_rate: Option<Decimal>,

    /// 折扣百分比
    pub discount_percent: Option<Decimal>,

    /// 备注
    pub notes: Option<String>,
}

/// 更新订单明细请求
#[derive(Debug, Default, Deserialize)]
pub struct UpdateOrderItemRequest {
    pub material_id: Option<i32>,
    pub unit_price: Option<Decimal>,
    pub quantity_ordered: Option<Decimal>,
    pub tax_rate: Option<Decimal>,
    pub notes: Option<String>,
}

// =====================================================
// 统一对外导出（兼容旧路径 + 子模块直接访问）
// =====================================================
// 批次 325 v10 复审修复：移除未使用的 pub use 重导出（外部通过 crate::services::po::order::PurchaseOrderService 引用）
