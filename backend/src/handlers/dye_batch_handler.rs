//! 缸号管理Handler（染色批次管理）

use axum::{
    Json,
    extract::{Path, Query, State},
};
use rust_decimal::Decimal;
use sea_orm::{
    ActiveModelTrait, ActiveValue::NotSet, ColumnTrait, EntityTrait, PaginatorTrait, QueryFilter,
    QueryOrder, QuerySelect, Set,
};
use serde::Deserialize;

use crate::container::AppState;
use crate::middleware::auth_context::AuthContext;
// V15 P0-S11：导出审计日志写入所需依赖
use crate::models::audit_log::{OperationType, Severity};
use crate::models::dye_batch;
use crate::services::audit_log_service::{AuditEvent, AuditLogService};
use crate::utils::error::AppError;
use crate::utils::response::{ApiResponse, PaginatedResponse};
use crate::utils::xlsx_export::{XlsxTable, build_xlsx_response};
use std::sync::Arc;

use crate::services::dye_batch_state_machine_validation;

#[allow(dead_code, reason = "反序列化输入字段")]
#[derive(Debug, Deserialize)]
pub struct DyeBatchListQuery {
    pub page: Option<u64>,
    pub page_size: Option<u64>,
    pub batch_no: Option<String>,
    pub color_no: Option<String>,
    pub dye_lot_no: Option<String>,
    pub status: Option<String>,
}

#[allow(dead_code, reason = "反序列化输入字段")]
#[derive(Debug, Deserialize)]
pub struct CreateDyeBatchRequest {
    pub batch_no: Option<String>,
    pub greige_fabric_id: Option<i32>,
    pub color_no: Option<String>,
    pub dye_lot_no: Option<String>,
    pub planned_quantity: Option<f64>,
    pub status: Option<String>,
}

#[allow(dead_code, reason = "反序列化输入字段")]
#[derive(Debug, Deserialize)]
pub struct UpdateDyeBatchRequest {
    pub greige_fabric_id: Option<i32>,
    pub color_no: Option<String>,
    pub dye_lot_no: Option<String>,
    pub planned_quantity: Option<f64>,
    pub status: Option<String>,
}

pub async fn list_dye_batches(
    State(state): State<AppState>,
    auth: AuthContext,
    Query(query): Query<DyeBatchListQuery>,
) -> Result<Json<ApiResponse<PaginatedResponse<dye_batch::Model>>>, AppError> {
    let _data_scope = auth.to_data_scope_context();
    let page = query.page.unwrap_or(1).clamp(1, 1000); // 批次 95 P3-3~8：分页 clamp 防 DoS
    let page_size = query.page_size.unwrap_or(20).clamp(1, 100);

    let mut q = dye_batch::Entity::find().filter(dye_batch::Column::IsDeleted.eq(false));

    if let Some(batch_no) = &query.batch_no {
        q = q.filter(dye_batch::Column::BatchNo.contains(batch_no));
    }
    if let Some(color_no) = &query.color_no {
        q = q.filter(dye_batch::Column::ColorNo.contains(color_no));
    }
    if let Some(dye_lot_no) = &query.dye_lot_no {
        q = q.filter(dye_batch::Column::DyeLotNo.contains(dye_lot_no));
    }
    if let Some(status) = &query.status {
        q = q.filter(dye_batch::Column::Status.eq(status));
    }

    q = q.order_by_desc(dye_batch::Column::CreatedAt);

    let paginator = q.paginate(&*state.db, page_size);
    let total = paginator.num_items().await?;
    // 批次 98 P2-A 修复（v5 复审）：page clamp 防 DoS
    let batches = paginator
        .fetch_page(page.clamp(1, 1000).saturating_sub(1))
        .await?;
    Ok(Json(ApiResponse::success_paginated(
        batches, total, page, page_size,
    )))
}

pub async fn get_dye_batch(
    State(state): State<AppState>,
    _auth: AuthContext,
    Path(id): Path<i32>,
) -> Result<Json<ApiResponse<dye_batch::Model>>, AppError> {
    let batch = dye_batch::Entity::find_by_id(id)
        .one(&*state.db)
        .await?
        .ok_or_else(|| AppError::not_found("缸号不存在"))?;
    Ok(Json(ApiResponse::success(batch)))
}

