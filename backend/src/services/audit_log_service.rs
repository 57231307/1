//! 审计日志服务（P13 批 1 P3-2 增强版）
//!
//! 提供两类能力：
//! 1. `update_with_audit`：SeaORM 中间件式 Update 审计
//! 2. `record(event: AuditEvent)`：P13 增强通用接口（自动注入请求上下文 + 异步落库）
//!
//! 设计要点：
//! - 列名兼容：`old_value` / `new_value` 与 `before_snapshot` / `after_snapshot` 双写
//! - JSON 快照：使用 `audit_log::AuditValue` 包装，PostgreSQL 自动用 JSONB 列存储
//!
//! L-32 修复（批次 380 v13 复审）：使用 mpsc channel + 单消费者模式，
//! 避免每次 record_async 都创建 detached spawn task；
//! handle 保存供 shutdown 时 abort，优雅关闭。

use crate::middleware::audit_context::AuditContext;
use crate::models::audit_log::{self, OperationType, Severity};
use crate::utils::error::AppError;
use chrono::Utc;
use futures::FutureExt;
use sea_orm::*;
use serde::Serialize;
use serde_json::Value;
use std::panic::AssertUnwindSafe;
use std::sync::Arc;
use tokio::sync::mpsc;

/// 审计日志服务（L-32 修复：不使用 define_service! 宏，添加 handle 字段）
#[derive(Debug)]
pub struct AuditLogService {
    db: Arc<DatabaseConnection>,
    /// L-32 修复：后台消费者 task handle，供 shutdown abort
    handle: std::sync::Mutex<Option<tokio::task::JoinHandle<()>>>,
    /// L-32 修复：事件发送端（消费者 spawn 时创建）
    sender: mpsc::UnboundedSender<(AuditEvent, Option<AuditContext>)>,
}

