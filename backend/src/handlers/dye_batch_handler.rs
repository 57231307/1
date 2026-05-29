//! 缸号管理Handler（染色批次管理）

use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    response::IntoResponse,
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
use crate::utils::response::{ApiResponse, PaginatedResponse};

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
}

impl DyeBatchStatus {
    pub fn from_chinese_str(s: &str) -> Option<Self> {
        match s {
            "待生产" => Some(Self::Pending),
            "生产中" => Some(Self::InProgress),
            "已完成" => Some(Self::Completed),
            "已取消" => Some(Self::Cancelled),
            _ => None,
        }
    }

    #[allow(dead_code)]
    pub fn to_str(&self) -> &'static str {
        match self {
            Self::Pending => "待生产",
            Self::InProgress => "生产中",
            Self::Completed => "已完成",
            Self::Cancelled => "已取消",
        }
    }

    /// 检查状态流转是否合法
    pub fn can_transition_to(&self, target: &Self) -> bool {
        match self {
            Self::Pending => matches!(target, Self::InProgress | Self::Cancelled),
            Self::InProgress => matches!(target, Self::Completed | Self::Cancelled),
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
) -> impl IntoResponse {
    let page = query.page.unwrap_or(1);
    let page_size = query.page_size.unwrap_or(20);

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
    match paginator.num_items().await {
        Ok(total) => match paginator.fetch_page(page - 1).await {
            Ok(batches) => {
                let paginated = PaginatedResponse::new(batches, total, page, page_size);
                paginated.into_response()
            }
            Err(e) => (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ApiResponse::<()>::error(format!("获取缸号列表失败：{}", e))),
            )
                .into_response(),
        },
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ApiResponse::<()>::error(format!("获取缸号总数失败：{}", e))),
        )
            .into_response(),
    }
}

pub async fn get_dye_batch(
    State(state): State<AppState>,
    Path(id): Path<i32>,
) -> impl IntoResponse {
    match dye_batch::Entity::find_by_id(id).one(&*state.db).await {
        Ok(Some(batch)) => (StatusCode::OK, Json(ApiResponse::success(batch))).into_response(),
        Ok(None) => (
            StatusCode::NOT_FOUND,
            Json(ApiResponse::<()>::error("缸号不存在")),
        )
            .into_response(),
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ApiResponse::<()>::error(format!("获取缸号失败：{}", e))),
        )
            .into_response(),
    }
}

pub async fn create_dye_batch(
    State(state): State<AppState>,
    _auth: AuthContext,
    Json(req): Json<CreateDyeBatchRequest>,
) -> impl IntoResponse {
    // 验证状态值
    let status = match req.status {
        Some(s) => {
            if DyeBatchStatus::from_chinese_str(&s).is_none() {
                return (
                    StatusCode::BAD_REQUEST,
                    Json(ApiResponse::<()>::error(format!("无效的缸号状态：{}", s))),
                )
                    .into_response();
            }
            Some(s)
        }
        None => Some("待生产".to_string()),
    };

    // 自动生成缸号
    let batch_no = req.batch_no.unwrap_or_else(|| {
        let timestamp = chrono::Utc::now().format("%Y%m%d%H%M%S");
        let random = rand::random::<u16>() % 10000;
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
        created_at: Set(
            chrono::Utc::now().with_timezone(&chrono::FixedOffset::east_opt(0).unwrap())
        ),
        updated_at: Set(
            chrono::Utc::now().with_timezone(&chrono::FixedOffset::east_opt(0).unwrap())
        ),
    };

    match batch.insert(&*state.db).await {
        Ok(created) => (
            StatusCode::CREATED,
            Json(ApiResponse::success_with_msg(created, "缸号创建成功")),
        )
            .into_response(),
        Err(e) => (
            StatusCode::BAD_REQUEST,
            Json(ApiResponse::<()>::error(format!("创建缸号失败：{}", e))),
        )
            .into_response(),
    }
}

pub async fn update_dye_batch(
    State(state): State<AppState>,
    Path(id): Path<i32>,
    _auth: AuthContext,
    Json(req): Json<UpdateDyeBatchRequest>,
) -> impl IntoResponse {
    let mut batch: dye_batch::ActiveModel =
        match dye_batch::Entity::find_by_id(id).one(&*state.db).await {
            Ok(Some(b)) => b.into(),
            Ok(None) => {
                return (
                    StatusCode::NOT_FOUND,
                    Json(ApiResponse::<()>::error("缸号不存在")),
                )
                    .into_response();
            }
            Err(e) => {
                return (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    Json(ApiResponse::<()>::error(format!("获取缸号失败：{}", e))),
                )
                    .into_response();
            }
        };

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
                return (
                    StatusCode::BAD_REQUEST,
                    Json(ApiResponse::<()>::error(format!(
                        "状态流转不合法：{} -> {}",
                        current_status, status
                    ))),
                )
                    .into_response();
            }
        } else {
            return (
                StatusCode::BAD_REQUEST,
                Json(ApiResponse::<()>::error(format!("无效的状态：{}", status))),
            )
                .into_response();
        }

        batch.status = Set(Some(status.clone()));

        // 自动设置时间戳
        if status == "生产中" {
            let needs_start_time = batch.started_at.as_ref().is_none();
            if needs_start_time {
                batch.started_at = Set(Some(
                    chrono::Utc::now().with_timezone(&chrono::FixedOffset::east_opt(0).unwrap()),
                ));
            }
        }
        if status == "已完成" {
            batch.completed_at = Set(Some(
                chrono::Utc::now().with_timezone(&chrono::FixedOffset::east_opt(0).unwrap()),
            ));
        }
    }

    batch.updated_at =
        Set(chrono::Utc::now().with_timezone(&chrono::FixedOffset::east_opt(0).unwrap()));

    match batch.update(&*state.db).await {
        Ok(updated) => (
            StatusCode::OK,
            Json(ApiResponse::success_with_msg(updated, "缸号更新成功")),
        )
            .into_response(),
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ApiResponse::<()>::error(format!("更新缸号失败：{}", e))),
        )
            .into_response(),
    }
}

