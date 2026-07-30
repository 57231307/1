# 安全说明（SECURITY）

> 本文档描述后端服务的安全机制，包含 2026-06-03 重构 + V15 P0/P1 修复阶段新增的安全加固。
> 适用版本：main 分支（PR #758 合并后，2026-07-28）。

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

- **未删除任何权限校验**：拆分路由时 `auth/permission` 中间件链路完整保留（多租户已于 2026-06-28 删除）
- **未替换 JWT 签名算法**：仍使用 HS256 + 启动时加载的 secret
- **未改变密码哈希**：仍使用 `argon2`，`hash_password` / `verify_password` 未变
- **未关闭审计日志**：`middleware/omni_audit.rs` 与 `operation_log.rs` 全量保留

## 八、V15 P0/P1 修复阶段新增安全机制（2026-07-28）

### 8.1 认证安全加固（P1-A）

| 机制 | 位置 | 说明 |
|------|------|------|
| refresh_token Cookie 有效期 | `auth_service.rs` | 调整为 7 天，与 access_token 2 小时匹配 |
| PUBLIC_PATHS 严格匹配 | `middleware/public_routes.rs` | 使用 `contains()` 精确匹配，防止路径绕过 |
| Webhook 日志脱敏 | `auth.rs` `mask_auth_header/mask_username` | Authorization 头截断 + 用户名 PII 脱敏 |
| 用户 is_active 实时校验 | `auth.rs` `is_user_active_cached` | 5 分钟内存缓存，禁用用户旧 JWT 最坏 5 分钟失效 |
| 用户级 Token 吊销 | `auth_service_ops/jti.rs` `revoke_user_jtis` | 软删除/封禁用户时即时吊销所有活跃 session |

### 8.2 文件上传安全

| 机制 | 位置 | 说明 |
|------|------|------|
| 文件 magic bytes 校验 | `file_upload_validator` | 校验文件实际类型而非扩展名 |
| Zip 炸弹防护 | `file_upload_validator` | 限制解压后大小 + 压缩比 |

### 8.3 数据脱敏与隐私合规

| 机制 | 位置 | 说明 |
|------|------|------|
| PII 字段脱敏 | `utils/field_mask.rs` `mask_phone/mask_email/mask_id_card/mask_bank_card` | 手机号/邮箱/身份证/银行卡脱敏 |
| JSON 递归脱敏 | `field_mask.rs` `desensitize_json` | 持久化前递归脱敏 JSON 敏感字段 |
| AI 推理数据脱敏 | `field_mask.rs` `mask_text_pii` | AI 推理输入捕获手机/邮箱/身份证 PII |
| 行为日志脱敏 | `tracking_service.rs` | 行为追踪持久化前调用 `desensitize_json` |
| 用户隐私同意 | `UserConsentService` | 用户可 opt-in/opt-out 追踪授权 |
| 90 天数据保留 | `tracking_cleanup_service.rs` | 行为追踪数据自动归档+清理 |

### 8.4 权限维度加固

| 机制 | 位置 | 说明 |
|------|------|------|
| 职责分离（SoD） | `permission.rs` `validate_sod_create_approve` | 采购/销售 create 与 approve 权限拆分 |
| admin 移除 audit:read | `init_admin_permissions.sql` | 审计职责独立到 auditor 角色 |
| 字段级权限 | `migration 20260730000001` | 敏感字段权限种子数据 |
| Redis 缓存热更新 | `permission.rs` `start_permission_cache_pubsub_subscriber` | 权限变更通过 pub/sub 即时失效 |
| 异常权限识别 | `permission_compliance_service.rs` | 6 类检测规则 + 定期合规审查 |
| role.code 不可修改 | `role_service.rs` | 防止权限提升攻击 |

### 8.5 导出安全

| 机制 | 位置 | 说明 |
|------|------|------|
| 导出审计 | `audit_log` 表 5 字段 | export_record_count/query_filter/file_format/approval_token/watermark_user |
| 永久禁止导出黑名单 | `export_policy` | lab_dip/production_recipe/flow_card 资源永久禁止导出 |
| 导出并发控制 | `ExportConcurrencyGuard` | AtomicUsize + MAX_CONCURRENT_EXPORTS=10 |
| 导出条数上限 | sales/purchase order 导出 | `.limit(10000)` 防止大量数据泄露 |
| 每日合规审查 | `export_compliance_scheduler` | 6 类异常导出检测规则 |

### 8.6 法律合规

| 机制 | 位置 | 说明 |
|------|------|------|
| 用户协议接入 | `user.agreed_to_terms_at` | 注册/登录时记录协议同意时间 |
| 销售合同电子签章 | `contract_signature_service.rs` | SHA-256 防篡改 + 签名验证 |
| 排污许可证管理 | `pollution_permit_service.rs` | 90/60/30 天三级预警 |
| 劳动合同电子化 | `labor_contract_service.rs` | 《劳动合同法》第19/20条合规校验 |
| 社保公积金扣缴 | `social_insurance_service.rs` | 五险一金费率 + 缴费基数合规校验 |
| 职业健康合规 | `occupational_health_service.rs` | GBZ 2.1/2.2 国标限值 + PPE 管理 |

## 九、已知限制与未来工作

| 限制 | 说明 | 建议方案 |
|------|------|---------|
| JTI 黑名单在内存 | 多实例不共享 | 替换为 Redis（已有 Redis 依赖） |
| SQL 审计为黑名单 | 无法覆盖所有攻击变种 | 主要依赖 SeaORM 参数化查询 |
| `unwrap()` 30+ 处 | 多数是 fail-fast | 持续重构为 `?` 操作符 |
| 前端 console.* | 46 个文件未统一 | 引入 `utils/logger.ts` |
| 分布式追踪未对接 OTel | 当前仅 W3C `traceparent` 透传 | 未来按需引入 `opentelemetry` + `tracing-opentelemetry` |
| `ErrorResponse.trace_id` | 当前每次错误独立生成 UUID | 后续可与 `trace_context` 中间件的 `trace_id` 关联 |

## 十、安全报告

如发现安全漏洞，请联系：[security@57231307.com](mailto:security@57231307.com)

请勿在公开 Issue 中披露，请遵循负责任的披露原则。
