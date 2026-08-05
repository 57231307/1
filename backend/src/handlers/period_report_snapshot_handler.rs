//! 期末报表快照 Handler

use crate::container::AppState;
use crate::middleware::auth_context::AuthContext;
use crate::models::period_report_snapshot;
use crate::services::period_report_snapshot_service::{
    CreateSnapshotRequest, PeriodReportSnapshotService, SnapshotQueryParams,
};
use crate::utils::error::AppError;
use crate::utils::ApiResponse;
use axum::{
    extract::{Path, Query, State},
    Json,
};
use serde::Deserialize;
use tracing::info;

/// 查询参数 DTO
#[derive(Debug, Deserialize)]
pub struct SnapshotQuery {
    pub period_id: Option<i32>,
    pub report_type: Option<String>,
    pub page: Option<u64>,
    pub page_size: Option<u64>,
}

/// 创建请求 DTO
#[derive(Debug, Deserialize)]
pub struct CreateSnapshotDto {
    pub period_id: i32,
    pub report_type: String,
    pub report_data: serde_json::Value,
}

/// 创建报表快照
pub async fn create_snapshot(
    State(state): State<AppState>,
    auth: AuthContext,
    Json(req): Json<CreateSnapshotDto>,
) -> Result<Json<ApiResponse<period_report_snapshot::Model>>, AppError> {
    info!(
        "用户 {} 正在创建报表快照：期间 {}，类型 {}",
        auth.user_id, req.period_id, req.report_type
    );

    let service = PeriodReportSnapshotService::new(state.db.clone());
    let snapshot = service
        .create(
            CreateSnapshotRequest {
                period_id: req.period_id,
                report_type: req.report_type,
                report_data: req.report_data,
            },
            auth.user_id,
        )
        .await?;

    Ok(Json(ApiResponse::success(snapshot)))
}

/// 查询报表快照列表
pub async fn list_snapshots(
    Query(params): Query<SnapshotQuery>,
    State(state): State<AppState>,
    auth: AuthContext,
) -> Result<Json<ApiResponse<Vec<period_report_snapshot::Model>>>, AppError> {
    info!("用户 {} 正在查询报表快照列表", auth.user_id);

    let service = PeriodReportSnapshotService::new(state.db.clone());
    let (snapshots, total) = service
        .list(SnapshotQueryParams {
            period_id: params.period_id,
            report_type: params.report_type,
            page: params.page.unwrap_or(0),
            page_size: params.page_size.unwrap_or(20),
        })
        .await?;

    Ok(Json(ApiResponse::with_total(snapshots, total)))
}

/// 获取报表快照详情
pub async fn get_snapshot(
    Path(id): Path<i32>,
    State(state): State<AppState>,
    auth: AuthContext,
) -> Result<Json<ApiResponse<period_report_snapshot::Model>>, AppError> {
    info!("用户 {} 正在查询报表快照 {}", auth.user_id, id);

    let service = PeriodReportSnapshotService::new(state.db.clone());
    let snapshot = service.get_by_id(id).await?;

    Ok(Json(ApiResponse::success(snapshot)))
}

/// 验证报表快照完整性
pub async fn verify_snapshot(
    Path(id): Path<i32>,
    State(state): State<AppState>,
    auth: AuthContext,
) -> Result<Json<ApiResponse<bool>>, AppError> {
    info!("用户 {} 正在验证报表快照 {} 的完整性", auth.user_id, id);

    let service = PeriodReportSnapshotService::new(state.db.clone());
    let is_valid = service.verify_integrity(id).await?;

    Ok(Json(ApiResponse::success(is_valid)))
}
