//! 通知模块 WebSocket handler
//!
//! 路径：`/api/v1/erp/ws/notifications?token=<JWT>`
//!
//! 核心功能：
//! 1. 握手时验证 JWT（提取 user_id）
//! 2. 升级到 WebSocket 协议
//! 3. 注册连接到 ConnectionManager（按 user_id 分组）
//! 4. 接收客户端消息（ping / mark_as_read）
//! 5. 接收服务端广播（来自 notification_service.send()）
//! 6. 断开时自动清理

use axum::extract::ws::{Message, WebSocket, WebSocketUpgrade};
use axum::extract::Query;
use axum::response::IntoResponse;
use dashmap::DashMap;
use futures::{FutureExt, SinkExt, StreamExt};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::panic::AssertUnwindSafe;
use std::sync::{Arc, OnceLock};
use tokio::sync::broadcast;

use crate::services::auth_service::AuthService;

/// WebSocket 消息类型
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum WsMessage {
    /// 通知消息（服务端 → 客户端）
    Notification { data: NotificationPayload },
    /// 心跳请求（客户端 → 服务端）
    Ping { timestamp: i64 },
    /// 心跳响应（服务端 → 客户端）
    Pong { timestamp: i64 },
    /// 错误消息（服务端 → 客户端）
    Error { code: String, message: String },
    /// 标记已读（客户端 → 服务端）
    MarkAsRead { id: i64 },
}

/// 通知数据载荷
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NotificationPayload {
    pub id: i64,
    pub title: String,
    pub content: String,
    pub category: String,
    pub priority: i32,
    pub created_at: String,
}

/// 鉴权信息（从 JWT 提取）
#[derive(Debug, Clone)]
pub struct AuthInfo {
    pub user_id: i64,
}

/// 连接管理器（全局单例）
///
/// Key: user_id
/// Value: broadcast::Sender（一个用户可能有多个连接，例如多端登录）
#[derive(Clone, Default)]
pub struct ConnectionManager {
    senders: Arc<DashMap<i64, broadcast::Sender<String>>>,
}

impl ConnectionManager {
    /// 创建新连接管理器
    pub fn new() -> Self {
        Self::default()
    }

    /// 注册新连接
    ///
    /// 返回 `broadcast::Receiver`，handler 用来接收广播消息
    pub fn register(&self, user_id: i64) -> broadcast::Receiver<String> {
        let key = user_id;
        // 防御 clippy::unused_mut：entry 仅通过 Deref 调用 subscribe()，无需 mut
        let entry = self.senders.entry(key).or_insert_with(|| {
            // 初始容量 100，多端登录时自动扩容
            let (tx, _rx) = broadcast::channel(100);
            tx
        });
        entry.subscribe()
    }

    /// 注销连接（handler drop 时调用）
    pub fn unregister(&self, user_id: i64) {
        let key = user_id;
        // 仅在无活跃订阅者时清理（dashmap entry API）
        if let Some(entry) = self.senders.get(&key) {
            if entry.receiver_count() == 0 {
                drop(entry);
                self.senders.remove(&key);
            }
        }
    }

    /// 广播通知给指定用户
    ///
    /// 用途：notification_service.send() 调用此方法推送新通知
    pub fn broadcast(&self, user_id: i64, message: String) {
        let key = user_id;
        if let Some(tx) = self.senders.get(&key) {
            // 发送失败说明无活跃订阅者，忽略即可
            let _ = tx.send(message);
        }
    }
}

/// 全局 NotificationBroadcaster
///
/// 用途：notification_service.send() 通过这个全局对象广播新通知
#[derive(Clone, Default)]
pub struct NotificationBroadcaster {
    manager: ConnectionManager,
}

impl NotificationBroadcaster {
    pub fn new() -> Self {
        Self {
            manager: ConnectionManager::new(),
        }
    }

    /// 广播通知（供 notification_service 调用）
    pub fn broadcast_notification(&self, user_id: i64, payload: &NotificationPayload) {
        let msg = WsMessage::Notification {
            data: payload.clone(),
        };
        if let Ok(json) = serde_json::to_string(&msg) {
            self.manager.broadcast(user_id, json);
        }
    }

    /// 获取连接管理器（供 ws handler 注册连接）
    pub fn manager(&self) -> ConnectionManager {
        self.manager.clone()
    }
}

