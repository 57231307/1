//! JTI 黑名单与用户级 Token 吊销管理子模块（auth_service_ops/jti）
//!
//! 从原 `auth_service.rs` 迁移的 free functions：
//! - revoke_jti / is_jti_revoked / cleanup_expired_jti：JTI 维度黑名单
//! - revoke_user_jtis / is_user_token_revoked / cleanup_revoked_users
//! - start_revoked_user_cleanup_task / unrevoke_user：用户维度 Token 吊销
//!
//! 这些函数为 `pub`，由 facade（`auth_service.rs`）通过 `pub use` 重新导出，
//! 外部调用路径 `crate::services::auth_service::revoke_jti` 等保持不变。

// =====================================================================
// JTI 黑名单（已吊销的 JWT ID）
// =====================================================================
//
// 用于实现 Refresh Token 轮换场景下的旧 Token 立即失效：
// - 登出时调用 `revoke_jti` 吊销当前 Token 的 JTI（session_id）
// - Refresh Token 旋转时调用 `revoke_jti` 吊销旧 Token 的 JTI
// - 每次受保护请求在 middleware 中调用 `is_jti_revoked` 检查
//
// 低危 #1 修复：JTI 黑名单从进程内 HashMap 迁移到 Redis（SETEX + TTL）。
// 进程内存储在多实例部署时不共享，撤销后的旧 JWT 在其他实例最多可继续使用
// 2 小时（JWT 过期时间）。Redis 后端保证所有实例共享同一黑名单视图。
// Redis 不可用时自动回退到内存（graceful degradation）。

use redis::aio::ConnectionManager;
use redis::AsyncCommands;
use std::collections::HashMap;
use std::sync::Arc;
use std::sync::LazyLock;
use tokio::sync::{OnceCell, RwLock};

/// JTI 黑名单 Redis key 前缀
const JTI_KEY_PREFIX: &str = "jwt:jti:revoked:";

/// JWT JTI 黑名单（进程内降级回退表：jti -> 过期时间戳）
///
/// 仅在 Redis 不可用时使用，避免阻塞业务。生产环境应配置 Redis。
static JTI_BLACKLIST: LazyLock<RwLock<HashMap<String, i64>>> =
    LazyLock::new(|| RwLock::new(HashMap::new()));

/// 分布式 JTI 黑名单 Redis 客户端（懒初始化）
///
/// 通过环境变量 `JTI_REDIS_URL` 或回退 `REDIS_URL` 启用。
static REDIS_JTI_BLACKLIST: OnceCell<Option<Arc<tokio::sync::Mutex<ConnectionManager>>>> =
    OnceCell::const_new();

/// 初始化 Redis JTI 黑名单客户端
async fn init_redis_jti_blacklist() -> Option<Arc<tokio::sync::Mutex<ConnectionManager>>> {
    let url = std::env::var("JTI_REDIS_URL")
        .or_else(|_| std::env::var("REDIS_URL"))
        .ok()
        .filter(|s| !s.is_empty());

    let url = match url {
        Some(u) => u,
        None => {
            tracing::debug!(
                "JTI_REDIS_URL/REDIS_URL 未配置，JTI 黑名单使用进程内存储（多实例部署不安全）"
            );
            return None;
        }
    };

    match redis::Client::open(url.as_str()) {
        Ok(client) => match ConnectionManager::new(client).await {
            Ok(conn) => {
                tracing::info!("JTI 黑名单已启用 Redis 分布式后端 (URL 已配置)");
                Some(Arc::new(tokio::sync::Mutex::new(conn)))
            }
            Err(e) => {
                tracing::warn!(
                    "JTI 黑名单 Redis 连接失败 ({:?})，回退到进程内存储",
                    e
                );
                None
            }
        },
        Err(e) => {
            tracing::warn!("JTI 黑名单 Redis URL 解析失败 ({:?})，回退到进程内存储", e);
            None
        }
    }
}

