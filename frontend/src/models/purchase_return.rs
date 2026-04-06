use rust_decimal::Decimal;
// 采购退货模型

use serde::{Deserialize, Serialize};

/// 采购退货单
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PurchaseReturn {
    pub id: i32,
    pub return_no: String,
    pub order_id: i32,
    pub order_no: Option<String>,
    pub supplier_id: i32,
    pub supplier_name: Option<String>,
    pub return_date: String,
    pub status: String,
    pub total_quantity: String,
    pub total_amount: String,
    pub warehouse_id: i32,
    pub warehouse_name: Option<String>,
    pub department_id: i32,
    pub department_name: Option<String>,
    pub reason: Option<String>,
    pub notes: Option<String>,
    pub created_at: String,
    pub updated_at: String,
}

/// 采购退货单明细
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PurchaseReturnItem {
    pub id: i32,
    pub return_id: i32,
    pub line_no: i32,
    pub material_id: i32,
    pub material_code: String,
    pub material_name: String,
    pub specification: Option<String>,
    pub batch_no: Option<String>,
    pub color_code: Option<String>,
    pub lot_no: Option<String>,
    pub grade: Option<String>,
    pub gram_weight: Option<String>,
    pub width: Option<String>,
    pub unit_price: String,
    pub quantity_ordered: String,
    pub quantity_returned: String,
    pub unit_master: String,
    pub unit_alt: Option<String>,
    pub conversion_factor: Option<String>,
    pub quantity_alt: Option<String>,
    pub tax_rate: Option<String>,
    pub discount_percent: Option<String>,
    pub warehouse_id: Option<i32>,
    pub notes: Option<String>,
}

/// 采购退货单查询参数
#[derive(Debug, Clone, Serialize)]
pub struct PurchaseReturnQuery {
    pub page: Option<u64>,
    pub page_size: Option<u64>,
    pub status: Option<String>,
    pub supplier_id: Option<i32>,
}

/// 创建采购退货单请求




/// 更新采购退货单请求
#[derive(Debug, Clone, Serialize)]
pub struct UpdatePurchaseReturnRequest {
    pub return_date: Option<String>,
    pub warehouse_id: Option<i32>,
    pub department_id: Option<i32>,
    pub reason: Option<String>,
    pub notes: Option<String>,
}

/// 更新退货明细请求
#[derive(Debug, Clone, Serialize)]
pub struct UpdateReturnItemRequest {
    pub batch_no: Option<String>,
    pub color_code: Option<String>,
    pub lot_no: Option<String>,
    pub grade: Option<String>,
    pub gram_weight: Option<String>,
    pub width: Option<String>,
    pub unit_price: Option<String>,
    pub quantity_returned: Option<String>,
    pub tax_rate: Option<String>,
    pub discount_percent: Option<String>,
    pub warehouse_id: Option<i32>,
    pub notes: Option<String>,
}

/// 拒绝退货单请求
#[derive(Debug, Clone, Serialize)]
pub struct RejectReturnRequest {
    pub reason: String,
}

/// 分页结果
#[derive(Debug, Clone, Deserialize)]
pub struct PaginatedReturns {
    pub items: Vec<PurchaseReturn>,
    pub total: i64,
    pub page: u64,
    pub page_size: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreatePurchaseReturnItemRequest {
    pub product_id: i32,
    pub quantity: Decimal,
    pub unit_price: Option<Decimal>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreatePurchaseReturnRequest {
    pub return_no: String,
    pub supplier_id: i32,
    pub order_id: Option<i32>,
    pub return_date: Option<String>,
    pub reason_type: String,
    pub reason_detail: Option<String>,
    pub remarks: Option<String>,
    pub items: Vec<CreatePurchaseReturnItemRequest>,
}
