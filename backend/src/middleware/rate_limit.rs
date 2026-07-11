use crate::middleware::auth_context::AuthContext;
use crate::utils::app_state::AppState;
use crate::utils::error::AppError;
use axum::{body::Body, extract::State, http::Request, middleware::Next, response::Response};
use redis::aio::ConnectionManager;
use redis::AsyncCommands;
use std::collections::HashMap;
use std::sync::Arc;
use std::sync::LazyLock;
use std::time::{Duration, Instant};
use tokio::sync::OnceCell;

// =====================================================
// 内存限流器
// =====================================================

/// 内存速率限制器
///
/// # 设计
/// - 内部使用 `std::sync::Mutex<HashMap>`，所有访问通过 `try_lock` 避免锁中毒
/// - 锁失败时默认放行（fail-open），记录 warn 日志
/// - 单 mutex 替代 DashMap（分片锁），简化锁中毒防御路径
/// - 高频场景下性能足够：180 req/min/user 是常见限流阈值
///
/// # 低危 #3 修复
/// 原实现使用 DashMap 分片锁，理论上不存在 PoisonError 暴露
/// （DashMap API 不返回 Result），但 audit 报告建议显式 try_lock
/// 防御极端 panic 场景。本实现满足该建议：锁不可用时放行而非 panic。
pub struct MemoryRateLimiter {
    storage: Arc<std::sync::Mutex<HashMap<String, RateLimitInfo>>>,
    max_requests: usize,
    window: Duration,
}

/// 速率限制信息
#[derive(Debug)]
struct RateLimitInfo {
    count: usize,
    reset_at: Instant,
}

impl MemoryRateLimiter {
    pub fn new(max_requests: usize, window: Duration) -> Self {
        Self {
            storage: Arc::new(std::sync::Mutex::new(HashMap::new())),
            max_requests,
            window,
        }
    }

    /// 清理过期的记录
    ///
    /// 使用 `try_lock` 避免锁中毒（PoisonError）。如果锁不可用则跳过本次清理。
    pub fn cleanup(&self) {
        // 低危 #3 修复：try_lock 防御锁中毒；失败时跳过清理（不影响主流程）
        let Ok(mut storage) = self.storage.try_lock() else {
            tracing::warn!("限流器存储锁不可用，跳过本次清理（fail-open）");
            return;
        };
        let now = Instant::now();
        storage.retain(|_, v| now < v.reset_at);
    }

    /// 检查是否允许请求
    ///
    /// # 低危 #3 修复
    /// 使用 `try_lock` 而非 `lock().unwrap()`：
    /// - 锁中毒（PoisonError）时不再 panic
    /// - 极端情况下（panic 蔓延）默认放行（fail-open），不阻塞业务
    ///
    /// # 返回
    /// - `true`: 允许请求
    /// - `false`: 拒绝请求（已达上限）
    pub fn check(&self, key: &str) -> bool {
        // 偶尔清理过期记录以防止内存泄漏
        if fastrand::usize(..1000) == 0 {
            self.cleanup();
        }

        let now = Instant::now();
        // 低危 #3 修复：try_lock 防御锁中毒；失败时 fail-open（放行）
        let Ok(mut storage) = self.storage.try_lock() else {
            tracing::warn!(
                "限流器存储锁不可用（PoisonError 或争用），默认放行；key={}",
                key
            );
            return true;
        };

        if let Some(entry) = storage.get_mut(key) {
            if now >= entry.reset_at {
                entry.count = 1;
                entry.reset_at = now + self.window;
                true
            } else {
                entry.count += 1;
                entry.count <= self.max_requests
            }
        } else {
            storage.insert(
                key.to_string(),
                RateLimitInfo {
                    count: 1,
                    reset_at: now + self.window,
                },
            );
            true
        }
    }
}

// 全局内存限流器实例（默认使用；当分布式限流不可用时回退）
static GLOBAL_LIMITER: LazyLock<MemoryRateLimiter> =
    LazyLock::new(|| MemoryRateLimiter::new(180, Duration::from_secs(60)));
static BRUTE_FORCE_LIMITER: LazyLock<MemoryRateLimiter> =
    LazyLock::new(|| MemoryRateLimiter::new(5, Duration::from_secs(300)));

// =====================================================
// 分布式限流器（漏洞 #6 修复）
// =====================================================

/// 分布式限流器后端（基于 Redis）
///
/// 使用 `INCR` + `EXPIRE` 原子操作实现固定窗口限流：
/// - 第一次请求：INCR 返回 1，EXPIRE 设置窗口
/// - 后续请求：INCR 返回累加值
/// - 计数 > max_requests → 拒绝
///
/// 优势：
/// - 多实例共享计数（解决 #6 漏洞）
/// - 失败时回退到内存限流（graceful degradation）
static REDIS_RATE_LIMITER: OnceCell<Option<Arc<tokio::sync::Mutex<ConnectionManager>>>> =
    OnceCell::const_new();

