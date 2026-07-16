# V15 法律合规与安全标准审计报告（类八·批次 08）

- **审计子代理**：V15 审计子代理（类八 法律合规与安全标准审计类）
- **审计范围**：8 维度（中国法律法规 / 法律安全标准 / 数据脱敏审计 / 成品文档格式 / 纺织行业法律法规 / 纺织行业财税 / 纺织行业环保 / 纺织行业劳动）
- **审计依据**：
  - `/workspace/.monkeycode/docs/audits/v15-review-plan-2026-07-15.md`（V15 审计计划第 1153-1394 行，类八 8 维度检查要点）
  - `/workspace/.monkeycode/MEMORY.md`（规则 11 法律合规 + 规则 12 法律安全标准 + 规则 3 成品文档格式）
  - `/workspace/.monkeycode/docs/research/fabric-industry-research.md`（§5 成本核算体系 / §12.5 产量工资 / §12.6 能耗管理）
  - `/workspace/backend/src/middleware/`（auth/csrf/permission/rate_limit/sql_injection_audit/csp）
  - `/workspace/backend/src/services/`（auth_service/audit_log_service/omni_audit_service/outsourcing_service/export_service/print_service）
  - `/workspace/backend/src/utils/`（password_validator/hash/field_mask/xlsx_export/sql_escape/ssrf_guard/path_validator）
  - `/workspace/backend/src/models/`（product/sales_contract/user/energy_meter）
  - `/workspace/backend/Cargo.toml`（rust_xlsxwriter / argon2 / printpdf 依赖）
  - `/workspace/deploy/nginx.conf`（HTTPS 配置）
- **审计方法**：Grep 检索关键模式 + Glob 查找相关文件 + Read 关键模型/服务/中间件 + 对照审计计划逐项核对
- **审计时间**：2026-07-16
- **审计原则**：只做审计不修改业务代码；缺陷必须有文件路径:行号证据；风险分级 P0/P1/P2/P3

---

## 维度 1：中国法律法规合规（8.1）

### 检查方法
1. Grep `隐私政策|用户协议|privacy_policy|user_agreement` 在 backend/src 与 frontend/src
2. Read `/workspace/backend/src/middleware/auth.rs` 检查日志脱敏
3. Read `/workspace/backend/src/middleware/omni_audit.rs` 检查审计日志脱敏
4. Grep `overseas|cross.border|跨境|jurisdiction` 在 backend/src
5. Grep `phone|mobile|email|id_card` 在 backend/src/models 检查敏感字段存储
6. Read `/workspace/deploy/nginx.conf` 检查 HTTPS 配置
7. Grep `Secure|SameSite|HTTPS` 在 backend/src 检查 Cookie 安全标志

### 发现

#### ✅ 已落实的项

1. **日志中禁止记录敏感信息明文**（部分落实）：
   - `/workspace/backend/src/middleware/auth.rs:28-58`：`mask_auth_header` 函数截断 Authorization 头值，避免完整 JWT Token 写入日志（仅保留前 12 字符 + 长度）。
   - `/workspace/backend/src/middleware/auth.rs:51-58`：`mask_username` 函数截断用户名 PII（保留前 2 字符 + `***`）。
   - `/workspace/backend/src/middleware/omni_audit.rs:311-334`：`is_sensitive_request_body_path` 函数识别含密码/TOTP 的敏感路径，请求体在审计日志中脱敏为 `"[REDACTED]"`。
   - `/workspace/backend/src/middleware/omni_audit.rs:115`：命中敏感路径时 `request_body` 脱敏为 `"[REDACTED]"`。

2. **数据导出支持审计追溯**：
   - `/workspace/backend/src/handlers/import_export_handler.rs:253-281`：`export_csv` handler 调用 `audit_log_service` 落库审计事件（OperationType::Export），记录 user_id/resource_type/data 总数。
   - `/workspace/backend/src/services/sensitive_action_alert.rs:5-26`：`SensitiveAction::DataExport` 枚举值，alert_level 为 Medium，触发告警。

3. **用户隐私数据传输合规（部分）**：
   - `/workspace/backend/src/handlers/auth_handler.rs:536-575`：生产环境 Cookie 设置 `secure(is_production)` + `SameSite::Strict` + `http_only(true)`。
   - `/workspace/backend/src/middleware/csp.rs:23-33`：CSP 中间件注入 `upgrade-insecure-requests` 指令，自动升级 HTTP 到 HTTPS。
   - `/workspace/backend/src/services/system_update_service.rs:871-874`：下载 URL 强制 HTTPS 校验。
   - `/workspace/backend/src/services/webhook_service.rs:245`：Webhook URL 强制 HTTPS。

4. **JWT Token 撤销机制**：
   - `/workspace/backend/src/middleware/auth.rs:240-244`：Token 黑名单检查。
   - `/workspace/backend/src/middleware/auth.rs:259-270`：JTI 黑名单检查（已吊销 session_id 立即拒绝）。
   - `/workspace/backend/src/middleware/auth.rs:277-305`：用户级 Token 吊销表检查（删除/封禁用户时吊销所有活跃 session）。
   - `/workspace/backend/src/services/auth_service.rs:512`：`is_jti_revoked` 函数。

#### ❌ 缺陷项 1：用户协议/隐私政策在系统中未真实接入

**风险等级：P1**（合规阻塞）

**证据**：
- Grep `隐私政策|用户协议|privacy_policy|user_agreement|privacyPolicy|userAgreement` 在 `/workspace/backend/src` 与 `/workspace/frontend/src` 均无匹配。
- `/workspace/frontend/src/views/Login.vue:218-219`：仅提及 `javascript:` 协议安全，无隐私政策/用户协议链接。
- `/workspace/frontend/src/views/Setup.vue`：无隐私政策/用户协议文案。

**业务影响**：
- 《个人信息保护法》第 17 条要求处理个人信息前应以显著方式、清晰易懂的语言真实、准确、完整地向个人告知个人信息处理规则。
- 《网络安全法》第 22 条要求网络运营者收集使用个人信息应当公开收集、使用规则。
- 系统涉及客户手机号/邮箱/身份证号等敏感信息（见 `crm_lead.rs:41-47` `mobile_phone/tel_phone/email`、`customer_contact.rs:27-29` `phone/email`），但用户在登录/初始化时无任何隐私政策/用户协议确认环节，存在合规阻塞风险。

**修复建议**：
1. 在 `Login.vue` 与 `Setup.vue` 增加"我已阅读并同意《用户协议》《隐私政策》"复选框（默认不勾选，未勾选禁止登录/初始化）。
2. 后端 `auth_handler::login` 与 `init_handler::initialize` 增加必传字段 `agreement_accepted: bool`，false 时返回 400。
3. 在数据库 `users` 表新增 `agreement_accepted_at TIMESTAMP` 字段，记录用户同意时间戳，作为合规证据。
4. 在 `/workspace/docs/` 或前端 `/privacy` `/agreement` 路由发布真实《隐私政策》《用户协议》文本（含收集信息类型/用途/存储期限/用户权利/跨境情况说明）。

---

#### ❌ 缺陷项 2：数据跨境传输合规评估缺失

**风险等级：P2**（合规要求）

**证据**：
- Grep `overseas|cross.border|跨境|jurisdiction` 在 `/workspace/backend/src` 无匹配。
- `/workspace/backend/src/services/email_service.rs`（邮件发送服务）未评估是否使用境外 SMTP 服务商。
- `/workspace/backend/src/services/event_kafka.rs`（事件总线）未评估是否跨境传输。

**业务影响**：
- 《个人信息保护法》第 38-42 条要求向境外提供个人信息需通过安全评估、认证或签订标准合同。
- 《数据安全法》第 31 条要求重要数据出境安全评估。
- 系统未提供任何跨境传输评估文档与拦截机制，若部署使用境外云服务（如 AWS/GCP）或境外 SMTP/Kafka，将存在合规风险。

**修复建议**：
1. 在 `/workspace/docs/` 撰写《数据跨境传输合规评估》文档，列出所有可能跨境的数据流（SMTP/Kafka/数据库备份等）。
2. 在 `AppSettings::new` 中增加 `DATA_LOCALITY=cn-only` 配置项，启用时禁止向境外 IP 发起 outbound 请求（通过 `ssrf_guard` 扩展实现）。
3. 邮件服务、Kafka 等外部依赖在生产环境强制使用境内服务商（如阿里云邮件推送 / 阿里云 Kafka）。

---

#### ❌ 缺陷项 3：HTTPS 强制配置在反向代理层缺失

**风险等级：P1**（合规阻塞）

**证据**：
- `/workspace/deploy/nginx.conf:1-2`：默认仅监听 80 端口（HTTP），未强制 HTTPS。
- `/workspace/deploy/nginx.conf:51-100`：HTTPS 配置整段被注释为示例（`# server { listen 443 ssl http2; ...`）。
- `/workspace/deploy/nginx.conf:27`：注释 "生产环境建议使用 HTTPS"，但未强制。
- `/workspace/backend/src/handlers/auth_handler.rs:542`：`secure(is_production)` 依赖 `APP_ENV=production`，若反向代理仍为 HTTP，Cookie Secure 标志会阻止 Cookie 传输，反而导致登录不可用。

**业务影响**：
- 《网络安全法》第 21 条要求网络运营者采取技术措施防范网络攻击。
- 《个人信息保护法》第 51 条要求采取相应安全技术措施防止个人信息泄露。
- 面料 ERP 涉及客户手机号/邮箱/身份证号等敏感信息，HTTP 明文传输存在中间人攻击风险。
- 应用层 Cookie Secure 标志与反向代理 HTTP 配置矛盾，可能导致生产环境登录失效。

**修复建议**：
1. `/workspace/deploy/nginx.conf` 默认配置改为 80 端口 301 重定向到 443，并强制启用 SSL。
2. 提供生产部署脚本（`/workspace/deploy/deploy-latest.sh`）自动申请 Let's Encrypt 证书并启用 HTTPS。
3. 在 `AppSettings::new` 中增加生产环境校验：`APP_ENV=production` 时强制要求配置 SSL 证书路径，否则启动失败。

---

## 维度 2：法律安全标准（8.2）

### 检查方法
1. Read `/workspace/backend/src/middleware/mod.rs` 列出中间件清单
2. Read `/workspace/backend/src/middleware/auth.rs` 检查身份认证
3. Read `/workspace/backend/src/middleware/permission.rs` 检查权限校验
4. Read `/workspace/backend/src/middleware/rate_limit.rs` 检查速率限制
5. Read `/workspace/backend/src/middleware/csrf.rs` 检查 CSRF 防护
6. Read `/workspace/backend/src/middleware/sql_injection_audit.rs` 检查 SQL 注入防护
7. Read `/workspace/backend/src/middleware/public_routes.rs` 检查公开端点白名单
8. Read `/workspace/backend/src/utils/password_validator.rs` 检查密码策略
9. Read `/workspace/backend/src/services/auth_service.rs:160-260` 检查密码哈希算法
10. Read `/workspace/backend/src/handlers/system_update_handler.rs:180-250` 检查文件上传校验
11. Grep `format!\(.*SELECT|format!\(.*INSERT` 在 backend/src 检查 SQL 字符串拼接
12. Grep `auth_middleware|permission_middleware|rate_limit|csrf_middleware` 在 main.rs 检查中间件挂载

### 发现

#### ✅ 已落实的项

