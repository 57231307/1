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
//! 异步落库使用 mpsc channel 加单消费者模式，避免每次调用 record_async 都创建
//! detached spawn task。通道与消费者由全局单例持有，与进程同生命周期。
//! 关停分两步：关停链先调用 flush 等待积压事件全部落库（未排空条数如实上报），
//! 再经 shutdown 从全局单例取出消费者 JoinHandle 真实 abort（take 后置空，天然防二次
//! abort），并返回句柄供调用方 await 确认消费者已终止，不靠 sleep 猜时序。

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
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::Duration;
use tokio::sync::Notify;
use tokio::sync::mpsc;

/// 审计日志服务。
///
/// 持库连接供同步写路径使用，持全局发送端供异步落库路径使用。
#[derive(Debug)]
pub struct AuditLogService {
    db: Arc<DatabaseConnection>,
    /// 事件发送端（全局单例，避免每次 new() 创建新 channel 后旧消费者退出）
    sender: &'static mpsc::UnboundedSender<(AuditEvent, Option<AuditContext>)>,
}

/// 全局单例：channel + 消费者只创建一次，避免每次 AuditLogService::new() 创建新 channel
/// 导致旧消费者退出（sender drop → receiver None → 消费者退出 → 审计日志丢失）
static AUDIT_SENDER: std::sync::OnceLock<
    mpsc::UnboundedSender<(AuditEvent, Option<AuditContext>)>,
> = std::sync::OnceLock::new();

/// 总线原子计数器，与 channel 同生命周期。
/// 消费者每处理完一条（无论成失）递增对应计数并唤醒 flush 等待者。
pub struct BusMetrics {
    pub sent: AtomicU64,
    pub inserted: AtomicU64,
    pub failed: AtomicU64,
    pub panicked: AtomicU64,
}

static BUS_METRICS: BusMetrics = BusMetrics {
    sent: AtomicU64::new(0),
    inserted: AtomicU64::new(0),
    failed: AtomicU64::new(0),
    panicked: AtomicU64::new(0),
};

/// 消费者处理完一条后唤醒 flush 等待者，避免忙等。
static BUS_NOTIFY: std::sync::OnceLock<Notify> = std::sync::OnceLock::new();

fn bus_notify() -> &'static Notify {
    BUS_NOTIFY.get_or_init(Notify::new)
}

/// 进程关停时等待审计总线落库的最长时间上限。
///
/// 一，约束对象。从关停链调用 flush 起最多允许等待这么久，到点立即返回已观测到的
/// 计数。通道是无界的，积压深度没有上限，所以该值只约束等待时长，不构成排空承诺。
/// 二，调用方。main.rs 的关停函数 shutdown_resources 在全部审计事件生产者停止之后、
/// 释放服务句柄之前读取本常量。
/// 三，取值依据。部署侧 systemd 服务单元把整条优雅关停的总预算 TimeoutStopSec 配为
/// 30 秒，其说明写明这 30 秒要覆盖在途请求排空、数据库连接池关闭与审计日志落库，
/// 超时后由 FinalKillSignal 发 SIGKILL 强杀进程。本常量取总预算的三分之一，为后续
/// 释放服务句柄与 runtime 自身退出留出 20 秒。
/// 四，未确证部分。三分之一这一比例没有实测排空耗时分布支撑，属保守取值。
/// 五，失效后果。撞满 30 秒时排空与后续关停步骤一起被 SIGKILL 砍掉；等待到点但未
/// 排空时，未处理条数由 FlushCounts 的 pending 如实上报，调用方不得把它当成功。
pub const SHUTDOWN_FLUSH_TIMEOUT: Duration = Duration::from_secs(10);

/// flush 返回值：截至等待结束时三类计数的累计快照。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FlushCounts {
    pub inserted: u64,
    pub failed: u64,
    pub panicked: u64,
    /// 调用时刻已入队、到等待结束仍未被消费者处理完的条数。取 0 表示本次等待已排空。
    pub pending: u64,
}

