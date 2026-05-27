use axum::{
    extract::{Query, State},
    http::StatusCode,
    Json,
};
use serde::{Deserialize, Serialize};

use crate::middleware::auth_context::AuthContext;
use crate::services::report_engine_service::{
    ReportEngineService, ReportTemplate, ExportFormat,
    DataSource, AggregationType, AggregateRequest, ReportFilter,
};
use crate::utils::app_state::AppState;
use crate::utils::response::ApiResponse;

#[derive(Debug, Serialize)]
pub struct ReportTemplateResponse {
    pub id: String,
    pub name: String,
    pub report_type: String,
    pub columns: Vec<ReportColumnResponse>,
}

#[derive(Debug, Serialize)]
pub struct ReportColumnResponse {
    pub field: String,
    pub title: String,
    pub data_type: String,
}

impl From<ReportTemplate> for ReportTemplateResponse {
    fn from(template: ReportTemplate) -> Self {
        Self {
            id: template.id,
            name: template.name,
            report_type: format!("{:?}", template.report_type),
            columns: template.columns.into_iter().map(|c| ReportColumnResponse {
                field: c.field,
                title: c.title,
                data_type: c.data_type,
            }).collect(),
        }
    }
}

pub async fn list_templates(
    _state: State<AppState>,
    _auth: AuthContext,
) -> Result<Json<ApiResponse<Vec<ReportTemplateResponse>>>, StatusCode> {
    let templates = ReportEngineService::get_predefined_templates();
    let responses: Vec<ReportTemplateResponse> = templates.into_iter().map(ReportTemplateResponse::from).collect();
    Ok(Json(ApiResponse::success(responses)))
}

#[derive(Debug, Deserialize)]
pub struct ExecuteReportQuery {
    pub template_id: String,
    pub page: Option<u64>,
    pub page_size: Option<u64>,
}

#[derive(Debug, Serialize)]
pub struct ReportDataResponse {
    pub columns: Vec<String>,
    pub rows: Vec<Vec<String>>,
    pub total_count: u64,
}

pub async fn execute_report(
    State(state): State<AppState>,
    _auth: AuthContext,
    Query(query): Query<ExecuteReportQuery>,
) -> Result<Json<ApiResponse<ReportDataResponse>>, StatusCode> {
    let service = ReportEngineService::new(state.db);
    let page = query.page.unwrap_or(1);
    let page_size = query.page_size.unwrap_or(50);

    match service.execute_report(&query.template_id, vec![], page, page_size).await {
        Ok(data) => {
            let response = ReportDataResponse {
                columns: data.columns,
                rows: data.rows,
                total_count: data.total_count,
            };
            Ok(Json(ApiResponse::success(response)))
        }
        Err(e) => {
            tracing::error!("执行报表失败: {}", e);
            Err(StatusCode::INTERNAL_SERVER_ERROR)
        }
    }
}

#[derive(Debug, Deserialize)]
pub struct ExportReportQuery {
    pub template_id: String,
    pub format: String,
}

#[derive(Debug, Serialize)]
pub struct ExportReportResponse {
    pub data: String,
    pub format: String,
    pub filename: String,
}

pub async fn export_report(
    State(state): State<AppState>,
    _auth: AuthContext,
    Query(query): Query<ExportReportQuery>,
) -> Result<Json<ApiResponse<ExportReportResponse>>, StatusCode> {
    let service = ReportEngineService::new(state.db);

    let export_format = match query.format.as_str() {
        "csv" => ExportFormat::CSV,
        "json" => ExportFormat::JSON,
        _ => ExportFormat::CSV,
    };

    // 先执行报表获取数据
    match service.execute_report(&query.template_id, vec![], 1, 1000).await {
        Ok(data) => {
            match service.export_report(&data, export_format) {
                Ok(bytes) => {
                    let data_str = String::from_utf8_lossy(&bytes).to_string();
                    let response = ExportReportResponse {
                        data: data_str,
                        format: query.format.clone(),
                        filename: format!("{}.{}", query.template_id, query.format),
                    };
                    Ok(Json(ApiResponse::success(response)))
                }
                Err(e) => {
                    tracing::error!("导出报表失败: {}", e);
                    Err(StatusCode::INTERNAL_SERVER_ERROR)
                }
            }
        }
        Err(e) => {
            tracing::error!("执行报表失败: {}", e);
            Err(StatusCode::INTERNAL_SERVER_ERROR)
        }
    }
}

