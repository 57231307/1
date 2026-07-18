use rust_decimal::Decimal;
use serde::Deserialize;

#[derive(Debug, Deserialize)]
pub struct TransferFundRequest {
    pub from_account_id: i32,
    pub to_account_id: i32,
    pub amount: Decimal,
    pub fee: Option<Decimal>,
    pub reason: Option<String>,
    /// V15 P0-B05：大额调拨二次确认标记（§17.6-D1）
    ///
    /// 当 `amount > large_transfer_threshold()`（10 万）时，必须由前端
    /// 显式传 `confirm_large=true` 才能放行；缺省为 `false` 拦截大额调拨。
    ///
    /// `#[serde(default)]` 保证旧客户端未传该字段时按 `false` 处理，
    /// 即对大额调拨采取"默认拒绝"策略，强制前端升级以接入二次确认。
    #[serde(default)]
    pub confirm_large: bool,
}
