use serde::{Deserialize, Serialize};
use crate::services::api::ApiService;

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

pub struct SupplierEvaluationService;

impl SupplierEvaluationService {
    pub async fn list_evaluations(
        supplier_id: Option<i32>,
        evaluation_period: Option<&str>,
        grade: Option<&str>,
        status: Option<&str>,
        page: i64,
        page_size: i64,
    ) -> Result<Vec<SupplierEvaluation>, String> {
        let mut query = String::new();
        if let Some(sid) = supplier_id {
            query.push_str(&format!("supplier_id={}&", sid));
        }
        if let Some(ep) = evaluation_period {
            query.push_str(&format!("evaluation_period={}&", ep));
        }
        if let Some(g) = grade {
            query.push_str(&format!("grade={}&", g));
        }
        if let Some(s) = status {
            query.push_str(&format!("status={}&", s));
        }
        query.push_str(&format!("page={}&page_size={}", page, page_size));
        
        ApiService::get(&format!("/supplier-evaluation/evaluations?{}", query)).await
    }

    pub async fn get_evaluation(id: i32) -> Result<SupplierEvaluation, String> {
        ApiService::get(&format!("/supplier-evaluation/evaluations/{}", id)).await
    }

    pub async fn create_evaluation(req: CreateEvaluationRequest) -> Result<SupplierEvaluation, String> {
        let payload = serde_json::to_value(&req).map_err(|e| e.to_string())?;
        ApiService::post("/supplier-evaluation/evaluations", &payload).await
    }

    pub async fn update_evaluation(id: i32, req: UpdateEvaluationRequest) -> Result<SupplierEvaluation, String> {
        let payload = serde_json::to_value(&req).map_err(|e| e.to_string())?;
        ApiService::put(&format!("/supplier-evaluation/evaluations/{}", id), &payload).await
    }

    pub async fn delete_evaluation(id: i32) -> Result<(), String> {
        ApiService::delete(&format!("/supplier-evaluation/evaluations/{}", id)).await
    }

    pub async fn calculate_score(supplier_id: i32, period: &str) -> Result<SupplierEvaluation, String> {
        ApiService::post(
            &format!("/supplier-evaluation/suppliers/{}/calculate?period={}", supplier_id, period),
            &serde_json::json!({})
        ).await
    }

    pub async fn get_rankings(
        period: &str,
        grade: Option<&str>,
        limit: i32,
    ) -> Result<Vec<SupplierEvaluation>, String> {
        let query = if let Some(g) = grade {
            format!("?period={}&grade={}&limit={}", period, g, limit)
        } else {
            format!("?period={}&limit={}", period, limit)
        };
        ApiService::get(&format!("/supplier-evaluation/rankings{}", query)).await
    }

    pub async fn list_indicators(
        category: Option<&str>,
        status: Option<&str>,
        page: i64,
        page_size: i64,
    ) -> Result<Vec<SupplierIndicator>, String> {
        let mut query = String::new();
        if let Some(c) = category {
            query.push_str(&format!("category={}&", c));
        }
        if let Some(s) = status {
            query.push_str(&format!("status={}&", s));
        }
        query.push_str(&format!("page={}&page_size={}", page, page_size));
        
        ApiService::get(&format!("/supplier-evaluation/indicators?{}", query)).await
    }

    pub async fn get_indicator(id: i32) -> Result<SupplierIndicator, String> {
        ApiService::get(&format!("/supplier-evaluation/indicators/{}", id)).await
    }

    pub async fn create_indicator(req: CreateIndicatorRequest) -> Result<SupplierIndicator, String> {
        let payload = serde_json::to_value(&req).map_err(|e| e.to_string())?;
        ApiService::post("/supplier-evaluation/indicators", &payload).await
    }

    pub async fn update_indicator(id: i32, req: UpdateIndicatorRequest) -> Result<SupplierIndicator, String> {
        let payload = serde_json::to_value(&req).map_err(|e| e.to_string())?;
        ApiService::put(&format!("/supplier-evaluation/indicators/{}", id), &payload).await
    }

    pub async fn delete_indicator(id: i32) -> Result<(), String> {
        ApiService::delete(&format!("/supplier-evaluation/indicators/{}", id)).await
    }
}