impl AuditLogService {
    /// 创建审计日志服务（L-32 修复：启动后台消费者 task）
    pub fn new(db: Arc<DatabaseConnection>) -> Self {
        // 创建 unbounded channel
        let (sender, mut receiver) = mpsc::unbounded_channel::<(AuditEvent, Option<AuditContext>)>();

        // 启动后台消费者 task
        let db_clone = db.clone();
        let handle = tokio::spawn(async move {
            while let Some((event, ctx)) = receiver.recv().await {
                // 批次 8（2026-06-28）：一次性 spawn panic 隔离
                let result = AssertUnwindSafe(async {
                    let log = build_active_model(&event, ctx.as_ref());
                    match log.insert(db_clone.as_ref()).await {
                        Ok(_) => {}
                        Err(e) => {
                            tracing::error!(
                                user_id = ?event.user_id,
                                operation = event.operation_type.as_str(),
                                error = %e,
                                "异步审计日志落库失败"
                            );
                        }
                    }
                })
                .catch_unwind()
                .await;
                if let Err(panic_payload) = result {
                    let panic_msg = panic_payload
                        .downcast_ref::<String>()
                        .map(|s| s.as_str())
                        .or_else(|| panic_payload.downcast_ref::<&'static str>().copied())
                        .unwrap_or("<非字符串 panic payload>");
                    tracing::error!(
                        panic = %panic_msg,
                        "⚠ 异步审计日志落库 spawn panic 已被隔离（单条日志丢失）"
                    );
                }
            }
            tracing::info!("AuditLogService 后台消费者 task 已退出");
        });

        Self {
            db,
            handle: std::sync::Mutex::new(Some(handle)),
            sender,
        }
    }

    /// L-32 修复（批次 380 v13 复审）：优雅关闭异步审计服务
    ///
    /// close channel + abort 后台消费者 task，防止 detached task 泄漏。
    /// 幂等：多次调用安全，仅首次调用实际 abort。
    pub fn shutdown(&self) {
        // 关闭 channel（停止接收新事件）
        self.sender.close();

        // abort 后台 task
        if let Some(handle) = self.handle.lock().unwrap().take() {
            handle.abort();
            tracing::info!("AuditLogService 异步记录引擎已关闭");
        }
    }
}

/// 通用审计事件（P13 批 1 P3-2 新增）
///
/// 调用方只需填充业务相关字段，service 内部自动从 `AuditContext` 补充请求上下文。
#[derive(Debug, Clone, Serialize)]
pub struct AuditEvent {
    /// 操作用户 ID
    pub user_id: Option<i32>,
    /// 操作用户名
    pub username: Option<String>,
    /// 操作类型（推荐使用 `OperationType::*` 枚举）
    pub operation_type: OperationType,
    /// 严重级别（缺省 INFO）
    pub severity: Severity,
    /// 资源类型（如 `user` / `order`）
    pub resource_type: Option<String>,
    /// 资源 ID（业务主键的字符串形式）
    pub resource_id: Option<String>,
    /// 资源名称（人类可读，便于审计追溯）
    pub resource_name: Option<String>,
    /// 操作描述（用户行为的中文说明）
    pub description: Option<String>,
    /// 业务方法名（如 `POST` / `PUT` / `DELETE`）
    pub request_method: Option<String>,
    /// 请求路径
    pub request_path: Option<String>,
    /// 变更前快照（推荐字段）
    pub before_snapshot: Option<Value>,
    /// 变更后快照（推荐字段）
    pub after_snapshot: Option<Value>,
}

impl AuditEvent {
    /// 构造最小可用事件（仅指定操作类型 + 资源类型）
    // v11 批次 148 P2-A：移除失效的 dead_code 标注（被 audit_log_service 单元测试 line 472/520/541 真实调用）
    pub fn new(operation_type: OperationType, resource_type: impl Into<String>) -> Self {
        Self {
            user_id: None,
            username: None,
            operation_type,
            severity: Severity::Info,
            resource_type: Some(resource_type.into()),
            resource_id: None,
            resource_name: None,
            description: None,
            request_method: None,
            request_path: None,
            before_snapshot: None,
            after_snapshot: None,
        }
    }
}

impl AuditLogService {
    /// 作为 SeaORM 中间件，自动拦截并生成 Update 审计日志
    ///
    /// P2 8-8 修复（批次 59）：update 前先查 old_model 序列化为 before_snapshot，
    /// update 后用 new_model 序列化为 after_snapshot。
    /// 原实现 old_value/new_value/before_snapshot/after_snapshot 全部为 None，
    /// 审计日志只能看到"更新了"但不知道从什么变什么，合规失效。
    ///
    /// 仅 i32 主键支持 before_snapshot 查询；其他主键类型保持 None（避免泛型膨胀）。
    pub async fn update_with_audit<E, A, C>(
        db: &C,
        resource_type: &str,
        active_model: A,
        user_id: Option<i32>,
    ) -> Result<<E as EntityTrait>::Model, AppError>
    where
        E: EntityTrait,
        A: ActiveModelTrait<Entity = E> + sea_orm::ActiveModelBehavior + Send + Sync,
        C: ConnectionTrait,
        <E as EntityTrait>::Model:
            Serialize + serde::de::DeserializeOwned + Sync + Send + Clone + IntoActiveModel<A>,
        <<E as EntityTrait>::PrimaryKey as sea_orm::PrimaryKeyTrait>::ValueType: From<i32>,
    {
        // 获取主键
        let pk_col = E::PrimaryKey::iter()
            .next()
            .ok_or_else(|| AppError::business("Entity has no primary key"))?
            .into_column();

        let pk_val = active_model.get(pk_col);

        let pk_val_unwrapped = pk_val.into_value().unwrap_or(sea_orm::Value::Int(None));

        let record_id = if let sea_orm::Value::Int(Some(id)) = pk_val_unwrapped.clone() {
            id.to_string()
        } else {
            "0".to_string()
        };

        // P2 8-8 修复：update 前查询 old_model 序列化为 before_snapshot
        // 仅 i32 主键支持；其他类型保持 before_snapshot=None
        let before_snapshot = if let sea_orm::Value::Int(Some(id)) = pk_val_unwrapped.clone() {
            let old_model = E::find_by_id(id).one(db).await?;
            old_model.and_then(|m| serde_json::to_value(&m).ok())
        } else {
            None
        };

        let new_model = active_model.update(db).await?;

        // P2 8-8 修复：update 后用 new_model 序列化为 after_snapshot
        let after_snapshot = serde_json::to_value(&new_model).ok();

        // P3 8-20 修复：如果有 user_id，查询 users 表填充 username
        let username = if let Some(uid) = user_id {
            use crate::models::user;
            user::Entity::find_by_id(uid)
                .one(db)
                .await?
                .map(|u| u.username)
        } else {
            None
        };

        // 记录审计日志
        let log = audit_log::ActiveModel {
            id: ActiveValue::NotSet,
            user_id: ActiveValue::Set(user_id),
            username: ActiveValue::Set(username),
            action: ActiveValue::Set("UPDATE".to_string()),
            resource_type: ActiveValue::Set(Some(resource_type.to_string())),
            resource_id: ActiveValue::Set(Some(record_id)),
            resource_name: ActiveValue::Set(None),
            description: ActiveValue::Set(None),
            ip_address: ActiveValue::Set(None),
            user_agent: ActiveValue::Set(None),
            request_method: ActiveValue::Set(None),
            request_path: ActiveValue::Set(None),
            request_body: ActiveValue::Set(None),
            response_status: ActiveValue::Set(None),
            duration_ms: ActiveValue::Set(None),
            // P2 8-8 修复：填充 old_value/new_value（旧字段兼容）
            old_value: ActiveValue::Set(before_snapshot.clone().map(audit_log::AuditValue)),
            new_value: ActiveValue::Set(after_snapshot.clone().map(audit_log::AuditValue)),
            created_at: ActiveValue::Set(Some(Utc::now())),
            operation_type: ActiveValue::Set(Some(OperationType::Update.as_str().to_string())),
            severity: ActiveValue::Set(Some(Severity::Info.as_str().to_string())),
            request_id: ActiveValue::Set(None),
            // P2 8-8 修复：填充 before_snapshot/after_snapshot（推荐字段）
            before_snapshot: ActiveValue::Set(before_snapshot.map(audit_log::AuditValue)),
            after_snapshot: ActiveValue::Set(after_snapshot.map(audit_log::AuditValue)),
        };
        log.insert(db).await?;

        Ok(new_model)
    }

