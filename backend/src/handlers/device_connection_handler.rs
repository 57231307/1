//! 设备连接管理 Handler（V15 P2 B05-P2-7）
//!
//! 实现 7 个 HTTP 端点：
//! - POST   /register                  设备注册（首次或重新上线）
//! - POST   /:device_id/heartbeat      心跳上报
//! - POST   /:device_id/disconnect     主动下线
//! - GET    /                          设备列表（按状态/类型/车间/操作员过滤分页）
//! - GET    /:device_id                设备详情
//! - GET    /online/count              在线设备数（可按车间过滤）
//! - POST   /cleanup-timeout           手动触发超时清理（运维排障用，正常由后台任务自动执行）

use axum::{
    extract::{Path, Query, State},
    Json,
};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use crate::container::AppState;
use crate::handlers::bad_debt_handler::PagedResponse;
use crate::middleware::auth_context::AuthContext;
use crate::models::device_connection::Model;
use crate::models::device_connection_dto::{
    HeartbeatRequest, ListDeviceConnectionQuery, RegisterDeviceRequest,
};
use crate::services::device_connection_service::DeviceConnectionService;
use crate::utils::error::AppError;
use crate::utils::response::ApiResponse;

// ==================== 响应 DTO ====================

/// 设备连接信息（响应前端用，与 Model 字段一一对应）
#[derive(Debug, Serialize, Clone)]
pub struct DeviceConnectionInfo {
    pub id: i64,
    pub device_id: String,
    pub device_name: Option<String>,
    pub device_type: String,
    pub user_id: Option<i32>,
    pub username: Option<String>,
    pub workshop: Option<String>,
    pub ip_address: Option<String>,
    pub session_token: Option<String>,
    pub status: String,
    pub last_heartbeat_at: DateTime<Utc>,
    pub connected_at: DateTime<Utc>,
    pub disconnected_at: Option<DateTime<Utc>>,
    pub metadata: Option<serde_json::Value>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl From<Model> for DeviceConnectionInfo {
    fn from(m: Model) -> Self {
        Self {
            id: m.id,
            device_id: m.device_id,
            device_name: m.device_name,
            device_type: m.device_type,
            user_id: m.user_id,
            username: m.username,
            workshop: m.workshop,
            ip_address: m.ip_address,
            session_token: m.session_token,
            status: m.status,
            last_heartbeat_at: m.last_heartbeat_at,
            connected_at: m.connected_at,
            disconnected_at: m.disconnected_at,
            metadata: m.metadata,
            created_at: m.created_at,
            updated_at: m.updated_at,
        }
    }
}

/// 在线设备数响应
#[derive(Debug, Serialize, Clone)]
pub struct OnlineCountResponse {
    pub online_count: u64,
    pub workshop: Option<String>,
}

/// 超时清理手动触发请求（运维排障用）
#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct CleanupTimeoutRequest {
    /// 心跳超时阈值（秒，默认 300=5 分钟）
    pub timeout_secs: Option<u64>,
}

/// 超时清理响应
#[derive(Debug, Serialize, Clone)]
pub struct CleanupTimeoutResponse {
    pub timed_out_count: u64,
}

// ==================== 端点实现 ====================

/// POST /api/v1/erp/device-connections/register - 设备注册（首次或重新上线）
pub async fn register_device(
    _auth: AuthContext,
    State(state): State<AppState>,
    Json(req): Json<RegisterDeviceRequest>,
) -> Result<Json<ApiResponse<DeviceConnectionInfo>>, AppError> {
    if req.device_id.trim().is_empty() {
        return Err(AppError::business("device_id 不能为空"));
    }
    let service = DeviceConnectionService::from_state(&state);
    let model = service.register(req).await?;
    Ok(Json(ApiResponse::success(model.into())))
}