/// 全局 NotificationBroadcaster 单例
///
/// 批次 24 v6 P0-2 修复：WebSocket 单例破坏
/// - 原实现：handle_socket 创建本地 `ConnectionManager::new()`，
///   与 NotificationBroadcaster 内部 manager 是两个不同实例，
///   即使 broadcast_notification 被调用也无法到达 ws 客户端。
/// - 修复后：所有 ws handler 通过本全局单例获取 manager，保证广播与订阅共享同一份数据。
static NOTIFICATION_BROADCASTER: OnceLock<NotificationBroadcaster> = OnceLock::new();

/// 获取全局 NotificationBroadcaster 单例
///
/// - ws handler 通过此函数注册连接（subscribe）
/// - notification_service 通过此函数广播新通知（send → broadcast_notification）
pub fn get_notification_broadcaster() -> &'static NotificationBroadcaster {
    NOTIFICATION_BROADCASTER.get_or_init(NotificationBroadcaster::new)
}

/// WebSocket 升级端点
///
/// 路径：`/api/v1/erp/ws/notifications?token=<JWT>`
///
/// 流程：
/// 1. 提取 URL query 中的 token
/// 2. 验证 JWT（提取 user_id）
/// 3. 升级 HTTP 到 WebSocket
/// 4. 进入 handle_socket() 处理消息
pub async fn ws_notifications_handler(
    ws: WebSocketUpgrade,
    Query(params): Query<HashMap<String, String>>,
) -> impl IntoResponse {
    // 1. 提取 token
    let token = match params.get("token") {
        Some(t) if !t.is_empty() => t.clone(),
        _ => {
            return Err((
                axum::http::StatusCode::UNAUTHORIZED,
                String::from("缺少 token 参数"),
            ));
        }
    };

    // 2. 验证 JWT（简化版：实际接入主项目 auth 中间件）
    let auth = match verify_jwt_token(&token) {
        Ok(a) => a,
        Err(e) => {
            return Err((
                axum::http::StatusCode::UNAUTHORIZED,
                format!("JWT 验证失败: {}", e),
            ));
        }
    };

    // 3. 升级到 WebSocket
    Ok(ws.on_upgrade(move |socket| handle_socket(socket, auth)))
}

/// 简化的 JWT 验证（占位实现）
///
/// 修复 bug.md #2 WebSocket 认证绕过：
/// - 之前实现未做 JWT 签名验证，存在认证绕过风险
/// - 当前实现复用 `AuthService::validate_token_static()` 进行真实 JWT 签名验证
pub fn verify_jwt_token(token: &str) -> Result<AuthInfo, String> {
    // 防御性检查：拒绝空 token 与过短 token（避免 jsonwebtoken panic）
    if token.is_empty() || token.len() < 16 {
        return Err("token 长度无效".to_string());
    }

    // 从环境变量获取 JWT 密钥
    let secret = std::env::var("JWT_SECRET")
        .map_err(|_| "JWT_SECRET 环境变量未配置".to_string())?;

    // 调用真实的 JWT 验证逻辑（auth_service.rs）
    let claims = AuthService::validate_token_static(token, &secret)
        .map_err(|e| format!("JWT 验证失败: {}", e))?;

    let user_id = claims.sub as i64;

    if user_id <= 0 {
        return Err("user_id 无效".to_string());
    }

    Ok(AuthInfo { user_id })
}

