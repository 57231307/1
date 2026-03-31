use serde::{Deserialize, Serialize};
use crate::services::api::ApiService;

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

pub struct FinancialAnalysisService;

impl FinancialAnalysisService {
    pub async fn analyze_ratios(period: &str) -> Result<Vec<FinancialRatio>, String> {
        ApiService::get(&format!("/financial-analysis/ratios?period={}", period)).await
    }

    pub async fn dupont_analysis(period: &str) -> Result<DupontAnalysis, String> {
        ApiService::get(&format!("/financial-analysis/dupont?period={}", period)).await
    }

    pub async fn list_indicators(
        indicator_type: Option<&str>,
        status: Option<&str>,
        page: i64,
        page_size: i64,
    ) -> Result<Vec<FinancialIndicator>, String> {
        let mut query = String::new();
        if let Some(it) = indicator_type {
            query.push_str(&format!("indicator_type={}&", it));
        }
        if let Some(s) = status {
            query.push_str(&format!("status={}&", s));
        }
        query.push_str(&format!("page={}&page_size={}", page, page_size));
        
        ApiService::get(&format!("/financial-analysis/indicators?{}", query)).await
    }

    pub async fn get_indicator(id: i32) -> Result<FinancialIndicator, String> {
        ApiService::get(&format!("/financial-analysis/indicators/{}", id)).await
    }

    pub async fn create_indicator(req: CreateIndicatorRequest) -> Result<FinancialIndicator, String> {
        let payload = serde_json::to_value(&req).map_err(|e| e.to_string())?;
        ApiService::post("/financial-analysis/indicators", &payload).await
    }

    pub async fn update_indicator(id: i32, req: UpdateIndicatorRequest) -> Result<FinancialIndicator, String> {
        let payload = serde_json::to_value(&req).map_err(|e| e.to_string())?;
        ApiService::put(&format!("/financial-analysis/indicators/{}", id), &payload).await
    }

    pub async fn delete_indicator(id: i32) -> Result<(), String> {
        ApiService::delete(&format!("/financial-analysis/indicators/{}", id)).await
    }

    pub async fn get_indicator_trends(
        indicator_code: &str,
        periods: i32,
    ) -> Result<Vec<FinancialRatio>, String> {
        ApiService::get(&format!(
            "/financial-analysis/indicators/{}/trends?periods={}",
            indicator_code, periods
        )).await
    }

    pub async fn list_analysis_results(
        result_type: Option<&str>,
        period: Option<&str>,
        page: i64,
        page_size: i64,
    ) -> Result<Vec<AnalysisResult>, String> {
        let mut query = String::new();
        if let Some(rt) = result_type {
            query.push_str(&format!("result_type={}&", rt));
        }
        if let Some(p) = period {
            query.push_str(&format!("period={}&", p));
        }
        query.push_str(&format!("page={}&page_size={}", page, page_size));
        
        ApiService::get(&format!("/financial-analysis/results?{}", query)).await
    }

    pub async fn get_analysis_result(id: i32) -> Result<AnalysisResult, String> {
        ApiService::get(&format!("/financial-analysis/results/{}", id)).await
    }

    pub async fn create_analysis_result(req: CreateAnalysisResultRequest) -> Result<AnalysisResult, String> {
        let payload = serde_json::to_value(&req).map_err(|e| e.to_string())?;
        ApiService::post("/financial-analysis/results", &payload).await
    }
}