/// POST /api/v1/erp/device-connections/:device_id/heartbeat - 心跳上报
pub async fn heartbeat(
    _auth: AuthContext,
    State(state): State<AppState>,
    Path(device_id): Path<String>,
    Json(req): Json<HeartbeatRequest>,
) -> Result<Json<ApiResponse<DeviceConnectionInfo>>, AppError> {
    let service = DeviceConnectionService::from_state(&state);
    let model = service.heartbeat(&device_id, req).await?;
    Ok(Json(ApiResponse::success(model.into())))
}

/// POST /api/v1/erp/device-connections/:device_id/disconnect - 主动下线
pub async fn disconnect(
    _auth: AuthContext,
    State(state): State<AppState>,
    Path(device_id): Path<String>,
) -> Result<Json<ApiResponse<Option<DeviceConnectionInfo>>>, AppError> {
    let service = DeviceConnectionService::from_state(&state);
    let model = service.disconnect(&device_id).await?;
    Ok(Json(ApiResponse::success(model.map(Into::into))))
}

/// GET /api/v1/erp/device-connections - 设备列表（按状态/类型/车间/操作员过滤分页）
pub async fn list_devices(
    _auth: AuthContext,
    State(state): State<AppState>,
    Query(query): Query<ListDeviceConnectionQuery>,
) -> Result<Json<ApiResponse<PagedResponse<DeviceConnectionInfo>>>, AppError> {
    let service = DeviceConnectionService::from_state(&state);
    let page = query.page.unwrap_or(1).clamp(1, 1000);
    let page_size = query.page_size.unwrap_or(20).clamp(1, 200);
    let (items, total) = service.list_devices(&query).await?;
    let infos: Vec<DeviceConnectionInfo> = items.into_iter().map(Into::into).collect();
    Ok(Json(ApiResponse::success(PagedResponse {
        items: infos,
        total,
        page,
        page_size,
    })))
}

/// GET /api/v1/erp/device-connections/:device_id - 设备详情
pub async fn get_device(
    _auth: AuthContext,
    State(state): State<AppState>,
    Path(device_id): Path<String>,
) -> Result<Json<ApiResponse<DeviceConnectionInfo>>, AppError> {
    let service = DeviceConnectionService::from_state(&state);
    let model = service.get_device(&device_id).await?;
    Ok(Json(ApiResponse::success(model.into())))
}

/// GET /api/v1/erp/device-connections/online/count - 在线设备数（可按车间过滤）
pub async fn count_online(
    _auth: AuthContext,
    State(state): State<AppState>,
    Query(params): Query<OnlineCountQuery>,
) -> Result<Json<ApiResponse<OnlineCountResponse>>, AppError> {
    let service = DeviceConnectionService::from_state(&state);
    // 先克隆 workshop 避免跨 await 借用，再传入 service（service 只持有 &str，await 期间不持有 workshop 所有权）
    let workshop: Option<String> = params.workshop;
    let count = service.count_online(workshop.as_deref()).await?;
    Ok(Json(ApiResponse::success(OnlineCountResponse {
        online_count: count,
        workshop,
    })))
}

/// 在线设备数查询参数
#[derive(Debug, Deserialize, Serialize, Clone, Default)]
pub struct OnlineCountQuery {
    pub workshop: Option<String>,
}

/// POST /api/v1/erp/device-connections/cleanup-timeout - 手动触发超时清理
pub async fn cleanup_timeout(
    _auth: AuthContext,
    State(state): State<AppState>,
    Json(req): Json<CleanupTimeoutRequest>,
) -> Result<Json<ApiResponse<CleanupTimeoutResponse>>, AppError> {
    let timeout_secs = req
        .timeout_secs
        .unwrap_or(crate::services::device_connection_service::DEFAULT_HEARTBEAT_TIMEOUT_SECS);
    let service = DeviceConnectionService::from_state(&state);
    let count = service.cleanup_timeout(timeout_secs).await?;
    Ok(Json(ApiResponse::success(CleanupTimeoutResponse {
        timed_out_count: count,
    })))
}
