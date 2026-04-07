use chrono::NaiveDate;
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use validator::Validate;

#[derive(Debug, Deserialize, Validate)]
pub struct CreateSalesDeliveryRequest {
    pub order_id: i32,
    pub customer_id: i32,
    pub delivery_date: NaiveDate,
    pub warehouse_id: i32,
    pub status: String,
    pub total_quantity: Decimal,
    pub total_amount: Decimal,
    pub remarks: Option<String>,
    #[validate(length(min = 1, message = "交货明细不能为空"))]
    pub items: Vec<CreateSalesDeliveryItemRequest>,
}

#[derive(Debug, Deserialize, Serialize, Validate)]
pub struct CreateSalesDeliveryItemRequest {
    pub product_id: i32,
    pub batch_no: Option<String>,
    pub color_no: Option<String>,
    pub quantity: Decimal,
    pub unit_price: Decimal,
    pub amount: Decimal,
    pub remarks: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct SalesDeliveryQueryParams {
    pub delivery_no: Option<String>,
    pub order_id: Option<i32>,
    pub customer_id: Option<i32>,
    pub warehouse_id: Option<i32>,
    pub status: Option<String>,
    pub start_date: Option<NaiveDate>,
    pub end_date: Option<NaiveDate>,
    pub page: Option<u64>,
    pub page_size: Option<u64>,
}
