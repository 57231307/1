use axum::{
    extract::{Query, State},
    Json,
};
use serde::{Deserialize, Serialize};

use crate::middleware::auth_context::AuthContext;
use crate::services::report::{
    AggregateRequest, AggregationType, DataSource, ExportFormat, ReportEngineService, ReportFilter,
};
use crate::utils::app_state::AppState;
use crate::utils::error::AppError;
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

impl From<crate::services::report::ReportTemplate> for ReportTemplateResponse {
    fn from(template: crate::services::report::ReportTemplate) -> Self {
        Self {
            id: template.id,
            name: template.name,
            report_type: template.report_type,
            columns: template
                .columns
                .into_iter()
                .map(|c| ReportColumnResponse {
                    field: c.key,
                    title: c.label,
                    data_type: c.data_type,
                })
                .collect(),
        }
    }
}

pub async fn list_templates(
    _state: State<AppState>,
    _auth: AuthContext,
) -> Result<Json<ApiResponse<Vec<ReportTemplateResponse>>>, AppError> {
    let service = ReportEngineService::new(_state.db.clone());
    let templates = service.get_predefined_templates();
    let responses: Vec<ReportTemplateResponse> = templates
        .into_iter()
        .map(ReportTemplateResponse::from)
        .collect();
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
) -> Result<Json<ApiResponse<ReportDataResponse>>, AppError> {
    let service = ReportEngineService::new(state.db);
    let _page = query.page.unwrap_or(1).max(1); // 批次 95 P3-3~8：分页 clamp 防 DoS
    let _page_size = query.page_size.unwrap_or(50).clamp(1, 100);

    let req = crate::services::report::ExecuteReportRequest {
        template_id: query.template_id.clone(),
        filters: vec![],
        parameters: None,
        date_range: None,
        format: "json".to_string(),
        use_cache: Some(false),
    };

    match service.execute_report(req).await {
        Ok(data) => {
            // 将 ReportColumn 转换为 String (label)
            let columns: Vec<String> = data.columns.iter().map(|c| c.label.clone()).collect();
            // 将 serde_json::Value 行转换为 Vec<String>
            let rows: Vec<Vec<String>> = data
                .rows
                .iter()
                .map(|row| {
                    if let Some(arr) = row.as_array() {
                        arr.iter()
                            .map(|v| match v {
                                serde_json::Value::String(s) => s.clone(),
                                _ => v.to_string().trim_matches('"').to_string(),
                            })
                            .collect()
                    } else if let Some(obj) = row.as_object() {
                        // 按列顺序提取值
                        data.columns
                            .iter()
                            .map(|c| {
                                obj.get(&c.key)
                                    .map(|v| match v {
                                        serde_json::Value::String(s) => s.clone(),
                                        _ => v.to_string().trim_matches('"').to_string(),
                                    })
                                    .unwrap_or_default()
                            })
                            .collect()
                    } else {
                        vec![row.to_string()]
                    }
                })
                .collect();

            let response = ReportDataResponse {
                columns,
                rows,
                total_count: data.total_rows,
            };
            Ok(Json(ApiResponse::success(response)))
        }
        Err(e) => {
            tracing::error!("执行报表失败: {}", e);
            Err(AppError::internal(format!("执行报表失败: {}", e)))
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
) -> Result<Json<ApiResponse<ExportReportResponse>>, AppError> {
    let service = ReportEngineService::new(state.db);

    let _export_format: ExportFormat = query.format.parse().unwrap_or(ExportFormat::Csv);

    // 先执行报表获取数据
    let req = crate::services::report::ExecuteReportRequest {
        template_id: query.template_id.clone(),
        filters: vec![],
        parameters: None,
        date_range: None,
        format: "json".to_string(),
        use_cache: Some(false),
    };

    match service.execute_report(req).await {
        Ok(data) => {
            let template_name = query.template_id.clone();
            let format_str = query.format.clone();
            match service
                .export_report(&data, &format_str, &template_name)
                .await
            {
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
                    Err(AppError::internal(format!("导出报表失败: {}", e)))
                }
            }
        }
        Err(e) => {
            tracing::error!("执行报表失败: {}", e);
            Err(AppError::internal(format!("执行报表失败: {}", e)))
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
) -> Result<Json<ApiResponse<AggregateReportResponse>>, AppError> {
    let service = ReportEngineService::new(state.db);

    let data_source: DataSource = match request.data_source.as_str() {
        "sales" => DataSource::Sales,
        "purchase" => DataSource::Purchase,
        "inventory" => DataSource::Inventory,
        "finance" => DataSource::Finance,
        _ => {
            return Err(AppError::bad_request(format!(
                "无效的数据源: {}",
                request.data_source
            )));
        }
    };

    let aggregation_type: AggregationType = match request.aggregation_type.as_str() {
        "sum" => AggregationType::Sum,
        "count" => AggregationType::Count,
        "average" | "avg" => AggregationType::Average,
        "min" => AggregationType::Min,
        "max" => AggregationType::Max,
        "group_by" | "group" => AggregationType::GroupBy,
        _ => {
            return Err(AppError::bad_request(format!(
                "无效的聚合类型: {}",
                request.aggregation_type
            )));
        }
    };

    let filters = request
        .filters
        .unwrap_or_default()
        .into_iter()
        .map(|f| ReportFilter {
            key: f.field.clone(),
            field_alias: Some(f.field),
            label: String::new(),
            operator: Some(f.operator),
            value: Some(f.value),
            filter_type: "custom".to_string(),
            default_value: None,
            options: None,
            required: false,
        })
        .collect();

    let aggregate_request = AggregateRequest {
        data_source,
        data_source_str: Some(request.data_source),
        aggregation_type,
        group_by: request.group_by.unwrap_or_default(),
        filters,
        date_range: None,
        parameters: None,
        limit: None,
        aggregation_field: request.aggregation_field,
    };

    let _page = request.page.unwrap_or(1).max(1); // 批次 95 P3-3~8：分页 clamp 防 DoS
    let _page_size = request.page_size.unwrap_or(50).clamp(1, 100);

    match service.aggregate_data(aggregate_request).await {
        Ok(results) => {
            // 取第一个结果作为响应（如果存在）
            if let Some(first) = results.into_iter().next() {
                let response = AggregateReportResponse {
                    columns: first.columns,
                    rows: first.rows,
                    total_count: first.total_count,
                };
                Ok(Json(ApiResponse::success(response)))
            } else {
                Ok(Json(ApiResponse::success(AggregateReportResponse {
                    columns: Vec::new(),
                    rows: Vec::new(),
                    total_count: 0,
                })))
            }
        }
        Err(e) => {
            tracing::error!("数据聚合失败: {}", e);
            Err(AppError::internal(format!("数据聚合失败: {}", e)))
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
) -> Result<Json<ApiResponse<ClearCacheResponse>>, AppError> {
    let service = ReportEngineService::new(state.db);

    if let Some(source) = request.data_source {
        let data_source: DataSource = match source.as_str() {
            "sales" => DataSource::Sales,
            "purchase" => DataSource::Purchase,
            "inventory" => DataSource::Inventory,
            "finance" => DataSource::Finance,
            _ => {
                return Err(AppError::bad_request(format!("无效的数据源: {}", source)));
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
