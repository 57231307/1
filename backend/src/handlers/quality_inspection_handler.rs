use crate::container::AppState;
use crate::middleware::auth_context::AuthContext;
use crate::models::quality_inspection;
use crate::models::quality_inspection_record;
use crate::models::unqualified_product;
use crate::services::quality_inspection_service::{
    CreateInspectionRecordRequest, CreateQualityInspectionStandardRequest,
    ProcessUnqualifiedRequest, QualityInspectionService,
};
use crate::utils::ApiResponse;
use crate::utils::error::AppError;
// V15 P0-S12/P0-S15 修复（Batch 475d）：导出端点使用水印版 xlsx 工具
use crate::utils::xlsx_export::{WatermarkConfig, XlsxTable, build_xlsx_response_with_watermark};
// V15 P0-S11：导出审计日志写入所需依赖
use crate::models::audit_log::{OperationType, Severity};
use crate::services::audit_log_service::{AuditEvent, AuditLogService};
use axum::{
    Json,
    extract::{Path, Query, State},
};
use serde::Deserialize;
use std::sync::Arc;
use tracing::info;
use validator::Validate;

/// 校验检验结论取值，越界值直接拒绝并报出允许值清单。
///
/// 该列此前是自由文本：界面自造过 pass/fail/pending，而唯一的自动写入方落的是中文结论，
/// 两套写法混在同一列会让按结论筛选与合格率统计静默失真，因此在入口处一次拦清。
pub fn validate_inspection_result(raw: &str) -> Result<(), AppError> {
    use crate::models::status::quality_inspection_result;

    if quality_inspection_result::ALL.contains(&raw) {
        Ok(())
    } else {
        Err(AppError::validation(format!(
            "检验结果 {raw} 不是合法取值，允许值：{}",
            quality_inspection_result::ALL.join("/")
        )))
    }
}

/// 校验检验类型取值，越界值拒绝并报出允许值清单。
///
/// 四个码由质检记录弹窗写入，`outsourcing_receipt` 是委外回仓自动建记录的来源标识；
/// 与 `ai_quality_predictions.inspection_type`（另一套 CHECK 词表）不可互抄。
pub fn validate_inspection_type(raw: &str) -> Result<(), AppError> {
    use crate::models::status::quality_inspection_type;

    for allowed in quality_inspection_type::ALL {
        if *allowed == raw {
            return Ok(());
        }
    }
    Err(AppError::validation(format!(
        "检验类型 {raw} 不是合法取值，允许值：{}",
        quality_inspection_type::ALL.join("/")
    )))
}

/// 列表与导出共用的记录筛选条件构造：空串按「未选择」处理，两个枚举入参越界一律拒绝。
fn build_record_list_params(
    query: &RecordQuery,
    page: i64,
    page_size: i64,
) -> Result<crate::services::quality_inspection_service::RecordListParams, AppError> {
    use crate::services::quality_inspection_service::RecordListParams;

    let normalized = |raw: &Option<String>| {
        raw.as_deref()
            .map(|s| s.trim())
            .filter(|s| !s.is_empty())
            .map(|s| s.to_string())
    };
    let inspection_type = normalized(&query.inspection_type);
    if let Some(value) = &inspection_type {
        validate_inspection_type(value)?;
    }
    let inspection_result = normalized(&query.inspection_result);
    if let Some(value) = &inspection_result {
        validate_inspection_result(value)?;
    }

    Ok(RecordListParams {
        inspection_type,
        inspection_result,
        product_id: query.product_id,
        batch_no: normalized(&query.batch_no),
        page,
        page_size,
    })
}

#[allow(dead_code, reason = "反序列化输入字段")]
#[derive(Debug, Deserialize)]
pub struct QualityInspectionQuery {
    pub inspection_type: Option<String>,
    pub status: Option<String>,
    pub page: Option<i64>,
    pub page_size: Option<i64>,
}