/// 处理 WebSocket 连接
async fn handle_socket(socket: WebSocket, auth: AuthInfo) {
    // 批次 24 v6 P0-2 修复：使用全局 NotificationBroadcaster 的 manager，
    // 与 notification_service 的 broadcast_notification 共享同一份数据。
    let manager = get_notification_broadcaster().manager();
    let mut rx = manager.register(auth.user_id);
    let (mut sender, mut receiver) = socket.split();

    tracing::info!("WebSocket 连接建立：user_id={}", auth.user_id);

    // 接收客户端消息任务
    let recv_task = tokio::spawn(async move {
        while let Some(msg) = receiver.next().await {
            // 批次 8（2026-06-28）：单次消息处理 panic 隔离
            let result = AssertUnwindSafe(async {
                match msg {
                    Ok(Message::Text(text)) => {
                        if let Ok(ws_msg) = serde_json::from_str::<WsMessage>(&text) {
                            match ws_msg {
                                WsMessage::Ping { timestamp } => {
                                    tracing::debug!(
                                        "收到 ping：user_id={}, timestamp={}",
                                        auth.user_id, timestamp
                                    );
                                }
                                WsMessage::MarkAsRead { id } => {
                                    tracing::info!(
                                        "客户端标记已读：user_id={}, id={}",
                                        auth.user_id, id
                                    );
                                }
                                _ => {
                                    tracing::warn!("收到不支持的客户端消息类型");
                                }
                            }
                        } else {
                            tracing::warn!("客户端消息 JSON 解析失败: {}", text);
                        }
                    }
                    Ok(Message::Close(_)) => {
                        tracing::info!("客户端主动关闭：user_id={}", auth.user_id);
                        return false; // break
                    }
                    Ok(Message::Ping(_)) => {}
                    Ok(Message::Pong(_)) => {}
                    Err(e) => {
                        tracing::error!("WebSocket 接收错误: {}", e);
                        return false; // break
                    }
                    _ => {}
                }
                true
            })
            .catch_unwind()
            .await;
            match result {
                Ok(true) => {}
                Ok(false) => break,
                Err(panic_payload) => {
                    let panic_msg = panic_payload
                        .downcast_ref::<String>()
                        .map(|s| s.as_str())
                        .or_else(|| panic_payload.downcast_ref::<&'static str>().copied())
                        .unwrap_or("<非字符串 panic payload>");
                    tracing::error!(
                        panic = %panic_msg,
                        "⚠ WebSocket 接收 spawn panic 已被隔离，继续运行（不退出循环）"
                    );
                }
            }
        }
    });

    // 推送消息任务（接收 broadcast 并写入 socket）
    let send_task = tokio::spawn(async move {
        while let Ok(msg) = rx.recv().await {
            // 批次 8（2026-06-28）：单次消息推送 panic 隔离
            let result = AssertUnwindSafe(async {
                if sender.send(Message::Text(msg)).await.is_err() {
                    tracing::debug!("WebSocket 发送失败，连接可能已关闭");
                    return false;
                }
                true
            })
            .catch_unwind()
            .await;
            match result {
                Ok(true) => {}
                Ok(false) => break,
                Err(panic_payload) => {
                    let panic_msg = panic_payload
                        .downcast_ref::<String>()
                        .map(|s| s.as_str())
                        .or_else(|| panic_payload.downcast_ref::<&'static str>().copied())
                        .unwrap_or("<非字符串 panic payload>");
                    tracing::error!(
                        panic = %panic_msg,
                        "⚠ WebSocket 发送 spawn panic 已被隔离，继续运行（不退出循环）"
                    );
                }
            }
        }
    });

    // 等待任一任务结束
    tokio::select! {
        _ = recv_task => {
            tracing::info!("接收任务结束");
        }
        _ = send_task => {
            tracing::info!("发送任务结束");
        }
    }

    // 清理
    manager.unregister(auth.user_id);
    tracing::info!("WebSocket 连接清理完成：user_id={}", auth.user_id);
}

#[cfg(test)]
mod tests {
    use super::*;

    /// 测试无效 token 场景：旧版 `"tenant:user"` 格式不再被接受
    ///
    /// 修复 bug.md #2：原占位实现接受 `"1:100"` 等格式字符串冒充任意用户
    /// 当前实现复用 `AuthService::validate_token_static()`，需要真实签名
    #[test]
    fn test_jwt_token_rejects_legacy_format() {
        // 旧版格式应被拒绝
        assert!(verify_jwt_token("1:100").is_err());
        assert!(verify_jwt_token("0:0").is_err());
    }

    /// 测试无效 token 场景：空 / 过短 / 无效签名
    #[test]
    fn test_jwt_token_invalid() {
        // 空 token
        assert!(verify_jwt_token("").is_err());
        // 过短 token
        assert!(verify_jwt_token("short").is_err());
        // 任意字符串（应被 JWT 签名验证拒绝）
        // 注：需要 JWT_SECRET 环境变量，否则返回"JWT_SECRET 未配置"
        std::env::set_var("JWT_SECRET", "test-secret-key-for-unit-test");
        assert!(verify_jwt_token(
            "eyJhbGciOiJIUzI1NiJ9.invalid.signature"
        )
        .is_err());
    }

    #[test]
    fn test_ws_message_serialize() {
        let msg = WsMessage::Ping { timestamp: 1234567890 };
        let json = serde_json::to_string(&msg).unwrap();
        assert!(json.contains("ping"));
        assert!(json.contains("1234567890"));
    }

    #[test]
    fn test_notification_broadcaster() {
        let broadcaster = NotificationBroadcaster::new();
        let payload = NotificationPayload {
            id: 1,
            title: "测试".to_string(),
            content: "内容".to_string(),
            category: "system".to_string(),
            priority: 5,
            created_at: "2026-06-17T10:30:00Z".to_string(),
        };
        // 广播给无订阅者的用户应不报错
        broadcaster.broadcast_notification(100, &payload);
    }
}