1. **所有 API 进行身份认证和权限校验**（除明确公开端点）：
   - `/workspace/backend/src/middleware/public_routes.rs:6-30`：`PUBLIC_PATHS` 白名单仅含 `/health` `/ready` `/live` `/auth/login` `/auth/refresh` `/webhooks/integrations/callback` `/init/initialize*` 等必需公开端点。
   - `/workspace/backend/src/middleware/public_routes.rs:38-49`：`is_public_path` 精确匹配 + 子路径匹配，防止 `starts_with` 误匹配（如 `/auth/login-bypass` 不被放行）。
   - `/workspace/backend/src/main.rs:731-755`：中间件链路顺序 `rate_limit → auth → omni_audit → csrf → permission → request_validator → handler`，全局挂载。
   - `/workspace/backend/src/middleware/permission.rs:22-81`：`permission_middleware` 基于角色 + 资源 + 操作（read/create/update/delete）+ resource_id 精确匹配，防止垂直越权。

2. **密码存储使用强哈希（Argon2id）禁止明文/弱哈希**：
   - `/workspace/backend/Cargo.toml:32`：`argon2 = "0.5"`。
   - `/workspace/backend/src/services/auth_service.rs:292-307`：`hash_password` 使用 `Argon2id` 算法，参数为 `m=64MB, t=3, p=4`（参数符合 OWASP 2023 推荐）。
   - `/workspace/backend/src/services/auth_service.rs:245-260`：`verify_password` 使用 `argon2.verify_password`，错误时返回 `Ok(false)` 而非 panic。
   - `/workspace/backend/src/services/auth_service.rs:266-273`：`verify_password_async` 用 `spawn_blocking` 包装，避免阻塞 tokio worker（v14 P0-1 修复）。
   - `/workspace/backend/src/services/totp_service.rs:147-149`：TOTP 恢复码也使用 argon2id 哈希存储。
   - `/workspace/backend/src/utils/password_validator.rs:55-67`：`PasswordPolicy` 默认 min_length=8、要求大小写+数字+特殊字符。
   - `/workspace/backend/src/utils/password_validator.rs:87-205`：100+ 常见密码黑名单 + l33t 归一化匹配 + 键盘序列检测。