/// 有界等待总线把截至调用时刻的全部事件处理完毕。
/// 完成语义：inserted + failed + panicked >= 调用时刻的 sent。
/// 超时后仍返回已观测到的计数，由调用方判定是否达标。未排空条数由 pending 给出，
/// 调用方据此区分确实排空与超时未排空，不得把后者当成功。
pub async fn flush(timeout: Duration) -> FlushCounts {
    let target = BUS_METRICS.sent.load(Ordering::Acquire);
    let deadline = tokio::time::Instant::now() + timeout;

    let (ins, fail, panick) = loop {
        let ins = BUS_METRICS.inserted.load(Ordering::Acquire);
        let fail = BUS_METRICS.failed.load(Ordering::Acquire);
        let panick = BUS_METRICS.panicked.load(Ordering::Acquire);
        if ins + fail + panick >= target {
            break (ins, fail, panick);
        }
        let remaining = deadline.saturating_duration_since(tokio::time::Instant::now());
        if remaining.is_zero() {
            break (ins, fail, panick);
        }
        tokio::select! {
            _ = bus_notify().notified() => {}
            _ = tokio::time::sleep(remaining) => {
                let ins = BUS_METRICS.inserted.load(Ordering::Acquire);
                let fail = BUS_METRICS.failed.load(Ordering::Acquire);
                let panick = BUS_METRICS.panicked.load(Ordering::Acquire);
                break (ins, fail, panick);
            }
        }
    };

    FlushCounts {
        inserted: ins,
        failed: fail,
        panicked: panick,
        pending: target.saturating_sub(ins.saturating_add(fail).saturating_add(panick)),
    }
}

/// 全局单例：消费者 task 的 JoinHandle，spawn 成功后真实保存，供 shutdown 取出并 abort。
/// 消费者是进程级唯一的，因此句柄必须与 AUDIT_SENDER 一样是全局单例，
/// 而不是实例字段（实例随请求 handler 反复创建，实例字段永远装不进这个唯一的消费者）。
static AUDIT_CONSUMER_HANDLE: std::sync::OnceLock<
    std::sync::Mutex<Option<tokio::task::JoinHandle<()>>>,
> = std::sync::OnceLock::new();