/// 获取或初始化 Redis JTI 黑名单客户端
async fn get_redis_jti_blacklist() -> Option<Arc<tokio::sync::Mutex<ConnectionManager>>> {
    REDIS_JTI_BLACKLIST
        .get_or_init(init_redis_jti_blacklist)
        .await
        .clone()
}

/// 吊销指定 JTI
///
/// 将给定 JTI 加入黑名单（优先写 Redis；Redis 不可用时回退到进程内 HashMap）。
/// 后续请求将拒绝持有该 JTI 的 Token。
///
/// # 参数
/// - `jti`: 待吊销的 Token 唯一标识（当前实现取自 `AppClaims::session_id`）
/// - `expires_at`: Token 的过期时间戳（Unix 秒）
pub async fn revoke_jti(jti: &str, expires_at: i64) {
    // 主路径：写入 Redis（SETEX 自动设置 TTL，过期自动清理，零维护成本）
    if let Some(conn_arc) = get_redis_jti_blacklist().await {
        let now = chrono::Utc::now().timestamp();
        let ttl_secs = (expires_at - now).max(1) as u64;
        let key = format!("{}{}", JTI_KEY_PREFIX, jti);

        let write_result: Result<(), redis::RedisError> = async {
            let mut conn = conn_arc.lock().await;
            let _: () = conn.set_ex(&key, expires_at.to_string(), ttl_secs).await?;
            Ok(())
        }
        .await;

        if let Err(e) = write_result {
            tracing::warn!(
                "JTI 写入 Redis 失败 ({:?})，回退到进程内存储；jti={}",
                e,
                jti
            );
            // 降级：写入内存
            let mut blacklist = JTI_BLACKLIST.write().await;
            blacklist.insert(jti.to_string(), expires_at);
        } else {
            tracing::info!("JTI 已吊销（Redis）：{}，TTL {} 秒", jti, ttl_secs);
        }
    } else {
        // 未配置 Redis：直接写内存
        let mut blacklist = JTI_BLACKLIST.write().await;
        blacklist.insert(jti.to_string(), expires_at);
        tracing::info!("JTI 已吊销（内存）：{}，过期时间：{}", jti, expires_at);
    }
}

/// 检查 JTI 是否在黑名单
///
/// # 参数
/// - `jti`: 待检查的 Token 唯一标识
///
/// # 返回
/// - `true`: 该 JTI 已被吊销
/// - `false`: 该 JTI 仍然有效
pub async fn is_jti_revoked(jti: &str) -> bool {
    // 主路径：查 Redis
    if let Some(conn_arc) = get_redis_jti_blacklist().await {
        let key = format!("{}{}", JTI_KEY_PREFIX, jti);
        let check_result: Result<bool, redis::RedisError> = async {
            let mut conn = conn_arc.lock().await;
            let exists: bool = conn.exists(&key).await?;
            Ok(exists)
        }
        .await;

        match check_result {
            Ok(exists) => return exists,
            Err(e) => {
                tracing::warn!(
                    "JTI 查 Redis 失败 ({:?})，回退到进程内检查；jti={}",
                    e,
                    jti
                );
                // 降级：查内存
            }
        }
    }

    // 降级：查内存
    let blacklist = JTI_BLACKLIST.read().await;
    blacklist.contains_key(jti)
}

