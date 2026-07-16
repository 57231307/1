//! 缸号管理Handler（染色批次管理）

use axum::{
    extract::{Path, Query, State},
    Json,
};
use rust_decimal::Decimal;
use sea_orm::{
    ActiveModelTrait, ColumnTrait, EntityTrait, PaginatorTrait, QueryFilter, QueryOrder, Set,
};
use serde::Deserialize;

use crate::middleware::auth_context::AuthContext;
use crate::models::dye_batch;
use crate::utils::app_state::AppState;
use crate::utils::error::AppError;
use crate::utils::response::{ApiResponse, PaginatedResponse};
use crate::utils::xlsx_export::{build_xlsx_response, XlsxTable};

/// 缸号状态枚举
#[derive(Debug, Clone, PartialEq)]
pub enum DyeBatchStatus {
    /// 待生产
    Pending,
    /// 生产中
    InProgress,
    /// 已完成
    Completed,
    /// 已取消
    Cancelled,
    /// 生产失败（L-23 修复，批次 369 v13 复审）：染色过程异常终止，可重试或放弃
    Failed,
    /// 已暂停（L-23 修复，批次 369 v13 复审）：生产过程中临时挂起，可恢复或取消
    OnHold,
}

impl DyeBatchStatus {
    pub fn from_chinese_str(s: &str) -> Option<Self> {
        match s {
            "待生产" => Some(Self::Pending),
            "生产中" => Some(Self::InProgress),
            "已完成" => Some(Self::Completed),
            "已取消" => Some(Self::Cancelled),
            "失败" => Some(Self::Failed),
            "暂停" => Some(Self::OnHold),
            _ => None,
        }
    }

    /// 检查状态流转是否合法
    pub fn can_transition_to(&self, target: &Self) -> bool {
        match self {
            Self::Pending => matches!(target, Self::InProgress | Self::Cancelled | Self::OnHold),
            Self::InProgress => {
                matches!(target, Self::Completed | Self::Cancelled | Self::Failed | Self::OnHold)
            }
            Self::OnHold => matches!(target, Self::InProgress | Self::Cancelled),
            Self::Failed => matches!(target, Self::Pending | Self::Cancelled),
            Self::Completed => false,
            Self::Cancelled => false,
        }
    }
}

