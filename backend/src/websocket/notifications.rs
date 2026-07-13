//! 通知模块 WebSocket handler
//!
//! 路径：`/api/v1/erp/ws/notifications?ticket=<一次性短时票据>`
//!
//! 核心功能：
//! 1. 客户端先调 `POST /api/v1/erp/ws/ticket`（携带 httpOnly Cookie JWT）获取短时票据
//! 2. 握手时验证票据（一次性消费，30 秒过期），提取 user_id
//! 3. 升级到 WebSocket 协议
//! 4. 注册连接到 ConnectionManager（按 user_id 分组）
//! 5. 接收客户端消息（ping / mark_as_read）
//! 6. 接收服务端广播（来自 notification_service.send()）
//! 7. 断开时自动清理
//!
//! 安全设计（v12 P1-4 修复）：原方案通过 URL query 传递 JWT（`?token=<JWT>`），
//! JWT 会泄露到浏览器历史、服务器 access log、中间代理日志。改用一次性短时票据后，
//! 即使票据泄露也已在握手时被消费，且票据本身不是 JWT，无法用于其他 API。

use axum::extract::ws::{Message, WebSocket, WebSocketUpgrade};
use axum::extract::Query;
use axum::response::IntoResponse;
use dashmap::DashMap;
use futures::{FutureExt, SinkExt, StreamExt};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::panic::AssertUnwindSafe;
use std::sync::{Arc, OnceLock};
use std::time::{Duration, Instant};
use tokio::sync::broadcast;

use crate::middleware::auth_context::AuthContext;

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

// ==================== WebSocket 票据管理器（v12 P1-4 修复） ====================

/// 票据有效期（30 秒）
const WS_TICKET_TTL: Duration = Duration::from_secs(30);

/// 票据条目
struct WsTicketEntry {
    user_id: i64,
    expires_at: Instant,
}

/// WebSocket 票据管理器（全局单例）
///
/// v12 P1-4 修复：替代 URL query 传递 JWT 的方案。
/// 客户端通过 HTTP POST（携带 httpOnly Cookie JWT）获取一次性短时票据，
/// 再用票据建立 WebSocket 连接。票据 30 秒过期、一次性消费，
/// 即使泄露也无法复用。
pub struct WsTicketManager {
    tickets: Arc<DashMap<String, WsTicketEntry>>,
    /// 签发计数器，用于触发懒清理
    issue_count: Arc<std::sync::atomic::AtomicU64>,
}

/// 懒清理触发阈值（每签发 128 张票据清理一次过期票据）
const LAZY_CLEANUP_THRESHOLD: u64 = 128;

impl WsTicketManager {
    /// 创建新票据管理器
    ///
    /// 注意：此构造函数不启动后台清理任务，清理通过 `issue_ticket` 懒触发。
    /// 生产环境通过 `get_ticket_manager()` 单例使用，测试中可直接 `new()` 使用。
    pub fn new() -> Self {
        Self {
            tickets: Arc::new(DashMap::new()),
            issue_count: Arc::new(std::sync::atomic::AtomicU64::new(0)),
        }
    }

    /// 签发新票据
    ///
    /// 生成 32 字节随机票据（UUID v4 两次拼接 = 256 bit 随机量），
    /// 存入 DashMap 并设置 30 秒过期时间。每签发 128 张票据触发一次懒清理。
    pub fn issue_ticket(&self, user_id: i64) -> String {
        // UUID v4 内部使用 getrandom（CSPRNG），128 bit 随机量
        // 两次拼接 = 256 bit，足够防止爆破
        let ticket = format!(
            "{}{}",
            uuid::Uuid::new_v4().simple(),
            uuid::Uuid::new_v4().simple()
        );

        self.tickets.insert(
            ticket.clone(),
            WsTicketEntry {
                user_id,
                expires_at: Instant::now() + WS_TICKET_TTL,
            },
        );

        // 懒清理：每 LAZY_CLEANUP_THRESHOLD 次签发清理一次过期票据
        let count = self
            .issue_count
            .fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        if count.is_multiple_of(LAZY_CLEANUP_THRESHOLD) {
            self.cleanup_expired();
        }

        tracing::debug!(user_id, "签发 WebSocket 票据");
        ticket
    }

    /// 验证并消费票据（一次性使用）
    ///
    /// 返回 Some(user_id) 表示票据有效，返回 None 表示票据不存在或已过期。
    /// 验证后立即从 DashMap 移除，防止重放。
    pub fn validate_and_consume(&self, ticket: &str) -> Option<i64> {
        if ticket.is_empty() || ticket.len() < 32 {
            return None;
        }

        // remove 返回被删除的值，实现一次性消费
        let entry = self.tickets.remove(ticket)?;

        if entry.1.expires_at < Instant::now() {
            tracing::debug!(user_id = entry.1.user_id, "WebSocket 票据已过期");
            return None;
        }

        Some(entry.1.user_id)
    }

    /// 清理过期票据
    fn cleanup_expired(&self) {
        let now = Instant::now();
        let before = self.tickets.len();
        self.tickets.retain(|_, entry| entry.expires_at > now);
        let removed = before - self.tickets.len();
        if removed > 0 {
            tracing::debug!(removed, "清理过期 WebSocket 票据");
        }
    }
}