    /// 作为 SeaORM 中间件，自动拦截并生成 Delete 审计日志（P0 8-3）
    ///
    /// 三步原子操作（同一事务/连接内）：
    /// 1. 查询 old_value 快照（删除前）
    /// 2. 删除记录
    /// 3. 写审计日志（含 before_snapshot）
    ///
    /// 适用于主键类型为 `i32` 的实体（绝大多数业务表）。
    pub async fn delete_with_audit<E, C>(
        db: &C,
        resource_type: &str,
        record_id: i32,
        user_id: Option<i32>,
    ) -> Result<(), AppError>
    where
        E: EntityTrait,
        C: ConnectionTrait,
        <E as EntityTrait>::Model:
            Serialize + serde::de::DeserializeOwned + Sync + Send + Clone,
        <<E as EntityTrait>::PrimaryKey as sea_orm::PrimaryKeyTrait>::ValueType: From<i32>,
    {
        // 1. 查询 old_value 快照（删除前）
        let old_model = E::find_by_id(record_id)
            .one(db)
            .await?
            .ok_or_else(|| {
                AppError::not_found(format!("{} 记录不存在", resource_type))
            })?;

        let before_snapshot = serde_json::to_value(&old_model).ok();
        let record_id_str = record_id.to_string();

        // 2. 删除记录
        let result = E::delete_by_id(record_id).exec(db).await?;
        if result.rows_affected == 0 {
            return Err(AppError::not_found(format!(
                "{} ID {} 不存在",
                resource_type, record_id_str
            )));
        }

        // 3. 写审计日志（Delete 操作，before_snapshot 保留删除前快照）
        Self::insert_delete_audit_log(
            db,
            resource_type,
            &record_id_str,
            user_id,
            before_snapshot,
        )
        .await
    }

