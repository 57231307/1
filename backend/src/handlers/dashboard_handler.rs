use axum::{
    extract::{Query, State},
    Json,
};
use chrono::{DateTime, NaiveDate, TimeZone, Utc};
use sea_orm::{ActiveModelTrait, ColumnTrait, EntityTrait, QueryFilter, Set};
use serde::Deserialize;

use crate::container::AppState;
use crate::middleware::auth_context::AuthContext;
use crate::models::dashboard_layout::{self, Entity as DashboardLayoutEntity};
use crate::services::dashboard_service::DashboardService;
use crate::services::dashboard_service::{
    DashboardOverview, InventoryStatistics, LowStockAlert, SalesStatistics,
};
use crate::utils::error::AppError;
use crate::utils::response::ApiResponse;

/// 查询参数 - 仪表板
#[derive(Debug, Deserialize)]
pub struct DashboardQuery {
    pub start_date: Option<NaiveDate>,
    pub end_date: Option<NaiveDate>,
}

/// 缺陷 4.1 修复：保存用户仪表板卡片布局请求
#[derive(Debug, Deserialize)]
pub struct SaveLayoutRequest {
    /// 卡片配置 JSON（卡片顺序、可见性、尺寸等）
    pub card_config: serde_json::Value,
    /// 是否默认布局
    pub is_default: Option<bool>,
}

/// 缺陷 4.1 修复：用户仪表板布局响应
#[derive(Debug, serde::Serialize)]
pub struct DashboardLayoutResponse {
    pub user_id: i32,
    pub card_config: serde_json::Value,
    pub is_default: bool,
    pub updated_at: chrono::DateTime<Utc>,
}

/// 将 NaiveDate 转换为 DateTime<Utc>（一天的开始）
fn naive_date_to_utc(date: NaiveDate) -> Option<DateTime<Utc>> {
    date.and_hms_opt(0, 0, 0)
        .map(|dt| Utc.from_utc_datetime(&dt))
}

/// 缺陷 4.1 修复：获取当前用户仪表板布局配置
pub async fn get_dashboard_layout(
    State(state): State<AppState>,
    auth: AuthContext,
) -> Result<Json<ApiResponse<DashboardLayoutResponse>>, AppError> {
    let layout = DashboardLayoutEntity::find()
        .filter(dashboard_layout::Column::UserId.eq(auth.user_id))
        .one(&*state.db)
        .await?;

    let response = match layout {
        Some(l) => DashboardLayoutResponse {
            user_id: l.user_id,
            card_config: l.card_config,
            is_default: l.is_default,
            updated_at: l.updated_at,
        },
        None => DashboardLayoutResponse {
            user_id: auth.user_id,
            card_config: serde_json::json!({
                "cards": ["overview", "sales", "inventory", "low_stock_alerts"]
            }),
            is_default: true,
            updated_at: Utc::now(),
        },
    };
    Ok(Json(ApiResponse::success(response)))
}

/// 缺陷 4.1 修复：保存或更新当前用户仪表板布局配置
pub async fn save_dashboard_layout(
    State(state): State<AppState>,
    auth: AuthContext,
    Json(req): Json<SaveLayoutRequest>,
) -> Result<Json<ApiResponse<DashboardLayoutResponse>>, AppError> {
    let now = Utc::now();
    let is_default = req.is_default.unwrap_or(false);

    let existing = DashboardLayoutEntity::find()
        .filter(dashboard_layout::Column::UserId.eq(auth.user_id))
        .one(&*state.db)
        .await?;

    let layout = if let Some(model) = existing {
        let mut active: dashboard_layout::ActiveModel = model.into();
        active.card_config = Set(req.card_config.clone());
        active.is_default = Set(is_default);
        active.updated_at = Set(now);
        active.update(&*state.db).await?
    } else {
        let active = dashboard_layout::ActiveModel {
            user_id: Set(auth.user_id),
            card_config: Set(req.card_config.clone()),
            is_default: Set(is_default),
            created_at: Set(now),
            updated_at: Set(now),
            ..Default::default()
        };
        active.insert(&*state.db).await?
    };

    let response = DashboardLayoutResponse {
        user_id: layout.user_id,
        card_config: layout.card_config,
        is_default: layout.is_default,
        updated_at: layout.updated_at,
    };
    Ok(Json(ApiResponse::success(response)))
}