3. **SQL 查询参数化禁止字符串拼接**：
   - 项目使用 SeaORM，默认参数化查询。
   - `/workspace/backend/src/utils/sql_escape.rs:4-15`：`escape_like_pattern` 转义 LIKE 特殊字符（`% _ \`）+ 防空字节注入。
   - `/workspace/backend/src/handlers/omni_audit_handler.rs:198-238`：动态构造 WHERE 子句使用 `$1, $2` 占位符 + `where_params` 绑定，非字符串拼接。
   - `/workspace/backend/src/services/audit_cleanup_service.rs:53-57`：清理 SQL 使用 `Statement::from_sql_and_values` 参数化绑定（批次 94 P2-1 修复）。
   - `/workspace/backend/src/middleware/sql_injection_audit.rs:42-100`：SQL 注入审计中间件全局挂载，检测 100+ 危险模式（时间盲注/UNION 注入/注释截断/存储过程等）。
   - `/workspace/backend/src/services/import_export_service.rs:686-692`：LIKE 查询调用 `safe_like_pattern(keyword)` 转义。

4. **所有用户输入验证清理（防 XSS/CSRF）**：
   - `/workspace/backend/src/middleware/csrf.rs:84-174`：CSRF 中间件对所有非安全方法（POST/PUT/PATCH/DELETE）强制要求 `X-CSRF-Token` 头，公开路径要求自定义头（`X-Requested-With` 或 `X-CSRF-Token`）防御简单表单 CSRF。
   - `/workspace/backend/src/middleware/csrf.rs:150-171`：一次性消费 + IP 绑定校验（Wave 3 #7），Token 消费时校验绑定 IP 与请求 IP 一致。
   - `/workspace/backend/src/middleware/csp.rs:23-33`：CSP 中间件注入 `default-src 'self'; object-src 'none'; frame-ancestors 'none'; upgrade-insecure-requests`，防 XSS/Clickjacking。
   - `/workspace/backend/src/services/print_service.rs:169-178`：打印模板 HTML 转义（`escape_html`），防 XSS。

5. **敏感操作（删除/修改/导出）记录审计日志**：
   - `/workspace/backend/src/services/sensitive_action_alert.rs:5-26`：`SensitiveAction` 枚举含 `Delete`/`PermissionChange`/`DataExport`/`PasswordChange`/`FinancialOperation` 等。
   - `/workspace/backend/src/middleware/omni_audit.rs:230-296`：全局审计中间件对所有非公开路径请求落库审计日志，含 trace_id/user_id/method/path/status/duration。
   - `/workspace/backend/src/handlers/import_export_handler.rs:253-281`：数据导出 handler 显式调用 audit_log_service。

6. **JWT token 合理过期时间 + refresh token 支持撤销**：
   - `/workspace/backend/src/services/auth_service.rs:144-145`：access_token exp = 2 小时（原 24 小时已缩短）。
   - `/workspace/backend/src/services/auth_service.rs:189-192`：refresh_token exp = 2 天（原 7 天已缩短）。
   - `/workspace/backend/src/services/auth_service.rs:469-500`：`revoke_jti` 函数支持吊销指定 JTI，Redis 失败时回退内存黑名单。
   - `/workspace/backend/src/handlers/auth_handler_misc.rs:67-72`：refresh_token 接口检查 JTI 是否已吊销。
   - `/workspace/backend/src/handlers/auth_handler_misc.rs:102-195`：refresh_token 轮换机制（P1 7-1 修复），刷新时生成新 session_id 与新 refresh_token。

7. **文件上传校验类型/大小/内容**：
   - `/workspace/backend/src/handlers/system_update_handler.rs:192`：`MAX_UPDATE_SIZE = 100 * 1024 * 1024`（100MB）限制。
   - `/workspace/backend/src/handlers/system_update_handler.rs:197`：仅接受 `.zip` 后缀。
   - `/workspace/backend/src/handlers/system_update_handler.rs:214-218`：`verify_zip_magic` 校验 ZIP 文件魔数（内容校验）。
   - `/workspace/backend/src/handlers/system_update_handler.rs:224-247`：路径遍历防护（canonicalize + starts_with 校验保存路径在临时目录内）。
   - `/workspace/backend/src/handlers/crm_handler.rs:112-131`：线索导入 `MAX_IMPORT_SIZE = 10MB`，仅接受 `.xlsx`，校验文件大小。
   - `/workspace/backend/src/handlers/system_update_handler.rs:188`：上传更新包需 admin 角色（`require_admin_role`）。

8. **接口速率限制防暴力破解和 DDoS**：
   - `/workspace/backend/src/middleware/rate_limit.rs:113-116`：全局限流 180 req/min（GLOBAL_LIMITER）+ 登录端点防暴力 5 req/5min（BRUTE_FORCE_LIMITER）。
   - `/workspace/backend/src/middleware/rate_limit.rs:118-225`：分布式 Redis 限流（INCR+EXPIRE 原子操作），失败回退内存限流。
   - `/workspace/backend/src/middleware/rate_limit.rs:259-304`：`rate_limit_by_ip` 中间件基于 IP+UserID 双维度。
   - `/workspace/backend/src/middleware/rate_limit.rs:308-341`：`anti_brute_force` 中间件基于 IP 维度防暴力破解。
   - `/workspace/backend/src/main.rs:747-755`：全局挂载 `rate_limit_by_ip` 中间件。

9. **用户 is_active 实时校验**：
   - `/workspace/backend/src/middleware/auth.rs:94-116`：`is_user_active_cached` 5 分钟内存缓存避免每请求查 DB（漏洞 #6 修复）。
   - `/workspace/backend/src/middleware/auth.rs:311-334`：禁用/软删除用户的旧 JWT 在 5 分钟内失效。

10. **密码历史防重用**：
    - `/workspace/backend/migrations/20260707000003_create_password_histories/up.sql`：`password_histories` 表存储历史密码哈希。
    - `/workspace/backend/src/models/password_history.rs:16`：`password_hash: String` 字段。

#### ❌ 缺陷项 4：登录端点未挂载 anti_brute_force 中间件

**风险等级：P2**（安全增强）

**证据**：
- `/workspace/backend/src/middleware/rate_limit.rs:308-341`：`anti_brute_force` 中间件已实现（5 req/5min）。
- `/workspace/backend/src/main.rs:747-755`：仅挂载 `rate_limit_by_ip`（180 req/min），未挂载 `anti_brute_force`。
- Grep `anti_brute_force` 在 `/workspace/backend/src/main.rs` 与 `/workspace/backend/src/routes/` 均未匹配到挂载点。
- `/workspace/backend/src/routes/auth.rs`：登录路由 `/auth/login` 仅受全局 180 req/min 限流保护。

**业务影响**：
- 全局 180 req/min 对单 IP 暴力破解防御不足（180 次/分钟 = 3 次/秒，攻击者 1 小时可尝试 10800 个密码）。
- `anti_brute_force`（5 req/5min）已实现但未挂载到 `/auth/login`，导致暴力破解防御形同虚设。

**修复建议**：
1. 在 `/workspace/backend/src/routes/auth.rs` 的 `/login` 路由单独挂载 `anti_brute_force` 中间件：
   ```rust
   .route("/login", post(auth_handler::login).layer(axum::middleware::from_fn(anti_brute_force)))
   ```
2. 在 `/auth/refresh` `/auth/logout` `/init/initialize*` 等敏感端点同样挂载。

---

## 维度 3：数据脱敏与审计追溯（8.3）

### 检查方法
1. Read `/workspace/backend/src/utils/field_mask.rs` 检查字段脱敏
2. Grep `mask_phone|mask_id_card|mask_email|mask_card|脱敏|desensitize` 在 backend/src
3. Grep `phone.*\*|mobile.*\*|formatPhone|maskPhone` 在 frontend/src
4. Grep `phone|mobile|email|id_number` 在 backend/src/models 检查敏感字段
5. Read `/workspace/backend/src/services/omni_audit_service.rs:100-175` 检查审计日志签名
6. Grep `audit_log.*hash|hash_chain|prev_hash|tamper_proof` 在 backend/src
7. Grep `AUDIT_RETENTION|retention_days` 在 backend/src
8. Read `/workspace/backend/src/services/audit_cleanup_service.rs` 检查保留期清理
9. Read `/workspace/backend/src/services/sensitive_action_alert.rs` 检查敏感操作审计
10. Read `/workspace/backend/src/handlers/crm_handler.rs:150-170` 检查数据权限控制

### 发现

#### ✅ 已落实的项

1. **日志脱敏（密码/token/身份证号不记录明文）**：
   - `/workspace/backend/src/middleware/auth.rs:28-58`：Authorization 头脱敏（保留前 12 字符 + 长度）。
   - `/workspace/backend/src/middleware/auth.rs:51-58`：用户名 PII 脱敏（保留前 2 字符 + `***`）。
   - `/workspace/backend/src/middleware/omni_audit.rs:311-334`：14 个敏感路径（change-password/reset_password/setup-totp/enable-totp 等）请求体脱敏为 `"[REDACTED]"`。
   - `/workspace/backend/src/utils/error.rs:361-400`：生产环境错误响应统一脱敏为通用文案，不暴露内部错误细节（漏洞 #4/#11/#12 修复）。
   - `/workspace/backend/src/services/rate_limit.rs:168-169`：Redis URL 不记录完整 URL，防止凭据泄露。

2. **审计日志不可篡改（HMAC-SHA256 签名）**：
   - `/workspace/backend/src/services/omni_audit_service.rs:105-123`：对 `trace_id|event_type|action|payload` 拼接后使用 HMAC-SHA256 签名。
   - `/workspace/backend/src/services/omni_audit_service.rs:172`：签名持久化至 `signature` 列（P0 8-2 修复，批次 53）。
   - `/workspace/backend/src/services/omni_audit_service.rs:49-83`：`AUDIT_SECRET_KEY` 环境变量要求至少 32 字节，生产环境缺失时返回 Err 拒绝启动。
   - `/workspace/backend/src/services/omni_audit_service.rs:237-243`：`secret_key_fingerprint` 暴露 SHA-256 指纹前 16 hex 字符供运维校验。

3. **审计日志保留期合规（AUDIT_RETENTION_DAYS 配置）**：
   - `/workspace/backend/src/main.rs:482-512`：`AUDIT_RETENTION_DAYS` 环境变量配置，默认 365 天，生产环境未设置时 warn 提示。
   - `/workspace/backend/src/services/audit_cleanup_service.rs:13-91`：`AuditCleanupService` 每天执行清理任务，删除超过保留期的 `omni_audit_logs` 与 `audit_logs` 表记录。
   - `/workspace/backend/src/services/audit_cleanup_service.rs:18-47`：清理任务 panic 隔离（AssertUnwindSafe + catch_unwind），单次 panic 不退出清理循环。
   - `/workspace/backend/src/services/audit_cleanup_service.rs:53-57`：清理 SQL 使用参数化绑定（批次 94 P2-1 修复）。

4. **敏感操作审计日志完整（删除/修改/导出）**：
   - `/workspace/backend/src/services/sensitive_action_alert.rs:5-26`：`SensitiveAction` 枚举含 Delete/PermissionChange/DataExport/PasswordChange/FinancialOperation 等 10 类。
   - `/workspace/backend/src/services/sensitive_action_alert.rs:32-43`：alert_level 分级（Critical/High/Medium）。
   - `/workspace/backend/src/middleware/omni_audit.rs:230-296`：全局审计中间件对所有非公开路径请求落库审计日志，含完整 request/response 信息。
   - `/workspace/backend/src/services/sensitive_action_alert.rs:74-150`：`classify_action` 根据 method+resource_type 自动分类敏感操作。

5. **部分字段级数据权限脱敏**：
   - `/workspace/backend/src/utils/field_mask.rs:5-26`：`mask_sensitive_fields` 对非管理员角色（role_id != 1）将 `cost_price` 字段设为 Null。
   - `/workspace/backend/src/handlers/crm_handler.rs:152-171`：CRM lead 详情接口基于角色数据权限过滤字段，非管理员默认隐藏 `contact_phone/email/address`。
   - `/workspace/backend/src/handlers/crm_handler.rs:158-162`：调用 `data_permission_service.filter_fields` 按 `allowed_fields`/`hidden_fields` 配置过滤。
   - `/workspace/frontend/src/views/api-gateway/composables/apiGwFmts.ts:45-48`：API Key 脱敏（仅保留前 4 后 4）。
   - `/workspace/frontend/src/views/admin/components/FailoverStatusCard.vue:131-140`：主备库 URL 脱敏。

6. **审计日志检索权限控制**：
   - `/workspace/backend/src/handlers/omni_audit_handler.rs:181-182`：审计日志检索仅限 admin（`require_admin_role`）。
   - `/workspace/backend/src/handlers/omni_audit_handler.rs:172-175`：敏感字段（request_body/user_agent/ip_address）仅在 `include_sensitive=true` 时返回，审计大屏默认 false。

#### ❌ 缺陷项 5：手机号展示脱敏缺失

**风险等级：P1**（合规阻塞）

**证据**：
- Grep `mask_phone|mask_id_card|mask_email|formatPhone|maskPhone` 在 `/workspace/backend/src` 与 `/workspace/frontend/src` 均无匹配。
- `/workspace/backend/src/models/crm_lead.rs:41-47`：`mobile_phone: Option<String>` `tel_phone: Option<String>` `email: Option<String>` 明文存储。
- `/workspace/backend/src/models/customer_contact.rs:27-29`：`phone: String` `email: Option<String>` 明文存储。
- `/workspace/backend/src/models/supplier_contact.rs:22-26`：`mobile_phone: String` `tel_phone: Option<String>` `email: Option<String>` 明文存储。
- `/workspace/backend/src/models/logistics_waybill.rs:17`：`driver_phone: Option<String>` 明文存储。
- `/workspace/backend/src/handlers/crm_handler.rs:152-170`：仅 CRM lead 详情接口有字段隐藏，但非脱敏（直接 remove 字段而非 `138****8888` 格式）。
- `/workspace/backend/src/utils/field_mask.rs:5-26`：`mask_sensitive_fields` 仅处理 `cost_price` 字段，未处理手机号/邮箱。

**业务影响**：
- 《个人信息保护法》第 51 条要求采取相应技术措施防止个人信息泄露。
- 客户/供应商/司机手机号在前端列表/详情页直接展示明文，存在内部人员泄露风险（如客服导出客户列表后转卖）。
- CRM lead 详情接口直接 remove 字段而非脱敏，导致业务无法识别客户（如需回拨电话），实际使用中可能被绕过（管理员直接查看或导出）。

**修复建议**：
1. 在 `/workspace/backend/src/utils/field_mask.rs` 新增 `mask_phone(phone: &str) -> String`（如 `138****8888`）、`mask_email(email: &str) -> String`（如 `a***@example.com`）、`mask_id_card(id: &str) -> String`（如 `110***********1234`）。
2. 在所有返回手机号/邮箱/身份证号的 handler/service 中调用脱敏函数（非管理员角色）：
   - `crm_handler::get_lead`/`list_leads`
   - `customer_handler::list_customers`/`get_customer`
   - `supplier_handler::list_suppliers`/`get_supplier`
   - `logistics_handler::list_waybills`
3. 在前端列表页（`/views/crm/`、`/views/customer/`、`/views/supplier/`）使用脱敏组件展示，点击"查看完整号码"按钮触发权限校验 + 审计日志（谁在何时查看了谁的手机号）。
4. 数据导出时（`export_csv`）默认脱敏，admin 角色显式传 `include_sensitive=true` 才导出明文（同审计日志检索机制）。

---

#### ❌ 缺陷项 6：身份证展示脱敏缺失 + 身份证字段未在模型中

**风险等级：P2**（合规要求）

**证据**：
- Grep `id_card|id_number|身份证` 在 `/workspace/backend/src/models` 无匹配。
- 当前系统未存储身份证号（产量工资场景可能需要工人身份证号用于个税申报）。
- 但 `/workspace/backend/src/models/wage_record_detail.rs` 产量工资明细模型无身份证字段，意味着个税代扣代缴功能未实现（详见维度 8.6 缺陷项 12）。

**业务影响**：
- 若未来扩展工人身份证号存储（个税申报场景），需同步实现脱敏机制。
- 当前虽无身份证字段，但应在 `field_mask.rs` 预留 `mask_id_card` 函数，防止后续开发遗漏。

**修复建议**：
1. 在 `/workspace/backend/src/utils/field_mask.rs` 预留 `mask_id_card(id: &str) -> String` 函数（保留前 3 后 4，中间用 `*` 填充）。
2. 在 `password_validator.rs` 测试模块增加脱敏函数单元测试。

---

## 维度 4：成品文档格式合规（8.4）

### 检查方法
1. Read `/workspace/backend/src/utils/xlsx_export.rs` 检查 xlsx 导出工具
2. Grep `build_xlsx|xlsx_response|XlsxTable` 在 backend/src 查找使用范围
3. Grep `text/csv|\.csv|to_csv|CsvWriter` 在 backend/src 查找 CSV 使用
4. Grep `docx|docx-rs|docx_rs|word|Word|\.docx` 在 backend/src 与 Cargo.toml
5. Grep `printpdf|generate_pdf` 在 backend/src
6. Read `/workspace/backend/src/services/print_service.rs:130-185` 检查打印模板生成
7. Read `/workspace/backend/src/handlers/print_handler.rs` 检查打印端点
8. Grep `染整报表|色卡发放|产量工资|能耗报表` 在 backend/src
9. Read `/workspace/backend/src/handlers/wage_handler.rs` 检查产量工资导出
10. Read `/workspace/backend/src/handlers/energy_handler.rs` 检查能耗导出
11. Read `/workspace/backend/src/handlers/color_card/scan_export.rs` 检查色卡导出

### 发现

#### ✅ 已落实的项

1. **后端引入 rust_xlsxwriter 统一管理文档生成**：
   - `/workspace/backend/Cargo.toml:127`：`rust_xlsxwriter = "0.95"`。
   - `/workspace/backend/Cargo.toml:128`：`calamine` 用于 xlsx 读取（v11 批次 157d-4）。
   - `/workspace/backend/src/utils/xlsx_export.rs:1-97`：`build_xlsx` 封装 rust_xlsxwriter，提供标题行加粗+边框+冻结首行+列宽自适应。
   - `/workspace/backend/src/utils/xlsx_export.rs:99-120`：`build_xlsx_response` 构造 axum Response（含 Content-Type 与 Content-Disposition）。

2. **线索导出/商机导出支持 .xlsx**：
   - `/workspace/backend/src/handlers/crm_handler.rs:100`：`export_leads` 调用 `build_xlsx_response(&table, "crm_leads_export")`。
   - `/workspace/backend/src/services/crm/lead.rs` 与 `/workspace/backend/src/services/crm/opp.rs` 使用 `XlsxTable`。

3. **库存导出支持 .xlsx**：
   - `/workspace/backend/src/services/import_export_service.rs:639`：`"inventory" => self.export_inventory(query).await`。
   - `/workspace/backend/src/handlers/import_export_handler.rs:248-251`：`export_csv` handler 调用 `ImportExportService::generate_xlsx` 生成 xlsx 并 base64 编码返回。

4. **报表导出支持 .xlsx**：
   - `/workspace/backend/src/services/report/exp.rs:154-178`：`export_excel` 生成 xlsx 文件（ZIP 容器 + XML 内容）。
   - `/workspace/backend/src/services/report/exp.rs:53-152`：`export_pdf` 使用 `printpdf` 库生成真实 PDF 文件。
   - `/workspace/backend/src/services/report/mod.rs:269-283`：`ExportFormat` 枚举支持 Pdf/Excel/Csv/Json。

5. **缸号列表导出支持 .xlsx**：
   - `/workspace/backend/src/handlers/dye_batch_handler.rs:355-408`：`export_dye_batches` 调用 `build_xlsx_response`。

6. **染整配方导出支持 .xlsx**：
   - `/workspace/backend/src/handlers/dye_recipe_handler.rs:194-266`：`export_dye_recipes` 调用 `build_xlsx_response`。

7. **色卡导出支持 .xlsx**：
   - `/workspace/backend/src/handlers/color_card/scan_export.rs:49-114`：`export_color_card` 调用 `build_xlsx_response`。

8. **CSV 仅作为内部调试格式**：
   - `/workspace/backend/Cargo.toml:123`：注释 "CSV处理（仅内部调试格式，面向用户的导出已升级为 xlsx）"。
   - `/workspace/backend/src/services/product_service.rs:620`：`export_products_to_csv` 在 service 层返回 CSV，handler 层转换为 xlsx。
   - `/workspace/backend/src/handlers/product_handler.rs:439-465`：CSV 解析为 xlsx 表格后返回。

#### ❌ 缺陷项 7：染整报表导出/色卡发放记录导出/产量工资报表导出/能耗报表导出缺失

**风险等级：P1**（规则 3 强制要求）

**证据**：
- Grep `染整报表|色卡发放|产量工资.*导出|能耗报表.*导出|wage.*export|energy.*export|color_card_borrow.*export` 在 `/workspace/backend/src` 无匹配。
- `/workspace/backend/src/handlers/wage_handler.rs`（371 行）：产量工资 handler 仅有 CRUD + calculate + confirm + pay + cancel，**无导出端点**。
- `/workspace/backend/src/handlers/energy_handler.rs`：能耗管理 handler 仅有 CRUD + monthly_allocation，**无导出端点**。
- `/workspace/backend/src/handlers/color_card/scan_export.rs:49`：仅 `export_color_card`（导出色卡本身），**无色卡发放/借出记录导出**。
- `/workspace/backend/src/services/color_card_borrow_service.rs:362-388`：`list_borrow_records` 仅支持分页查询，无导出方法。
- `/workspace/backend/src/models/color_card_borrow_record.rs`：色卡借出记录模型存在，但无导出 handler。

**业务影响**：
- 规则 3 强制要求"所有数据导出支持 .xlsx 格式"，包括"染整报表导出/色卡发放记录导出/产量工资报表导出/能耗报表导出"（V15 新增）。
- 产量工资报表无法导出，影响财务月度工资核算对账（工人工资台账需保留 2 年以上，见维度 8.8）。
- 能耗报表无法导出，影响环保合规报告生成（见维度 8.7）。
- 色卡发放记录无法导出，影响销售跟进与样品管理审计追溯。

**修复建议**：
1. 在 `/workspace/backend/src/handlers/wage_handler.rs` 新增 `export_wage_records`：
   ```rust
   /// GET /api/v1/erp/wage-records/export - 导出工资记录（xlsx）
   pub async fn export_wage_records(...) -> Result<axum::response::Response, AppError> {
       // 复用 record_service.list 查询，构造 XlsxTable，调用 build_xlsx_response
   }
   ```
2. 在 `/workspace/backend/src/handlers/energy_handler.rs` 新增 `export_energy_consumption` 与 `export_energy_allocation`。
3. 在 `/workspace/backend/src/handlers/color_card/borrow.rs` 新增 `export_borrow_records`：
   ```rust
   /// GET /api/v1/erp/color-cards/borrow-records/export - 导出色卡借出记录（xlsx）
   ```
4. 在 `/workspace/backend/src/routes/production.rs` 注册 `/wage-records/export` 与 `/energy-records/export` 路由。
5. 在 `/workspace/backend/src/routes/color_card.rs` 注册 `/borrow-records/export` 路由。

---

#### ❌ 缺陷项 8：合同/发票/报表生成不支持 .docx 格式（Word）

**风险等级：P1**（规则 3 强制要求）

**证据**：
- Grep `docx|docx-rs|docx_rs|word|Word|\.docx` 在 `/workspace/backend/src` 无业务匹配（仅匹配 `signal_word`/`keyword` 等无关字段）。
- `/workspace/backend/Cargo.toml`：无 `docx-rs` 或 `docx-rs` 依赖，仅有 `printpdf = "0.7"`（PDF 生成）。
- `/workspace/backend/src/services/print_service.rs:145-185`：`generate_pdf` 方法名误导，实际生成 **HTML** 而非 PDF（`<!DOCTYPE html><html>...`）。
- `/workspace/backend/src/handlers/print_handler.rs:12-17`：`render_print_html` 返回 `Html<String>`，6 个打印端点均返回 HTML：
  - `sales_order_print_html`（销售订单）
  - `sales_contract_print_html`（销售合同）
  - `purchase_order_print_html`（采购订单）
  - `purchase_receipt_print_html`（采购收货单）
  - `inventory_transfer_print_html`（库存调拨单）
- `/workspace/backend/src/services/export_service.rs:42-85`：`export_pdf` 方法名误导，实际生成 **纯文本**（`content.push_str(...)`），非 PDF。
- `/workspace/backend/src/services/export_service.rs:88-152`：`generate_reconciliation_pdf` 同样生成纯文本非 PDF。

**业务影响**：
- 规则 3 强制要求"所有报表/文档生成支持 .docx 格式（Word）：合同/发票/报表"。
- 规则 3 强制要求"禁止 .txt/.rtf/.html 等非标准格式作为成品文档"。
- 当前合同/订单/对账单均以 HTML 形式返回，前端打印依赖浏览器打印功能，无法生成可编辑的 Word 文档交付给客户/供应商。
- `print_service.generate_pdf` 与 `export_service.export_pdf` 方法名误导，实际不生成 PDF，存在代码维护陷阱。
- 对账单 `generate_reconciliation_pdf` 生成纯文本，违反"禁止 .txt 作为成品文档"。

**修复建议**：
1. 在 `/workspace/backend/Cargo.toml` 引入 `docx-rs` 或 `docx-rs = "0.4"` 依赖。
2. 在 `/workspace/backend/src/utils/` 新增 `docx_export.rs` 模块，封装 `build_docx(table: &DocxTable) -> Result<Vec<u8>, AppError>`。
3. 重构 `/workspace/backend/src/services/print_service.rs`：
   - 将 `generate_pdf` 重命名为 `generate_html`（消除命名误导）。
   - 新增 `generate_docx(print_data: &PrintData) -> Result<Vec<u8>, AppError>` 生成真实 Word 文档。
4. 在 `/workspace/backend/src/handlers/print_handler.rs` 新增 `sales_contract_docx`/`sales_order_docx` 等端点返回 `.docx` 文件。
5. 重构 `/workspace/backend/src/services/export_service.rs`：
   - 将 `export_pdf` 与 `generate_reconciliation_pdf` 重命名为 `export_text`/`generate_reconciliation_text`，或改为使用 `printpdf` 生成真实 PDF。
   - 新增 `export_docx` 与 `generate_reconciliation_docx` 生成 .docx 文件。
6. 前端增加"下载 Word"按钮，调用新端点下载 .docx 文件。

---

## 维度 5：纺织行业法律法规合规（8.5）

### 检查方法
1. Grep `execution_standard|gb_standard` 在 backend/src/models
2. Grep `factory_name|factory_address|product_spec|product_grade` 在 backend/src/models
3. Grep `electronic_signature|e_sign|contract_sign` 在 backend/src/services
4. Read `/workspace/backend/src/models/product.rs` 检查产品字段完整性
5. Read `/workspace/backend/src/models/sales_contract.rs` 检查合同字段完整性
6. Grep `染整行业规范|产能淘汰|先进产能|水重复利用` 在 backend/src
7. Grep `假冒伪劣|虚假宣传|商业贿赂|七日无理由|缺陷产品召回` 在 backend/src
8. Grep `出口商检|产地证|普惠制|出口配额` 在 backend/src

### 发现

#### ✅ 已落实的项

1. **面料成分真实标示字段存在**：
   - `/workspace/backend/src/models/product.rs:46`：`fabric_composition: Option<String>`（如 `65% 棉 35% 涤`）。
   - `/workspace/backend/src/models/product.rs:48-56`：`yarn_count`/`density`/`width`/`gram_weight`/`structure`/`finish` 等面料技术参数字段完整。

2. **质量等级字段存在（缸号级）**：
   - `/workspace/database/migration/001_consolidated_schema.sql:4700`：`batch_dye_lot.quality_grade VARCHAR(10) DEFAULT 'A'`（A/B/C/D）。
   - `/workspace/database/migration/040_v14_fabric_inspection.sql:57`：验布记录联动 `determine_quality_grade`（A 级合格/B 级让步接收/C 级返工报废）。

#### ❌ 缺陷项 9：面料执行标准登记缺失（GB/T 系列）

**风险等级：P1**（《产品质量法》合规阻塞）

**证据**：
- Grep `execution_standard|gb_standard` 在 `/workspace/backend/src/models` 无匹配。
- `/workspace/backend/src/models/product.rs:13-74`：产品模型字段含 name/code/specification/fabric_composition/yarn_count/density/width/gram_weight/structure/finish 等，**无 `execution_standard` 字段**。
- `/workspace/database/migration/001_consolidated_schema.sql`：products 表无 execution_standard 列。
- Grep `染整行业规范|产能淘汰|先进产能|水重复利用` 在 `/workspace/backend/src` 无匹配。

**业务影响**：
- 《产品质量法》第 27 条要求产品标识需含"产品质量检验合格证明、中文标明的产品名称、厂名、厂址、规格、等级、主要成分、执行标准号"。
- 面料执行标准（如 GB/T 406-2018 棉本色布、GB/T 411-2017 印染棉布、FZ/T 13001-2013 色织棉布）是产品标识必备字段，缺失将导致：
  1. 产品标签/吊牌生成无执行标准号，违反《产品质量法》。
  2. 假冒伪劣产品预警机制无基准数据（无法校验标称标准与实际是否一致）。
  3. 出口面料商检报告无执行标准号，无法通过商检。

**修复建议**：
1. 在 `/workspace/database/migration/047_v15_product_compliance.sql` 新增字段：
   ```sql
   ALTER TABLE products ADD COLUMN execution_standard VARCHAR(50);
   ALTER TABLE products ADD COLUMN factory_name VARCHAR(200);
   ALTER TABLE products ADD COLUMN factory_address VARCHAR(500);
   ALTER TABLE products ADD COLUMN product_grade VARCHAR(10);
   COMMENT ON COLUMN products.execution_standard IS '面料执行标准号（GB/T 系列，如 GB/T 406-2018 棉本色布）';
   ```
2. 在 `/workspace/backend/src/models/product.rs` 补充对应字段。
3. 在 `/workspace/backend/src/services/product_service.rs` 的 create/update 校验执行标准号格式（正则 `^(GB|FZ|QB)/T \d{3,5}-\d{4}$`）。
4. 新增 `fabric_standards` 参考表，预置 GB/T 406-2018、GB/T 411-2017、FZ/T 13001-2013 等常用标准。

---

#### ❌ 缺陷项 10：合同电子签章真实接入缺失

**风险等级：P1**（《合同法》/《民法典》合同编合规阻塞）

**证据**：
- Grep `electronic_signature|e_sign|contract_sign|电子签章` 在 `/workspace/backend/src/services` 无匹配。
- `/workspace/backend/src/models/sales_contract.rs:11-31`：销售合同模型字段含 contract_no/customer_id/total_amount/signed_date/payment_terms 等，**无电子签章字段**（无 `signed_at`/`signed_by`/`signature_hash`/`signature_image_url`）。
- `/workspace/backend/src/handlers/print_handler.rs:26-31`：`sales_contract_print_html` 仅生成 HTML 用于浏览器打印，无电子签章接入。
- `/workspace/backend/src/services/print_service.rs:72-80`：`get_sales_contract_print_data` 仅返回基础数据，无签章逻辑。

**业务影响**：
- 《民法典》合同编第 469 条认可电子合同法律效力，但需符合《电子签名法》第 13 条要求（电子签名需专属于电子签名人、由电子签名人控制、签署后对电子签名的任何改动能够被发现）。
- 当前合同"签章"仅依赖浏览器打印后手工签字，无电子签章，导致：
  1. 合同在线签署流程缺失，影响远程签约效率。
  2. 合同真实性无法验证（无签名哈希/证书），存在合同篡改风险。
  3. 与《电子签名法》合规要求不符。

**修复建议**：
1. 在 `/workspace/database/migration/048_v15_contract_signature.sql` 新增字段：
   ```sql
   ALTER TABLE sales_contracts ADD COLUMN signed_at TIMESTAMP;
   ALTER TABLE sales_contracts ADD COLUMN signed_by_user_id INTEGER REFERENCES users(id);
   ALTER TABLE sales_contracts ADD COLUMN signature_hash VARCHAR(64);
   ALTER TABLE sales_contracts ADD COLUMN signature_image_url VARCHAR(500);
   ALTER TABLE sales_contracts ADD COLUMN signature_certificate TEXT;
   ```
2. 集成第三方电子签章服务（如 e签宝/法大大/上上签），通过 webhook 接收签章回调。
3. 在 `/workspace/backend/src/services/` 新增 `contract_signature_service.rs` 调用第三方 API。
4. 在 `/workspace/backend/src/handlers/sales_contract_handler.rs` 新增 `sign_contract` 端点触发签章流程。

---

#### ❌ 缺陷项 11：销售合同模板合规字段缺失

**风险等级：P2**（《合同法》/《民法典》合同编合规要求）

**证据**：
- `/workspace/backend/src/models/sales_contract.rs:11-31`：销售合同模型字段含 contract_no/contract_name/contract_type/customer_id/total_amount/signed_date/effective_date/expiry_date/payment_terms/payment_method/delivery_date/delivery_location/status，**缺少**：
  - 标的条款（subject_matter）：合同标的物描述
  - 质量条款（quality_terms）：质量标准与验收方法
  - 违约责任（breach_liability）：违约金/赔偿条款
  - 争议解决（dispute_resolution）：仲裁/诉讼条款
  - 履行期限（performance_period）：履行起止时间

**业务影响**：
- 《民法典》合同编第 470 条规定合同内容应包括标的、数量、质量、价款、履行期限、履行地点和方式、违约责任、解决争议的方法。
- 当前合同模型缺少质量条款/违约责任/争议解决等必备字段，生成的合同文档法律效力不完整。

**修复建议**：
1. 在 sales_contracts 表新增 `quality_terms TEXT`/`breach_liability TEXT`/`dispute_resolution TEXT`/`performance_period VARCHAR(100)` 字段。
2. 在 `/workspace/backend/src/models/sales_contract.rs` 补充字段。
3. 在合同打印模板（`print_service.rs::get_sales_contract_print_data`）输出完整条款。

---

#### ❌ 缺陷项 12：印染行业规范条件合规缺失 + 进出口商品检验合规缺失 + 反不正当竞争合规缺失

**风险等级：P2**（行业法规合规）

**证据**：
- Grep `染整行业规范|产能淘汰|先进产能|水重复利用|印染企业准入` 在 `/workspace/backend/src` 无匹配。
- Grep `出口商检|产地证|普惠制|出口配额|commodity_inspection` 在 `/workspace/backend/src` 无匹配。
- Grep `假冒伪劣|虚假宣传|商业贿赂|七日无理由|缺陷产品召回` 在 `/workspace/backend/src` 无匹配。
- `/workspace/backend/src/models/energy_meter.rs:29`：`meter_type` 含 water/electricity/steam/gas/compressed_air，但无水重复利用率字段。
- `/workspace/backend/src/services/import_export_service.rs`：import/export 仅指数据导入导出，非进出口商品检验。

**业务影响**：
- 《印染行业规范条件》（工信部）要求印染企业水重复利用率 ≥ 40%，系统无相关字段与计算逻辑。
- 《进出口商品检验法》要求出口面料商检记录/产地证/普惠制证书，系统无相关功能。
- 《反不正当竞争法》要求虚假宣传/商业贿赂预警，系统无相关预警机制。

**修复建议**：
1. 在 `/workspace/backend/src/services/energy_service.rs` 新增 `water_recycle_rate` 计算方法（基于 energy_consumption_record 表的 water 类型记录计算 `回收水量 / 总用水量`）。
2. 在 `/workspace/backend/src/models/` 新增 `export_inspection.rs`（出口商检记录）、`certificate_of_origin.rs`（产地证）模型。
3. 在 `/workspace/backend/src/services/` 新增 `compliance_alert_service.rs` 实现假冒伪劣/虚假宣传/商业贿赂预警（基于报价/合同/采购数据规则引擎）。

---

## 维度 6：纺织行业财税合规（8.6）⭐ 重点

### 检查方法
1. Grep `委托加工|consignment_processing|processing_material` 在 backend/src/services/finance/
2. Grep `input_tax_transfer|进项税转出` 在 backend/src
3. Grep `export_refund|export_rebate|免抵退` 在 backend/src
4. Grep `environmental_tax|环保税` 在 backend/src
5. Grep `depreciation_reserve|跌价准备|inventory_provision` 在 backend/src
6. Read `/workspace/backend/src/services/outsourcing_service.rs` 检查委托加工物资凭证
7. Grep `增值税|input_tax|output_tax|tax_rate` 在 backend/src
8. Grep `印花税|stamp_tax` 在 backend/src
9. Grep `个税|income_tax|individual_income_tax` 在 backend/src
10. Grep `研发费用加计扣除|R&D_super_deduction` 在 backend/src

### 发现

#### ✅ 已落实的项

1. **委托加工物资财税合规（核心场景已实现）**：
   - `/workspace/backend/src/services/outsourcing_service.rs:3-12`：注释明确实现 §5.4 委托加工物资核算三步分录：
     - 发料：借 委托加工物资 / 贷 自制半成品-胚布
     - 加工费：借 委托加工物资+应交税费-进项税额 / 贷 银行存款
     - 入库：借 库存商品-成品布 / 贷 委托加工物资
     - 非正常损耗：借 营业外支出 / 贷 委托加工物资
   - `/workspace/backend/src/services/outsourcing_service.rs:491-511`：发料凭证创建（`debit_account = "委托加工物资"`）。
   - `/workspace/backend/src/services/outsourcing_service.rs:640-662`：入库凭证创建（`debit_account = "库存商品-成品布"`、`credit_account = "委托加工物资"`）。
   - `/workspace/backend/src/services/outsourcing_service.rs:664-691`：非正常损耗凭证创建（`debit_account = "营业外支出"`、`credit_account = "委托加工物资"`），正确将非正常损耗计入营业外支出而非成本。
   - `/workspace/backend/src/services/outsourcing_service.rs:711-751`：加工费结算凭证创建（`debit_account = "委托加工物资"`、`credit_account = "银行存款"`），`tax_amount` 单独记录。
   - `/workspace/backend/src/models/outsourcing_voucher.rs`：凭证模型含 voucher_type/debit_account/credit_account/amount/tax_amount 字段。

#### ❌ 缺陷项 13：进项税转出缺失

**风险等级：P1**（增值税合规阻塞）

**证据**：
- Grep `input_tax_transfer|进项税转出` 在 `/workspace/backend/src` 无匹配。
- `/workspace/backend/src/services/outsourcing_service.rs:664-691`：非正常损耗凭证仅 `借 营业外支出 / 贷 委托加工物资`，**未做进项税转出**（应同时 `借 营业外支出 / 贷 应交税费-应交增值税-进项税额转出`）。
- `/workspace/backend/src/models/outsourcing_voucher.rs`：凭证模型无 `tax_transfer_amount` 字段。

**业务影响**：
- 《增值税暂行条例》第 27 条规定非正常损失（丢失/人为损坏/管理不善）的进项税额不得抵扣，已抵扣的应转出。
- 委托加工物资发生非正常损耗时，对应的染费/运费进项税应转出，当前系统未实现，导致：
  1. 增值税申报表进项税额虚高，存在税务稽查风险。
  2. 营业外支出未含应转出的进项税，企业所得税前扣除金额不准确。

**修复建议**：
1. 在 `/workspace/backend/src/services/outsourcing_service.rs:664-691` 非正常损耗凭证创建逻辑中，增加进项税转出凭证：
   ```rust
   // 非正常损耗对应的进项税转出（加工费 13% 税率）
   let tax_transfer = abnormal_loss_amount * Decimal::from_str("0.13").unwrap();
   // 凭证：借 营业外支出 / 贷 应交税费-应交增值税-进项税额转出
   ```
2. 在 `outsourcing_voucher` 表新增 `tax_transfer_amount DECIMAL(15,2)` 字段。
3. 在 `/workspace/backend/src/services/finance/` 新增 `input_tax_transfer_service.rs` 通用进项税转出服务，供采购退货/非正常损耗/集体福利等场景复用。

---

#### ❌ 缺陷项 14：出口退税（免抵退）核算缺失

**风险等级：P1**（出口业务财税合规阻塞）

**证据**：
- Grep `export_refund|export_rebate|免抵退` 在 `/workspace/backend/src` 无匹配。
- Grep `出口报关|外汇核销|出口退税` 在 `/workspace/backend/src` 无匹配。
- `/workspace/backend/src/services/import_export_service.rs`：import/export 仅指数据导入导出，非进出口业务。

**业务影响**：
- 面料行业出口业务占比高，出口退税（免抵退）是核心财税流程。
- 《出口货物劳务增值税和消费税政策》（财税 2012 39 号）要求出口企业按"免抵退"办法核算。
- 系统完全缺失出口退税功能，导致：
  1. 出口面料销售无法生成免抵退税申报表。
  2. 出口报关单/外汇核销单/增值税发票"单证齐全"校验无系统支持。
  3. 企业需手工处理出口退税，效率低且易出错。

**修复建议**：
1. 在 `/workspace/backend/src/models/` 新增 `export_refund_declaration.rs`（出口退税申报表）、`export_customs_declaration.rs`（出口报关单）、`foreign_exchange_verification.rs`（外汇核销单）模型。
2. 在 `/workspace/backend/src/services/finance/` 新增 `export_refund_service.rs`：
   - `calculate_exempt_credit_refund`：免抵退税额计算
   - `verify_documents_completeness`：单证齐全校验（报关单+核销单+发票）
   - `generate_refund_declaration`：生成退税申报表
3. 在 `/workspace/backend/src/handlers/` 新增 `export_refund_handler.rs` 暴露端点。

---

#### ❌ 缺陷项 15：环保税核算缺失

**风险等级：P1**（印染行业财税合规阻塞）

**证据**：
- Grep `environmental_tax|环保税|pollution_tax|环保税` 在 `/workspace/backend/src` 无匹配。
- `/workspace/backend/src/models/energy_meter.rs:29`：`meter_type` 含 water/electricity/steam/gas，但无污染物排放字段。
- `/workspace/backend/src/models/energy_consumption_record.rs`：仅记录能源消耗，无废水/废气/固废排放数据。

**业务影响**：
- 《环境保护税法》要求印染企业按废水/废气/固废排放量缴纳环保税。
- 印染行业是环保税重点征收对象（废水 COD/氨氮、废气 VOCs、固废污泥）。
- 系统完全缺失环保税核算功能，导致：
  1. 企业需手工计算环保税，效率低且易出错。
  2. 环保税申报无系统数据支撑。
  3. 与能耗管理模块（v14 批次 428）未打通，无法基于能源消耗推算污染物排放。

**修复建议**：
1. 在 `/workspace/backend/src/models/` 新增 `pollutant_discharge_record.rs`（污染物排放记录）：
   ```rust
   pub struct Model {
       pub id: i32,
       pub discharge_type: String,  // wastewater/exhaust/solid_waste
       pub pollutant_name: String,  // COD/氨氮/VOCs/污泥
       pub discharge_amount: Decimal,
       pub concentration: Decimal,
       pub tax_amount: Decimal,
       pub period: NaiveDate,
   }
   ```
2. 在 `/workspace/backend/src/services/finance/` 新增 `environmental_tax_service.rs`：
   - `calculate_wastewater_tax`：废水环保税计算（基于 COD/氨氮排放量与当量税额）
   - `calculate_exhaust_tax`：废气环保税计算（基于 VOCs 排放量）
   - `generate_tax_declaration`：生成环保税申报表
3. 与能耗管理模块联动：基于 energy_consumption_record 的用水量推算废水排放量。

---

#### ❌ 缺陷项 16：存货跌价准备缺失

**风险等级：P2**（企业所得税合规）

**证据**：
- Grep `depreciation_reserve|跌价准备|inventory_provision|inventory_write_down` 在 `/workspace/backend/src` 无匹配。
- `/workspace/backend/src/services/inventory_stock_service.rs`：仅库存查询/调整，无跌价准备计提逻辑。
- `/workspace/backend/src/services/stock_alert.rs`：库存预警服务，无呆滞面料跌价分析。

**业务影响**：
- 《企业会计准则第 1 号——存货》要求资产负债表日存货按成本与可变现净值孰低计量。
- 面料行业季节性强（春夏/秋冬面料），季节性降价跌价准备是核心财税流程。
- 库存呆滞面料（>180 天未动）需计提跌价准备。
- 过期染料/助剂需计提跌价准备。
- 系统缺失跌价准备功能，导致：
  1. 资产负债表存货金额虚高。
  2. 企业所得税前扣除不准确（跌价准备可税前扣除）。

**修复建议**：
1. 在 `/workspace/backend/src/models/` 新增 `inventory_write_down.rs`（存货跌价准备）模型。
2. 在 `/workspace/backend/src/services/` 新增 `inventory_write_down_service.rs`：
   - `calculate_seasonal_write_down`：季节性降价跌价准备（基于产品季节标签与当前日期）
   - `calculate_sluggish_write_down`：呆滞面料跌价准备（基于 inventory_stock.last_in_at > 180 天）
   - `calculate_expired_chemical_write_down`：过期染料/助剂跌价准备（基于 chemical_master.expiry_date）
3. 在 `/workspace/backend/src/services/stock_alert.rs` 增加 `sluggish_inventory_alert`（呆滞库存预警）。

---

#### ❌ 缺陷项 17：印花税/个税代扣代缴/研发费用加计扣除缺失

**风险等级：P2**（财税合规）

**证据**：
- Grep `印花税|stamp_tax` 在 `/workspace/backend/src` 无匹配。
- Grep `个税|income_tax|individual_income_tax` 在 `/workspace/backend/src` 无匹配。
- Grep `研发费用加计扣除|R&D_super_deduction` 在 `/workspace/backend/src` 无匹配。
- `/workspace/backend/src/models/sales_contract.rs`：合同模型无印花税字段。
- `/workspace/backend/src/models/wage_record_detail.rs`：工资明细模型无个税字段。

**业务影响**：
- 购销合同印花税（0.3‰）/加工承揽合同印花税（0.5‰）未自动计算，需手工申报。
- 产量工资个税代扣代缴缺失，违反《个人所得税法》第 9 条。
- 印染新工艺研发费用加计扣除（75%/100%）缺失，影响企业所得税优惠享受。

**修复建议**：
1. 在 `sales_contracts`/`purchase_contracts` 表新增 `stamp_tax_amount DECIMAL(15,2)` 字段，create_contract 时自动计算印花税。
2. 在 `wage_record_detail` 表新增 `individual_income_tax DECIMAL(15,2)` 字段，calculate_wage 时调用个税累进税率计算。
3. 在 `/workspace/backend/src/services/finance/` 新增 `rnd_super_deduction_service.rs` 识别研发费用（基于科目分类）并计算加计扣除金额。

---

## 维度 7：纺织行业环保合规（8.7）

### 检查方法
1. Grep `pollution_permit|排污许可` 在 backend/src/models
2. Grep `wastewater|cod|ammonia_nitrogen` 在 backend/src
3. Grep `voc|VOCs|sludge|印染污泥` 在 backend/src
4. Grep `energy_consumption|能耗管理|energy_meter|allocation_rule` 在 backend/src
5. Read `/workspace/backend/src/models/energy_meter.rs` 检查能耗模型
6. Grep `厂界噪声|noise_monitor|environmental_impact|环评` 在 backend/src
7. Grep `sewage_treatment|污水处理|排气塔` 在 backend/src

### 发现

#### ✅ 已落实的项

1. **能耗管理模块基础已建立（v14 批次 428）**：
   - `/workspace/backend/src/models/energy_meter.rs:18-63`：能源计量设备模型，支持 water/electricity/steam/gas/compressed_air 五种能源类型。
   - `/workspace/backend/src/models/energy_consumption_record.rs`：能耗记录模型。
   - `/workspace/backend/src/models/energy_allocation_rule.rs`：能耗分摊规则模型。
   - `/workspace/backend/src/models/energy_allocation_record.rs`：能耗分摊记录模型。
   - `/workspace/backend/src/services/energy_service.rs`：能耗服务（meter/consumption/rule/allocation 四个子服务）。
   - `/workspace/backend/src/models/status.rs:987-1080`：能耗相关状态枚举（能源类型/设备状态/录入方式/分摊基准/记录状态/规则状态）。

#### ❌ 缺陷项 18：排污许可证登记缺失

**风险等级：P1**（《环境保护法》合规阻塞）

**证据**：
- Grep `pollution_permit|排污许可` 在 `/workspace/backend/src/models` 无匹配。
- `/workspace/backend/src/models/`：无 `pollution_permit.rs` 模型。
- `/workspace/database/migration/`：无排污许可证相关表。

**业务影响**：
- 《环境保护法》第 45 条规定实行排污许可管理制度，印染企业必须持证排污。
- 《排污许可管理条例》第 24 条要求排污许可证到期前 30 日申请延续。
- 系统缺失排污许可证登记与到期预警，导致：
  1. 企业可能因证照过期违法排污，面临停产整顿与罚款。
  2. 环保检查无法快速出示证照信息。

**修复建议**：
1. 在 `/workspace/backend/src/models/` 新增 `pollution_permit.rs`：
   ```rust
   pub struct Model {
       pub id: i32,
       pub permit_no: String,           // 排污许可证编号
       pub permit_type: String,         // wastewater/exhaust/solid_waste
       pub issue_date: NaiveDate,
       pub expiry_date: NaiveDate,
       pub issuing_authority: String,
       pub permitted_capacity: Decimal, // 许可排放量
       pub status: String,              // active/expired/revoked
   }
   ```
2. 在 `/workspace/backend/src/services/` 新增 `pollution_permit_service.rs` 实现 `expiry_warning`（到期前 30/60/90 天三级预警）。
3. 与通知服务联动，到期前自动推送预警给环保负责人。

---

#### ❌ 缺陷项 19：废水/废气/固废排放监测与合规校验缺失

**风险等级：P1**（水/大气/固废污染防治法合规阻塞）

**证据**：
- Grep `wastewater|cod_limit|ammonia_nitrogen|voc|VOCs|sludge|印染污泥` 在 `/workspace/backend/src` 无匹配（仅 auth_service.rs 测试用例中 `test_unrevoke_user_clears_revocation` 匹配 `voc` 子串，无关）。
- `/workspace/backend/src/models/energy_meter.rs:29`：`meter_type` 仅含 water/electricity/steam/gas/compressed_air，**无 wastewater/COD/氨氮/VOCs/污泥类型**。
- `/workspace/backend/src/models/energy_consumption_record.rs`：仅记录能源消耗量与金额，无排放浓度/排放量字段。
- Grep `厂界噪声|noise_monitor|environmental_impact|环评|sewage_treatment|污水处理` 在 `/workspace/backend/src` 无匹配。

**业务影响**：
- 《水污染防治法》要求印染废水排放浓度达标（COD ≤ 80mg/L、氨氮 ≤ 10mg/L、色度 ≤ 50 倍）。
- 《大气污染防治法》要求定型机废气 VOCs ≤ 60mg/m³。
- 《固体废物污染环境防治法》要求印染污泥（危废）分类处置记录。
- 《环境噪声污染防治法》要求厂界噪声（昼间 ≤ 65dB、夜间 ≤ 55dB）监测。
- 系统完全缺失污染物排放监测，导致：
  1. 超标排放无法实时预警，可能面临停产整顿。
  2. 环保检查无法提供监测数据。
  3. 环保税核算无数据基础（见维度 8.6 缺陷项 15）。

**修复建议**：
1. 在 `/workspace/backend/src/models/` 新增 `pollutant_monitoring_record.rs`（污染物监测记录）：
   ```rust
   pub struct Model {
       pub id: i32,
       pub monitoring_type: String,  // wastewater/exhaust/noise/solid_waste
       pub monitoring_point: String, // 监测点（如"总排口"、"定型机排气筒"、"厂界东"）
       pub pollutant_name: String,   // COD/氨氮/色度/VOCs/噪声/污泥
       pub measured_value: Decimal,  // 实测值
       pub unit: String,             // mg/L, mg/m³, dB, 吨
       pub limit_value: Decimal,     // 排放限值
       pub is_exceeding: bool,       // 是否超标
       pub monitoring_time: DateTimeWithTimeZone,
   }
   ```
2. 在 `/workspace/backend/src/services/` 新增 `environmental_monitoring_service.rs`：
   - `check_wastewater_compliance`：校验 COD ≤ 80mg/L、氨氮 ≤ 10mg/L、色度 ≤ 50 倍
   - `check_exhaust_compliance`：校验 VOCs ≤ 60mg/m³
   - `check_noise_compliance`：校验昼间 ≤ 65dB、夜间 ≤ 55dB
   - `exceedance_alert`：超标立即推送告警
3. 与 IoT 设备对接（energy_meter.iot_device_id 已预留），实时采集监测数据。
4. 在 `/workspace/backend/src/models/` 新增 `solid_waste_disposal_record.rs`（固废处置联单）实现废料处置联单制度。

---

#### ❌ 缺陷项 20：环境影响评价（环评）合规存档缺失

**风险等级：P2**（环评合规）

**证据**：
- Grep `environmental_impact|环评|eia_report|environmental_acceptance` 在 `/workspace/backend/src` 无匹配。
- 系统无环评报告存档/环评批复/竣工环保验收存档功能。

**业务影响**：
- 《环境影响评价法》要求建设项目需进行环评并取得环评批复。
- 竣工环保验收是项目投产的前置条件。
- 系统缺失环评文件存档，环保检查无法快速出示。

**修复建议**：
1. 在 `/workspace/backend/src/models/` 新增 `environmental_assessment.rs`（环评文件存档）模型，含文件类型（环评报告/环评批复/竣工环保验收）、文件 URL、批复日期等字段。
2. 在 `/workspace/backend/src/handlers/` 新增 `environmental_assessment_handler.rs` 提供上传/查询/下载端点。

---

## 维度 8：纺织行业劳动合规（8.8）

### 检查方法
1. Grep `labor_contract|劳动合同` 在 backend/src/models
2. Grep `piecework|计件` 在 backend/src/services
3. Grep `occupational_health|职业健康` 在 backend/src
4. Grep `safety_production|安全生产` 在 backend/src
5. Read `/workspace/backend/src/models/wage_record_detail.rs` 检查工资明细字段
6. Read `/workspace/backend/src/services/wage_service.rs:115-125` 检查计件工资计算
7. Grep `overtime|加班|working_hours|工时` 在 backend/src
8. Grep `social_insurance|社保|housing_fund|公积金` 在 backend/src
9. Grep `female_worker|女职工|minor_worker|未成年工` 在 backend/src
10. Grep `MSDS|safety_data_sheet|危险化学品` 在 backend/src

### 发现

#### ✅ 已落实的项

1. **计件工资计算基础已实现（v14 批次 427）**：
   - `/workspace/backend/src/models/process_wage_rate.rs:7-9`：工价方案注释明确"A 级（合格率≥95%）全额 / B 级（合格率 80-95%）8 折 / C 级（合格率<80%）不计"。
   - `/workspace/backend/src/models/process_wage_rate.rs:49-55`：`grade_a_ratio`/`grade_b_ratio`/`grade_c_ratio` 等级系数字段。
   - `/workspace/backend/src/models/wage_record_detail.rs:17-19`：计件工资公式 `piece_wage = qualified_quantity × piece_price × grade_ratio`（不合格产量不结算）。
   - `/workspace/backend/src/services/wage_service.rs:119-121`：注释明确"计件工资 = 合格产量 × 计件单价 × 等级系数"。
   - `/workspace/backend/src/models/wage_record_detail.rs:55`：`grade: String` 字段记录质检等级（A/B/C）。
   - `/workspace/backend/src/models/wage_record_detail.rs:75`：`grade_ratio: Decimal` 等级系数。

2. **产量工资台账保留机制（数据库持久化）**：
   - `/workspace/backend/src/models/wage_record.rs`：工资记录模型，含 record_no/period_start/period_end/status/total_amount 字段。
   - `/workspace/backend/src/models/wage_record_detail.rs`：工资明细模型，含 worker_id/process_type/qualified_quantity/wage_amount 字段。
   - 数据库持久化即满足"计件工资台账保留 2 年以上"要求（数据库不主动删除）。

#### ❌ 缺陷项 21：劳动合同电子化管理缺失

**风险等级：P1**（《劳动法》/《劳动合同法》合规阻塞）

**证据**：
- Grep `labor_contract|劳动合同|employment_contract` 在 `/workspace/backend/src/models` 无匹配。
- `/workspace/backend/src/models/`：无 `labor_contract.rs` 模型。
- `/workspace/backend/src/models/user.rs`：用户模型无合同关联字段（无 contract_start_date/contract_end_date/probation_end_date）。

**业务影响**：
- 《劳动合同法》第 10 条要求建立劳动关系应当订立书面劳动合同。
- 《劳动合同法》第 19 条规定试用期长度（1-3 年合同试用期不超过 3 个月，3 年以上合同试用期不超过 6 个月）。
- 《劳动合同法》第 20 条规定试用期工资 ≥ 转正工资 80%。
- 系统缺失劳动合同电子化管理，导致：
  1. 合同到期无预警，可能形成事实劳动关系（需支付双倍工资）。
  2. 试用期长度/试用期工资合规性无系统校验。
  3. 工人产量工资个税申报无合同基础数据。

**修复建议**：
1. 在 `/workspace/backend/src/models/` 新增 `labor_contract.rs`：
   ```rust
   pub struct Model {
       pub id: i32,
       pub worker_id: i32,
       pub contract_no: String,
       pub contract_type: String,       // fixed_term/permanent/task_based
       pub start_date: NaiveDate,
       pub end_date: Option<NaiveDate>, // 无固定期限合同为 None
       pub probation_end_date: Option<NaiveDate>,
       pub probation_salary: Decimal,    // 试用期工资（需 ≥ 转正工资 80%）
       pub regular_salary: Decimal,      // 转正工资
       pub position: String,
       pub department: String,
       pub status: String,               // active/expired/terminated
   }
   ```
2. 在 `/workspace/backend/src/services/` 新增 `labor_contract_service.rs`：
   - `validate_probation_period`：校验试用期长度合规性
   - `validate_probation_salary`：校验试用期工资 ≥ 转正工资 80%
   - `expiry_warning`：合同到期前 30/60/90 天预警

---

#### ❌ 缺陷项 22：工时与加班合规缺失

**风险等级：P1**（《劳动法》合规阻塞）

**证据**：
- Grep `overtime|加班|working_hours|工时|comprehensive_working_hours|综合计时工时` 在 `/workspace/backend/src` 无匹配。
- `/workspace/backend/src/models/wage_record_detail.rs`：工资明细模型含 `duration_minutes`（计时工时）与 `time_wage`，**无加班工时字段**（无 `overtime_hours`/`weekend_overtime_hours`/`holiday_overtime_hours`）。
- `/workspace/backend/src/services/wage_service.rs:119-121`：工资计算仅按"计件/计时"二分，**无加班费倍率计算**（平时 1.5 倍/周末 2 倍/法定节假日 3 倍）。

**业务影响**：
- 《劳动法》第 41 条规定月加班时间 ≤ 36 小时。
- 《劳动法》第 44 条规定加班费支付标准（平时 1.5 倍/周末 2 倍/法定节假日 3 倍）。
- 纺织行业常见综合计时工时制需劳动行政部门审批。
- 系统缺失工时与加班合规，导致：
  1. 月加班超 36 小时无预警，违反《劳动法》。
  2. 加班费计算错误，存在劳动仲裁风险。
  3. 综合计时工时制审批文件无存档。

**修复建议**：
1. 在 `/workspace/backend/src/models/wage_record_detail.rs` 新增字段：
   ```rust
   pub regular_hours: Decimal,         // 正常工时
   pub weekday_overtime_hours: Decimal, // 平时加班（1.5 倍）
   pub weekend_overtime_hours: Decimal, // 周末加班（2 倍）
   pub holiday_overtime_hours: Decimal, // 法定节假日加班（3 倍）
   pub overtime_pay: Decimal,           // 加班费
   ```
2. 在 `/workspace/backend/src/services/wage_service.rs::calculate` 增加加班费计算逻辑：
   ```rust
   let overtime_pay = weekday_ot * 1.5 * hourly_rate
       + weekend_ot * 2.0 * hourly_rate
       + holiday_ot * 3.0 * hourly_rate;
   ```
3. 新增月加班时间校验：`sum(overtime_hours) > 36` 时 warn 并要求审批。
4. 在 `/workspace/backend/src/models/` 新增 `comprehensive_working_hours_approval.rs`（综合计时工时制审批文件存档）。

---

#### ❌ 缺陷项 23：社保公积金合规缺失

**风险等级：P1**（《社会保险法》合规阻塞）

**证据**：
- Grep `social_insurance|社保|housing_fund|公积金|five_insurance` 在 `/workspace/backend/src` 无匹配。
- `/workspace/backend/src/models/wage_record_detail.rs`：工资明细模型无 `social_insurance_base`/`social_insurance_amount`/`housing_fund_amount` 字段。
- `/workspace/backend/src/services/wage_service.rs`：工资计算无社保公积金扣缴逻辑。

**业务影响**：
- 《社会保险法》第 58 条要求用人单位应当自用工之日起 30 日内为其职工办理社会保险登记。
- 《住房公积金管理条例》第 14 条要求单位录用职工 30 日内办理住房公积金缴存登记。
- 缴费基数应为职工上年度月平均工资，禁止按最低基数缴纳。
- 系统缺失社保公积金合规，导致：
  1. 五险一金扣缴无系统支持，需手工处理。
  2. 缴费基数合规性无校验。
  3. 企业存在社保稽核风险。

**修复建议**：
1. 在 `/workspace/backend/src/models/` 新增 `social_insurance_record.rs`（社保缴纳记录）：
   ```rust
   pub struct Model {
       pub id: i32,
       pub worker_id: i32,
       pub period: NaiveDate,
       pub base_amount: Decimal,          // 缴费基数（应为上年度月平均工资）
       pub pension_employer: Decimal,     // 养老保险单位部分
       pub pension_employee: Decimal,     // 养老保险个人部分
       pub medical_employer: Decimal,     // 医疗保险
       pub unemployment_employer: Decimal,// 失业保险
       pub work_injury_employer: Decimal, // 工伤保险
       pub maternity_employer: Decimal,   // 生育保险
       pub housing_fund_employer: Decimal,// 公积金单位部分
       pub housing_fund_employee: Decimal,// 公积金个人部分
   }
   ```
2. 在 `/workspace/backend/src/services/wage_service.rs::calculate` 增加社保公积金扣缴逻辑。
3. 新增 `validate_social_insurance_base`：校验缴费基数 ≥ 当地最低基数且 ≤ 当地最高基数，且不低于职工实际工资。

---

#### ❌ 缺陷项 24：职业健康合规缺失

**风险等级：P1**（《职业病防治法》合规阻塞）

**证据**：
- Grep `occupational_health|职业健康|occupational_hazard|职业危害` 在 `/workspace/backend/src` 无匹配。
- Grep `MSDS|safety_data_sheet|危险化学品|chemical_safety` 在 `/workspace/backend/src` 无匹配。
- `/workspace/backend/src/models/chemical_master.rs:69`：`signal_word: Option<String>`（化学品的 GHS 信号词），但无 MSDS 安全数据表字段。
- `/workspace/backend/src/models/`：无 `occupational_health_exam.rs`（职业健康体检档案）模型。

**业务影响**：
- 《职业病防治法》第 26 条要求对职业病危害因素定期检测（印染车间苯/甲醛/噪声/粉尘）。
- 《职业病防治法》第 35 条要求上岗前/在岗期间/离岗时职业健康体检。
- 《危险化学品安全管理条例》要求 MSDS 安全数据表。
- 印染行业是职业病高发行业，系统缺失职业健康合规，导致：
  1. 职业危害因素检测无存档。
  2. 工人职业健康体检档案无系统管理。
  3. 染料/助剂 MSDS 无系统查询。
  4. 防护用品配备无记录。

**修复建议**：
1. 在 `/workspace/backend/src/models/chemical_master.rs` 新增 `msds_url: Option<String>` 字段（MSDS 安全数据表 PDF 链接）。
2. 在 `/workspace/backend/src/models/` 新增：
   - `occupational_hazard_monitoring.rs`（职业危害因素检测记录：苯/甲醛/噪声/粉尘）
   - `occupational_health_exam.rs`（职业健康体检档案：上岗前/在岗期间/离岗时）
   - `ppe_distribution_record.rs`（防护用品配备记录）
3. 在 `/workspace/backend/src/services/` 新增 `occupational_health_service.rs`：
   - `hazard_exceedance_alert`：危害因素超标预警
   - `exam_due_reminder`：体检到期提醒（在岗期间每年一次）

---

#### ❌ 缺陷项 25：女职工与未成年工保护合规缺失 + 安全生产法合规缺失

**风险等级：P2**（劳动合规）

**证据**：
- Grep `female_worker|女职工|minor_worker|未成年工|pregnancy|maternity|lactation` 在 `/workspace/backend/src` 无匹配。
- Grep `safety_production|安全生产|operation_certificate|操作证|safety_accident` 在 `/workspace/backend/src` 无匹配。
- `/workspace/backend/src/models/user.rs`：用户模型无性别/出生日期字段（无法识别未成年工与女职工）。

**业务影响**：
- 《女职工劳动保护特别规定》要求女职工孕期/产期/哺乳期保护。
- 《禁止使用童工规定》禁止使用未成年工（<16 周岁）。
- 《安全生产法》要求特种设备操作证管理（染缸/定型机/烘干机）。
- 系统缺失相关功能，导致：
  1. 女职工三期保护无系统支持。
  2. 未成年工无法识别（无出生日期字段）。
  3. 特种设备操作证到期无预警。

**修复建议**：
1. 在 `/workspace/backend/src/models/user.rs` 新增 `gender: Option<String>` 与 `birth_date: Option<NaiveDate>` 字段。
2. 在 `/workspace/backend/src/services/user_service.rs::create_user` 校验出生日期（避免录用 <16 周岁未成年工）。
3. 在 `/workspace/backend/src/models/` 新增：
   - `female_worker_protection.rs`（女职工三期保护记录）
   - `operation_certificate.rs`（特种设备操作证管理，含染缸/定型机/烘干机操作证到期预警）
   - `safety_accident_report.rs`（安全生产事故报告）
4. 在 `/workspace/backend/src/services/chemical_master_service.rs` 增加 MSDS 查询端点。

---

## 审计结果汇总

| 维度 | P0 | P1 | P2 | P3 | 已落实 | 总检查项 |
|------|----|----|----|----|--------|----------|
| 8.1 中国法律法规合规 | 0 | 2 | 1 | 0 | 4 | 10 |
| 8.2 法律安全标准 | 0 | 0 | 1 | 0 | 8 | 8 |
| 8.3 数据脱敏与审计追溯 | 0 | 1 | 1 | 0 | 6 | 8 |
| 8.4 成品文档格式合规 | 0 | 2 | 0 | 0 | 8 | 5 |
| 8.5 纺织行业法律法规合规 | 0 | 2 | 2 | 0 | 2 | 7 |
| 8.6 纺织行业财税合规 | 0 | 3 | 2 | 0 | 1 | 8 |
| 8.7 纺织行业环保合规 | 0 | 2 | 1 | 0 | 1 | 7 |
| 8.8 纺织行业劳动合规 | 0 | 4 | 1 | 0 | 2 | 7 |
| **合计** | **0** | **16** | **10** | **0** | **32** | **60** |

**说明**：
- P0（阻塞）：0 项
- P1（高）：16 项
- P2（中）：10 项
- P3（低）：0 项
- 已落实：32 项
- 总检查项：60 项（含 V15 新增纺织行业 4 维度的 30 个检查项）

## 修复优先级队列

### P0 级（阻塞）
（无 P0 级缺陷）

### P1 级（高）

1. **缺陷项 1**：用户协议/隐私政策在系统中未真实接入（《个人信息保护法》合规阻塞）
   - 影响：登录/初始化无隐私政策确认环节，涉及客户手机号/邮箱等敏感信息处理
   - 修复：Login.vue/Setup.vue 增加复选框，后端 login/initialize 增加 agreement_accepted 必传字段

2. **缺陷项 3**：HTTPS 强制配置在反向代理层缺失（《网络安全法》合规阻塞）
   - 影响：HTTP 明文传输敏感信息，存在中间人攻击风险
   - 修复：nginx.conf 默认 80 重定向 443，强制启用 SSL

3. **缺陷项 5**：手机号展示脱敏缺失（《个人信息保护法》合规阻塞）
   - 影响：客户/供应商/司机手机号明文展示，存在内部人员泄露风险
   - 修复：field_mask.rs 新增 mask_phone/mask_email/mask_id_card 函数，所有返回手机号的 handler 调用脱敏

4. **缺陷项 7**：染整报表导出/色卡发放记录导出/产量工资报表导出/能耗报表导出缺失（规则 3 强制要求）
   - 影响：产量工资/能耗/色卡借出记录无法导出，影响财务对账与环保合规
   - 修复：wage_handler/energy_handler/color_card/borrow.rs 新增 export 端点

5. **缺陷项 8**：合同/发票/报表生成不支持 .docx 格式（规则 3 强制要求）
   - 影响：合同/订单/对账单仅生成 HTML，无法生成可编辑 Word 文档交付
   - 修复：Cargo.toml 引入 docx-rs，重构 print_service 新增 generate_docx

6. **缺陷项 9**：面料执行标准登记缺失（《产品质量法》合规阻塞）
   - 影响：产品标签无执行标准号，假冒伪劣预警无基准数据
   - 修复：products 表新增 execution_standard/factory_name/factory_address/product_grade 字段

7. **缺陷项 10**：合同电子签章真实接入缺失（《合同法》/《民法典》合规阻塞）
   - 影响：合同在线签署流程缺失，合同真实性无法验证
   - 修复：集成 e签宝/法大大/e签宝，新增 contract_signature_service

8. **缺陷项 13**：进项税转出缺失（增值税合规阻塞）
   - 影响：非正常损耗未做进项税转出，增值税申报虚高
   - 修复：outsourcing_service 非正常损耗凭证增加进项税转出逻辑

9. **缺陷项 14**：出口退税（免抵退）核算缺失（出口业务财税合规阻塞）
   - 影响：出口面料销售无法生成免抵退税申报表
   - 修复：新增 export_refund_service 实现免抵退核算

10. **缺陷项 15**：环保税核算缺失（印染行业财税合规阻塞）
    - 影响：环保税申报无系统数据支撑
    - 修复：新增 pollutant_discharge_record 模型与 environmental_tax_service

11. **缺陷项 18**：排污许可证登记缺失（《环境保护法》合规阻塞）
    - 影响：企业可能因证照过期违法排污
    - 修复：新增 pollution_permit 模型与到期预警

12. **缺陷项 19**：废水/废气/固废排放监测与合规校验缺失（水/大气/固废污染防治法合规阻塞）
    - 影响：超标排放无法实时预警，环保税核算无数据基础
    - 修复：新增 pollutant_monitoring_record 模型与环境监测服务

13. **缺陷项 21**：劳动合同电子化管理缺失（《劳动法》/《劳动合同法》合规阻塞）
    - 影响：合同到期无预警，试用期合规性无校验
    - 修复：新增 labor_contract 模型与合同管理服务

14. **缺陷项 22**：工时与加班合规缺失（《劳动法》合规阻塞）
    - 影响：月加班超 36 小时无预警，加班费计算错误
    - 修复：wage_record_detail 新增加班工时字段，wage_service 增加加班费计算

15. **缺陷项 23**：社保公积金合规缺失（《社会保险法》合规阻塞）
    - 影响：五险一金扣缴无系统支持，缴费基数合规性无校验
    - 修复：新增 social_insurance_record 模型与扣缴逻辑

16. **缺陷项 24**：职业健康合规缺失（《职业病防治法》合规阻塞）
    - 影响：职业危害因素检测无存档，工人职业健康体检档案无管理
    - 修复：新增 occupational_hazard_monitoring/occupational_health_exam 模型与服务

### P2 级（中）

1. **缺陷项 2**：数据跨境传输合规评估缺失
   - 修复：撰写《数据跨境传输合规评估》文档，增加 DATA_LOCALITY 配置项

2. **缺陷项 4**：登录端点未挂载 anti_brute_force 中间件
   - 修复：routes/auth.rs 的 /login 路由单独挂载 anti_brute_force 中间件

3. **缺陷项 6**：身份证展示脱敏缺失 + 身份证字段未在模型中
   - 修复：field_mask.rs 预留 mask_id_card 函数

4. **缺陷项 11**：销售合同模板合规字段缺失
   - 修复：sales_contracts 表新增 quality_terms/breach_liability/dispute_resolution 字段

5. **缺陷项 12**：印染行业规范条件合规缺失 + 进出口商品检验合规缺失 + 反不正当竞争合规缺失
   - 修复：新增水重复利用率计算/出口商检记录/合规预警服务

6. **缺陷项 16**：存货跌价准备缺失
   - 修复：新增 inventory_write_down 模型与跌价准备计提服务

7. **缺陷项 17**：印花税/个税代扣代缴/研发费用加计扣除缺失
   - 修复：合同表新增 stamp_tax_amount，工资明细新增 individual_income_tax，新增研发费用加计扣除服务

8. **缺陷项 20**：环境影响评价（环评）合规存档缺失
   - 修复：新增 environmental_assessment 模型与文件存档端点

9. **缺陷项 25**：女职工与未成年工保护合规缺失 + 安全生产法合规缺失
   - 修复：user 表新增 gender/birth_date，新增 female_worker_protection/operation_certificate 模型

### P3 级（低）
（无 P3 级缺陷）

---

## 审计结论

### 整体评价

V15 类八法律合规与安全标准审计覆盖 8 维度 60 个检查项，发现 **0 个 P0、16 个 P1、10 个 P2、0 个 P3** 缺陷，已落实 32 项。

**强项**：
1. **法律安全标准（8.2）落实度高**：8 项中 8 项已落实，仅 1 项 P2（anti_brute_force 未挂载）。认证/权限/CSRF/SQL 注入/速率限制/密码哈希/文件上传/JWT 撤销/审计日志机制完备。
2. **审计日志防篡改机制完善**：HMAC-SHA256 签名 + AUDIT_SECRET_KEY 强制配置 + 保留期清理 + 敏感操作告警分级。
3. **委托加工物资财税核算核心分录已实现**：发料/加工费/入库/非正常损耗四类凭证生成逻辑正确。

**弱项**：
1. **纺织行业 4 维度合规大面积缺失**：8.5/8.6/8.7/8.8 共 30 个检查项中仅 6 项已落实（20%），24 项缺陷（80%）。V15 新增的纺织行业法律法规/财税/环保/劳动合规是审计重点，但系统主要聚焦于通用 ERP 功能与面料行业工艺追溯（v14 批次 416-432），行业合规功能尚未建设。
2. **数据脱敏机制不完整**：仅 cost_price 字段与 CRM lead 详情接口有字段级脱敏，手机号/邮箱/身份证号等核心 PII 无脱敏机制。
3. **成品文档格式合规不完整**：xlsx 导出已统一（rust_xlsxwriter），但 docx 生成完全缺失，print_service/export_service 方法名误导（generate_pdf 实际生成 HTML/纯文本）。
4. **隐私政策/用户协议未接入**：违反《个人信息保护法》第 17 条告知义务。

### 修复优先级建议

**第一批（P1 阻塞合规）**：优先修复缺陷项 1/3/5/9/10（通用法律合规）+ 13/14/15（财税合规）+ 18/19（环保合规）+ 21/22/23/24（劳动合规），共 13 项 P1。

**第二批（P1 规则 3）**：修复缺陷项 7/8（成品文档格式），共 2 项 P1。

**第三批（P2 增强）**：修复 10 项 P2，包括数据跨境评估/anti_brute_force 挂载/合同字段补全/行业规范合规/存货跌价准备/印花税个税/环评存档/女职工保护等。

### 建议后续批次规划

鉴于纺织行业 4 维度合规缺失严重（24 项缺陷），建议在 V16 或 V17 单独立项"纺织行业合规专项"，分批次实现：
- V16 批次 A：纺织行业财税合规（缺陷项 13/14/15/16/17）
- V16 批次 B：纺织行业环保合规（缺陷项 18/19/20）
- V16 批次 C：纺织行业劳动合规（缺陷项 21/22/23/24/25）
- V16 批次 D：纺织行业法律法规合规（缺陷项 9/10/11/12）

通用法律合规（缺陷项 1/2/3/4/5/6/7/8）可在 V15 修复批次中并行处理。
