# 安全说明（SECURITY）

> 本文档描述后端服务在 2026-06-03 重构中新增及加强的安全机制。
> 适用版本：commit `f891419` + P3 收尾（mod.rs 精简 + metrics 增强 + W3C Trace Context）之后的 main 分支。

## 一、HTTP 安全响应头

`backend/src/middleware/security_headers.rs` 在所有响应（含错误响应）上统一附加 6 个安全头：

| 头 | 值 | 作用 |
|----|----|------|
| `Content-Security-Policy` | `default-src 'self'; script-src 'self' 'unsafe-inline'; ...` | 限制资源加载源，防止 XSS |
| `Strict-Transport-Security` | `max-age=63072000; includeSubDomains; preload` | 强制 HTTPS（2 年） |
| `X-Content-Type-Options` | `nosniff` | 禁止 MIME 嗅探 |
| `X-Frame-Options` | `DENY` | 禁止 iframe 嵌入（防 clickjacking） |
| `Referrer-Policy` | `no-referrer` | 不向外站发送来源 |
| `Permissions-Policy` | `geolocation=(), microphone=(), camera=(), payment=()` | 关闭敏感 API |

中间件挂载位置：`backend/src/main.rs` 的 `main()` 函数中，通过 7 个 `SetResponseHeaderLayer::overriding(...)` 直接挂载在路由之外（7 个头：上述 6 个 + `X-XSS-Protection`）。

## 二、SQL 注入审计

`backend/src/middleware/sql_injection_audit.rs` 维护 15 个危险模式白名单：

```rust
const DANGEROUS_PATTERNS: &[&str] = &[
    "' OR '1'='1", "' OR 1=1", "'; DROP TABLE", "'; DELETE FROM",
    "'; UPDATE ", "'; INSERT INTO", "UNION SELECT", "/*", "*/",
    "xp_cmdshell", "sp_executesql", "INFORMATION_SCHEMA.TABLES",
    "INFORMATION_SCHEMA.COLUMNS", "LOAD_FILE(", "INTO OUTFILE",
];
```

- **审计范围**：仅检查 URL 路径与查询字符串，不读 body（避免性能开销）
- **命中行为**：返回 `400 BadRequest` 并记录 WARN 日志
- **本质防护**：SeaORM 默认使用参数化查询，中间件为粗粒度兜底

## 三、JWT JTI 黑名单

`backend/src/services/auth_service.rs` 新增进程级 JTI 黑名单：

```rust
static JTI_BLACKLIST: Lazy<RwLock<HashSet<String>>> =
    Lazy::new(|| RwLock::new(HashSet::new()));

pub async fn revoke_jti(jti: &str) { ... }
pub async fn is_jti_revoked(jti: &str) -> bool { ... }
pub async fn cleanup_expired_jti(_max_age_secs: i64) { ... }
```

- 登出接口调用 `revoke_jti()`，将 JTI 加入黑名单
- `middleware/auth.rs` 解析 JWT 后调用 `is_jti_revoked()` 校验
- 定时任务（建议每小时）调用 `cleanup_expired_jti()` 清理过期项
- 限制：当前实现为进程内 HashSet，多实例部署需替换为 Redis

## 四、统一错误响应

`backend/src/utils/error.rs` 新增 `ErrorResponse` 结构体：

```rust
#[derive(Debug, Clone, Serialize)]
pub struct ErrorResponse {
    pub code: String,        // NOT_FOUND / BAD_REQUEST / UNAUTHORIZED ...
    pub message: String,     // 文案（生产环境脱敏）
    pub trace_id: String,    // UUID，每次错误唯一
    pub timestamp: i64,      // UTC 秒
}
```

- 通过 `cfg!(debug_assertions)` 区分环境：
  - `debug`（dev/test）：返回 `Display` 完整信息便于排查
  - `release`（生产）：返回通用脱敏文案，SQL 片段/堆栈不外泄
- 调用方式：`app_error.to_response()` 即可获得结构化错误

## 五、CORS 配置

`backend/src/config/settings.rs` 新增 `CorsConfig`：

```rust
pub struct CorsConfig {
    pub allowed_origins: Vec<String>,
    pub allow_credentials: bool,
    pub allowed_methods: Vec<String>,
    pub allowed_headers: Vec<String>,
    pub max_age_secs: u64,
}
```

- 默认值仅允许 `http://localhost:3000` 与 `http://localhost:5173`
- 通过环境变量 `CORS_ALLOWED_ORIGINS`（逗号分隔）覆盖
- `CorsConfig::from_env()` 提供兜底加载能力

## 六、输入校验提取器

`backend/src/middleware/validation.rs` 提供 `ValidatedJson<T>`：

- 自动为请求生成 trace_id
- 校验失败时统一返回 `ErrorResponse` 结构
- 业务 handler 改用 `ValidatedJson<T>` 替换裸 `Json<T>` 即可启用

## 七、路由/服务重构期间保持的安全边界

- **未删除任何权限校验**：拆分路由时 `auth/permission/tenant` 中间件链路完整保留
- **未替换 JWT 签名算法**：仍使用 HS256 + 启动时加载的 secret
- **未改变密码哈希**：仍使用 `argon2`，`hash_password` / `verify_password` 未变
- **未关闭审计日志**：`middleware/omni_audit.rs` 与 `operation_log.rs` 全量保留

## 八、已知限制与未来工作

| 限制 | 说明 | 建议方案 |
|------|------|---------|
| JTI 黑名单在内存 | 多实例不共享 | 替换为 Redis（已有 Redis 依赖） |
| SQL 审计为黑名单 | 无法覆盖所有攻击变种 | 主要依赖 SeaORM 参数化查询 |
| `unwrap()` 30+ 处 | 多数是 fail-fast | 持续重构为 `?` 操作符 |
| 前端 console.* | 46 个文件未统一 | 引入 `utils/logger.ts` |
| 分布式追踪未对接 OTel | 当前仅 W3C `traceparent` 透传 | 未来按需引入 `opentelemetry` + `tracing-opentelemetry` |
| `ErrorResponse.trace_id` | 当前每次错误独立生成 UUID | 后续可与 `trace_context` 中间件的 `trace_id` 关联 |

## 九、安全报告

如发现安全漏洞，请联系：[TODO: 添加内部邮箱]

请勿在公开 Issue 中披露，请遵循负责任的披露原则。
