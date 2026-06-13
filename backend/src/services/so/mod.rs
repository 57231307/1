//! 销售服务模块（so = sales order）
//!
//! 由原 `services/sales_service.rs`（1661 行）按业务子领域拆分而来。
//! 子模块：
//! - `order`    销售订单（核心 CRUD / 生命周期）
//! - `delivery` 销售发货（发货、库存扣减/释放、订单号生成）
//! - `price`    销售价格（占位模块，待后续扩展）
//! - `contract` 销售合同（占位模块，待后续扩展）
//! - `sales_return` 销售退货（占位模块，待后续扩展）
//!
//! 兼容说明：原 `crate::services::so::order::*` 路径需要由上层
//! `services/mod.rs` 通过 `pub use super::so::*;` 重新导出以保持向后兼容。

use sea_orm::FromQueryResult;
use serde::{Deserialize, Serialize};
use validator::Validate;

pub mod contract;
pub mod delivery;
pub mod order;
pub mod price;
pub mod sales_return;

// =====================================================
// DTO 数据结构
// =====================================================

/// 销售订单详情响应
#[derive(Debug, Serialize, Deserialize, Clone, FromQueryResult)]
pub struct SalesOrderDetail {
    pub id: i32,
    pub order_no: String,
    pub customer_id: i32,
    pub customer_name: Option<String>,
    pub opportunity_id: Option<i32>,
    pub order_date: chrono::DateTime<chrono::Utc>,
    pub required_date: chrono::DateTime<chrono::Utc>,
    pub ship_date: Option<chrono::DateTime<chrono::Utc>>,
    pub status: String,
    pub subtotal: rust_decimal::Decimal,
    pub tax_amount: rust_decimal::Decimal,
    pub discount_amount: rust_decimal::Decimal,
    pub shipping_cost: rust_decimal::Decimal,
    pub total_amount: rust_decimal::Decimal,
    pub paid_amount: rust_decimal::Decimal,
    pub balance_amount: rust_decimal::Decimal,
    pub shipping_address: Option<String>,
    pub billing_address: Option<String>,
    pub notes: Option<String>,
    pub created_by: Option<i32>,
    pub approved_by: Option<i32>,
    pub approved_at: Option<chrono::DateTime<chrono::Utc>>,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub updated_at: chrono::DateTime<chrono::Utc>,
    #[sea_orm(skip)]
    pub items: Vec<SalesOrderItemDetail>,
}

/// 销售订单明细项详情
#[derive(Debug, Serialize, Deserialize, Clone, FromQueryResult)]
pub struct SalesOrderItemDetail {
    pub id: i32,
    pub order_id: i32,
    pub product_id: i32,
    pub product_code: Option<String>,
    pub product_name: Option<String>,
    pub quantity: rust_decimal::Decimal,
    pub unit_price: rust_decimal::Decimal,
    pub discount_percent: rust_decimal::Decimal,
    pub tax_percent: rust_decimal::Decimal,
    pub subtotal: rust_decimal::Decimal,
    pub tax_amount: rust_decimal::Decimal,
    pub discount_amount: rust_decimal::Decimal,
    pub total_amount: rust_decimal::Decimal,
    pub shipped_quantity: rust_decimal::Decimal,
    pub notes: Option<String>,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub updated_at: chrono::DateTime<chrono::Utc>,
    pub color_no: String,
    pub color_name: Option<String>,
    pub pantone_code: Option<String>,
    pub grade_required: Option<String>,
    pub quantity_meters: rust_decimal::Decimal,
    pub quantity_kg: rust_decimal::Decimal,
    pub gram_weight: Option<rust_decimal::Decimal>,
    pub width: Option<rust_decimal::Decimal>,
    pub paper_tube_weight: Option<rust_decimal::Decimal>,
    pub is_net_weight: Option<bool>,
    pub batch_requirement: Option<String>,
    pub dye_lot_requirement: Option<String>,
    pub base_price: Option<rust_decimal::Decimal>,
    pub color_extra_cost: rust_decimal::Decimal,
    pub grade_price_diff: rust_decimal::Decimal,
    pub final_price: Option<rust_decimal::Decimal>,
    pub shipped_quantity_meters: rust_decimal::Decimal,
    pub shipped_quantity_kg: rust_decimal::Decimal,
}

