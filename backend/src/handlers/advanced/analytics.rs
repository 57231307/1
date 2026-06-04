//! 报表分析 handler
//!
//! 提供报表模板查询、报表执行与导出能力。
//! 适配重构后的 `services::report` 模块 API。

use axum::{extract::State, response::IntoResponse, Json};
use chrono::Utc;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

use crate::middleware::auth_context::AuthContext;
use crate::services::report::{
    ExecuteReportRequest, ReportEngineService, ReportFilter,
};
use crate::utils::app_state::AppState;
use crate::utils::response::ApiResponse;

// ============================================================================
// 报表相关端点 - 使用真实报表引擎
// ============================================================================

/// 报表模板列表
pub async fn list_report_templates(
    State(state): State<AppState>,
) -> impl IntoResponse {
    let service = ReportEngineService::new(state.db);
    let templates = service.get_predefined_templates();

    let items: Vec<ReportTemplateDto> = templates
        .into_iter()
        .map(|t| ReportTemplateDto {
            template_name: t.name,
            template_code: t.id,
            category: t.category,
            description: format!(
                "包含 {} 个列: {}",
                t.columns.len(),
                t.columns
                    .iter()
                    .map(|c| c.label.as_str())
                    .collect::<Vec<_>>()
                    .join(", ")
            ),
            created_at: Utc::now().format("%Y-%m-%d").to_string(),
        })
        .collect();

    Json(ApiResponse::success(items))
}

#[derive(Debug, Serialize)]
struct ReportTemplateDto {
    pub template_name: String,
    pub template_code: String,
    pub category: String,
    pub description: String,
    pub created_at: String,
}

/// 执行报表
pub async fn execute_report(
    State(state): State<AppState>,
    _auth: AuthContext,
    Json(payload): Json<ReportExecuteRequest>,
) -> impl IntoResponse {
    let service = ReportEngineService::new(state.db);

    let req = ExecuteReportRequest {
        template_id: payload.template_code,
        filters: Vec::<ReportFilter>::new(),
        parameters: None,
        date_range: None,
        format: "json".to_string(),
        use_cache: Some(true),
    };

    match service.execute_report(req).await {
        Ok(report_data) => {
            let columns_json: Vec<serde_json::Value> = report_data
                .columns
                .iter()
                .map(|c| serde_json::json!({"key": c.key, "label": c.label}))
                .collect();

            // rows 是 Vec<serde_json::Value>，统一转为字符串映射
            let rows: Vec<HashMap<String, String>> = report_data
                .rows
                .into_iter()
                .map(|row| {
                    let mut map: HashMap<String, String> = HashMap::new();
                    if let serde_json::Value::Object(obj) = row {
                        for (k, v) in obj.into_iter() {
                            let s = match v {
                                serde_json::Value::String(s) => s,
                                other => other.to_string(),
                            };
                            map.insert(k, s);
                        }
                    }
                    map
                })
                .collect();

            Json(ApiResponse::success(serde_json::json!({
                "columns": columns_json,
                "data": rows,
                "total_count": report_data.total_rows,
            })))
        }
        Err(e) => {
            tracing::error!("执行报表失败: {}", e);
            Json(ApiResponse::error(format!("执行报表失败: {}", e)))
        }
    }
}

#[derive(Debug, Deserialize)]
pub struct ReportExecuteRequest {
    pub template_code: String,
}

/// 导出报表
pub async fn export_report(
    State(state): State<AppState>,
    _auth: AuthContext,
    Json(payload): Json<ReportExportRequest>,
) -> impl IntoResponse {
    let service = ReportEngineService::new(state.db.clone());

    let req = ExecuteReportRequest {
        template_id: payload.template_code.clone(),
        filters: Vec::<ReportFilter>::new(),
        parameters: None,
        date_range: None,
        format: payload.format.clone(),
        use_cache: Some(true),
    };

    match service.execute_report(req).await {
        Ok(report_data) => {
            let format_str = match payload.format.as_str() {
                "csv" => "csv",
                "excel" | "xlsx" => "excel",
                "pdf" => "pdf",
                _ => "json",
            };

            match service
                .export_report(&report_data, format_str, &payload.template_code)
                .await
            {
                Ok(bytes) => {
                    let size_kb = bytes.len() / 1024;
                    Json(ApiResponse::success(serde_json::json!({
                        "status": "success",
                        "format": payload.format,
                        "size_bytes": bytes.len(),
                        "size_kb": size_kb,
                        "record_count": report_data.total_rows,
                    })))
                }
                Err(e) => {
                    tracing::error!("导出报表失败: {}", e);
                    Json(ApiResponse::error(format!("导出报表失败: {}", e)))
                }
            }
        }
        Err(e) => {
            tracing::error!("执行报表失败: {}", e);
            Json(ApiResponse::error(format!("执行报表失败: {}", e)))
        }
    }
}

#[derive(Debug, Deserialize)]
pub struct ReportExportRequest {
    pub template_code: String,
    pub format: String,
}

// ============================================================================
// 销售分析
// ============================================================================

#[derive(Debug, Serialize)]
pub struct SalesAnalyticsResponse {
    pub total_sales: f64,
    pub order_count: u64,
    pub avg_order_value: f64,
    pub top_products: Vec<TopProduct>,
}

#[derive(Debug, Serialize)]
pub struct TopProduct {
    pub product_id: i32,
    pub product_name: String,
    pub total_sales: f64,
    pub quantity: f64,
}

/// 销售汇总分析
pub async fn sales_analytics(
    State(state): State<AppState>,
    _auth: AuthContext,
    Json(payload): Json<AnalyticsRequest>,
) -> impl IntoResponse {
    use sea_orm::{EntityTrait, QueryFilter, ColumnTrait, PaginatorTrait};
    use crate::models::sales_order;

    let query = sales_order::Entity::find()
        .filter(sales_order::Column::OrderDate.between(payload.start_date, payload.end_date));

    let page_size = payload.page_size.unwrap_or(20).max(0) as u64;
    let paginator = query.paginate(&*state.db, page_size);
    let total = paginator.num_items().await.unwrap_or(0);

    let total_sales = total as f64 * 1000.0; // 示例数据
    let order_count = total;
    let avg_order_value = if order_count > 0 { total_sales / order_count as f64 } else { 0.0 };

    Json(ApiResponse::success(SalesAnalyticsResponse {
        total_sales,
        order_count,
        avg_order_value,
        top_products: vec![],
    }))
}

#[derive(Debug, Deserialize)]
pub struct AnalyticsRequest {
    pub start_date: chrono::NaiveDate,
    pub end_date: chrono::NaiveDate,
    pub page_size: Option<i32>,
}

#[allow(dead_code)]
fn _ensure_date_range_used() -> Option<()> {
    None
}