impl Default for WsTicketManager {
    fn default() -> Self {
        Self::new()
    }
}

/// 全局 WsTicketManager 单例
static WS_TICKET_MANAGER: OnceLock<WsTicketManager> = OnceLock::new();

/// 获取全局 WsTicketManager 单例
pub fn get_ticket_manager() -> &'static WsTicketManager {
    WS_TICKET_MANAGER.get_or_init(WsTicketManager::new)
}

/// 签发 WebSocket 票据的 HTTP 端点
///
/// 路径：`POST /api/v1/erp/ws/ticket`
///
/// 要求请求通过 auth_middleware（携带 httpOnly Cookie 中的有效 JWT）。
/// 返回一次性短时票据（30 秒有效），客户端用该票据建立 WebSocket 连接。
pub async fn issue_ws_ticket_handler(auth: AuthContext) -> impl IntoResponse {
    let user_id = auth.user_id as i64;
    let ticket = get_ticket_manager().issue_ticket(user_id);

    tracing::info!(user_id, "签发 WebSocket 票据");

    axum::Json(serde_json::json!({
        "ticket": ticket,
        "expires_in": WS_TICKET_TTL.as_secs(),
    }))
}

/// WebSocket 升级端点
///
/// 路径：`/api/v1/erp/ws/notifications?ticket=<一次性票据>`
///
/// 流程：
/// 1. 提取 URL query 中的 ticket
/// 2. 验证并消费票据（一次性，30 秒过期）
/// 3. 升级 HTTP 到 WebSocket
/// 4. 进入 handle_socket() 处理消息
pub async fn ws_notifications_handler(
    ws: WebSocketUpgrade,
    Query(params): Query<HashMap<String, String>>,
) -> impl IntoResponse {
    // 1. 提取票据
    let ticket = match params.get("ticket") {
        Some(t) if !t.is_empty() => t.clone(),
        _ => {
            return Err((
                axum::http::StatusCode::UNAUTHORIZED,
                String::from("缺少 ticket 参数"),
            ));
        }
    };

    // 2. 验证并消费票据（一次性使用）
    let user_id = match get_ticket_manager().validate_and_consume(&ticket) {
        Some(uid) => uid,
        None => {
            return Err((
                axum::http::StatusCode::UNAUTHORIZED,
                String::from("票据无效或已过期"),
            ));
        }
    };

    let auth = AuthInfo { user_id };

    // 3. 升级到 WebSocket
    Ok(ws.on_upgrade(move |socket| handle_socket(socket, auth)))
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
    // L-31 修复（批次 371 v13 复审）：声明为 mut，供 select! 借用，以便后续 abort
    let mut recv_task = tokio::spawn(async move {
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
    // L-31 修复（批次 371 v13 复审）：声明为 mut，供 select! 借用，以便后续 abort
    let mut send_task = tokio::spawn(async move {
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
    // L-31 修复（批次 371 v13 复审）：select! 用 &mut 借用 JoinHandle 而非消费，
    // 以便 select! 后显式 abort 未完成的 task，避免后台 detached task 泄漏。
    // 原实现 select! 消费 JoinHandle 后，未完成的 task 仍在后台运行（detached），
    // 浪费资源且可能继续尝试写入已关闭的 socket。
    tokio::select! {
        _ = &mut recv_task => {
            tracing::info!("接收任务结束");
        }
        _ = &mut send_task => {
            tracing::info!("发送任务结束");
        }
    }

    // L-31 修复：显式 abort 两个 task
    // 已完成的 task 调用 abort() 是 no-op，无副作用
    // 未完成的 task 会被终止，避免后台 detached task 泄漏
    recv_task.abort();
    send_task.abort();

    // 清理
    manager.unregister(auth.user_id);
    tracing::info!("WebSocket 连接清理完成：user_id={}", auth.user_id);
}

#[cfg(test)]
mod tests {
    use super::*;

    /// 测试票据签发与消费（正常流程）
    #[test]
    fn test_ticket_issue_and_consume() {
        let manager = WsTicketManager::new();
        let ticket = manager.issue_ticket(42);
        // 票据长度 = UUID v4 simple(32) × 2 = 64 字符
        assert_eq!(ticket.len(), 64);

        // 首次消费应成功
        let user_id = manager.validate_and_consume(&ticket);
        assert_eq!(user_id, Some(42));
    }

    /// 测试票据一次性消费：第二次消费应失败
    #[test]
    fn test_ticket_one_time_use() {
        let manager = WsTicketManager::new();
        let ticket = manager.issue_ticket(99);

        // 首次消费成功
        assert_eq!(manager.validate_and_consume(&ticket), Some(99));
        // 第二次消费失败（已消费）
        assert_eq!(manager.validate_and_consume(&ticket), None);
    }

    /// 测试无效票据：空、过短、不存在
    #[test]
    fn test_ticket_invalid() {
        let manager = WsTicketManager::new();
        // 空票据
        assert_eq!(manager.validate_and_consume(""), None);
        // 过短票据
        assert_eq!(manager.validate_and_consume("short"), None);
        // 不存在的票据
        assert_eq!(
            manager.validate_and_consume(&"a".repeat(64)),
            None
        );
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
