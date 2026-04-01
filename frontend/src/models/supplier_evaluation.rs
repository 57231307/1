//! 供应商评估模型
//!
//! 供应商评估相关的数据结构

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SupplierEvaluation {
    pub id: i32,
    pub supplier_id: i32,
    pub supplier_name: Option<String>,
    pub evaluation_period: String,
    pub quality_score: String,
    pub delivery_score: String,
    pub price_score: String,
    pub service_score: String,
    pub technology_score: String,
    pub overall_score: String,
    pub grade: String,
    pub rank: Option<i32>,
    pub evaluation_date: String,
    pub evaluator_id: i32,
    pub evaluator_name: Option<String>,
    pub status: String,
    pub remark: Option<String>,
    pub created_at: Option<String>,
    pub updated_at: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SupplierIndicator {
    pub id: i32,
    pub indicator_code: String,
    pub indicator_name: String,
    pub category: String,
    pub weight: String,
    pub formula: String,
    pub unit: String,
    pub benchmark_value: Option<String>,
    pub status: String,
    pub created_at: Option<String>,
    pub updated_at: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateEvaluationRequest {
    pub supplier_id: i32,
    pub evaluation_period: String,
    pub quality_score: String,
    pub delivery_score: String,
    pub price_score: String,
    pub service_score: String,
    pub technology_score: String,
    pub evaluator_id: i32,
    pub remark: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpdateEvaluationRequest {
    pub quality_score: Option<String>,
    pub delivery_score: Option<String>,
    pub price_score: Option<String>,
    pub service_score: Option<String>,
    pub technology_score: Option<String>,
    pub status: Option<String>,
    pub remark: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateIndicatorRequest {
    pub indicator_code: String,
    pub indicator_name: String,
    pub category: String,
    pub weight: String,
    pub formula: String,
    pub unit: String,
    pub benchmark_value: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpdateIndicatorRequest {
    pub indicator_name: Option<String>,
    pub weight: Option<String>,
    pub benchmark_value: Option<String>,
    pub status: Option<String>,
}