// 记录列表与导出共用该查询参数；字段名与 sales 侧契约一致（batch_no 而非 batch_number），
// 每个字段都真正参与筛选，因此不再需要 dead_code 豁免
#[derive(Debug, Clone, Deserialize)]
pub struct RecordQuery {
    pub product_id: Option<i32>,
    pub batch_no: Option<String>,
    pub inspection_type: Option<String>,
    pub inspection_result: Option<String>,
    pub page: Option<i64>,
    pub page_size: Option<i64>,
}

#[allow(dead_code, reason = "反序列化输入字段")]
#[derive(Debug, Deserialize)]
pub struct DefectQuery {
    pub record_id: Option<i32>,
    pub status: Option<String>,
    pub page: Option<i64>,
    pub page_size: Option<i64>,
}

pub async fn list_standards(
    Query(params): Query<QualityInspectionQuery>,
    State(state): State<AppState>,
    auth: AuthContext,
) -> Result<Json<ApiResponse<Vec<quality_inspection::Model>>>, AppError> {
    info!("用户 {} 正在查询质量检验标准列表", auth.user_id);

    let service = QualityInspectionService::new(state.db.clone());
    let query_params = crate::services::quality_inspection_service::QualityInspectionQueryParams {
        inspection_type: params.inspection_type,
        status: params.status,
        page: params.page.unwrap_or(1).clamp(1, 1000),
        page_size: params.page_size.unwrap_or(10).clamp(1, 100),
    };

    let (standards, _total) = service.get_standards_list(query_params).await?;
    info!("质量检验标准列表查询成功，共 {} 条记录", standards.len());

    Ok(Json(ApiResponse::success(standards)))
}

pub async fn create_standard(
    State(state): State<AppState>,
    auth: AuthContext,
    Json(req): Json<CreateQualityInspectionStandardRequest>,
) -> Result<Json<ApiResponse<quality_inspection::Model>>, AppError> {
    info!("用户 {} 正在创建质量检验标准", auth.user_id);

    let service = QualityInspectionService::new(state.db.clone());
    let standard = service.create_standard(req, auth.user_id).await?;
    info!("质量检验标准创建成功，ID：{}", standard.id);

    Ok(Json(ApiResponse::success(standard)))
}

pub async fn list_records(
    Query(params): Query<RecordQuery>,
    State(state): State<AppState>,
    auth: AuthContext,
) -> Result<
    Json<ApiResponse<crate::utils::response::PaginatedResponse<quality_inspection_record::Model>>>,
    AppError,
> {
    info!("用户 {} 正在查询质量检验记录列表", auth.user_id);

    let page = params.page.unwrap_or(1).clamp(1, 1000) as u64;
    let page_size = params.page_size.unwrap_or(10).clamp(1, 100) as u64;
    let service = QualityInspectionService::new(state.db.clone());
    let query_params = build_record_list_params(&params, page as i64, page_size as i64)?;

    let (records, total) = service.get_records_list(query_params).await?;
    info!("质量检验记录列表查询成功，共 {} 条记录", records.len());

    // v11 批次 161 P2-5 修复：返回 PaginatedResponse（含 total），替代原先丢弃 _total 的 Vec 返回
    Ok(Json(ApiResponse::success_paginated(
        records, total, page, page_size,
    )))
}

#[axum::debug_handler]
pub async fn create_record(
    State(state): State<AppState>,
    auth: AuthContext,
    Json(req): Json<CreateInspectionRecordRequest>,
) -> Result<Json<ApiResponse<quality_inspection_record::Model>>, AppError> {
    info!("用户 {} 正在创建质量检验记录", auth.user_id);
    validate_inspection_result(&req.inspection_result)?;

    let service = QualityInspectionService::new(state.db.clone());
    let record = service.create_record(req, auth.user_id).await?;
    info!("质量检验记录创建成功，ID：{}", record.id);

    Ok(Json(ApiResponse::success(record)))
}