/// 获取仪表板概览数据（500ms 超时保护，避免慢查询阻塞首屏）
pub async fn get_dashboard_overview(
    State(state): State<AppState>,
    auth: AuthContext,
    Query(query): Query<DashboardQuery>,
) -> Result<Json<ApiResponse<DashboardOverview>>, AppError> {
    let ctx = auth.to_data_scope_context();
    let dashboard_service =
        DashboardService::new_with_data_scope(state.db.clone(), state.cache.clone(), ctx);
    let start_datetime = query.start_date.and_then(naive_date_to_utc);
    let end_datetime = query.end_date.and_then(naive_date_to_utc);
    let overview = tokio::time::timeout(
        std::time::Duration::from_millis(500),
        dashboard_service.get_overview(start_datetime, end_datetime),
    )
    .await
    .map_err(|_| AppError::internal("仪表板概览查询超时（500ms）".to_string()))??;

    // 缺陷 4.2 修复：仪表板数据更新触发 WebSocket 实时推送
    crate::websocket::notifications::get_notification_broadcaster().broadcast_dashboard_update(
        auth.user_id as i64,
        "overview_refresh",
        &serde_json::to_value(&overview).unwrap_or(serde_json::Value::Null),
    );

    Ok(Json(ApiResponse::success(overview)))
}

/// 获取销售统计数据
pub async fn get_sales_statistics(
    State(state): State<AppState>,
    auth: AuthContext,
    Query(query): Query<DashboardQuery>,
) -> Result<Json<ApiResponse<SalesStatistics>>, AppError> {
    let ctx = auth.to_data_scope_context();
    let dashboard_service =
        DashboardService::new_with_data_scope(state.db.clone(), state.cache.clone(), ctx);
    let start_datetime = query.start_date.and_then(naive_date_to_utc);
    let end_datetime = query.end_date.and_then(naive_date_to_utc);
    let stats = dashboard_service
        .get_sales_statistics(start_datetime, end_datetime)
        .await?;
    Ok(Json(ApiResponse::success(stats)))
}

/// 获取库存统计数据
pub async fn get_inventory_statistics(
    State(state): State<AppState>,
    _auth: AuthContext,
    Query(query): Query<DashboardQuery>,
) -> Result<Json<ApiResponse<InventoryStatistics>>, AppError> {
    let dashboard_service = DashboardService::new(state.db.clone(), state.cache.clone());
    let start_datetime = query.start_date.and_then(naive_date_to_utc);
    let end_datetime = query.end_date.and_then(naive_date_to_utc);
    let stats = dashboard_service
        .get_inventory_statistics(start_datetime, end_datetime)
        .await?;
    Ok(Json(ApiResponse::success(stats)))
}

/// 获取低库存预警数据
pub async fn get_low_stock_alerts(
    State(state): State<AppState>,
    _auth: AuthContext,
) -> Result<Json<ApiResponse<Vec<LowStockAlert>>>, AppError> {
    let dashboard_service = DashboardService::new(state.db.clone(), state.cache.clone());
    let alerts = dashboard_service.get_low_stock_alerts().await?;
    Ok(Json(ApiResponse::success(alerts)))
}

/// batch-17 P3: 系统资源看板数据
#[derive(Debug, serde::Serialize)]
pub struct SystemResourceDashboard {
    pub cpu_usage: f64,
    pub memory_usage: f64,
    pub disk_usage: f64,
    pub uptime_seconds: u64,
    pub active_connections: u32,
    pub database_connections: u32,
    pub cache_hit_rate: f64,
}

/// GET /api/v1/erp/dashboard/system-resources - 系统资源看板
pub async fn get_system_resources(
    State(state): State<AppState>,
    _auth: AuthContext,
) -> Result<Json<ApiResponse<SystemResourceDashboard>>, AppError> {
    // 获取数据库连接数
    let db_connections_sql = "SELECT COUNT(*) as count FROM pg_stat_activity WHERE state = 'active'";
    let db_result = state
        .db
        .query_one(sea_orm::Statement::from_string(
            sea_orm::DatabaseBackend::Postgres,
            db_connections_sql.to_string(),
        ))
        .await
        .map_err(|e| AppError::internal(format!("查询数据库连接数失败: {}", e)))?;
    let database_connections = db_result
        .map(|r| r.try_get::<i64>("", "count").unwrap_or(0) as u32)
        .unwrap_or(0);

    // 获取缓存命中率（从 metrics 服务）
    let cache_hit_rate = 0.0; // TODO: 从 metrics 服务获取

    let dashboard = SystemResourceDashboard {
        cpu_usage: 0.0,    // TODO: 从系统指标获取
        memory_usage: 0.0, // TODO: 从系统指标获取
        disk_usage: 0.0,   // TODO: 从系统指标获取
        uptime_seconds: 0, // TODO: 从系统指标获取
        active_connections: 0,
        database_connections,
        cache_hit_rate,
    };

    Ok(Json(ApiResponse::success(dashboard)))
}
