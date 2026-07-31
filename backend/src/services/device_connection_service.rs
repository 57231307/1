//! 设备连接管理服务（V15 P2 B05-P2-7）
//!
//! 提供 PDA / 工控终端 / 扫码枪等车间设备的连接生命周期管理：
//! - register：设备注册（首次或重新上线），返回服务端签发的 session_token
//! - heartbeat：刷新心跳时间（设备在线时定期调用）
//! - disconnect：主动下线（清理 session_token、置 offline）
//! - cleanup_timeout：批量标记心跳超时设备为 timeout（由后台定时任务调用）
//! - list_devices：按状态/类型/车间/操作员过滤分页查询
//! - count_online：在线设备数（看板高频查询）
//! - get_device：按 device_id 查询详情
//!
//! 心跳超时阈值由环境变量 DEVICE_HEARTBEAT_TIMEOUT_SECS 控制（默认 300 秒=5 分钟）。
//! 调用方：handlers/device_connection_handler.rs + bootstrap/service_bootstrap.rs（定时清理任务）

use chrono::{DateTime, Utc};
use sea_orm::{
    ActiveModelTrait, ColumnTrait, DatabaseConnection, EntityTrait, PaginatorTrait, QueryFilter,
    QueryOrder, QuerySelect, Set,
};
use std::sync::Arc;
use tracing::{info, warn};

use crate::container::AppState;
use crate::models::device_connection::connection_status;
use crate::models::device_connection::{self, ActiveModel, Entity, Model};
use crate::models::device_connection_dto::{
    HeartbeatRequest, ListDeviceConnectionQuery, RegisterDeviceRequest,
};
use crate::utils::error::AppError;

/// 默认心跳超时阈值（秒）— 5 分钟内无心跳则视为超时
pub const DEFAULT_HEARTBEAT_TIMEOUT_SECS: u64 = 300;

/// 默认后台清理任务扫描间隔（秒）— 每 60 秒扫描一次超时设备
pub const DEFAULT_CLEANUP_INTERVAL_SECS: u64 = 60;

/// 默认设备类型（请求未指定时使用）
const DEFAULT_DEVICE_TYPE: &str = "other";

/// 设备连接服务
pub struct DeviceConnectionService {
    db: Arc<DatabaseConnection>,
}

impl DeviceConnectionService {
    pub fn new(db: Arc<DatabaseConnection>) -> Self {
        Self { db }
    }

    pub fn from_state(state: &AppState) -> Self {
        Self::new(state.db.clone())
    }

    /// 注册设备（首次 INSERT 或重复注册 UPDATE 切 online），返回服务端签发的 session_token。
    pub async fn register(&self, req: RegisterDeviceRequest) -> Result<Model, AppError> {
        let now = Utc::now();
        let session_token = generate_session_token(&req.device_id, now);
        let device_type = req
            .device_type
            .filter(|s| !s.trim().is_empty())
            .unwrap_or_else(|| DEFAULT_DEVICE_TYPE.to_string());

        let existing = Entity::find()
            .filter(device_connection::Column::DeviceId.eq(&req.device_id))
            .one(&*self.db)
            .await?;

        let model = if let Some(existing_model) = existing {
            // 重复注册：重新上线，刷新连接时间与状态
            let mut active: ActiveModel = existing_model.into();
            active.device_name = Set(req.device_name);
            active.device_type = Set(device_type);
            active.user_id = Set(req.user_id);
            active.username = Set(req.username);
            active.workshop = Set(req.workshop);
            active.ip_address = Set(req.ip_address);
            active.session_token = Set(Some(session_token));
            active.status = Set(connection_status::ONLINE.to_string());
            active.last_heartbeat_at = Set(now);
            active.connected_at = Set(now);
            active.disconnected_at = Set(None);
            active.metadata = Set(req.metadata);
            active.updated_at = Set(now);
            active.update(&*self.db).await?
        } else {
            // 首次注册
            let active = ActiveModel {
                id: Default::default(),
                device_id: Set(req.device_id),
                device_name: Set(req.device_name),
                device_type: Set(device_type),
                user_id: Set(req.user_id),
                username: Set(req.username),
                workshop: Set(req.workshop),
                ip_address: Set(req.ip_address),
                session_token: Set(Some(session_token)),
                status: Set(connection_status::ONLINE.to_string()),
                last_heartbeat_at: Set(now),
                connected_at: Set(now),
                disconnected_at: Set(None),
                metadata: Set(req.metadata),
                created_at: Set(now),
                updated_at: Set(now),
            };
            active.insert(&*self.db).await?
        };
        info!(
            device_id = %model.device_id,
            device_type = %model.device_type,
            "设备已注册上线（B05-P2-7 设备连接资源管理）"
        );
        Ok(model)
    }

