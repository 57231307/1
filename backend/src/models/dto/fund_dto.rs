use serde::{Deserialize, Serialize};
use rust_decimal::Decimal;

#[derive(Debug, Deserialize)]
pub struct TransferFundRequest {
    pub from_account_id: i32,
    pub to_account_id: i32,
    pub amount: Decimal,
    pub fee: Option<Decimal>,
    pub reason: Option<String>,
}
