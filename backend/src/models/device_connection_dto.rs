#![allow(dead_code)]
//! 设备连接管理 DTO（V15 P2 B05-P2-7 创建）
//!
//! 包含 PDA / 工控终端设备注册、心跳、下线、查询的请求/响应 DTO

use serde::{Deserialize, Serialize};

/// 设备注册请求（首次注册或重新上线均走此端点）
#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct RegisterDeviceRequest {
    /// 设备唯一标识（PDA 序列号 / MAC / 自定义编号）
    pub device_id: String,
    /// 设备名称（友好描述）
    pub device_name: Option<String>,
    /// 设备类型：pda / industrial_terminal / scanner / other（默认 other）
    pub device_type: Option<String>,
    /// 关联操作员 ID（可选，登录后绑定）
    pub user_id: Option<i32>,
    /// 操作员姓名（冗余，便于报表）
    pub username: Option<String>,
    /// 车间编码
    pub workshop: Option<String>,
    /// 设备 IP 地址
    pub ip_address: Option<String>,
    /// 附加元数据（固件版本 / 屏幕分辨率等）
    pub metadata: Option<serde_json::Value>,
}

/// 心跳请求（设备定期上报，仅刷新 last_heartbeat_at；可选携带新元数据）
#[derive(Debug, Deserialize, Serialize, Clone, Default)]
pub struct HeartbeatRequest {
    /// 操作员 ID（可选，登录态变更时同步更新）
    pub user_id: Option<i32>,
    /// 操作员姓名（冗余）
    pub username: Option<String>,
    /// 车间编码（可选，设备移动时变更）
    pub workshop: Option<String>,
    /// 设备 IP（可选，网络切换时变更）
    pub ip_address: Option<String>,
    /// 附加元数据（可选）
    pub metadata: Option<serde_json::Value>,
}

/// 设备列表查询参数
#[derive(Debug, Deserialize, Serialize, Clone, Default)]
pub struct ListDeviceConnectionQuery {
    /// 按状态过滤：online / offline / timeout
    pub status: Option<String>,
    /// 按设备类型过滤：pda / industrial_terminal / scanner / other
    pub device_type: Option<String>,
    /// 按车间过滤
    pub workshop: Option<String>,
    /// 按操作员过滤
    pub user_id: Option<i32>,
    /// 分页页码（默认 1）
    pub page: Option<u64>,
    /// 分页大小（默认 20，最大 200）
    pub page_size: Option<u64>,
}