/// 创建销售订单请求
#[derive(Debug, Deserialize, Validate)]
pub struct CreateSalesOrderRequest {
    #[validate(range(min = 1, message = "客户ID必须大于0"))]
    pub customer_id: i32,
    pub opportunity_id: Option<i32>,
    pub required_date: Option<chrono::DateTime<chrono::Utc>>,
    #[validate(length(max = 50, message = "状态长度不能超过50个字符"))]
    pub status: Option<String>,
    #[validate(length(max = 500, message = "收货地址长度不能超过500个字符"))]
    pub shipping_address: Option<String>,
    #[validate(length(max = 500, message = "账单地址长度不能超过500个字符"))]
    pub billing_address: Option<String>,
    #[validate(length(max = 1000, message = "备注长度不能超过1000个字符"))]
    pub notes: Option<String>,
    #[validate(length(min = 1, message = "订单项不能为空"))]
    pub items: Vec<SalesOrderItemRequest>,
    #[validate(length(max = 100, message = "付款条件长度不能超过100个字符"))]
    pub payment_terms: Option<String>,
    #[validate(length(max = 500, message = "备注长度不能超过500个字符"))]
    pub remarks: Option<String>,
    #[validate(length(max = 50, message = "批次号长度不能超过50个字符"))]
    pub batch_no: Option<String>,
    #[validate(length(max = 50, message = "色号长度不能超过50个字符"))]
    pub color_no: Option<String>,
    #[validate(length(max = 50, message = "染缸号长度不能超过50个字符"))]
    pub dye_lot_no: Option<String>,
    #[validate(length(max = 20, message = "等级长度不能超过20个字符"))]
    pub grade: Option<String>,
    #[validate(length(max = 200, message = "包装要求长度不能超过200个字符"))]
    pub packaging_requirement: Option<String>,
    #[validate(length(max = 200, message = "质量标准长度不能超过200个字符"))]
    pub quality_standard: Option<String>,
}

#[derive(Debug, Deserialize, Serialize, Validate)]
pub struct SalesOrderItemRequest {
    #[validate(range(min = 1, message = "产品ID必须大于0"))]
    pub product_id: i32,
    pub quantity: rust_decimal::Decimal,
    pub unit_price: rust_decimal::Decimal,
    pub discount_percent: Option<rust_decimal::Decimal>,
    pub tax_percent: Option<rust_decimal::Decimal>,
    #[validate(length(max = 500, message = "备注长度不能超过500个字符"))]
    pub notes: Option<String>,
    #[validate(length(max = 50, message = "色号长度不能超过50个字符"))]
    pub color_no: Option<String>,
    #[validate(length(max = 50, message = "颜色名称长度不能超过50个字符"))]
    pub color_name: Option<String>,
    #[validate(length(max = 50, message = "潘通色号长度不能超过50个字符"))]
    pub pantone_code: Option<String>,
    #[validate(length(max = 20, message = "要求等级长度不能超过20个字符"))]
    pub grade_required: Option<String>,
    pub quantity_meters: Option<rust_decimal::Decimal>,
    pub quantity_kg: Option<rust_decimal::Decimal>,
    pub gram_weight: Option<rust_decimal::Decimal>,
    pub width: Option<rust_decimal::Decimal>,
    pub paper_tube_weight: Option<rust_decimal::Decimal>,
    pub is_net_weight: Option<bool>,
    #[validate(length(max = 100, message = "批次要求长度不能超过100个字符"))]
    pub batch_requirement: Option<String>,
    #[validate(length(max = 100, message = "染批要求长度不能超过100个字符"))]
    pub dye_lot_requirement: Option<String>,
    pub base_price: Option<rust_decimal::Decimal>,
    pub color_extra_cost: Option<rust_decimal::Decimal>,
    pub grade_price_diff: Option<rust_decimal::Decimal>,
    pub final_price: Option<rust_decimal::Decimal>,
}

/// 更新销售订单请求
#[derive(Debug, Deserialize)]
pub struct UpdateSalesOrderRequest {
    pub required_date: Option<chrono::DateTime<chrono::Utc>>,
    pub status: Option<String>,
    pub shipping_address: Option<String>,
    pub billing_address: Option<String>,
    pub notes: Option<String>,
    pub items: Option<Vec<SalesOrderItemRequest>>,
}

// =====================================================
// 统一对外导出
// =====================================================

#[allow(unused_imports)]
pub use delivery::ShipOrderRequest;
#[allow(unused_imports)]
pub use order::SalesService;