    /// 心跳上报（刷新 last_heartbeat_at，可选更新部分字段）。
    /// 幂等：若设备不存在则返回业务错误（要求先调用 register）；若已 offline/timeout 自动恢复 online。
    pub async fn heartbeat(
        &self,
        device_id: &str,
        req: HeartbeatRequest,
    ) -> Result<Model, AppError> {
        let existing = Entity::find()
            .filter(device_connection::Column::DeviceId.eq(device_id))
            .one(&*self.db)
            .await?
            .ok_or_else(|| {
                AppError::business(format!("设备 {} 未注册，请先调用 register 端点", device_id))
            })?;

        let now = Utc::now();
        let was_online = existing.status == connection_status::ONLINE;
        let mut active: ActiveModel = existing.into();
        active.last_heartbeat_at = Set(now);
        active.updated_at = Set(now);
        // 心跳自动恢复在线状态（设备从 timeout/offline 重新上报心跳视为重新活跃）
        if !was_online {
            active.status = Set(connection_status::ONLINE.to_string());
            active.connected_at = Set(now);
            active.disconnected_at = Set(None);
        }
        if req.user_id.is_some() {
            active.user_id = Set(req.user_id);
        }
        if req.username.is_some() {
            active.username = Set(req.username);
        }
        if req.workshop.is_some() {
            active.workshop = Set(req.workshop);
        }
        if req.ip_address.is_some() {
            active.ip_address = Set(req.ip_address);
        }
        if req.metadata.is_some() {
            active.metadata = Set(req.metadata);
        }
        let updated = active.update(&*self.db).await?;
        Ok(updated)
    }

    /// 主动下线（清理 session_token，置 offline，记录 disconnected_at）。
    /// 幂等：设备不存在或已下线均不报错（返回 None）。
    pub async fn disconnect(&self, device_id: &str) -> Result<Option<Model>, AppError> {
        let existing = Entity::find()
            .filter(device_connection::Column::DeviceId.eq(device_id))
            .filter(device_connection::Column::Status.eq(connection_status::ONLINE))
            .one(&*self.db)
            .await?;
        let Some(model) = existing else {
            warn!(
                device_id,
                "设备无在线记录或不存在，跳过下线（B05-P2-7 幂等）"
            );
            return Ok(None);
        };
        let now = Utc::now();
        let mut active: ActiveModel = model.into();
        active.session_token = Set(None);
        active.status = Set(connection_status::OFFLINE.to_string());
        active.disconnected_at = Set(Some(now));
        active.updated_at = Set(now);
        let updated = active.update(&*self.db).await?;
        info!(
            device_id = %updated.device_id,
            "设备已主动下线（B05-P2-7 设备连接资源管理）"
        );
        Ok(Some(updated))
    }