#[derive(Debug, Deserialize)]
pub struct AggregateReportRequest {
    pub data_source: String,
    pub filters: Option<Vec<FilterRequest>>,
    pub group_by: Option<Vec<String>>,
    pub aggregation_type: String,
    pub aggregation_field: Option<String>,
    pub page: Option<u64>,
    pub page_size: Option<u64>,
}

#[derive(Debug, Deserialize)]
pub struct FilterRequest {
    pub field: String,
    pub operator: String,
    pub value: String,
}

#[derive(Debug, Serialize)]
pub struct AggregateReportResponse {
    pub columns: Vec<String>,
    pub rows: Vec<Vec<String>>,
    pub total_count: u64,
}

pub async fn aggregate_report(
    State(state): State<AppState>,
    _auth: AuthContext,
    Json(request): Json<AggregateReportRequest>,
) -> Result<Json<ApiResponse<AggregateReportResponse>>, StatusCode> {
    let service = ReportEngineService::new(state.db);

    let data_source = match request.data_source.as_str() {
        "sales" => DataSource::Sales,
        "purchase" => DataSource::Purchase,
        "inventory" => DataSource::Inventory,
        "finance" => DataSource::Finance,
        _ => {
            return Err(StatusCode::BAD_REQUEST);
        }
    };

    let aggregation_type = match request.aggregation_type.as_str() {
        "sum" => AggregationType::Sum,
        "count" => AggregationType::Count,
        "average" => AggregationType::Average,
        "min" => AggregationType::Min,
        "max" => AggregationType::Max,
        "group_by" => AggregationType::GroupBy,
        _ => {
            return Err(StatusCode::BAD_REQUEST);
        }
    };

    let filters = request.filters
        .unwrap_or_default()
        .into_iter()
        .map(|f| ReportFilter {
            field: f.field,
            operator: f.operator,
            value: f.value,
        })
        .collect();

    let aggregate_request = AggregateRequest {
        data_source,
        filters,
        group_by: request.group_by,
        aggregation_type,
        aggregation_field: request.aggregation_field,
    };

    let page = request.page.unwrap_or(1);
    let page_size = request.page_size.unwrap_or(50);

    match service.aggregate_data(aggregate_request, page, page_size).await {
        Ok(result) => {
            let response = AggregateReportResponse {
                columns: result.columns,
                rows: result.rows,
                total_count: result.total_count,
            };
            Ok(Json(ApiResponse::success(response)))
        }
        Err(e) => {
            tracing::error!("数据聚合失败: {}", e);
            Err(StatusCode::INTERNAL_SERVER_ERROR)
        }
    }
}

#[derive(Debug, Deserialize)]
pub struct ClearCacheRequest {
    pub data_source: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct ClearCacheResponse {
    pub success: bool,
    pub message: String,
}

pub async fn clear_report_cache(
    State(state): State<AppState>,
    _auth: AuthContext,
    Json(request): Json<ClearCacheRequest>,
) -> Result<Json<ApiResponse<ClearCacheResponse>>, StatusCode> {
    let service = ReportEngineService::new(state.db);

    if let Some(source) = request.data_source {
        let data_source = match source.as_str() {
            "sales" => DataSource::Sales,
            "purchase" => DataSource::Purchase,
            "inventory" => DataSource::Inventory,
            "finance" => DataSource::Finance,
            _ => {
                return Err(StatusCode::BAD_REQUEST);
            }
        };
        service.clear_cache_by_source(&data_source).await;
    } else {
        service.clear_all_cache().await;
    }

    let response = ClearCacheResponse {
        success: true,
        message: "缓存已清除".to_string(),
    };
    Ok(Json(ApiResponse::success(response)))
}