    /// `delete_with_audit` 的 `i64` 主键变体（如 color_price_tier / crm_recycle_rule）
    pub async fn delete_with_audit_i64<E, C>(
        db: &C,
        resource_type: &str,
        record_id: i64,
        user_id: Option<i32>,
    ) -> Result<(), AppError>
    where
        E: EntityTrait,
        C: ConnectionTrait,
        <E as EntityTrait>::Model:
            Serialize + serde::de::DeserializeOwned + Sync + Send + Clone,
        <<E as EntityTrait>::PrimaryKey as sea_orm::PrimaryKeyTrait>::ValueType: From<i64>,
    {
        // 1. 查询 old_value 快照（删除前）
        let old_model = E::find_by_id(record_id)
            .one(db)
            .await?
            .ok_or_else(|| {
                AppError::not_found(format!("{} 记录不存在", resource_type))
            })?;

        let before_snapshot = serde_json::to_value(&old_model).ok();
        let record_id_str = record_id.to_string();

        // 2. 删除记录
        let result = E::delete_by_id(record_id).exec(db).await?;
        if result.rows_affected == 0 {
            return Err(AppError::not_found(format!(
                "{} ID {} 不存在",
                resource_type, record_id_str
            )));
        }

        // 3. 写审计日志
        Self::insert_delete_audit_log(
            db,
            resource_type,
            &record_id_str,
            user_id,
            before_snapshot,
        )
        .await
    }

    /// 内部辅助：写入 DELETE 类型的审计日志（before_snapshot 保留删除前快照）
    async fn insert_delete_audit_log<C>(
        db: &C,
        resource_type: &str,
        record_id_str: &str,
        user_id: Option<i32>,
        before_snapshot: Option<Value>,
    ) -> Result<(), AppError>
    where
        C: ConnectionTrait,
    {
        let log = audit_log::ActiveModel {
            id: ActiveValue::NotSet,
            user_id: ActiveValue::Set(user_id),
            username: ActiveValue::Set(None),
            action: ActiveValue::Set("DELETE".to_string()),
            resource_type: ActiveValue::Set(Some(resource_type.to_string())),
            resource_id: ActiveValue::Set(Some(record_id_str.to_string())),
            resource_name: ActiveValue::Set(None),
            description: ActiveValue::Set(None),
            ip_address: ActiveValue::Set(None),
            user_agent: ActiveValue::Set(None),
            request_method: ActiveValue::Set(None),
            request_path: ActiveValue::Set(None),
            request_body: ActiveValue::Set(None),
            response_status: ActiveValue::Set(None),
            duration_ms: ActiveValue::Set(None),
            old_value: ActiveValue::Set(
                before_snapshot.clone().map(audit_log::AuditValue),
            ),
            new_value: ActiveValue::Set(None),
            created_at: ActiveValue::Set(Some(Utc::now())),
            operation_type: ActiveValue::Set(Some(
                OperationType::Delete.as_str().to_string(),
            )),
            severity: ActiveValue::Set(Some(Severity::Info.as_str().to_string())),
            request_id: ActiveValue::Set(None),
            before_snapshot: ActiveValue::Set(
                before_snapshot.map(audit_log::AuditValue),
            ),
            after_snapshot: ActiveValue::Set(None),
        };
        log.insert(db).await?;
        Ok(())
    }

    /// 同步记录审计事件（不接管业务事务）
    ///
    /// 调用方负责异常处理；推荐使用 `record_async` 在 tokio runtime 中异步落库。
    pub async fn record(
        &self,
        event: AuditEvent,
        ctx: Option<&AuditContext>,
    ) -> Result<(), AppError> {
        let log = build_active_model(&event, ctx);
        log.insert(self.db.as_ref()).await?;
        Ok(())
    }