pub async fn delete_dye_batch(
    State(state): State<AppState>,
    Path(id): Path<i32>,
    _auth: AuthContext,
) -> impl IntoResponse {
    // 检查缸号状态，生产中的缸号不允许删除
    let batch = match dye_batch::Entity::find_by_id(id).one(&*state.db).await {
        Ok(Some(b)) => b,
        Ok(None) => {
            return (
                StatusCode::NOT_FOUND,
                Json(ApiResponse::<()>::error("缸号不存在")),
            )
                .into_response();
        }
        Err(e) => {
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ApiResponse::<()>::error(format!("获取缸号失败：{}", e))),
            )
                .into_response();
        }
    };

    if batch.status.as_deref() == Some("生产中") {
        return (
            StatusCode::BAD_REQUEST,
            Json(ApiResponse::<()>::error(
                "生产中的缸号不允许删除，请先取消或完成",
            )),
        )
            .into_response();
    }

    // 软删除
    let mut active: dye_batch::ActiveModel = batch.into();
    active.is_deleted = Set(Some(true));
    active.updated_at =
        Set(chrono::Utc::now().with_timezone(&chrono::FixedOffset::east_opt(0).unwrap()));

    match active.update(&*state.db).await {
        Ok(_) => (
            StatusCode::OK,
            Json(ApiResponse::success_with_msg((), "缸号删除成功")),
        )
            .into_response(),
        Err(e) => (
            StatusCode::BAD_REQUEST,
            Json(ApiResponse::<()>::error(format!("删除缸号失败：{}", e))),
        )
            .into_response(),
    }
}

pub async fn complete_dye_batch(
    State(state): State<AppState>,
    Path(id): Path<i32>,
    _auth: AuthContext,
) -> impl IntoResponse {
    let mut batch: dye_batch::ActiveModel =
        match dye_batch::Entity::find_by_id(id).one(&*state.db).await {
            Ok(Some(b)) => b.into(),
            Ok(None) => {
                return (
                    StatusCode::NOT_FOUND,
                    Json(ApiResponse::<()>::error("缸号不存在")),
                )
                    .into_response();
            }
            Err(e) => {
                return (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    Json(ApiResponse::<()>::error(format!("获取缸号失败：{}", e))),
                )
                    .into_response();
            }
        };

    // 检查当前状态是否允许完成
    let current_status = match &batch.status {
        sea_orm::ActiveValue::Set(Some(s)) => s.as_str(),
        _ => "待生产",
    };
    let current = DyeBatchStatus::from_chinese_str(current_status).unwrap_or(DyeBatchStatus::Pending);

    if !current.can_transition_to(&DyeBatchStatus::Completed) {
        return (
            StatusCode::BAD_REQUEST,
            Json(ApiResponse::<()>::error(format!(
                "状态流转不合法：{} -> 已完成",
                current_status
            ))),
        )
            .into_response();
    }

    batch.status = Set(Some("已完成".to_string()));
    batch.completed_at = Set(Some(
        chrono::Utc::now().with_timezone(&chrono::FixedOffset::east_opt(0).unwrap()),
    ));
    batch.updated_at =
        Set(chrono::Utc::now().with_timezone(&chrono::FixedOffset::east_opt(0).unwrap()));

    match batch.update(&*state.db).await {
        Ok(updated) => (
            StatusCode::OK,
            Json(ApiResponse::success_with_msg(updated, "缸号完成成功")),
        )
            .into_response(),
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ApiResponse::<()>::error(format!("完成缸号失败：{}", e))),
        )
            .into_response(),
    }
}

pub async fn get_dye_batches_by_color(
    State(state): State<AppState>,
    Path(color_no): Path<String>,
) -> impl IntoResponse {
    match dye_batch::Entity::find()
        .filter(dye_batch::Column::ColorNo.eq(color_no))
        .filter(dye_batch::Column::IsDeleted.eq(false))
        .order_by_desc(dye_batch::Column::CreatedAt)
        .all(&*state.db)
        .await
    {
        Ok(batches) => (StatusCode::OK, Json(ApiResponse::success(batches))).into_response(),
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ApiResponse::<()>::error(format!("获取缸号列表失败：{}", e))),
        )
            .into_response(),
    }
}
