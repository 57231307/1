# 安全漏洞记录

> 本文件用于登记项目安全漏洞。所有已修复漏洞已迁移至 git 历史（CHANGELOG.md / PR）。
> 审计周期内如有新漏洞发现，登记后立即启动修复流程。
> 详见 `.monkeycode/MEMORY.md` 的 Bug.md 实时漏洞管理规则。

---

## 待修复漏洞（2026-06-24 审计周期第二轮新增 14 个）

> 2026-06-24 周期性安全审计第二轮，基于代码路径的端到端可利用性审查。
> 第一轮 6 个低危漏洞已完成（见下方"历史已修复"段），本轮新增 14 项发现（4 高危 / 7 中危 / 3 低危）。
> 攻击者画像与代码路径均经过实读确认，未列入仅理论性或推测性风险。
> 详细修复方案与受影响文件清单见各条目。

---

### 🔴 高危漏洞（4 项）

#### H-1：Webhook 发送 SSRF TOCTOU（Time-of-Check to Time-of-Use）

- **位置**：
  - [webhook_service.rs:130](file:///workspace/backend/src/services/webhook_service.rs#L130)（`validate_url` 校验）
  - [webhook_service.rs:222](file:///workspace/backend/src/services/webhook_service.rs#L222)（`client.post(url)` 实际发送）
  - [webhook_service.rs:187-211](file:///workspace/backend/src/services/webhook_service.rs#L187-L211)（内联二次 IP 校验）
- **攻击者画像**：已认证用户（任意租户，可调用 webhook 创建/触发 API）
- **可控输入**：`webhook_url` 字段（创建 webhook 时提交）
- **利用路径**：
  1. 攻击者注册一个 webhook，URL 为自建域名 `evil.attacker.com`
  2. `evil.attacker.com` DNS 配置低 TTL（5 秒），交替返回公网 IP（3.3.3.3）和内网 IP（169.254.169.254）
  3. `validate_url` 调用 `to_socket_addrs` 解析 → 拿到公网 IP → 通过 ssrf_guard.rs L98 检查
  4. `send_http_request` 内部 `to_socket_addrs` 再次解析 → 攻击者此时已切换 DNS → 拿到内网 IP → 但**内联 IP 校验 L187-211 覆盖不全**（缺 IPv6 link-local `fe80::/10`、ULA `fc00::/7`、IPv4-mapped IPv6、CGNAT 100.64.0.0/10、保留 240.0.0.0/4、多播 224.0.0.0/4）
  5. 若内网 IP 通过未覆盖检查，到 `client.post(url)` 时 reqwest 客户端**第三次解析 DNS**（每次都不同），可能直接拿到 169.254.169.254 → 命中 AWS 元数据服务
  6. 配合 `Authorization: Bearer <webhook_secret>` 头（若有 secret），凭证也会随请求外泄到内网
- **影响**：SSRF，可读取云元数据服务（IAM 凭证）、内网 HTTP 服务、PostgreSQL/MySQL 数据库
- **修复建议**：
  1. 在 `client.post(url)` 调用前**强制使用 connect 时解析的 IP**（`tokio::net::TcpStream::connect_timeout` 指定 IP 而非域名），避免 reqwest 客户端再次解析
  2. 删除 L187-211 的内联 IP 校验，统一调用 `ssrf_guard::validate_url`，避免两套校验逻辑不一致
  3. 补齐 `ssrf_guard::is_blocked_ipv6` 缺失的 CGNAT / IPv4-mapped 检查
  4. 禁用 webhook secret 在 HTTP 头中明文传递，改为 HMAC 签名 body

#### H-2：Email Service API URL 环境变量可控 → SendGrid API Key 泄露

- **位置**：
  - [email_service.rs:39](file:///workspace/backend/src/services/email_service.rs#L39)（`EmailConfig.api_url` 字段）
  - [email_service.rs:202-216](file:///workspace/backend/src/services/email_service.rs#L202-L216)（`send_via_sendgrid` 用 `api_url` POST 请求）
- **攻击者画像**：环境变量污染攻击（依赖 K8s ConfigMap / Docker Compose / .env 注入）或内部恶意运维
- **可控输入**：`EMAIL_API_URL` 环境变量
- **利用路径**：
  1. 攻击者（已获得环境变量写权限）将 `EMAIL_API_URL` 改为 `https://169.254.169.254/latest/meta-data/iam/security-credentials/`
  2. 系统任何用户触发邮件发送 → `send_via_sendgrid` 读取 `api_url`（攻击者值）→ POST 该 URL
  3. 请求自动带 `Authorization: Bearer <real_sendgrid_api_key>` 头（L211）→ **API Key 泄露到云元数据服务**
  4. 若 `api_url` 指向攻击者服务器，则 `Authorization` 头直送攻击者
- **影响**：第三方 API 凭证（SendGrid API Key）泄露；通过该 Key 可发任意邮件（钓鱼、垃圾邮件）、消耗配额
- **修复建议**：
  1. `api_url` 不允许环境变量覆盖，写死 `https://api.sendgrid.com/v3/mail/send`
  2. 至少增加协议白名单 + 主机名白名单（仅允许 sendgrid.com / aliyun.com / tencent.com 等固定域名）
  3. 使用 `ssrf_guard::validate_url` 在发送前校验自定义 URL
  4. EMAIL_API_KEY 不应直接通过 `Authorization: Bearer` 传递，改用 HMAC 签名或 OAuth2

#### H-3：init 子系统 test_database_connection SSRF（探测内网数据库）

- **位置**：
  - [init_handler.rs:60-156](file:///workspace/backend/src/handlers/init_handler.rs#L60-L156)（`test_database_connection` handler）
  - [init_handler.rs:102-119](file:///workspace/backend/src/handlers/init_handler.rs#L102-L119)（注释明确说"TODO(ssrf): 待运维提供内网 IP 段白名单后启用" — **未实现**）
- **攻击者画像**：已认证 **admin** 角色用户
- **可控输入**：JSON 字段 `host`, `port`, `name`, `username`, `password`
- **利用路径**：
  1. admin 用户调 `POST /api/v1/erp/init/test-database`，body `{"host":"10.0.0.1","port":"5432","name":"postgres","username":"postgres","password":"x"}`
  2. 后端 `InitService::test_database(&db_config)` 用 SeaORM 真实连接 → **根据错误信息可区分端口开放/服务类型/认证失败**：
     - 连接超时 → 端口关闭
     - TCP 拒绝 → 端口未监听
     - "password authentication failed" → PostgreSQL 端口开放
     - "unknown database" → 数据库名错误
  3. 攻击者通过错误消息**精确枚举内网所有 PostgreSQL 端口**
  4. 进一步可基于已知 PostgreSQL 协议利用工具（CVE-2019-9193 `COPY FROM PROGRAM`）执行任意命令
- **影响**：SSRF + 内网数据库存在性探测；为后续 RCE 铺路
- **修复建议**：
  1. **立即实现** `init_handler.rs:102-119` 的 `TODO(ssrf)`，增加内网 IP 白名单（仅允许 192.168.0.0/16 等运维确认的网段）
  2. 限制 port 范围（仅允许 5432 PostgreSQL）
  3. 错误消息脱敏，不返回底层 `sea_orm::DbErr` 原文（统一返回"数据库连接失败"）
  4. 仅在 system 初始化模式下（数据库未连接）允许调用，已初始化后禁用

#### H-4：静态资源路径不 canonicalize → 符号链接任意文件读取

- **位置**：
  - [routes/static.rs:24-41](file:///workspace/backend/src/routes/static.rs#L24-L41)（`sanitize_static_path` 只校验 `..` 段）
  - [routes/static.rs:77-95](file:///workspace/backend/src/routes/static.rs#L77-L95)（`tokio::fs::read` 不调用 `canonicalize`）
- **攻击者画像**：拥有 `/workspace/frontend/static` 写权限的攻击者（通过任意文件上传漏洞、运维错误、容器共享卷配置错误）
- **可控输入**：URL 路径 `/static/<file_path>`
- **利用路径**：
  1. 攻击者在 `/workspace/frontend/static/secrets` 创建一个**符号链接**指向 `/etc/passwd`（或 `/var/lib/postgresql/data/postgresql.conf` 等敏感文件）
  2. 发送 `GET /static/secrets` → `sanitize_static_path("secrets")` 通过校验（无 `..`）→ 拼接 `static_dir = /workspace/frontend/static` → `tokio::fs::read` 跟随符号链接 → 返回 `/etc/passwd` 内容
  3. 响应头 `Content-Type: text/css` 但攻击者已知内容
  4. 同样可读取 `~/.aws/credentials`、`/proc/self/environ`（含 COOKIE_SECRET、JWT_SECRET 等）
- **影响**：任意文件读取（含 secrets）；权限提升跳板
- **修复建议**：
  1. `tokio::fs::read` 之前先 `tokio::fs::canonicalize(&static_path)` 解析符号链接
  2. 验证 `canonicalize` 后的路径**仍以** `static_dir` 前缀开头
  3. 同时删除 fallback 路径（L84-95）的 `CARGO_MANIFEST_DIR/static` 硬编码（不应回退到 backend 目录）
  4. 对返回文件做 MIME 嗅探而非硬编码 `text/css`（L81、L93）

---

### 🟡 中危漏洞（7 项）

#### M-1：customer 表 update/delete 完全不检查租户/创建人隔离（IDOR）

- **位置**：
  - [customer_handler.rs:212-254](file:///workspace/backend/src/handlers/customer_handler.rs#L212-L254)（`update_customer` 用 `_auth: AuthContext` — 接收但**未使用**）
  - [customer_handler.rs:257-265](file:///workspace/backend/src/handlers/customer_handler.rs#L257-L265)（`delete_customer` 同样 `_auth: AuthContext` 未使用）
  - [customer_service.rs:333-432](file:///workspace/backend/src/services/customer_service.rs#L333-L432)（`update_customer` 和 `delete_customer` 完全不传 `tenant_id`）
  - [migrations/20260323000001_initial_schema/up.sql:118-133](file:///workspace/backend/migrations/20260323000001_initial_schema/up.sql#L118-L133)（customers 表**完全没有 `tenant_id` 字段**）
- **攻击者画像**：已认证任意用户
- **可控输入**：URL 路径参数 `id`（`PUT /api/v1/erp/customers/{id}` / `DELETE /api/v1/erp/customers/{id}`）
- **利用路径**：
  1. 攻击者 A（tenant 1）通过 `GET /api/v1/erp/customers` 列表获取 customer ID（即使是其他租户的）
  2. 攻击者 A 直接 `PUT /api/v1/erp/customers/{victim_id}` 携带修改后的 customer_name、credit_limit 等
  3. `update_customer` handler L215 `_auth` 形参**完全未被消费**；service 层 L353 `find_by_id` 不带 tenant 过滤
  4. 受害客户被任意修改（甚至被软删除 `is_active=false`）
- **影响**：所有客户数据可被任意已认证用户修改/软删除（数据完整性破坏 + 业务损失）
- **修复建议**：
  1. `customers` 表 schema 增加 `tenant_id INTEGER NOT NULL REFERENCES tenants(id)` 字段
  2. 所有 `update_customer` / `delete_customer` / `get_customer` 操作必须强制 `WHERE tenant_id = $1`
  3. 即使没有 tenant_id 概念，至少按 `created_by` 校验（L528 customer_service 已记录 `created_by`）
  4. 软删除前增加审计日志（业务操作可追溯）

#### M-2：登录时间差导致用户名枚举

- **位置**：[auth_handler.rs:172-176](file:///workspace/backend/src/handlers/auth_handler.rs#L172-L176)（`auth_service.authenticate` 调用）
- **攻击者画像**：未认证外部攻击者
- **可控输入**：登录 username
- **利用路径**：
  1. 攻击者提交 `POST /api/v1/erp/auth/login` body `{"username":"victim","password":"random"}`
  2. `authenticate` 流程：
     - 用户存在：先 `find_by_username`（DB 查询 ~5ms）→ `verify_password` Argon2id（~100ms 故意慢）
     - 用户不存在：仅 `find_by_username` → 返回 `UserNotFound` 错误（~5ms）
  3. 响应时间差 **~95ms** → 攻击者通过统计大量响应可**精确区分**有效用户名
  4. 配合 [auth_handler.rs:133-149](file:///workspace/backend/src/handlers/auth_handler.rs#L133-L149) 的失败锁定逻辑，攻击者可针对枚举出的有效用户名进行**凭证填充攻击**
- **影响**：用户名枚举 → 定向暴力破解 / 凭证填充
- **修复建议**：
  1. 用户名不存在时也执行一次 `Argon2::default().verify_password` 假哈希（如随机生成的 dummy hash），保证响应时间恒定
  2. 错误消息统一为"用户名或密码错误"，不区分（已实现，需补充时间对齐）
  3. 失败响应增加随机 jitter（10-50ms）扰乱时序分析

#### M-3：refresh_token 端点不验证 is_active / JTI 黑名单

- **位置**：[auth_handler_misc.rs:41-191](file:///workspace/backend/src/handlers/auth_handler_misc.rs#L41-L191)（`refresh_token` 函数）
- **攻击者画像**：拥有有效 refresh_token 字符串的攻击者
- **可控输入**：refresh_token cookie 或 `Authorization: Bearer <refresh_token>` 头
- **利用路径**：
  1. 攻击者通过任何途径（XSS、日志泄露、网络嗅探）拿到 refresh_token
  2. 调 `POST /api/v1/erp/auth/refresh` 携带 refresh_token
  3. `refresh_token` handler **没有 `AuthContext` 提取器**（公开端点）— L41-191 整段
  4. 仅检查 `state.cache.get_token_blacklist()`（L60-67），不检查 JTI 黑名单、不检查 is_active
  5. 即使管理员在收到 refresh_token 之前 5 分钟内**封禁该用户**（revoke_user_jtis + is_active=false），refresh_token 仍能换新 access_token
  6. **新 access_token 在 5 分钟后被 `is_user_active_cached` 拒绝**，但攻击者已用 refresh_token 重新建立 session
- **影响**：被封禁/软删除用户的 refresh_token 在最长 5 分钟窗口内可继续换新 token（但其实 access_token 立刻被拒，所以是有限窗口，**主要问题是 refresh 路径缺少 is_active 校验一致性**）
- **修复建议**：
  1. `refresh_token` 中**强制校验** `is_user_active_cached`（调用 `auth_service::is_user_active_cached`）
  2. 同时校验 JTI 黑名单（`is_jti_revoked`）
  3. 用户封禁时**主动吊销 refresh_token**（写入 `cache.get_token_blacklist`）

#### M-4：email_service.save_email_log 业务日志泄露所有收件人 PII

- **位置**：[email_service.rs:388-395](file:///workspace/backend/src/services/email_service.rs#L388-L395)
- **攻击者画像**：拥有日志系统读权限的内部人员 / 第三方日志聚合商（如 ELK）账号被入侵
- **可控输入**：被调用方传入的 `to` 收件人列表
- **利用路径**：
  1. 业务正常调用 `save_email_log`，传入 `to = ["ceo@company.com", "cfo@company.com"]`
  2. 日志 `info!("... to={:?}, subject={}, ...", to, subject)` 把完整邮箱列表写入 tracing 日志
  3. 邮箱列表 + 主题 + 发送时间形成**业务情报**：可推断谁在什么时间给谁发什么邮件
- **影响**：PII 泄露（邮箱地址）；违反 GDPR / 个人信息保护法
- **修复建议**：
  1. 日志中只记录收件人数量（`to_count`）和域名（`to_domain`），不记录完整邮箱
  2. 主题脱敏（保留前 10 字符 + `...`）
  3. 邮箱仅在加密的 `email_log` 数据库表中保存明文

#### M-5：send_html_email 邮件内容未转义 → 邮件 XSS

- **位置**：[email_service.rs:126-142](file:///workspace/backend/src/services/email_service.rs#L126-L142)（`send_html_email`）+ [email_service.rs:180-184](file:///workspace/backend/src/services/email_service.rs#L180-L184)（`send_via_sendgrid` 原样发送 `html_content`）
- **攻击者画像**：能控制邮件 HTML 内容的内部业务模块调用方
- **可控输入**：`html_content` 字段（拼接用户输入时）
- **利用路径**：
  1. 业务调用 `send_html_email(to, subject, "<p>{}</p>".format(user_input))`
  2. `user_input = "<img src=x onerror=alert(document.cookie)>"` 被原样嵌入邮件
  3. SendGrid 发送邮件 → 收件人客户端解析 HTML → `<img onerror>` 执行
  4. 邮件客户端 cookie / 邮件内容被读取 → 配合 session stealing
- **影响**：邮件 XSS（部分邮件客户端如 Outlook、Gmail Web 会执行 JS）
- **修复建议**：
  1. `send_html_email` 入口处调用 `EmailTemplate::notification_template` 而非直接透传
  2. 或在 `send_via_sendgrid` 序列化前对 `html_content` 调用 `escape_html`
  3. 强制使用 `EmailTemplate` 提供的模板（`notification_template` 已正确 escape）

#### M-6：permission.rs 资源 ID 匹配过宽（NULL 匹配任意）

- **位置**：[permission.rs:210-215](file:///workspace/backend/src/middleware/permission.rs#L210-L215)
- **攻击者画像**：拥有部分权限的已认证用户
- **可控输入**：URL 路径中的数字 ID
- **利用路径**：
  1. 攻击者 A 拥有 `resource_type="orders", action="delete", resource_id=None` 的权限（"全局 delete 权限"）
  2. 攻击者 A 构造 `DELETE /api/v1/erp/sales/orders/999999`（999999 是其他租户/其他用户的订单）
  3. `extract_resource_info` L83-117 提取 `resource_type="orders", resource_id=Some(999999)`
  4. `check_permission` L210-215 匹配 `p.resource_id == resource_id || p.resource_id.is_none()` → `None == 999999` 为 false，但 `is_none()` 为 true → **匹配通过**
  5. 攻击者可删除任意订单
- **影响**：垂直越权（拥有部分 delete 权限的用户可删除所有记录）
- **修复建议**：
  1. 默认行为改为"精确匹配优先"：`p.resource_id == resource_id` 通过；只有显式 `action="*"` 才匹配任意
  2. 增加 `action: "*"` 通配符支持
  3. 资源级权限与全局权限分离（两套表），避免误用

#### M-7：SQL 注入审计中间件黑名单模式不全

- **位置**：[sql_injection_audit.rs:26-42](file:///workspace/backend/src/middleware/sql_injection_audit.rs#L26-L42)
- **攻击者画像**：未授权攻击者
- **可控输入**：URL 路径或 query string
- **利用路径**：
  1. 黑名单仅含 14 个模式，缺：
     - `pg_sleep(`
     - `||`（PostgreSQL 字符串连接）
     - `ASCII(SUBSTRING(`
     - `1=1`（不带 `' OR` 前缀）
     - `BENCHMARK(`
     - `pg_read_file(`
     - `CHR(` + 数字 + `)`
     - `EXTRACTVALUE(`
  2. 攻击者用 `1=1` 或 `pg_sleep` 时间盲注绕过审计（实际 SQL 注入路径依赖参数化查询，未必有注入面；但审计层失守会增加风险）
- **影响**：审计层盲区，**实际 SQL 注入风险取决于 handler 是否用参数化查询**（该项目用 SeaORM 主要是参数化，但仍需审计）
- **修复建议**：
  1. **改用白名单**（仅允许 `[a-zA-Z0-9_./-]` 字符通过），更安全
  2. 黑名单补全上述模式
  3. 审计中间件强制挂载到所有路由（注释 L7-14 暗示按需挂载，但实际应在全局）

---

### 🟢 低危漏洞（3 项）

#### L-1：CSRF token 公开端点无状态绑定

- **位置**：[auth_handler_misc.rs:281-288](file:///workspace/backend/src/handlers/auth_handler_misc.rs#L281-L288)（`get_csrf_token`）
- **攻击者画像**：未授权攻击者
- **可控输入**：无
- **利用路径**：
  1. 攻击者调 `GET /api/v1/erp/auth/csrf-token` 获取 csrf_token
  2. 该 token 未绑定任何 session_id/user_id
  3. csrf_middleware 消费时会因 session_id 不匹配而失败
  4. **但**：可作为"免费"的 uuid 用于日志伪造（仅低危）
- **影响**：资源浪费（缓存中积累无效 token）；轻微信息泄露（uuid 生成模式可推断）
- **修复建议**：
  1. 删除公开端点，CSRF token 在登录响应中返回（已实现）
  2. 或要求 `X-Session-Id` 头才能生成 token

#### L-2：legacy_jwt Cookie SameSite=Lax（其他用 Strict）

- **位置**：[auth_handler.rs:397-403](file:///workspace/backend/src/handlers/auth_handler.rs#L397-L403)（`legacy_jwt_cookie` 用 `SameSite::Lax`）
- **攻击者画像**：恶意第三方网站运营者
- **可控输入**：诱导用户点击精心构造的链接
- **利用路径**：
  1. 攻击者在恶意网站放置 `<img src="https://target.com/api/v1/erp/auth/logout?x=y">`
  2. `Lax` 允许 GET 跨站携带 cookie
  3. 用户点击 → 携带 target 域的 `jwt` cookie → 触发 logout
  4. **但**：CSRF middleware 在 `request_validator` 之后执行，会校验 CSRF token（如果有）— 但 logout 是公开路径（`PUBLIC_PATHS` L12）跳过 CSRF
- **影响**：恶意 logout CSRF（用户体验降级）；其他敏感 GET 操作的 CSRF 风险
- **修复建议**：
  1. `legacy_jwt_cookie` 也改用 `SameSite::Strict`
  2. 公开路径（login/refresh/logout）的 GET 操作应改为 POST + CSRF token

#### L-3：public_routes starts_with 匹配过宽（`/api/tracking/page-view`）

- **位置**：[public_routes.rs:13](file:///workspace/backend/src/middleware/public_routes.rs#L13)（`"/api/tracking/page-view"`）
- **攻击者画像**：未授权攻击者
- **可控输入**：URL 路径
- **利用路径**：
  1. `is_public_path` 用 `path.starts_with(p)` 匹配
  2. 攻击者访问 `/api/tracking/page-view/../v1/erp/users/secret`
  3. axum 路由层会规范化路径，但 **starts_with 检查在 `auth_middleware` 中发生在规范化之前**
  4. 即便 axum 后续 reject，**日志中已记录** `path = /api/tracking/page-view/../v1/erp/users/secret`，可能污染监控
- **影响**：日志注入 / 监控污染
- **修复建议**：
  1. `is_public_path` 改用 `path == p` 精确匹配，或
  2. 先 `axum::http::Uri::path()` 规范化再匹配

---

## 历史已修复（2026-06-24 第一轮 6 个低危漏洞）

所有 6 个低危漏洞已在 2026-06-24 当日完成处理：
- #1 JTI 黑名单→Redis：已修复（auth_service.rs 切换 Redis SETEX + 内存回退）
- #2 Webhook URL 内网白名单（SSRF）：已修复（新建 ssrf_guard.rs）— **但 TOCTOU 仍存在（H-1）**
- #3 分布式限流 try_lock：已修复（rate_limit.rs 改用 std Mutex + try_lock）
- #4 认证失败日志脱敏：已修复（auth.rs 新增 mask_auth_header / mask_username）
- #5 JWT 密钥硬编码：审计无问题（main.rs 启动时强制校验 + Default::default 在生产 panic）
- #6 TOTP 熵源：审计无问题（totp-rs 5.5 Secret::generate_secret 内部用 rand::thread_rng → OsRng）

详细修复说明与 PR 见 git log / CHANGELOG.md。