/// 初始化 Redis 分布式限流客户端
///
/// 通过环境变量 `RATE_LIMIT_REDIS_URL` 或 `REDIS_URL` 启用；
/// 未配置或连接失败时返回 `None`，调用方回退到内存限流。
async fn init_redis_rate_limiter() -> Option<Arc<tokio::sync::Mutex<ConnectionManager>>> {
    let url = std::env::var("RATE_LIMIT_REDIS_URL")
        .or_else(|_| std::env::var("REDIS_URL"))
        .ok()
        .filter(|s| !s.is_empty());

    let url = match url {
        Some(u) => u,
        None => {
            tracing::debug!(
                "RATE_LIMIT_REDIS_URL/REDIS_URL 未配置，分布式限流未启用（使用内存限流）"
            );
            return None;
        }
    };

    match redis::Client::open(url.as_str()) {
        Ok(client) => match ConnectionManager::new(client).await {
            Ok(conn) => {
                // 规则 12 合规：不记录完整 URL，防止 URL 中的 user:password@host 凭据泄露
                tracing::info!("分布式限流已启用（RATE_LIMIT_REDIS_URL 已配置）");
                Some(Arc::new(tokio::sync::Mutex::new(conn)))
            }
            Err(e) => {
                tracing::warn!(
                    "Redis 连接失败 ({:?})，分布式限流回退到内存限流",
                    e
                );
                None
            }
        },
        Err(e) => {
            tracing::warn!("Redis URL 解析失败 ({:?})，分布式限流回退到内存限流", e);
            None
        }
    }
}

/// 获取或初始化 Redis 限流客户端
async fn get_redis_rate_limiter(
) -> Option<Arc<tokio::sync::Mutex<ConnectionManager>>> {
    REDIS_RATE_LIMITER
        .get_or_init(init_redis_rate_limiter)
        .await
        .clone()
}

/// 分布式限流检查（Redis 后端）
///
/// # 参数
/// - `key`: 限流键（如 `rate:ip:userid`）
/// - `max_requests`: 窗口内允许的最大请求数
/// - `window`: 时间窗口（秒）
///
/// # 返回
/// - `Ok(Some(true))`: Redis 判定放行
/// - `Ok(Some(false))`: Redis 判定拒绝
/// - `Ok(None)`: 未配置 Redis（应回退到内存限流）
/// - `Err(_)`: Redis 调用错误（应回退到内存限流）
async fn check_redis_rate_limit(
    key: &str,
    max_requests: usize,
    window: Duration,
) -> Result<Option<bool>, redis::RedisError> {
    let conn_arc = match get_redis_rate_limiter().await {
        Some(c) => c,
        None => return Ok(None), // 未启用分布式限流（调用方回退到内存限流）
    };

    let mut conn = conn_arc.lock().await;
    let count: i64 = conn.incr(key, 1i64).await?;
    if count == 1 {
        // 第一次请求时设置过期时间（避免长尾 key）
        let _: () = conn.expire(key, window.as_secs() as i64).await?;
    }
    Ok(Some((count as usize) <= max_requests))
}

/// 通用限流检查：优先 Redis 分布式，回退到内存
///
/// M6 修复（v8 复审）：改为 pub(crate) 以供 webhook_handler 等模块复用，
/// 统一分布式限流策略（Redis 优先 + 内存回退），避免各处自行实现内存限流
pub(crate) async fn check_rate_limit(
    key: &str,
    max_requests: usize,
    window: Duration,
    memory_limiter: &MemoryRateLimiter,
) -> bool {
    match check_redis_rate_limit(key, max_requests, window).await {
        Ok(Some(allowed)) => allowed,
        Ok(None) => {
            // 未配置 Redis：直接回退到内存限流（graceful degradation）
            memory_limiter.check(key)
        }
        Err(e) => {
            tracing::warn!(
                "Redis 限流检查失败 {:?}，回退到内存限流 key={}",
                e,
                key
            );
            memory_limiter.check(key)
        }
    }
}

// =====================================================
// 中间件
// =====================================================

/// 基于 IP + UserID 的双维度速率限制中间件
pub async fn rate_limit_by_ip(
    State(_state): State<AppState>,
    req: Request<Body>,
    next: Next,
) -> Result<Response, AppError> {
    // P3 维度 12 修复（批次 87）：复用 audit_context::extract_client_ip helper 消除重复实现
    // P2-12b 修复（批次 83 v1 复审）：三层降级全部失败时返回 400，避免 unknown_ip 聚合
    let ip = {
        let extracted = crate::middleware::audit_context::extract_client_ip(&req);
        if extracted == "unknown" {
            tracing::warn!(
                "限流中间件无法识别客户端 IP（X-Real-IP / X-Forwarded-For / ConnectInfo 均缺失），拒绝请求"
            );
            Err(AppError::bad_request("无法识别客户端 IP，请通过反向代理访问"))
        } else {
            Ok(extracted)
        }
    }?;

    let user_id = req
        .extensions()
        .get::<AuthContext>()
        .map(|auth| auth.user_id.to_string())
        .unwrap_or_else(|| "anonymous".to_string());

    let rate_key = format!("rate:{}:{}", ip, user_id);

    // 漏洞 #6 修复：分布式优先，失败回退内存
    let allowed = check_rate_limit(
        &rate_key,
        180,
        Duration::from_secs(60),
        &GLOBAL_LIMITER,
    )
    .await;

    if !allowed {
        tracing::warn!("Rate limit exceeded for {}", rate_key);
        return Err(AppError::TooManyRequests {
            retry_after: Some(60),
            message: "请求过于频繁".to_string(),
        });
    }

    Ok(next.run(req).await)
}