pub async fn create_dye_batch(
    State(state): State<AppState>,
    _auth: AuthContext,
    Json(req): Json<CreateDyeBatchRequest>,
) -> Result<Json<ApiResponse<dye_batch::Model>>, AppError> {
    // 验证状态值（14 态 lifecycle_status 英文 key）
    let status = match req.status {
        Some(s) => {
            if !dye_batch_state_machine_validation::is_valid_status(&s) {
                return Err(AppError::bad_request(format!("无效的缸号状态：{}", s)));
            }
            Some(s)
        }
        None => Some("pending_schedule".to_string()),
    };

    // 自动生成缸号
    let batch_no = req.batch_no.unwrap_or_else(|| {
        let timestamp = chrono::Utc::now().format("%Y%m%d%H%M%S");
        let random = crate::utils::random::random_4_digit();
        format!("DB-{}-{:04}", timestamp, random)
    });

    // V15 P0-F01：dye_lot_no 默认 'DEFAULT'，新接口允许调用方传入实际染色批号
    // 术语：dye_lot_no（染色批号）≠ batch_no（缸号=染色批次号，同一概念不同叫法）
    let dye_lot_no = req.dye_lot_no.unwrap_or_else(|| "DEFAULT".to_string());

    let batch = dye_batch::ActiveModel {
        id: NotSet,
        batch_no: Set(batch_no),
        greige_fabric_id: Set(req.greige_fabric_id),
        color_code: Set(req.color_no.clone().unwrap_or_else(|| "TEST".to_string())),
        color_name: Set(req
            .color_no
            .clone()
            .unwrap_or_else(|| "测试色号".to_string())),
        color_no: Set(req.color_no),
        dye_lot_no: Set(dye_lot_no),
        planned_quantity: Set(req.planned_quantity.and_then(Decimal::from_f64_retain)),
        status: Set(status),
        started_at: Set(None),
        completed_at: Set(None),
        is_deleted: Set(Some(false)),
        created_at: Set(crate::utils::date_utils::utc_now_fixed()),
        updated_at: Set(crate::utils::date_utils::utc_now_fixed()),
    };

    // 使用 insert 获取返回的 Model
    dye_batch::Entity::insert(batch)
        .exec_without_returning(&*state.db)
        .await?;

    // 重新查询获取创建的记录
    // 批次 407 修复：DB 回查错误不能吞，返回空模型但消息说"创建成功"会误导用户，改为返回错误
    let created = dye_batch::Entity::find()
        .order_by_desc(dye_batch::Column::Id)
        .one(&*state.db)
        .await
        .map_err(|e| AppError::internal(format!("缸号创建后回查失败: {}", e)))?
        .ok_or_else(|| AppError::internal("缸号创建后回查未找到记录"))?;
    Ok(Json(ApiResponse::success_with_message(
        created,
        "缸号创建成功",
    )))
}

pub async fn update_dye_batch(
    State(state): State<AppState>,
    Path(id): Path<i32>,
    _auth: AuthContext,
    Json(req): Json<UpdateDyeBatchRequest>,
) -> Result<Json<ApiResponse<dye_batch::Model>>, AppError> {
    let mut batch: dye_batch::ActiveModel = dye_batch::Entity::find_by_id(id)
        .one(&*state.db)
        .await?
        .ok_or_else(|| AppError::not_found("缸号不存在"))?
        .into();

    if let Some(greige_fabric_id) = req.greige_fabric_id {
        batch.greige_fabric_id = Set(Some(greige_fabric_id));
    }
    if let Some(color_no) = req.color_no {
        batch.color_no = Set(Some(color_no));
    }
    if let Some(dye_lot_no) = req.dye_lot_no {
        batch.dye_lot_no = Set(dye_lot_no);
    }
    if let Some(planned_quantity) = req.planned_quantity {
        batch.planned_quantity = Set(Decimal::from_f64_retain(planned_quantity));
    }
    if let Some(status) = req.status {
        // 验证状态值合法性（14 态 lifecycle_status）
        if !dye_batch_state_machine_validation::is_valid_status(&status) {
            return Err(AppError::bad_request(format!("无效的状态：{}", status)));
        }
        // 验证状态流转合法性
        let current_status = match &batch.status {
            sea_orm::ActiveValue::Set(Some(s)) => s.as_str(),
            _ => "pending_schedule",
        };
        if !dye_batch_state_machine_validation::is_valid_status_transition(current_status, &status) {
            return Err(AppError::business(format!(
                "状态流转不合法：{} -> {}",
                current_status, status
            )));
        }

        batch.status = Set(Some(status.clone()));

        // 自动设置时间戳：染色中及之后工序记录开始时间
        let in_production = matches!(
            status.as_str(),
            "preparing" | "dyeing" | "washing" | "fixing" | "dehydrating" | "drying" | "inspecting"
        );
        if in_production {
            let needs_start_time = batch.started_at.as_ref().is_none();
            if needs_start_time {
                batch.started_at = Set(Some(crate::utils::date_utils::utc_now_fixed()));
            }
        }
        // 入库及之后状态记录完成时间
        let is_finished = matches!(
            status.as_str(),
            "stored" | "shipped"
        );
        if is_finished {
            batch.completed_at = Set(Some(crate::utils::date_utils::utc_now_fixed()));
        }
    }

    batch.updated_at = Set(crate::utils::date_utils::utc_now_fixed());

    let updated = batch.update(&*state.db).await?;
    Ok(Json(ApiResponse::success_with_message(
        updated,
        "缸号更新成功",
    )))
}

