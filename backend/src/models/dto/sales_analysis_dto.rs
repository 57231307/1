use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Default, Serialize)]
pub struct SalesOverviewStats {
    pub month_orders: i64,
    pub month_amount: Decimal,
    pub gross_profit_rate: Decimal,
    pub active_customers: i64,
    pub order_trend: f64,
    pub amount_trend: f64,
    pub profit_trend: f64,
    pub customer_trend: f64,
}

#[derive(Debug, Clone, Serialize)]
pub struct ProductRankingItem {
    pub product_name: String,
    pub amount: Decimal,
    pub quantity: Decimal,
    pub percentage: Decimal,
}

#[derive(Debug, Clone, Serialize)]
pub struct CustomerRankingItem {
    pub customer_name: String,
    pub amount: Decimal,
    pub order_count: i32,
    pub percentage: Decimal,
}

#[derive(Debug, Clone, Deserialize)]
pub struct ProductRankingParams {
    #[serde(rename = "type")]
    // v11 批次 152 P2-A：接入 dimension_type 字段，指定产品排名的维度
    // - None 或 "product"：按产品维度排名（默认）
    // - 其他值（如 "product_category"）：按指定维度排名，需数据库有对应 dimension_type 记录
    pub dimension_type: Option<String>,
    pub period: Option<String>,
    pub limit: Option<i64>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct CustomerRankingParams {
    #[serde(rename = "type")]
    // v11 批次 152 P2-A：接入 dimension_type 字段，指定客户排名的维度
    // - None 或 "customer"：按客户维度排名（默认）
    // - 其他值（如 "customer_industry"）：按指定维度排名，需数据库有对应 dimension_type 记录
    pub dimension_type: Option<String>,
    pub period: Option<String>,
    pub limit: Option<i64>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct UpdateSalesTargetRequest {
    pub target_amount: Option<Decimal>,
    pub status: Option<String>,
    pub remarks: Option<String>,
}

#[derive(Debug, Clone, Default, Serialize)]
pub struct SalesTargetDto {
    pub id: i32,
    pub period: String,
    pub target_amount: Decimal,
    pub actual_amount: Decimal,
    pub completion_rate: Decimal,
    pub variance: Decimal,
    pub status: String,
}

#[derive(Debug, Clone, Default, Deserialize)]
pub struct ExportParams {
    pub period: Option<String>,
    // v11 批次 151 P2-A：接入 format 字段，指定导出格式
    // - None 或 "xlsx"：xlsx 格式（默认，规则 3 合规）
    // - "csv"：拒绝（规则 3 禁止 CSV 作为最终交付格式）
    // - 其他值：validation 错误
    pub format: Option<String>,
}

#[derive(Debug, Clone, Default)]
pub struct SalesStatisticQueryParams {
    pub statistic_type: Option<String>,
    pub period: Option<String>,
    pub page: i64,
    pub page_size: i64,
}

#[derive(Debug, Clone, Deserialize)]
pub struct CreateSalesTargetInput {
    pub target_type: String,
    pub target_id: i32,
    pub period: String,
    pub target_amount: Decimal,
    pub start_date: String,
    pub end_date: String,
}