/// 防暴力攻击中间件（针对登录端点）
/// 基于 IP + Username 双维度检查，防止从同一 IP 尝试不同用户名的暴力破解
pub async fn anti_brute_force(req: Request<Body>, next: Next) -> Result<Response, AppError> {
    // P3 维度 12 修复（批次 87）：复用 audit_context::extract_client_ip helper 消除重复实现
    // P2-12b 修复（批次 83 v1 复审）：与 rate_limit_by_ip 对齐三级降级 + 400 兜底
    let ip = {
        let extracted = crate::middleware::audit_context::extract_client_ip(&req);
        if extracted == "unknown" {
            tracing::warn!(
                "防暴力中间件无法识别客户端 IP（X-Real-IP / X-Forwarded-For / ConnectInfo 均缺失），拒绝请求"
            );
            Err(AppError::bad_request("无法识别客户端 IP，请通过反向代理访问"))
        } else {
            Ok(extracted)
        }
    }?;

    // 漏洞 #6 修复：分布式优先，失败回退内存
    let allowed = check_rate_limit(
        &format!("bf:{}", ip),
        5,
        Duration::from_secs(300),
        &BRUTE_FORCE_LIMITER,
    )
    .await;

    if !allowed {
        tracing::warn!("Brute force blocked for IP {}", ip);
        return Err(AppError::TooManyRequests {
            retry_after: Some(300),
            message: "登录尝试次数过多，请5分钟后再试".to_string(),
        });
    }

    Ok(next.run(req).await)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// 漏洞 #6 修复单元测试：未配置 Redis 时，check_redis_rate_limit 返回 Ok(None)
    ///
    /// 验证：默认（无 REDIS_URL / RATE_LIMIT_REDIS_URL）环境下，
    /// Redis 限流器应返回 `Ok(None)`，由调用方（`check_rate_limit`）回退到内存限流
    #[tokio::test]
    async fn test_redis_rate_limiter_disabled_when_no_url() {
        // 确保没有 RATE_LIMIT_REDIS_URL / REDIS_URL
        std::env::remove_var("RATE_LIMIT_REDIS_URL");
        std::env::remove_var("REDIS_URL");

        let result = check_redis_rate_limit("test:key", 5, Duration::from_secs(60)).await;
        assert!(
            result.is_ok(),
            "未配置 Redis URL 时 check_redis_rate_limit 应返回 Ok"
        );
        assert!(
            result.unwrap().is_none(),
            "未配置 Redis URL 时应返回 Ok(None) 指示调用方回退内存限流"
        );
    }

    /// 漏洞 #6 修复单元测试：check_rate_limit 在无 Redis 时回退内存
    ///
    /// 验证：check_rate_limit 优先 Redis，未配置时回退到内存限流器
    #[tokio::test]
    async fn test_check_rate_limit_falls_back_to_memory() {
        std::env::remove_var("RATE_LIMIT_REDIS_URL");
        std::env::remove_var("REDIS_URL");

        let limiter = MemoryRateLimiter::new(2, Duration::from_secs(60));
        let key = "test:fallback:key";

        // 前 2 次允许
        assert!(check_rate_limit(key, 2, Duration::from_secs(60), &limiter).await);
        assert!(check_rate_limit(key, 2, Duration::from_secs(60), &limiter).await);
        // 第 3 次拒绝（内存限流器 max=2）
        assert!(
            !check_rate_limit(key, 2, Duration::from_secs(60), &limiter).await,
            "回退内存限流器后 max=2 时第 3 次应被拒绝"
        );
    }

    /// 漏洞 #6 修复单元测试：MemoryRateLimiter 基础功能
    #[tokio::test]
    async fn test_memory_rate_limiter_basic() {
        let limiter = MemoryRateLimiter::new(3, Duration::from_millis(100));
        let key = "test:basic";

        // 前 3 次允许
        assert!(limiter.check(key));
        assert!(limiter.check(key));
        assert!(limiter.check(key));
        // 第 4 次拒绝
        assert!(!limiter.check(key));

        // 等待窗口过期
        tokio::time::sleep(Duration::from_millis(150)).await;
        // 窗口重置后又允许
        assert!(
            limiter.check(key),
            "窗口过期后计数应重置并放行"
        );
    }
}
