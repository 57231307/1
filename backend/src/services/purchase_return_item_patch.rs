use crate::models::{product, purchase_return_item};
use rust_decimal::Decimal;
use sea_orm::FromQueryResult;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateReturnItemRequest {
    pub line_no: i32,
    pub material_id: i32,
    pub quantity_ordered: Option<Decimal>,
    pub quantity_returned: Decimal,
    pub unit_price: Decimal,
    pub tax_rate: Option<Decimal>,
    pub discount_percent: Option<Decimal>,
    pub notes: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpdateReturnItemRequest {
    pub line_no: Option<i32>,
    pub material_id: Option<i32>,
    pub quantity_returned: Option<Decimal>,
    pub unit_price: Option<Decimal>,
    pub tax_rate: Option<Decimal>,
    pub discount_percent: Option<Decimal>,
    pub notes: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, FromQueryResult)]
pub struct PurchaseReturnItemDto {
    pub id: i32,
    pub return_id: i32,
    pub line_no: i32,
    pub material_id: i32,
    pub material_code: Option<String>,
    pub material_name: Option<String>,
    pub quantity_returned: Decimal,
    pub unit_price: Decimal,
    pub tax_rate: Decimal,
    pub discount_percent: Decimal,
    pub subtotal: Decimal,
    pub tax_amount: Decimal,
    pub discount_amount: Decimal,
    pub total_amount: Decimal,
    pub notes: Option<String>,
}
