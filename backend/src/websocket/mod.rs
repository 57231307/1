//! WebSocket 实时通信模块
//!
//! P3-2 关键路径 demo：通知模块 WebSocket 实现
//!
//! 子模块：
//! - [`notifications`]：通知模块 WebSocket handler + 连接管理 + 广播 + 票据鉴权
//!
//! 路由：
//! - `POST /api/v1/erp/ws/ticket`：签发一次性短时票据（需 JWT 认证）
//! - `GET /api/v1/erp/ws/notifications?ticket=<票据>`：通知实时推送
//!
//! 功能：
//! - 服务端主动推送新通知给客户端
//! - 客户端心跳（ping → pong）
//! - 连接管理（按 user_id 分组）
//! - 鉴权（一次性短时票据，v12 P1-4 修复：替代 URL query JWT）

pub mod notifications;
