#![allow(dead_code)]
use rust_decimal::Decimal;
use serde::Deserialize;

#[derive(Debug, Deserialize)]
pub struct TransferFundRequest {
    pub from_account_id: i32,
    pub to_account_id: i32,
    pub amount: Decimal,
    pub fee: Option<Decimal>,
    pub reason: Option<String>,
    /// V15 P0-B05：大额调拨二次确认标记（§17.6-D1，amount>10 万须 confirm_large=true 放行，#[serde(default)] 默认 false 拒绝）
    #[serde(default)]
    pub confirm_large: bool,
}