/// 清理过期 JTI（建议定期调用，如每小时）
///
/// 当使用 Redis 后端时，TTL 自动清理过期条目，此函数为 noop。
/// 当回退到进程内存储时，主动清理已超过过期时间的记录，避免内存泄漏。
///
/// # 参数
/// - `_max_age_secs`: 允许的最大存活时间（秒），当前实现忽略该参数
pub async fn cleanup_expired_jti(_max_age_secs: i64) {
    // Redis 后端下，SETEX TTL 自动清理，无需手动操作
    if get_redis_jti_blacklist().await.is_some() {
        tracing::debug!("JTI 黑名单使用 Redis 后端，过期条目由 TTL 自动清理");
        return;
    }

    // 进程内存储降级路径：手动清理过期记录
    let mut blacklist = JTI_BLACKLIST.write().await;
    let now = chrono::Utc::now().timestamp();
    let before = blacklist.len();
    blacklist.retain(|_, expires_at| *expires_at > now);
    let removed = before - blacklist.len();
    tracing::info!(
        "清理 JTI 黑名单（内存）：移除 {} 条过期记录，剩余 {} 条",
        removed,
        blacklist.len()
    );
}

// =====================================================================
// 用户级 Token 吊销表（修复安全漏洞 #9：删除/封禁用户后即时撤销其所有活跃 JWT）
// =====================================================================
//
// 设计动机：现有 JTI 黑名单按 session_id（UUID）维度存储，
// 但应用层在删除/封禁用户时无法枚举该用户历史上颁发的全部 session_id。
// 为此新增 user_id -> revoked_at 的全局表，middleware 在校验完 Claims 后
// 再检查 `claims.iat < user_revoke_ts` 以决定是否放行。
//
// 语义：
// - `revoke_user_jtis(user_id, reason)`：将 user_id 标记为已吊销，记录当前时间戳。
//   后续所有 iat < 该时间戳的 Token 一律拒绝；iat >= 该时间戳的 Token 仍然有效。
// - `is_user_token_revoked(user_id, token_iat)`：供 middleware 调用的快速判定。
// - 该表为进程内内存表，进程重启后失效。生产环境如需持久化，
//   应迁移到 Redis/DB（按 user_id 维度持久化 revoked_at），此实现仅做 MVP。

/// 用户级 Token 吊销表（user_id -> 吊销时间戳，Unix 秒）
///
/// `pub(crate)` 以便 facade 测试夹具直接注入"过期"记录验证清理逻辑。
pub(crate) static REVOKED_USERS: LazyLock<RwLock<HashMap<i32, i64>>> =
    LazyLock::new(|| RwLock::new(HashMap::new()));

/// 吊销指定用户的所有活跃 JWT
///
/// 将 user_id 加入内存吊销表，记录当前时间戳为吊销点。
/// 后续 middleware 收到该用户 Token 时，若 `iat < revoked_at` 则拒绝。
///
/// # 参数
/// - `user_id`: 被吊销用户的 ID
/// - `reason`: 吊销原因（如 `"USER_DELETED"`、`"USER_DEACTIVATED"`），仅用于日志
///
/// # 返回
/// - `Ok(())`: 成功加入吊销表
/// - `Err(AuthError::InternalError)`: 当前实现下不会失败，保留 Result 供后续扩展
pub async fn revoke_user_jtis(
    user_id: i32,
    reason: &str,
) -> Result<(), crate::utils::error::AppError> {
    let now = chrono::Utc::now().timestamp();
    let mut table = REVOKED_USERS.write().await;
    table.insert(user_id, now);
    tracing::warn!(
        target: "security_audit",
        event = "USER_TOKENS_REVOKED",
        user_id = user_id,
        reason = reason,
        revoked_at = now,
        "[SECURITY] 用户级 Token 吊销：user_id={} reason={} revoked_at={}",
        user_id,
        reason,
        now
    );
    Ok(())
}

/// 检查某用户 Token 是否已被吊销
///
/// 判定规则：
/// - 若 user_id 不在吊销表中，返回 `false`（未吊销）
/// - 若 token_iat >= revoked_at，返回 `false`（Token 在吊销后签发，仍有效）
/// - 若 token_iat < revoked_at，返回 `true`（Token 在吊销前签发，必须拒绝）
///
/// # 参数
/// - `user_id`: Token 所属用户 ID
/// - `token_iat`: Token 签发时间戳（Unix 秒）
pub async fn is_user_token_revoked(user_id: i32, token_iat: i64) -> bool {
    let table = REVOKED_USERS.read().await;
    if let Some(&revoked_at) = table.get(&user_id) {
        token_iat < revoked_at
    } else {
        false
    }
}