pub async fn delete_dye_batch(
    State(state): State<AppState>,
    Path(id): Path<i32>,
    _auth: AuthContext,
) -> Result<Json<ApiResponse<()>>, AppError> {
    // 检查缸号状态，生产中的缸号不允许删除
    let batch = dye_batch::Entity::find_by_id(id)
        .one(&*state.db)
        .await?
        .ok_or_else(|| AppError::not_found("缸号不存在"))?;

    if matches!(batch.status.as_deref(), Some("preparing") | Some("dyeing") | Some("washing") | Some("fixing") | Some("dehydrating") | Some("drying") | Some("inspecting")) {
        return Err(AppError::business("生产中的缸号不允许删除，请先取消或完成"));
    }

    // 软删除
    let mut active: dye_batch::ActiveModel = batch.into();
    active.is_deleted = Set(Some(true));
    active.updated_at = Set(crate::utils::date_utils::utc_now_fixed());

    active.update(&*state.db).await?;
    Ok(Json(ApiResponse::success_with_message((), "缸号删除成功")))
}

pub async fn complete_dye_batch(
    State(state): State<AppState>,
    Path(id): Path<i32>,
    _auth: AuthContext,
) -> Result<Json<ApiResponse<dye_batch::Model>>, AppError> {
    let mut batch: dye_batch::ActiveModel = dye_batch::Entity::find_by_id(id)
        .one(&*state.db)
        .await?
        .ok_or_else(|| AppError::not_found("缸号不存在"))?
        .into();

    // 检查当前状态是否允许完成（流转到 stored 终态前态）
    let current_status = match &batch.status {
        sea_orm::ActiveValue::Set(Some(s)) => s.as_str(),
        _ => "pending_schedule",
    };
    if !dye_batch_state_machine_validation::is_valid_status_transition(current_status, "stored") {
        return Err(AppError::business(format!(
            "状态流转不合法：{} -> stored",
            current_status
        )));
    }

    // 染色完成时发布 DyeBatchCompleted 业务事件，供质检单生成、染缸产能统计、
    // 成本结转、BI 生产报表等下游被动感知
    batch.status = Set(Some("stored".to_string()));
    batch.completed_at = Set(Some(crate::utils::date_utils::utc_now_fixed()));
    batch.updated_at = Set(crate::utils::date_utils::utc_now_fixed());

    let updated = batch.update(&*state.db).await?;

    // 落库成功后发布 DyeBatchCompleted 事件
    crate::services::event_bus::EVENT_BUS.publish(
        crate::services::event_bus::BusinessEvent::DyeBatchCompleted {
            batch_id: updated.id,
            batch_no: updated.batch_no.clone(),
            color_no: updated.color_no.clone(),
            greige_fabric_id: updated.greige_fabric_id,
            planned_quantity: updated.planned_quantity,
            completed_by: None,
        },
    );
    tracing::info!(
        batch_id = updated.id,
        batch_no = %updated.batch_no,
        "染色完成，已发布 DyeBatchCompleted 事件"
    );

    Ok(Json(ApiResponse::success_with_message(
        updated,
        "缸号完成成功",
    )))
}

pub async fn get_dye_batches_by_color(
    State(state): State<AppState>,
    Path(color_no): Path<String>,
) -> Result<Json<ApiResponse<Vec<dye_batch::Model>>>, AppError> {
    let batches = dye_batch::Entity::find()
        .filter(dye_batch::Column::ColorNo.eq(color_no))
        .filter(dye_batch::Column::IsDeleted.eq(false))
        .order_by_desc(dye_batch::Column::CreatedAt)
        .all(&*state.db)
        .await?;
    Ok(Json(ApiResponse::success(batches)))
}