pub async fn get_record(
    State(state): State<AppState>,
    Path(id): Path<i32>,
    auth: AuthContext,
) -> Result<Json<ApiResponse<quality_inspection_record::Model>>, AppError> {
    info!("用户 {} 正在查询质量检验记录，ID: {}", auth.user_id, id);

    let service = QualityInspectionService::new(state.db.clone());
    let record = service.get_record_by_id(id).await?;
    info!("质量检验记录查询成功，ID：{}", record.id);

    Ok(Json(ApiResponse::success(record)))
}

/// 更新质检记录请求（对应前端 api/quality.ts updateQualityRecord 传 Partial<QualityRecord>，全字段可选）
#[allow(dead_code, reason = "反序列化输入字段")]
#[derive(Debug, Deserialize, Validate)]
pub struct UpdateInspectionRecordRequest {
    #[validate(length(max = 50, message = "检验类型长度不得超过 50 字符"))]
    pub inspection_type: Option<String>,
    #[validate(length(max = 100, message = "批次号长度不得超过 100 字符"))]
    pub batch_no: Option<String>,
    pub inspection_date: Option<chrono::NaiveDate>,
    pub inspector_id: Option<i32>,
    pub total_qty: Option<rust_decimal::Decimal>,
    pub inspected_qty: Option<rust_decimal::Decimal>,
    pub qualified_qty: Option<rust_decimal::Decimal>,
    pub unqualified_qty: Option<rust_decimal::Decimal>,
    pub qualification_rate: Option<rust_decimal::Decimal>,
    #[validate(length(max = 20, message = "检验结果长度不得超过 20 字符"))]
    pub inspection_result: Option<String>,
    pub remark: Option<String>,
    #[validate(length(max = 50, message = "缺陷类型长度不得超过 50 字符"))]
    pub defect_type: Option<String>,
    #[validate(length(max = 10, message = "等级长度不得超过 10 字符"))]
    pub grade: Option<String>,
    #[validate(length(max = 100, message = "色号长度不得超过 100 字符"))]
    pub color_no: Option<String>,
    #[validate(length(max = 100, message = "缸号长度不得超过 100 字符"))]
    pub dye_lot_no: Option<String>,
}

/// PUT /api/v1/erp/production/quality-inspection/records/{id} - 编辑质检记录
/// （对应前端 api/quality.ts updateQualityRecord；参照 create_record 的字段集，仅更新请求中显式提供的字段）
pub async fn update_record(
    State(state): State<AppState>,
    Path(id): Path<i32>,
    auth: AuthContext,
    Json(req): Json<UpdateInspectionRecordRequest>,
) -> Result<Json<ApiResponse<quality_inspection_record::Model>>, AppError> {
    use sea_orm::{ActiveModelTrait, EntityTrait, Set};

    info!("用户 {} 正在更新质量检验记录，ID: {}", auth.user_id, id);
    req.validate()
        .map_err(|e| AppError::validation(e.to_string()))?;
    if let Some(v) = &req.inspection_result {
        validate_inspection_result(v)?;
    }

    let existing = quality_inspection_record::Entity::find_by_id(id)
        .one(state.db.as_ref())
        .await?
        .ok_or_else(|| AppError::not_found(format!("质量检验记录不存在：{}", id)))?;

    let mut active: quality_inspection_record::ActiveModel = existing.into();
    if let Some(v) = req.inspection_type {
        active.inspection_type = Set(v);
    }
    if let Some(v) = req.batch_no {
        active.batch_no = Set(Some(v));
    }
    if let Some(v) = req.inspection_date {
        active.inspection_date = Set(v);
    }
    if let Some(v) = req.inspector_id {
        active.inspector_id = Set(Some(v));
    }
    if let Some(v) = req.total_qty {
        active.total_qty = Set(v);
    }
    if let Some(v) = req.inspected_qty {
        active.inspected_qty = Set(v);
    }
    if let Some(v) = req.qualified_qty {
        active.qualified_qty = Set(Some(v));
    }
    if let Some(v) = req.unqualified_qty {
        active.unqualified_qty = Set(Some(v));
    }
    if let Some(v) = req.qualification_rate {
        active.qualification_rate = Set(Some(v));
    }
    if let Some(v) = req.inspection_result {
        active.inspection_result = Set(v);
    }
    if let Some(v) = req.remark {
        active.remark = Set(Some(v));
    }
    if let Some(v) = req.defect_type {
        active.defect_type = Set(Some(v));
    }
    if let Some(v) = req.grade {
        active.grade = Set(Some(v));
    }
    if let Some(v) = req.color_no {
        active.color_no = Set(Some(v));
    }
    if let Some(v) = req.dye_lot_no {
        active.dye_lot_no = Set(Some(v));
    }
    active.updated_at = Set(chrono::Utc::now());

    let updated = active.update(state.db.as_ref()).await?;
    info!("质量检验记录更新成功，ID：{}", updated.id);

    Ok(Json(ApiResponse::success_with_message(
        updated,
        "质量检验记录更新成功",
    )))
}