    /// 清理心跳超时设备（批量将 status=online 且 last_heartbeat_at < 阈值的记录置为 timeout）。
    /// 由后台定时任务调用，返回本次被标记超时的设备数量。
    pub async fn cleanup_timeout(&self, timeout_secs: u64) -> Result<u64, AppError> {
        let threshold = Utc::now() - chrono::Duration::seconds(timeout_secs as i64);
        // 先查询符合条件的设备 ID（避免全表 UPDATE 影响行锁范围过大）
        let timed_out = Entity::find()
            .filter(device_connection::Column::Status.eq(connection_status::ONLINE))
            .filter(device_connection::Column::LastHeartbeatAt.lt(threshold))
            .limit(500)
            .all(&*self.db)
            .await?;

        let count = timed_out.len() as u64;
        if count == 0 {
            return Ok(0);
        }

        let now = Utc::now();
        for model in timed_out {
            let mut active: ActiveModel = model.into();
            active.status = Set(connection_status::TIMEOUT.to_string());
            active.disconnected_at = Set(Some(now));
            active.updated_at = Set(now);
            if let Err(e) = active.update(&*self.db).await {
                warn!(
                    error = %e,
                    "设备超时标记失败，跳过继续（B05-P2-7 单条失败不阻断）"
                );
            }
        }
        info!(
            count,
            threshold_secs = timeout_secs,
            "心跳超时清理完成：本轮标记 {} 台设备为 timeout（B05-P2-7）",
            count
        );
        Ok(count)
    }

    /// 按条件分页查询设备列表。
    pub async fn list_devices(
        &self,
        query: &ListDeviceConnectionQuery,
    ) -> Result<(Vec<Model>, u64), AppError> {
        let page = query.page.unwrap_or(1).clamp(1, 1000);
        let page_size = query.page_size.unwrap_or(20).clamp(1, 200);

        let mut select = Entity::find().order_by_desc(device_connection::Column::LastHeartbeatAt);
        if let Some(s) = &query.status {
            select = select.filter(device_connection::Column::Status.eq(s));
        }
        if let Some(t) = &query.device_type {
            select = select.filter(device_connection::Column::DeviceType.eq(t));
        }
        if let Some(w) = &query.workshop {
            select = select.filter(device_connection::Column::Workshop.eq(w));
        }
        if let Some(u) = query.user_id {
            select = select.filter(device_connection::Column::UserId.eq(u));
        }

        let total = select.clone().count(&*self.db).await?;
        let items = select
            .offset((page - 1) * page_size)
            .limit(page_size)
            .all(&*self.db)
            .await?;
        Ok((items, total))
    }

    /// 按 device_id 查询设备详情。
    pub async fn get_device(&self, device_id: &str) -> Result<Model, AppError> {
        Entity::find()
            .filter(device_connection::Column::DeviceId.eq(device_id))
            .one(&*self.db)
            .await?
            .ok_or_else(|| AppError::not_found(format!("设备 {} 不存在", device_id)))
    }

    /// 在线设备数（看板高频查询，可按车间过滤）。
    pub async fn count_online(&self, workshop: Option<&str>) -> Result<u64, AppError> {
        let mut select =
            Entity::find().filter(device_connection::Column::Status.eq(connection_status::ONLINE));
        if let Some(w) = workshop {
            select = select.filter(device_connection::Column::Workshop.eq(w));
        }
        let count = select.count(&*self.db).await?;
        Ok(count)
    }
}

/// 生成会话 token（device_id + 当前时间戳 + 随机数 拼接后哈希）。
/// 不引入额外依赖，使用简单拼接保证唯一性与可追溯性。
fn generate_session_token(device_id: &str, now: DateTime<Utc>) -> String {
    let nonce: u64 = rand_u64();
    format!(
        "dc_{}_{}_{}",
        device_id.replace('|', "_"),
        now.timestamp_millis(),
        nonce
    )
}

/// 简单线性同余生成器（无需引入 rand crate，足够生成会话 token 随机后缀）。
fn rand_u64() -> u64 {
    use std::time::{SystemTime, UNIX_EPOCH};
    let seed = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_nanos() as u64)
        .unwrap_or(0x9E37_79B9_7F4A_7C15);
    // LCG: x = a * x + c
    let x = seed
        .wrapping_mul(6_364_136_223_846_793_005)
        .wrapping_add(1_442_695_040_888_963_407);
    x
}