/// GET /api/v1/erp/dye-batches/export - 导出缸号列表（xlsx）
pub async fn export_dye_batches(
    State(state): State<AppState>,
    auth: AuthContext,
    Query(query): Query<DyeBatchListQuery>,
) -> Result<axum::response::Response, AppError> {
    let mut q = dye_batch::Entity::find().filter(dye_batch::Column::IsDeleted.eq(false));
    q = apply_dye_batch_filters(q, &query);
    q = q.order_by_desc(dye_batch::Column::CreatedAt);

    // V15 缺陷 9-2 修复：导出全量查询加 limit 防止滥用（最大 10000 条）
    let batches = q.limit(10000).all(&*state.db).await?;

    let table = build_dye_batch_xlsx_table(&batches);
    let row_count = batches.len();

    // V15 P0-S11：导出审计日志写入（best-effort，异步不阻塞响应）
    let event = build_dye_batch_audit_event(&auth, &query, row_count);
    let svc = Arc::new(AuditLogService::new(state.db.clone()));
    svc.record_async(event, None);

    // 规则 3：导出统一使用 xlsx 格式，错误用 AppError 表达，成功返回 200 + xlsx 响应体
    build_xlsx_response(&table, "dye_batches_export")
}

/// 应用缸号列表查询过滤条件
fn apply_dye_batch_filters(
    mut q: sea_orm::Select<dye_batch::Entity>,
    query: &DyeBatchListQuery,
) -> sea_orm::Select<dye_batch::Entity> {
    if let Some(batch_no) = &query.batch_no {
        q = q.filter(dye_batch::Column::BatchNo.contains(batch_no));
    }
    if let Some(color_no) = &query.color_no {
        q = q.filter(dye_batch::Column::ColorNo.contains(color_no));
    }
    if let Some(dye_lot_no) = &query.dye_lot_no {
        q = q.filter(dye_batch::Column::DyeLotNo.contains(dye_lot_no));
    }
    if let Some(status) = &query.status {
        q = q.filter(dye_batch::Column::Status.eq(status));
    }
    q
}

/// 构造缸号列表导出表格
fn build_dye_batch_xlsx_table(batches: &[dye_batch::Model]) -> XlsxTable {
    XlsxTable {
        sheet_name: "缸号列表".to_string(),
        headers: vec![
            "ID".to_string(),
            "缸号".to_string(),
            "染色批号".to_string(),
            "色号".to_string(),
            "坯布ID".to_string(),
            "计划数量".to_string(),
            "状态".to_string(),
            "创建时间".to_string(),
        ],
        rows: batches
            .iter()
            .map(|b| {
                vec![
                    b.id.to_string(),
                    b.batch_no.clone(),
                    b.dye_lot_no.clone(),
                    b.color_no.clone().unwrap_or_default(),
                    b.greige_fabric_id
                        .map(|i| i.to_string())
                        .unwrap_or_default(),
                    b.planned_quantity
                        .map(|d| d.to_string())
                        .unwrap_or_default(),
                    b.status.clone().unwrap_or_default(),
                    b.created_at.format("%Y-%m-%d %H:%M:%S").to_string(),
                ]
            })
            .collect(),
    }
}

/// 构造缸号导出审计事件
fn build_dye_batch_audit_event(
    auth: &AuthContext,
    query: &DyeBatchListQuery,
    row_count: usize,
) -> AuditEvent {
    AuditEvent {
        user_id: Some(auth.user_id),
        username: Some(auth.username.clone()),
        operation_type: OperationType::Export,
        severity: Severity::Info,
        resource_type: Some("dye_batch".to_string()),
        resource_id: None,
        resource_name: Some("dye_batches_export.xlsx".to_string()),
        description: Some(format!(
            "用户 {} 导出染色缸号列表（共 {} 条）",
            auth.username, row_count
        )),
        request_method: Some("GET".to_string()),
        request_path: Some("/api/v1/erp/dye-batches/export".to_string()),
        before_snapshot: None,
        after_snapshot: Some(serde_json::json!({
            "format": "xlsx",
            "total": row_count,
            "batch_no_filter": query.batch_no,
            "color_no_filter": query.color_no,
            "status_filter": query.status,
        })),
    }
}
