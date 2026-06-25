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

## 中危 (Medium) - 业务级风险

### M-1 客户表 IDOR 漏洞（已修复 2026-06-25）

**业务影响**：低（客户数据非高敏感），但符合 IDOR 通用模式  
**复现**：

```text
POST /api/v1/erp/customers/{id}
Authorization: Bearer {user_A_token}
# user_A 可修改 user_B 创建的客户
```

**修复**：
- 客户表无 tenant_id 字段，使用 `created_by` 做数据隔离
- 管理员（role_id=1）可修改/删除所有客户
- 普通用户只能修改/删除自己创建的客户
- 提交: `fix(security): M-1 客户数据权限隔离（created_by 校验）` (PR #253)

### M-2 用户枚举（密码错误 vs 账号不存在）（保留）

**业务影响**：可批量识别有效账号
**复现**：

```text
POST /api/v1/erp/auth/login
{"username": "admin", "password": "wrong"}
# 响应: "用户不存在"  vs  "密码错误"  -> 差异
```

**修复方案**：
- 统一响应为"用户名或密码错误"
- 增加固定时延防时序侧信道

### M-3 refresh_token 缺少 is_active/JTI 校验（已修复 2026-06-25）

**业务影响**：用户被禁用后旧 token 仍可刷新
**修复**：
- 增加 JTI（session_id）吊销检查
- 增加用户 is_active 状态检查
- 提交: `fix(security): M-3 refresh_token 增加 JTI 吊销检查和用户状态校验` (PR #253)

### M-4 邮件服务 PII 日志泄露（已修复 2026-06-25）

**业务影响**：日志中明文保存收件人邮箱和邮件主题
**修复**：
- save_email_log 日志只记录收件人数量和域名
- 主题只记录长度，不记录内容
- 提交: `fix(security): H-2 + M-5 + M-4 邮件服务多项安全加固` (PR #253)

### M-5 邮件 HTML XSS（已修复 2026-06-25）

**业务影响**：用户输入未 escape 直接拼接 HTML
**修复**：
- send_html_email 增加危险模式检测
- 新增 send_notification_email 安全方法（内部使用 notification_template）
- 提交: `fix(security): H-2 + M-5 + M-4 邮件服务多项安全加固` (PR #253)

### M-6 权限匹配 resource_id NULL 匹配过宽（已修复 2026-06-25）

**业务影响**：拥有 resource_id=None 权限的用户可操作该类型所有资源（垂直越权）
**修复**：
- resource_id 改为精确匹配（None 匹配 None，Some(id) 匹配 Some(id)）
- action 支持 "*" 通配符（显式授予全局权限）
- 提交: `fix(security): M-6 权限匹配 resource_id 精确匹配` (PR #253)

### M-7 SQL 注入审计黑名单不全（已修复 2026-06-25）

**业务影响**：常见 SQL 注入向量未被拦截
**修复**：
- 黑名单从 14 → 60+ 模式
- 覆盖：时间盲注、布尔盲注、堆查询、文件操作、编码绕过、布尔函数
- 提交: `fix(security): M-7 SQL 注入审计中间件黑名单扩展` (PR #253)

---

## 低危 (Low) - 加固建议

### L-1 CSRF 公开端点缺少二次验证（已修复 2026-06-25）

**业务影响**：公开端点的 POST/PUT/PATCH/DELETE 无 CSRF 保护
**修复**：
- 公开路径的非安全方法必须携带 X-Requested-With 或 X-CSRF-Token 头
- 阻止简单表单提交型 CSRF 攻击
- 提交: `fix(security): L-1 CSRF 公开端点非安全方法要求自定义请求头` (PR #253)

### L-2 legacy_jwt Cookie SameSite=Lax（已修复 2026-06-25）

**业务影响**：CSRF 攻击可携带 legacy_jwt
**修复**：
- 三处 legacy_jwt 全部从 Lax 改为 Strict
- 与 access_token/refresh_token/csrf_token 保持一致
- 提交: `fix(security): L-2 legacy_jwt Cookie SameSite 从 Lax 改为 Strict` (PR #253)

### L-3 public_routes starts_with 误匹配（已修复 2026-06-25 之前）

**业务影响**：/api/v1/erp/auth/logout-bypass 等路径被误放行
**修复**：精确路径匹配（commit e3a97ea4）

### L-4 静态资源符号链接越界（H-4，已修复 2026-06-25 之前）

**业务影响**：攻击者可读取 /etc/passwd 等敏感文件
**修复**：canonicalize 校验边界（commit 5e6e1ac0）

---

## 业务优化（非安全漏洞，用户需求）

### 优化 1: public_routes 公开端点列表收敛（已修复 2026-06-25）

**业务影响**：公开端点过多，未遵循最小权限原则
**修复**：
- 仅保留登录/刷新/健康检查
- 其他端点均需授权
- 提交: `refactor(security): 公开端点收敛至登录/刷新/健康检查` (PR #253)

### 优化 2: import_export 全表查询（已修复 2026-06-25）

**业务影响**：导出全表导致内存溢出，无权限过滤
**修复**：
- 新增 ExportQuery 参数：status/date_from/date_to/keyword/limit
- 单次导出最大 1 万行
- 导出操作增加审计日志
- 提交: `refactor(perf): 数据导出优化 - 条件过滤 + 行数限制 + 审计日志` (PR #253)

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