pub async fn list_defects(
    Query(params): Query<DefectQuery>,
    State(state): State<AppState>,
    auth: AuthContext,
) -> Result<Json<ApiResponse<Vec<unqualified_product::Model>>>, AppError> {
    info!("用户 {} 正在查询质量缺陷列表", auth.user_id);

    let service = QualityInspectionService::new(state.db.clone());
    let query_params = crate::services::quality_inspection_service::QualityInspectionQueryParams {
        inspection_type: None,
        status: params.status,
        page: params.page.unwrap_or(1).clamp(1, 1000),
        page_size: params.page_size.unwrap_or(10).clamp(1, 100),
    };

    let (defects, _total) = service.get_defects_list(query_params).await?;
    info!("质量缺陷列表查询成功，共 {} 条记录", defects.len());

    Ok(Json(ApiResponse::success(defects)))
}

#[axum::debug_handler]
pub async fn process_defect(
    State(state): State<AppState>,
    Path(id): Path<i32>,
    auth: AuthContext,
    Json(req): Json<ProcessUnqualifiedRequest>,
) -> Result<Json<ApiResponse<unqualified_product::Model>>, AppError> {
    info!("用户 {} 正在处理质量缺陷，记录ID: {}", auth.user_id, id);

    let service = QualityInspectionService::new(state.db.clone());
    let result = service.process_unqualified(id, req, auth.user_id).await?;
    info!("质量缺陷处理成功，ID：{}", result.id);

    Ok(Json(ApiResponse::success(result)))
}

/// 质量检验记录导出表头（13 列）
fn record_export_headers() -> Vec<String> {
    vec![
        "ID".to_string(),
        "检验编号".to_string(),
        "检验类型".to_string(),
        "产品ID".to_string(),
        "批次号".to_string(),
        "检验日期".to_string(),
        "检验员ID".to_string(),
        "总数量".to_string(),
        "已检数量".to_string(),
        "合格数量".to_string(),
        "不合格数量".to_string(),
        "检验结果".to_string(),
        "等级".to_string(),
    ]
}

/// 从质量检验记录 JSON 对象构建 xlsx 行
fn build_record_row(obj: &serde_json::Map<String, serde_json::Value>) -> Vec<String> {
    let get_str = |key: &str| -> String {
        obj.get(key)
            .map(|v| {
                if v.is_null() {
                    String::new()
                } else if v.is_string() {
                    v.as_str().unwrap_or("").to_string()
                } else {
                    v.to_string()
                }
            })
            .unwrap_or_default()
    };
    vec![
        get_str("id"),
        get_str("inspection_no"),
        get_str("inspection_type"),
        get_str("product_id"),
        get_str("batch_no"),
        get_str("inspection_date"),
        get_str("inspector_id"),
        get_str("total_qty"),
        get_str("inspected_qty"),
        get_str("qualified_qty"),
        get_str("unqualified_qty"),
        get_str("inspection_result"),
        get_str("grade"),
    ]
}

