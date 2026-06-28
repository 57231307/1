//! WebSocket 实时通信模块
//!
//! P3-2 关键路径 demo：通知模块 WebSocket 实现
//!
//! 子模块：
//! - [`notifications`]：通知模块 WebSocket handler + 连接管理 + 广播
//!
//! 路由：`/api/v1/erp/ws/notifications?token=<JWT>`
//!
//! 功能：
//! - 服务端主动推送新通知给客户端
//! - 客户端心跳（ping → pong）
//! - 连接管理（按 user_id 分组）
//! - 鉴权（URL query 携带 JWT）

pub mod notifications;

pub use notifications::{ConnectionManager, NotificationBroadcaster, WsMessage};