/// 清理过期的用户吊销记录（建议定期调用）
///
/// v11 批次 145 P1-7 修复：原实现为占位（仅打印日志），现实现真实 TTL 清理。
///
/// 清理策略：
/// - 吊销记录超过 `REVOKED_USER_TTL_SECS`（默认 7 天）后自动清理
/// - 理由：JWT 最长有效期 2 小时，7 天后所有旧 Token 已过期，吊销记录无意义
/// - 清理后该用户的旧 Token 已自然过期，无需再保留吊销标记
///
/// 返回被清理的记录数。
pub async fn cleanup_revoked_users() -> usize {
    let now = chrono::Utc::now().timestamp();
    let ttl = REVOKED_USER_TTL_SECS;
    let cutoff = now - ttl;

    let mut table = REVOKED_USERS.write().await;
    let before = table.len();

    // 保留 revoked_at >= cutoff 的记录（未过期的），移除已过期的
    table.retain(|_, revoked_at| *revoked_at >= cutoff);

    let removed = before - table.len();
    if removed > 0 {
        tracing::info!(
            "已清理 {} 条过期用户吊销记录（TTL={}秒，剩余{}条）",
            removed,
            ttl,
            table.len()
        );
    } else {
        tracing::debug!(
            "用户吊销记录清理完成：无过期记录（当前{}条）",
            table.len()
        );
    }
    removed
}

/// 吊销记录 TTL（秒），默认 7 天
///
/// v11 批次 145 P1-7：吊销记录超过此时间后自动清理。
/// JWT 最长有效期 2 小时，7 天后所有旧 Token 已过期，吊销记录无意义。
///
/// `pub(crate)` 以便 facade 测试夹具构造"过期"记录时引用同一常量。
pub(crate) const REVOKED_USER_TTL_SECS: i64 = 7 * 24 * 60 * 60;

/// 启动吊销记录定期清理后台任务
///
/// v11 批次 145 P1-7：接入 app_state 初始化流程，每 24 小时清理一次过期吊销记录。
/// 此任务为 best-effort，单次清理 panic 不会退出循环。
pub fn start_revoked_user_cleanup_task() -> tokio::task::JoinHandle<()> {
    use futures::FutureExt;
    use std::panic::AssertUnwindSafe;
    use tokio::time::{interval, Duration};

    tokio::spawn(async move {
        // 每 24 小时执行一次清理
        let mut ticker = interval(Duration::from_secs(24 * 60 * 60));
        // 跳过首次立即触发（启动时无需清理）
        ticker.tick().await;
        loop {
            ticker.tick().await;
            // 单次清理 panic 隔离，确保循环不退出
            let result = AssertUnwindSafe(async {
                cleanup_revoked_users().await;
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
                    "⚠ 用户吊销记录清理 spawn 任务内 panic 已被隔离，清理循环继续运行（不退出）"
                );
            }
        }
    })
}

/// 显式注销用户吊销标记（用于用户重新激活场景）
///
/// v11 批次 145 P1-7 修复：接入 user_service.update_user 业务，
/// 当用户状态从"禁用"恢复为"active"时调用此函数清除吊销标记，
/// 允许用户重新登录获取新 Token。
///
/// # 参数
/// - `user_id`: 需注销的用户 ID
pub async fn unrevoke_user(user_id: i32) {
    let mut table = REVOKED_USERS.write().await;
    if table.remove(&user_id).is_some() {
        tracing::info!(
            target: "security_audit",
            event = "USER_UNREVOKED",
            user_id = user_id,
            "用户吊销标记已清除（用户重新激活）"
        );
    }
}