/// 构造质量检验记录列表 xlsx 表格
fn build_records_table(records_json: Vec<serde_json::Value>) -> Result<XlsxTable, AppError> {
    let mut rows: Vec<Vec<String>> = Vec::with_capacity(records_json.len());
    for r in records_json {
        let obj = r
            .as_object()
            .ok_or_else(|| AppError::internal("质量检验记录序列化失败：期望 JSON 对象"))?;
        rows.push(build_record_row(obj));
    }
    Ok(XlsxTable {
        sheet_name: "质量检验记录".to_string(),
        headers: record_export_headers(),
        rows,
    })
}

/// 异步记录质量检验记录导出操作（审计自身）
fn record_records_export_audit(
    state: &AppState,
    auth: &AuthContext,
    row_count: usize,
    filename: &str,
) {
    let event = AuditEvent {
        user_id: Some(auth.user_id),
        username: Some(auth.username.clone()),
        operation_type: OperationType::Export,
        severity: Severity::Info,
        resource_type: Some("quality_inspection_record".to_string()),
        resource_id: None,
        resource_name: Some(format!("{}.xlsx", filename)),
        description: Some(format!(
            "用户 {} 导出质量检验记录列表（共 {} 条）",
            auth.username, row_count
        )),
        request_method: Some("GET".to_string()),
        request_path: Some("/api/v1/erp/quality-inspection/records/export".to_string()),
        before_snapshot: None,
        after_snapshot: Some(serde_json::json!({
            "format": "xlsx",
            "total": row_count,
        })),
    };
    let svc = Arc::new(AuditLogService::new(state.db.clone()));
    svc.record_async(event, None);
}

/// GET /api/v1/erp/quality-inspection/records/export - 导出质量检验记录列表（带水印 + 异步审计日志）；V15 P0-S12 修复（Batch 475d）：导出接入后端 - 注入水印（operator/exported_at/extra 含条数） - 异步审计日志（OperationType::Export） - 直接调
/// service.get_records_list 取全量数据（page=1/page_size=10000） - 筛选条件与 list_records 走同一个
/// `build_record_list_params`（同一套取值域校验），保证导出与列表口径一致
pub async fn export_records(
    State(state): State<AppState>,
    auth: AuthContext,
    Query(query): Query<RecordQuery>,
) -> Result<axum::response::Response, AppError> {
    let service = QualityInspectionService::new(state.db.clone());

    // V15 P0-S12 修复（Batch 475d）：导出全量数据；筛选条件与 list_records 共用同一构造函数，
    // 保证导出数据与列表口径一致（含取值域校验，越界同样拒绝）
    let query_params = build_record_list_params(&query, 1, 10000)?;

    let (records, _total) = service.get_records_list(query_params).await?;
    let row_count = records.len();

    // 序列化为 JSON 以统一字段处理
    let records_json: Vec<serde_json::Value> = records
        .into_iter()
        .map(|r| serde_json::to_value(r).map_err(AppError::from))
        .collect::<Result<Vec<_>, _>>()?;

    let table = build_records_table(records_json)?;
    let filename = format!(
        "quality_inspection_records_export_{}",
        chrono::Utc::now().format("%Y%m%d_%H%M%S")
    );
    record_records_export_audit(&state, &auth, row_count, &filename);

    // V15 P0-S15 修复（Batch 475d）：注入水印（操作员/导出时间/导出条数）
    let watermark = WatermarkConfig {
        operator: Some(auth.username.clone()),
        ip_address: None,
        exported_at: Some(chrono::Utc::now().to_rfc3339()),
        extra: Some(format!("质量检验记录导出（共 {} 条）", row_count)),
    };

    build_xlsx_response_with_watermark(&table, &filename, &watermark)
}