impl AuditLogService {
    /// 创建审计日志服务（全局单例 channel，消费者只启动一次）
    pub fn new(db: Arc<DatabaseConnection>) -> Self {
        let sender = AUDIT_SENDER.get_or_init(|| {
            let (sender, mut receiver) =
                mpsc::unbounded_channel::<(AuditEvent, Option<AuditContext>)>();

            // 启动后台消费者 task（全局唯一，不会被 drop 关闭）
            let db_clone = db.clone();
            let handle = tokio::spawn(async move {
                while let Some((event, ctx)) = receiver.recv().await {
                    let result = AssertUnwindSafe(async {
                        let log = build_active_model(&event, ctx.as_ref());
                        match log.insert(db_clone.as_ref()).await {
                            Ok(_) => true,
                            Err(e) => {
                                tracing::error!(
                                    user_id = ?event.user_id,
                                    operation = event.operation_type.as_str(),
                                    error = %e,
                                    "异步审计日志落库失败"
                                );
                                false
                            }
                        }
                    })
                    .catch_unwind()
                    .await;

                    match &result {
                        Ok(true) => {
                            BUS_METRICS.inserted.fetch_add(1, Ordering::Release);
                        }
                        Ok(false) => {
                            BUS_METRICS.failed.fetch_add(1, Ordering::Release);
                        }
                        Err(panic_payload) => {
                            BUS_METRICS.panicked.fetch_add(1, Ordering::Release);
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
                    bus_notify().notify_one();
                }
                tracing::info!("AuditLogService 后台消费者 task 已退出");
            });

            // 真实保存消费者句柄（与全局 channel 同生命周期），shutdown 从这里取出 abort。
            // AUDIT_CONSUMER_HANDLE 与 AUDIT_SENDER 在同一次 get_or_init 闭包内初始化，
            // OnceLock 保证闭包只执行一次，set 必然成功。
            if AUDIT_CONSUMER_HANDLE
                .set(std::sync::Mutex::new(Some(handle)))
                .is_err()
            {
                tracing::error!(
                    "AuditLogService 消费者句柄重复注册（理论上不可达：闭包仅执行一次）"
                );
            }

            sender
        });

        Self { db, sender }
    }

    /// 优雅关闭异步审计服务：abort 全局后台消费者 task，防止 detached task 泄漏。
    ///
    /// 幂等：首次调用从全局单例 `take()` 出句柄并实际 abort，句柄置空后
    /// 后续调用返回 `None`，不会再触发第二次 abort。
    /// 注：tokio mpsc UnboundedSender 没有 close 方法；abort 消费者 task 后
    /// 其 future 被 drop，receiver 随之释放，后续 `record_async` 的 send() 将返回 Err（走告警日志分支）。
    /// shutdown 路径禁止 panic：锁中毒时用 `unwrap_or_else(|e| e.into_inner())` 安全取出内部数据。
    ///
    /// 返回被 abort 的 JoinHandle（仅首次调用为 `Some`），供调用方 await 确定性验证
    /// 消费者已取消（await 返回 `JoinError::is_cancelled()`），不靠 sleep 猜时序。
    /// 调用方为进程关停链：必须先 flush 排空积压，再经本方法终止消费者。
    pub fn shutdown(&self) -> Option<tokio::task::JoinHandle<()>> {
        shutdown_consumer()
    }
}

/// 取出全局消费者 JoinHandle 并 abort 的单点实现（实例方法 shutdown 与关停链共用）。
///
/// 消费者句柄是进程级全局单例，关停链（main.rs shutdown_resources）在没有服务实例
/// 引用的位置也需要终止消费者，故逻辑落在自由函数，方法侧只做转发。
/// 幂等与返回值语义同 `AuditLogService::shutdown`。
pub fn shutdown_consumer() -> Option<tokio::task::JoinHandle<()>> {
    let handle = AUDIT_CONSUMER_HANDLE
        .get()
        .and_then(|slot| slot.lock().unwrap_or_else(|e| e.into_inner()).take())?;
    handle.abort();
    tracing::info!("AuditLogService 异步记录引擎已关闭（消费者 task 已 abort）");
    Some(handle)
}

/// 通用审计事件（P13 批 1 P3-2 新增）（调用方只需填充业务相关字段，service 内部自动从 `AuditContext` 补充请求上下文。）
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
    /// P2 8-8 修复（批次 59）：update 前先查 old_model 序列化为 before_snapshot，；update 后用 new_model 序列化为 after_snapshot。；原实现 old_value/new_value/before_snapshot/after_snapshot 全部为 None，；审计日志只能看到"更新了"但不知道从什么变什么，合规失效。；仅 i32 主键支持 before_snapshot 查询；其他主键类型保持 None（避免泛型膨胀）。
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
        let pk_val_unwrapped = active_model
            .get(pk_col)
            .into_value()
            .unwrap_or(sea_orm::Value::Int(None));
        let record_id = if let sea_orm::Value::Int(Some(id)) = pk_val_unwrapped.clone() {
            id.to_string()
        } else {
            "0".to_string()
        };
        // P2 8-8 修复：update 前查询 old_model 序列化为 before_snapshot
        let before_snapshot = if let sea_orm::Value::Int(Some(id)) = pk_val_unwrapped.clone() {
            let old_model = E::find_by_id(id).one(db).await?;
            old_model.and_then(|m| serde_json::to_value(&m).ok())
        } else {
            None
        };
        let new_model = active_model.update(db).await?;
        let after_snapshot = serde_json::to_value(&new_model).ok();
        // P3 8-20 修复：如果有 user_id，查询 users 表填充 username
        let username = Self::fetch_username(db, user_id).await?;
        // 记录审计日志
        let log = Self::build_update_audit_log(
            resource_type,
            &record_id,
            user_id,
            username,
            before_snapshot,
            after_snapshot,
        );
        log.insert(db).await?;
        Ok(new_model)
    }

    /// 查询用户名（用于审计日志），无 user_id 返回 None
    async fn fetch_username<C: ConnectionTrait>(
        db: &C,
        user_id: Option<i32>,
    ) -> Result<Option<String>, AppError> {
        if let Some(uid) = user_id {
            use crate::models::user;
            Ok(user::Entity::find_by_id(uid)
                .one(db)
                .await?
                .map(|u| u.username))
        } else {
            Ok(None)
        }
    }

    /// 构建 UPDATE 审计日志 ActiveModel（双写 old/new 与 before/after 快照）
    fn build_update_audit_log(
        resource_type: &str,
        record_id: &str,
        user_id: Option<i32>,
        username: Option<String>,
        before_snapshot: Option<Value>,
        after_snapshot: Option<Value>,
    ) -> audit_log::ActiveModel {
        audit_log::ActiveModel {
            id: ActiveValue::NotSet,
            user_id: ActiveValue::Set(user_id),
            username: ActiveValue::Set(username),
            action: ActiveValue::Set("UPDATE".to_string()),
            resource_type: ActiveValue::Set(Some(resource_type.to_string())),
            resource_id: ActiveValue::Set(Some(record_id.to_string())),
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
            // V15 P0-S19 补齐：update_with_audit 无 query string，condition 为 None
            condition: ActiveValue::Set(None),
            ..Default::default()
        }
    }

