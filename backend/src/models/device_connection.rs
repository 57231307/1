#![allow(dead_code)]
//! 设备连接管理 Model（V15 P2 B05-P2-7 创建）
//!
//! 表 device_connection：记录 PDA / 工控终端 / 扫码枪等车间设备与服务端的连接资源。
//! 设备注册（register）后通过心跳（heartbeat）维持在线状态；超时未心跳则被定时任务标记为 timeout。
//! 状态机：online（在线）→ offline（主动下线）/ timeout（心跳超时）
//! 唯一约束：device_id 一条记录代表一台设备的最新连接状态（重复注册走 upsert 路径）

use chrono::{DateTime, Utc};
use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

/// 设备连接状态常量（status 字段值）
pub mod connection_status {
    /// 在线：设备已注册且心跳在阈值内
    pub const ONLINE: &str = "online";

    /// 离线：设备主动调用 disconnect 下线
    pub const OFFLINE: &str = "offline";

    /// 超时：心跳超时被定时任务自动标记
    pub const TIMEOUT: &str = "timeout";
}

/// 设备类型常量（device_type 字段值）
pub mod device_type {
    /// PDA 手持终端
    pub const PDA: &str = "pda";

    /// 工控终端（触摸屏一体机）
    pub const INDUSTRIAL_TERMINAL: &str = "industrial_terminal";

    /// 扫码枪
    pub const SCANNER: &str = "scanner";

    /// 其他设备
    pub const OTHER: &str = "other";
}

/// 设备连接记录模型（一台设备一条记录，状态随注册/心跳/下线流转）
#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Eq, Serialize, Deserialize)]
#[sea_orm(table_name = "device_connection")]
pub struct Model {
    #[sea_orm(primary_key)]
    pub id: i64,
    /// 设备唯一标识（PDA 序列号 / MAC / 自定义编号）
    pub device_id: String,
    /// 设备名称（友好描述，便于运维识别）
    pub device_name: Option<String>,
    /// 设备类型：pda / industrial_terminal / scanner / other
    pub device_type: String,
    /// 关联操作员 ID（可选，登录后绑定）
    pub user_id: Option<i32>,
    /// 冗余操作员姓名（便于报表查询）
    pub username: Option<String>,
    /// 车间编码（设备所属车间，便于按车间汇总在线设备数）
    pub workshop: Option<String>,
    /// 设备 IP 地址（便于网络诊断与审计）
    pub ip_address: Option<String>,
    /// 当前会话 token（注册时由服务端签发，下线时清理）
    pub session_token: Option<String>,
    /// 状态：online / offline / timeout
    pub status: String,
    /// 最近一次心跳时间（定时任务依据此字段判定超时）
    pub last_heartbeat_at: DateTime<Utc>,
    /// 当前连接建立时间（注册或重新上线时刷新）
    pub connected_at: DateTime<Utc>,
    /// 最近一次断开时间（下线或超时时刷新）
    pub disconnected_at: Option<DateTime<Utc>>,
    /// 附加元数据（JSONB，例如固件版本 / 屏幕分辨率）
    #[sea_orm(column_type = "Json", nullable)]
    pub metadata: Option<serde_json::Value>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {
    #[sea_orm(
        belongs_to = "super::user::Entity",
        from = "Column::UserId",
        to = "super::user::Column::Id"
    )]
    User,
}

impl Related<super::user::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::User.def()
    }
}

impl ActiveModelBehavior for ActiveModel {}