    /// 异步记录审计事件（推荐使用）
    ///
    /// L-32 修复（批次 380 v13 复审）：改为通过 mpsc channel 发送事件，
    /// 由后台单消费者 task 落库，避免每次调用都创建 detached spawn task。
    /// 写库失败由消费者 task 内部 catch_unwind + error 日志处理。
    pub fn record_async(self: Arc<Self>, event: AuditEvent, ctx: Option<AuditContext>) {
        if let Err(e) = self.sender.send((event.clone(), ctx.clone())) {
            tracing::warn!(
                user_id = ?event.user_id,
                operation = event.operation_type.as_str(),
                error = %e,
                "AuditLogService channel 已关闭，审计事件丢弃（服务已 shutdown？）"
            );
        }
    }
}

/// 从 `AuditEvent` + `AuditContext` 构造 `ActiveModel`（service 内部共享）
fn build_active_model(event: &AuditEvent, ctx: Option<&AuditContext>) -> audit_log::ActiveModel {
    // 从 ctx 注入请求上下文（缺省值兜底）
    let (request_id, ip_address, user_agent) = match ctx {
        Some(c) => (
            Some(c.request_id.clone()).filter(|s| !s.is_empty()),
            Some(c.ip_address.clone()).filter(|s| !s.is_empty()),
            Some(c.user_agent.clone()).filter(|s| !s.is_empty()),
        ),
        None => (None, None, None),
    };

    audit_log::ActiveModel {
        id: ActiveValue::NotSet,
        user_id: ActiveValue::Set(event.user_id),
        username: ActiveValue::Set(event.username.clone()),
        action: ActiveValue::Set(event.operation_type.as_str().to_string()),
        resource_type: ActiveValue::Set(event.resource_type.clone()),
        resource_id: ActiveValue::Set(event.resource_id.clone()),
        resource_name: ActiveValue::Set(event.resource_name.clone()),
        description: ActiveValue::Set(event.description.clone()),
        ip_address: ActiveValue::Set(ip_address),
        user_agent: ActiveValue::Set(user_agent),
        request_method: ActiveValue::Set(event.request_method.clone()),
        request_path: ActiveValue::Set(event.request_path.clone()),
        request_body: ActiveValue::Set(None),
        response_status: ActiveValue::Set(None),
        duration_ms: ActiveValue::Set(None),
        old_value: ActiveValue::Set(event.before_snapshot.clone().map(audit_log::AuditValue)),
        new_value: ActiveValue::Set(event.after_snapshot.clone().map(audit_log::AuditValue)),
        created_at: ActiveValue::Set(Some(Utc::now())),
        operation_type: ActiveValue::Set(Some(event.operation_type.as_str().to_string())),
        severity: ActiveValue::Set(Some(event.severity.as_str().to_string())),
        request_id: ActiveValue::Set(request_id),
        before_snapshot: ActiveValue::Set(event.before_snapshot.clone().map(audit_log::AuditValue)),
        after_snapshot: ActiveValue::Set(event.after_snapshot.clone().map(audit_log::AuditValue)),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::middleware::audit_context::AuditContext;
    use serde_json::json;

    /// AuditEvent::new 默认值正确
    #[test]
    fn test_audit_event_new_defaults() {
        let event = AuditEvent::new(OperationType::Login, "auth");
        assert_eq!(event.operation_type, OperationType::Login);
        assert_eq!(event.resource_type, Some("auth".to_string()));
        assert_eq!(event.severity, Severity::Info);
        assert!(event.user_id.is_none());
    }

    /// 无 ctx 时请求上下文字段全部为空
    #[test]
    fn test_build_active_model_without_ctx() {
        let event = AuditEvent {
            user_id: Some(42),
            username: Some("alice".to_string()),
            operation_type: OperationType::Update,
            severity: Severity::Warn,
            resource_type: Some("order".to_string()),
            resource_id: Some("1001".to_string()),
            resource_name: Some("订单 A".to_string()),
            description: Some("修改订单金额".to_string()),
            request_method: Some("PUT".to_string()),
            request_path: Some("/api/v1/erp/orders/1001".to_string()),
            before_snapshot: Some(json!({"amount": 100})),
            after_snapshot: Some(json!({"amount": 200})),
        };
        let model = build_active_model(&event, None);
        // 关键字段透传
        if let ActiveValue::Set(s) = model.severity {
            assert_eq!(s, Some("WARN".to_string()));
        } else {
            panic!("severity 应为 Set");
        }
        if let ActiveValue::Set(o) = model.operation_type {
            assert_eq!(o, Some("UPDATE".to_string()));
        } else {
            panic!("operation_type 应为 Set");
        }
        // 无 ctx 时请求上下文为 None
        if let ActiveValue::Set(ip) = model.ip_address {
            assert!(ip.is_none(), "无 ctx 时 ip_address 应为 None");
        }
        if let ActiveValue::Set(rid) = model.request_id {
            assert!(rid.is_none(), "无 ctx 时 request_id 应为 None");
        }
    }

    /// 有 ctx 时请求上下文自动注入
    #[test]
    fn test_build_active_model_with_ctx() {
        let event = AuditEvent::new(OperationType::Login, "auth");
        let ctx = AuditContext {
            request_id: "trace-123".to_string(),
            ip_address: "203.0.113.1".to_string(),
            user_agent: "Mozilla/5.0".to_string(),
        };
        let model = build_active_model(&event, Some(&ctx));
        if let ActiveValue::Set(rid) = model.request_id {
            assert_eq!(rid, Some("trace-123".to_string()));
        }
        if let ActiveValue::Set(ip) = model.ip_address {
            assert_eq!(ip, Some("203.0.113.1".to_string()));
        }
        if let ActiveValue::Set(ua) = model.user_agent {
            assert_eq!(ua, Some("Mozilla/5.0".to_string()));
        }
    }

    /// ctx 字段为空字符串时不会写入数据库（避免污染日志）
    #[test]
    fn test_build_active_model_with_empty_ctx() {
        let event = AuditEvent::new(OperationType::Logout, "auth");
        let ctx = AuditContext::empty();
        let model = build_active_model(&event, Some(&ctx));
        if let ActiveValue::Set(rid) = model.request_id {
            assert!(rid.is_none(), "空 ctx request_id 应为 None");
        }
        if let ActiveValue::Set(ip) = model.ip_address {
            assert!(ip.is_none(), "空 ctx ip_address 应为 None");
        }
        if let ActiveValue::Set(ua) = model.user_agent {
            assert!(ua.is_none(), "空 ctx user_agent 应为 None");
        }
    }

    /// 旧字段 old_value/new_value 与新字段 before_snapshot/after_snapshot 内容一致
    #[test]
    fn test_dual_write_snapshots() {
        let before = json!({"price": 100});
        let after = json!({"price": 200});
        let event = AuditEvent {
            user_id: Some(1),
            username: None,
            operation_type: OperationType::Update,
            severity: Severity::Info,
            resource_type: Some("product".to_string()),
            resource_id: Some("1".to_string()),
            resource_name: None,
            description: None,
            request_method: None,
            request_path: None,
            before_snapshot: Some(before.clone()),
            after_snapshot: Some(after.clone()),
        };
        let model = build_active_model(&event, None);
        // old_value/new_value 同步填充
        if let ActiveValue::Set(Some(av)) = model.old_value {
            assert_eq!(av.0, before);
        } else {
            panic!("old_value 应填充 before_snapshot");
        }
        if let ActiveValue::Set(Some(av)) = model.new_value {
            assert_eq!(av.0, after);
        } else {
            panic!("new_value 应填充 after_snapshot");
        }
        // before_snapshot / after_snapshot 也填充
        if let ActiveValue::Set(Some(av)) = model.before_snapshot {
            assert_eq!(av.0, before);
        }
        if let ActiveValue::Set(Some(av)) = model.after_snapshot {
            assert_eq!(av.0, after);
        }
    }
}
