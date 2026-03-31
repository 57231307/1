//! 采购收货模型

use serde::{Deserialize, Serialize};

/// 采购收货单
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PurchaseReceipt {
    pub id: i32,
    pub receipt_no: String,
    pub order_id: i32,
    pub order_no: Option<String>,
    pub supplier_id: i32,
    pub supplier_name: Option<String>,
    pub receipt_date: String,
    pub status: String,
    pub total_quantity: String,
    pub total_amount: String,
    pub warehouse_id: i32,
    pub warehouse_name: Option<String>,
    pub department_id: i32,
    pub department_name: Option<String>,
    pub inspector: Option<String>,
    pub inspection_date: Option<String>,
    pub notes: Option<String>,
    pub created_at: String,
    pub updated_at: String,
}

/// 采购收货单明细
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PurchaseReceiptItem {
    pub id: i32,
    pub receipt_id: i32,
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
    pub quantity_received: String,
    pub unit_master: String,
    pub unit_alt: Option<String>,
    pub conversion_factor: Option<String>,
    pub quantity_alt: Option<String>,
    pub tax_rate: Option<String>,
    pub discount_percent: Option<String>,
    pub warehouse_id: Option<i32>,
    pub notes: Option<String>,
}

/// 采购收货单查询参数
#[derive(Debug, Clone, Serialize)]
pub struct PurchaseReceiptQuery {
    pub page: Option<u64>,
    pub page_size: Option<u64>,
    pub status: Option<String>,
    pub supplier_id: Option<i32>,
    pub order_id: Option<i32>,
}

/// 创建采购收货单请求
#[derive(Debug, Clone, Serialize)]
pub struct CreatePurchaseReceiptRequest {
    pub order_id: i32,
    pub receipt_date: String,
    pub warehouse_id: i32,
    pub department_id: i32,
    pub inspector: Option<String>,
    pub notes: Option<String>,
}

/// 更新采购收货单请求
#[derive(Debug, Clone, Serialize)]
pub struct UpdatePurchaseReceiptRequest {
    pub receipt_date: Option<String>,
    pub warehouse_id: Option<i32>,
    pub department_id: Option<i32>,
    pub inspector: Option<String>,
    pub notes: Option<String>,
}

/// 创建收货明细请求
#[derive(Debug, Clone, Serialize)]
pub struct CreateReceiptItemRequest {
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
    pub quantity_received: String,
    pub unit_master: String,
    pub unit_alt: Option<String>,
    pub conversion_factor: Option<String>,
    pub quantity_alt: Option<String>,
    pub tax_rate: Option<String>,
    pub discount_percent: Option<String>,
    pub warehouse_id: Option<i32>,
    pub notes: Option<String>,
}

/// 更新收货明细请求
#[derive(Debug, Clone, Serialize)]
pub struct UpdateReceiptItemRequest {
    pub batch_no: Option<String>,
    pub color_code: Option<String>,
    pub lot_no: Option<String>,
    pub grade: Option<String>,
    pub gram_weight: Option<String>,
    pub width: Option<String>,
    pub unit_price: Option<String>,
    pub quantity_received: Option<String>,
    pub tax_rate: Option<String>,
    pub discount_percent: Option<String>,
    pub warehouse_id: Option<i32>,
    pub notes: Option<String>,
}

/// 分页结果
#[derive(Debug, Clone, Deserialize)]
pub struct PaginatedReceipts {
    pub items: Vec<PurchaseReceipt>,
    pub total: i64,
    pub page: u64,
    pub page_size: u64,
}
