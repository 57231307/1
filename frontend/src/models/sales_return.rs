#![allow(dead_code, unused_variables, unused_imports, unused_mut)]
use crate::models::api_response::PaginatedResponse;
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SalesReturn {
    pub id: i32,
    pub return_no: String,
    pub sales_order_id: Option<i32>,
    pub customer_id: i32,
    pub return_date: String,
    pub warehouse_id: i32,
    pub reason: String,
    pub status: String,
    pub total_amount: Decimal,
    pub remarks: Option<String>,
    pub approved_by: Option<i32>,
    pub approved_at: Option<String>,
    pub rejected_reason: Option<String>,
    pub created_by: i32,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SalesReturnItem {
    pub id: i32,
    pub return_id: i32,
    pub line_no: i32,
    pub product_id: i32,
    pub quantity: Decimal,
    pub quantity_alt: Decimal,
    pub unit_price: Decimal,
    pub unit_price_foreign: Decimal,
    pub discount_percent: Decimal,
    pub tax_percent: Decimal,
    pub subtotal: Decimal,
    pub tax_amount: Decimal,
    pub discount_amount: Decimal,
    pub total_amount: Decimal,
    pub notes: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct SalesReturnQuery {
    pub return_no: Option<String>,
    pub status: Option<String>,
    pub customer_id: Option<i32>,
    pub page: Option<u64>,
    pub page_size: Option<u64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateSalesReturnItemRequest {
    pub product_id: i32,
    pub quantity: Decimal,
    pub unit_price: Option<Decimal>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateSalesReturnRequest {
    pub return_no: String,
    pub sales_order_id: Option<i32>,
    pub customer_id: i32,
    pub return_date: Option<String>,
    pub warehouse_id: i32,
    pub reason: String,
    pub remarks: Option<String>,
    pub items: Vec<CreateSalesReturnItemRequest>,
}
