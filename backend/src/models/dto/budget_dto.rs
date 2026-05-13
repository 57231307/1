use serde::Deserialize;
use rust_decimal::Decimal;

#[derive(Debug, Deserialize)]
pub struct AdjustBudgetRequest {
    pub item_id: i32,
    pub adjust_amount: Decimal,
    pub reason: Option<String>,
}
