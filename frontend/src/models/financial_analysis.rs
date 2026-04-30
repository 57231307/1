//! 财务分析模型
//!
//! 财务分析相关的数据结构

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FinancialRatio {
    pub id: i32,
    pub indicator_code: String,
    pub indicator_name: String,
    pub indicator_value: String,
    pub industry_average: String,
    pub ratio_level: String,
    pub analysis_result: String,
    pub period: String,
    pub created_at: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DupontAnalysis {
    pub period: String,
    pub roe: String,
    pub net_profit_margin: String,
    pub asset_turnover: String,
    pub equity_multiplier: String,
    pub roa: String,
    pub analysis_summary: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FinancialIndicator {
    pub id: i32,
    pub indicator_code: String,
    pub indicator_name: String,
    pub indicator_type: String,
    pub formula: String,
    pub unit: String,
    pub benchmark_value: Option<String>,
    pub weight: Option<String>,
    pub status: String,
    pub created_at: Option<String>,
    pub updated_at: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AnalysisResult {
    pub id: i32,
    pub result_type: String,
    pub period: String,
    pub data: String,
    pub conclusion: String,
    pub recommendation: String,
    pub created_by: i32,
    pub created_at: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateIndicatorRequest {
    pub indicator_code: String,
    pub indicator_name: String,
    pub indicator_type: String,
    pub formula: String,
    pub unit: String,
    pub benchmark_value: Option<String>,
    pub weight: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpdateIndicatorRequest {
    pub indicator_name: Option<String>,
    pub benchmark_value: Option<String>,
    pub weight: Option<String>,
    pub status: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateAnalysisResultRequest {
    pub result_type: String,
    pub period: String,
    pub data: String,
    pub conclusion: String,
    pub recommendation: String,
}