    /// 作为 SeaORM 中间件，自动拦截并生成 Delete 审计日志（P0 8-3）
    /// 三步原子操作（同一事务/连接内）：1. 查询 old_value 快照（删除前）；2. 删除记录；3. 写审计日志（含 before_snapshot）；适用于主键类型为 `i32` 的实体（绝大多数业务表）。
    pub async fn delete_with_audit<E, C>(
        db: &C,
        resource_type: &str,
        record_id: i32,
        user_id: Option<i32>,
    ) -> Result<(), AppError>
    where
        E: EntityTrait,
        C: ConnectionTrait,
        <E as EntityTrait>::Model: Serialize + serde::de::DeserializeOwned + Sync + Send + Clone,
        <<E as EntityTrait>::PrimaryKey as sea_orm::PrimaryKeyTrait>::ValueType: From<i32>,
    {
        // 1. 查询 old_value 快照（删除前）
        let old_model = E::find_by_id(record_id)
            .one(db)
            .await?
            .ok_or_else(|| AppError::not_found(format!("{} 记录不存在", resource_type)))?;

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
        Self::insert_delete_audit_log(db, resource_type, &record_id_str, user_id, before_snapshot)
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
        <E as EntityTrait>::Model: Serialize + serde::de::DeserializeOwned + Sync + Send + Clone,
        <<E as EntityTrait>::PrimaryKey as sea_orm::PrimaryKeyTrait>::ValueType: From<i64>,
    {
        // 1. 查询 old_value 快照（删除前）
        let old_model = E::find_by_id(record_id)
            .one(db)
            .await?
            .ok_or_else(|| AppError::not_found(format!("{} 记录不存在", resource_type)))?;

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
        Self::insert_delete_audit_log(db, resource_type, &record_id_str, user_id, before_snapshot)
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
            old_value: ActiveValue::Set(before_snapshot.clone().map(audit_log::AuditValue)),
            new_value: ActiveValue::Set(None),
            created_at: ActiveValue::Set(Some(Utc::now())),
            operation_type: ActiveValue::Set(Some(OperationType::Delete.as_str().to_string())),
            severity: ActiveValue::Set(Some(Severity::Info.as_str().to_string())),
            request_id: ActiveValue::Set(None),
            before_snapshot: ActiveValue::Set(before_snapshot.map(audit_log::AuditValue)),
            after_snapshot: ActiveValue::Set(None),
            // V15 P0-S19 补齐：delete_with_audit 无 query string，condition 为 None
            condition: ActiveValue::Set(None),
            ..Default::default()
        };
        log.insert(db).await?;
        Ok(())
    }

    /// 同步记录审计事件（不接管业务事务）（调用方负责异常处理；推荐使用 `record_async` 在 tokio runtime 中异步落库。）
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
    /// L-32 修复（批次 380 v13 复审）：改为通过 mpsc channel 发送事件，；由后台单消费者 task 落库，避免每次调用都创建 detached spawn task。；写库失败由消费者 task 内部 catch_unwind + error 日志处理。
    pub fn record_async(self: Arc<Self>, event: AuditEvent, ctx: Option<AuditContext>) {
        if let Err(e) = self.sender.send((event.clone(), ctx.clone())) {
            tracing::warn!(
                user_id = ?event.user_id,
                operation = event.operation_type.as_str(),
                error = %e,
                "AuditLogService channel 已关闭，审计事件丢弃（服务已 shutdown？）"
            );
        } else {
            BUS_METRICS.sent.fetch_add(1, Ordering::Release);
        }
    }
}

/// 从 `AuditEvent` + `AuditContext` 构造 `ActiveModel`（service 内部共享）
pub fn build_active_model(
    event: &AuditEvent,
    ctx: Option<&AuditContext>,
) -> audit_log::ActiveModel {
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
        // V15 P0-S19 补齐：AuditEvent 未携带 query string，condition 为 None
        condition: ActiveValue::Set(None),
        ..Default::default()
    }
}
