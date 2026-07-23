//! 委外加工 DTO 子模块（outsourcing_ops/types）
//!
//! 批次 489 D10-2b 拆分：从原 `outsourcing_service.rs` 迁移 10 个 DTO struct。
//! 包含委外订单/发料明细/收回入库单/凭证的 Create/Update/Query 请求体。

use rust_decimal::Decimal;
use serde::Deserialize;

// ============================================================================
// 委外加工订单 DTO
// ============================================================================

/// 创建委外订单请求
#[derive(Debug, Clone, Deserialize)]
pub struct CreateOutsourcingOrderRequest {
    pub order_no: String,
    pub order_type: String,
    pub supplier_id: i32,
    pub production_order_id: Option<i32>,
    pub dye_batch_id: Option<i32>,
    pub color_no: Option<String>,
    pub dye_lot_no: Option<String>,
    pub issue_date: chrono::NaiveDate,
    pub expected_return_date: Option<chrono::NaiveDate>,
    pub issue_quantity: Decimal,
    pub issue_unit: Option<String>,
    pub material_cost: Decimal,
    pub standard_loss_rate: Option<Decimal>,
    pub remarks: Option<String>,
    pub created_by: Option<i32>,
}

/// 更新委外订单请求（仅 draft 状态可更新）
#[derive(Debug, Clone, Deserialize)]
pub struct UpdateOutsourcingOrderRequest {
    pub order_type: Option<String>,
    pub supplier_id: Option<i32>,
    pub production_order_id: Option<i32>,
    pub dye_batch_id: Option<i32>,
    pub color_no: Option<String>,
    pub dye_lot_no: Option<String>,
    pub issue_date: Option<chrono::NaiveDate>,
    pub expected_return_date: Option<chrono::NaiveDate>,
    pub issue_quantity: Option<Decimal>,
    pub issue_unit: Option<String>,
    pub material_cost: Option<Decimal>,
    pub standard_loss_rate: Option<Decimal>,
    pub remarks: Option<String>,
}

/// 委外订单查询参数
#[derive(Debug, Clone, Deserialize)]
pub struct OutsourcingOrderQuery {
    pub order_type: Option<String>,
    pub supplier_id: Option<i32>,
    pub production_order_id: Option<i32>,
    pub dye_batch_id: Option<i32>,
    pub dye_lot_no: Option<String>,
    pub status: Option<String>,
    pub issue_date_from: Option<chrono::NaiveDate>,
    pub issue_date_to: Option<chrono::NaiveDate>,
    pub keyword: Option<String>,
    pub page: Option<u64>,
    pub page_size: Option<u64>,
}

// ============================================================================
// 委外加工发料明细 DTO
// ============================================================================

/// 创建委外发料明细请求
#[derive(Debug, Clone, Deserialize)]
pub struct CreateOutsourcingOrderItemRequest {
    pub outsourcing_order_id: i32,
    pub product_id: i32,
    pub color_no: Option<String>,
    pub dye_lot_no: Option<String>,
    pub batch_no: Option<String>,
    pub warehouse_id: Option<i32>,
    pub quantity: Decimal,
    pub unit: Option<String>,
    pub unit_cost: Decimal,
    pub remarks: Option<String>,
}

/// 更新委外发料明细请求
#[derive(Debug, Clone, Deserialize)]
pub struct UpdateOutsourcingOrderItemRequest {
    pub color_no: Option<String>,
    pub dye_lot_no: Option<String>,
    pub batch_no: Option<String>,
    pub warehouse_id: Option<i32>,
    pub quantity: Option<Decimal>,
    pub unit: Option<String>,
    pub unit_cost: Option<Decimal>,
    pub remarks: Option<String>,
}

// ============================================================================
// 委外收回入库单 DTO
// ============================================================================

/// 创建委外收回入库单请求
#[derive(Debug, Clone, Deserialize)]
pub struct CreateOutsourcingReceiptRequest {
    pub receipt_no: String,
    pub outsourcing_order_id: i32,
    pub receipt_date: chrono::NaiveDate,
    pub product_id: i32,
    pub color_no: Option<String>,
    pub dye_lot_no: Option<String>,
    pub batch_no: Option<String>,
    pub warehouse_id: Option<i32>,
    pub return_quantity: Decimal,
    pub loss_quantity: Option<Decimal>,
    pub quality_status: Option<String>,
    pub grade: Option<String>,
    pub remarks: Option<String>,
}

/// 更新委外收回入库单请求（仅 draft 状态可更新）
#[derive(Debug, Clone, Deserialize)]
pub struct UpdateOutsourcingReceiptRequest {
    pub receipt_date: Option<chrono::NaiveDate>,
    pub product_id: Option<i32>,
    pub color_no: Option<String>,
    pub dye_lot_no: Option<String>,
    pub batch_no: Option<String>,
    pub warehouse_id: Option<i32>,
    pub return_quantity: Option<Decimal>,
    pub loss_quantity: Option<Decimal>,
    pub quality_status: Option<String>,
    pub grade: Option<String>,
    pub remarks: Option<String>,
}

/// 委外收回入库单查询参数
#[derive(Debug, Clone, Deserialize)]
pub struct OutsourcingReceiptQuery {
    pub outsourcing_order_id: Option<i32>,
    pub product_id: Option<i32>,
    pub dye_lot_no: Option<String>,
    pub status: Option<String>,
    pub receipt_date_from: Option<chrono::NaiveDate>,
    pub receipt_date_to: Option<chrono::NaiveDate>,
    pub page: Option<u64>,
    pub page_size: Option<u64>,
}

// ============================================================================
// 委外加工会计分录凭证 DTO
// ============================================================================

/// 创建委外凭证请求
#[derive(Debug, Clone, Deserialize)]
pub struct CreateOutsourcingVoucherRequest {
    pub voucher_no: String,
    pub outsourcing_order_id: i32,
    pub voucher_type: String,
    pub debit_account: String,
    pub credit_account: String,
    pub amount: Decimal,
    pub tax_amount: Option<Decimal>,
    pub voucher_date: chrono::NaiveDate,
    pub remarks: Option<String>,
    pub created_by: Option<i32>,
}

/// 委外凭证查询参数
#[derive(Debug, Clone, Deserialize)]
pub struct OutsourcingVoucherQuery {
    pub outsourcing_order_id: Option<i32>,
    pub voucher_type: Option<String>,
    pub is_posted: Option<bool>,
    pub voucher_date_from: Option<chrono::NaiveDate>,
    pub voucher_date_to: Option<chrono::NaiveDate>,
    pub page: Option<u64>,
    pub page_size: Option<u64>,
}