#[derive(Debug, Deserialize)]
pub struct DyeBatchListQuery {
    pub page: Option<u64>,
    pub page_size: Option<u64>,
    pub batch_no: Option<String>,
    pub color_no: Option<String>,
    pub status: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct CreateDyeBatchRequest {
    pub batch_no: Option<String>,
    pub greige_fabric_id: Option<i32>,
    pub color_no: Option<String>,
    pub planned_quantity: Option<f64>,
    pub status: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct UpdateDyeBatchRequest {
    pub greige_fabric_id: Option<i32>,
    pub color_no: Option<String>,
    pub planned_quantity: Option<f64>,
    pub status: Option<String>,
}

pub async fn list_dye_batches(
    State(state): State<AppState>,
    Query(query): Query<DyeBatchListQuery>,
) -> Result<Json<ApiResponse<PaginatedResponse<dye_batch::Model>>>, AppError> {
    let page = query.page.unwrap_or(1).clamp(1, 1000); // 批次 95 P3-3~8：分页 clamp 防 DoS
    let page_size = query.page_size.unwrap_or(20).clamp(1, 100);

    let mut q = dye_batch::Entity::find().filter(dye_batch::Column::IsDeleted.eq(false));

    if let Some(batch_no) = &query.batch_no {
        q = q.filter(dye_batch::Column::BatchNo.contains(batch_no));
    }
    if let Some(color_no) = &query.color_no {
        q = q.filter(dye_batch::Column::ColorNo.contains(color_no));
    }
    if let Some(status) = &query.status {
        q = q.filter(dye_batch::Column::Status.eq(status));
    }

    q = q.order_by_desc(dye_batch::Column::CreatedAt);

    let paginator = q.paginate(&*state.db, page_size);
    let total = paginator.num_items().await?;
    // 批次 98 P2-A 修复（v5 复审）：page clamp 防 DoS
    let batches = paginator.fetch_page(page.clamp(1, 1000).saturating_sub(1)).await?;
    Ok(Json(ApiResponse::success_paginated(
        batches,
        total,
        page,
        page_size,
    )))
}

pub async fn get_dye_batch(
    State(state): State<AppState>,
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
    // 验证状态值
    let status = match req.status {
        Some(s) => {
            if DyeBatchStatus::from_chinese_str(&s).is_none() {
                return Err(AppError::bad_request(format!("无效的缸号状态：{}", s)));
            }
            Some(s)
        }
        None => Some("待生产".to_string()),
    };

    // 自动生成缸号
    let batch_no = req.batch_no.unwrap_or_else(|| {
        let timestamp = chrono::Utc::now().format("%Y%m%d%H%M%S");
        let random = crate::utils::random::random_4_digit();
        format!("DB-{}-{:04}", timestamp, random)
    });

    let batch = dye_batch::ActiveModel {
        id: Set(0),
        batch_no: Set(batch_no),
        greige_fabric_id: Set(req.greige_fabric_id),
        color_no: Set(req.color_no),
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
    if let Some(planned_quantity) = req.planned_quantity {
        batch.planned_quantity = Set(Decimal::from_f64_retain(planned_quantity));
    }
    if let Some(status) = req.status {
        // 验证状态流转
        let current_status = match &batch.status {
            sea_orm::ActiveValue::Set(Some(s)) => s.as_str(),
            _ => "待生产",
        };
        let target_status = DyeBatchStatus::from_chinese_str(&status);

        if let Some(target) = target_status {
            let current =
                DyeBatchStatus::from_chinese_str(current_status).unwrap_or(DyeBatchStatus::Pending);
            if !current.can_transition_to(&target) {
                return Err(AppError::business(format!(
                    "状态流转不合法：{} -> {}",
                    current_status, status
                )));
            }
        } else {
            return Err(AppError::bad_request(format!("无效的状态：{}", status)));
        }

        batch.status = Set(Some(status.clone()));

        // 自动设置时间戳
        if status == "生产中" {
            let needs_start_time = batch.started_at.as_ref().is_none();
            if needs_start_time {
                batch.started_at = Set(Some(crate::utils::date_utils::utc_now_fixed()));
            }
        }
        if status == "已完成" {
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

    if batch.status.as_deref() == Some("生产中") {
        return Err(AppError::business("生产中的缸号不允许删除，请先取消或完成"));
    }

    // 软删除
    let mut active: dye_batch::ActiveModel = batch.into();
    active.is_deleted = Set(Some(true));
    active.updated_at = Set(crate::utils::date_utils::utc_now_fixed());

    active.update(&*state.db).await?;
    Ok(Json(ApiResponse::success_with_message(
        (),
        "缸号删除成功",
    )))
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

    // 检查当前状态是否允许完成
    let current_status = match &batch.status {
        sea_orm::ActiveValue::Set(Some(s)) => s.as_str(),
        _ => "待生产",
    };
    let current =
        DyeBatchStatus::from_chinese_str(current_status).unwrap_or(DyeBatchStatus::Pending);

    if !current.can_transition_to(&DyeBatchStatus::Completed) {
        return Err(AppError::business(format!(
            "状态流转不合法：{} -> 已完成",
            current_status
        )));
    }

    // v14 批次 420 修复 T-P1-2：染色完成发布业务事件
    // 原实现仅做状态更新，未发布任何业务事件，导致下游（质检单生成、染缸产能统计、
    // 成本结转、BI 生产报表）无法被动感知染色完成节点。
    // 修复：在 update 成功后构造 DyeBatchCompleted 事件并发布到事件总线。
    batch.status = Set(Some("已完成".to_string()));
    batch.completed_at = Set(Some(crate::utils::date_utils::utc_now_fixed()));
    batch.updated_at = Set(crate::utils::date_utils::utc_now_fixed());

    let updated = batch.update(&*state.db).await?;

    // v14 批次 420 修复 T-P1-2：update 成功后发布 DyeBatchCompleted 事件
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
    _auth: AuthContext,
    Query(query): Query<DyeBatchListQuery>,
) -> Result<axum::response::Response, AppError> {
    let mut q = dye_batch::Entity::find().filter(dye_batch::Column::IsDeleted.eq(false));

    if let Some(batch_no) = &query.batch_no {
        q = q.filter(dye_batch::Column::BatchNo.contains(batch_no));
    }
    if let Some(color_no) = &query.color_no {
        q = q.filter(dye_batch::Column::ColorNo.contains(color_no));
    }
    if let Some(status) = &query.status {
        q = q.filter(dye_batch::Column::Status.eq(status));
    }

    q = q.order_by_desc(dye_batch::Column::CreatedAt);

    let batches = q.all(&*state.db).await?;

    let table = XlsxTable {
        sheet_name: "缸号列表".to_string(),
        headers: vec![
            "ID".to_string(),
            "缸号".to_string(),
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
    };

    // 规则 3：导出统一使用 xlsx 格式，错误用 AppError 表达，成功返回 200 + xlsx 响应体
    build_xlsx_response(&table, "dye_batches_export")
}
