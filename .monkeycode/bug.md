# 安全漏洞记录

> 本文件用于登记项目安全漏洞。所有已修复漏洞已迁移至 git 历史（CHANGELOG.md / PR）。
> 审计周期内如有新漏洞发现，登记后立即启动修复流程。
> 详见 `.monkeycode/MEMORY.md` 的 Bug.md 实时漏洞管理规则。

---

## 待修复漏洞（截至 2026-06-25 综合审计）

> 来源：2026-06-25 综合审计报告 `.monkeycode/docs/audits/2026-06-25-comprehensive-audit.md`
> 三个高危漏洞在 PR #253 中部分修复 / 未修复，状态如下。

---

### 🔴 高危漏洞（3 项，未完全修复）

#### H-1：Webhook 发送 SSRF TOCTOU（⚠️ 部分修复）

- **位置**：
  - [webhook_service.rs:221-224](file:///workspace/backend/src/services/webhook_service.rs#L221-L224)（`client.post(url)` 传 URL 字符串，reqwest 第三次解析 DNS）
  - [webhook_service.rs:187-211](file:///workspace/backend/src/services/webhook_service.rs#L187-L211)（内联 IP 校验未删除，与 ssrf_guard 重复）
- **已修复部分**：
  - `validate_url` 已调用 `ssrf_guard::validate_url`
  - `ssrf_guard.rs` 已补齐 CGNAT / IPv4-mapped IPv6 / ULA fc00::/7 / link-local fe80::/10
  - secret 已改用 HMAC-SHA256（`webhook_signature::sign_webhook_payload`）
- **未修复部分**：
  - `client.post(url)` 仍传 URL 字符串 → reqwest 内部第三次解析 DNS → TOCTOU 窗口仍可被低 TTL DNS 利用
  - L187-211 内联 IP 校验逻辑未删除，与 `ssrf_guard` 重复且覆盖窄
- **攻击路径**：注册 webhook URL 为低 TTL 域名（5 秒切换公网↔内网）→ validate_url 通过 → reqwest 第三次解析命中 169.254.169.254 → 命中云元数据服务
- **修复方案**：
  1. 在 connect 时强制使用解析的 IP（`tokio::net::TcpStream::connect_timeout` 指定 IP 而非域名）
  2. 删除 L187-211 内联 IP 校验，统一调用 `ssrf_guard::validate_url`
  3. 禁用 secret 在 HTTP 头明文传递（已改 HMAC，部分完成）

#### H-2：Email Service `EmailConfig.api_url` 死字段残留（⚠️ 部分修复）

- **位置**：[email_service.rs:44](file:///workspace/backend/src/services/email_service.rs#L44)
- **已修复部分**：
  - `from_env()` 不再读取 `EMAIL_API_URL` 环境变量
  - `send_via_sendgrid` 改用 const `SENDGRID_API_URL = "https://api.sendgrid.com/v3/mail/send"` 硬编码
  - `self.config.api_url` 在代码中零引用
- **未修复部分**：
  - `EmailConfig.api_url: Option<String>` 字段仍保留在结构体定义中，未来若被误用可复活环境变量注入路径
  - `Authorization: Bearer` 明文传递 API Key（属 SendGrid 第三方 API 约束，不可独立修复）
- **修复方案**：直接删除 `EmailConfig.api_url` 字段及 `EMAIL_API_URL` 注释残留

#### H-3：init 子系统 test_database_connection SSRF（❌ 完全未修复）

- **位置**：
  - [init_handler.rs:102-119](file:///workspace/backend/src/handlers/init_handler.rs#L102-L119)（TODO 注释仍在，IP 白名单全部被注释）
  - [init_service.rs:38-45](file:///workspace/backend/src/services/init_service.rs#L38-L45)（`DatabaseConfig.port: String` 无范围校验）
  - [init_service.rs:107-130](file:///workspace/backend/src/services/init_service.rs#L107-L130)（错误消息直接透传 `sea_orm::DbErr` 原文）
- **完全未修复**：5 个检查点全部未实现
  1. TODO(ssrf) 注释仍在
  2. IP 白名单逻辑全部被注释
  3. port 范围无校验（任意 1-65535 TCP 端口可探测）
  4. 错误消息直接透传底层 `sea_orm::DbErr` 原文（区分超时/TCP 拒绝/认证失败）
  5. 未限定 system 初始化模式（已初始化后仍可调用）
- **攻击路径**：admin 用户调 `POST /api/v1/erp/init/test-database`，body `{"host":"10.0.0.1","port":"5432",...}` → 通过错误差异精确枚举内网 PostgreSQL 服务
- **修复方案**：
  1. 实现 IP 白名单（仅允许运维确认的内网网段）
  2. 限制 port 范围（仅 5432 PostgreSQL）
  3. 错误消息脱敏（统一返回"数据库连接失败"）
  4. 仅在 system 初始化模式（数据库未连接）下允许调用

---

## 历史已修复（迁移至 CHANGELOG.md / git 历史）

> 以下漏洞已修复，详细修复内容见 `.monkeycode/CHANGELOG.md` 对应 PR 条目。

### 2026-06-25 PR #253 修复（9 项）
- ✅ M-1 客户表 IDOR（created_by 校验）
- ✅ M-2 邮件服务 PII 日志泄露（save_email_log 脱敏）
- ✅ M-3 refresh_token 缺 is_active/JTI 校验
- ✅ M-4 邮件 HTML XSS（危险模式检测）
- ✅ M-5 邮件服务多项安全加固
- ✅ M-6 权限匹配 resource_id NULL 匹配过宽（精确匹配）
- ✅ M-7 SQL 注入审计黑名单不全（14 → 60+ 模式）
- ✅ L-1 CSRF 公开端点缺少二次验证
- ✅ L-2 legacy_jwt Cookie SameSite=Lax → Strict

### 2026-06-25 凌晨 H-4 修复（1 项）
- ✅ H-4 静态资源路径不 canonicalize → 符号链接任意文件读取（commit 5e6e1ac0）

### 2026-06-24 PR #250 修复（8 项）
- ✅ P0 路径遍历（文件下载沙箱化）
- ✅ P0 WebSocket 认证绕过（ws 握手 + JWT 校验）
- ✅ P1 init_token 缺失（subtle::ConstantTimeEq）
- ✅ P2 错误响应信息泄漏（移除 error_type/detail）
- ✅ P2 API Key 撤销失效（黑名单 + is_api_key_revoked）
- ✅ P2 分布式限流缺失（Redis INCR+EXPIRE + 内存回退）
- ✅ P2 弱密码接受（Top 100 黑名单 + l33t 归一 + 键盘序列）
- ✅ P2 错误响应类型泄漏（与 #4 同步脱敏）

### 2026-06-24 第一轮 6 个低危漏洞
- ✅ #1 JTI 黑名单→Redis（auth_service.rs SETEX + 内存回退）
- ✅ #2 Webhook URL 内网白名单（新建 ssrf_guard.rs）— **TOCTOU 仍存在（H-1）**
- ✅ #3 分布式限流 try_lock（rate_limit.rs std Mutex + try_lock）
- ✅ #4 认证失败日志脱敏（auth.rs mask_auth_header / mask_username）
- ✅ #5 JWT 密钥硬编码（审计无问题，main.rs 启动强制校验）
- ✅ #6 TOTP 熵源（审计无问题，totp-rs 5.5 Secret::generate_secret 用 OsRng）

### 2026-06-25 之前修复
- ✅ L-3 public_routes starts_with 误匹配（精确路径匹配，commit e3a97ea4）
- ✅ L-4 静态资源符号链接越界（与 H-4 同步，commit 5e6e1ac0）
- ✅ 优化 1 public_routes 公开端点收敛至登录/刷新/健康检查
- ✅ 优化 2 import_export 全表查询优化（条件过滤 + 行数限制 + 审计日志）

---

## 审计发现的新漏洞（2026-06-25 综合审计，按 P0/P1 优先级）

### P0-1 AP 发票默认汇率 0.01（应为 1.0），财务数据缩小 100 倍

- **位置**：[ap_invoice_service.rs:91,154](file:///workspace/backend/src/services/ap_invoice_service.rs#L91)
- **代码**：`exchange_rate: Set(Decimal::new(1, 2))` —— `Decimal::new(1, 2)` = `0.01`，应为 `Decimal::new(1, 0)` = 1.0
- **影响**：从采购入库单/退货单自动生成的 AP 发票，按汇率换算本位币金额的下游计算被缩小 100 倍
- **修复方案**：改 `Decimal::new(1, 0)`；订正历史数据；补单元测试覆盖汇率字段
- **登记日期**：2026-06-25
- **优先级**：P0（财务数据正确性）

### P1-11 销售订单审批 user_id 硬编码为 0，审计日志无法追溯

- **位置**：[so/order_workflow.rs:142,194,223](file:///workspace/backend/src/services/so/order_workflow.rs#L142)
- **影响**：审计完整性破坏，submit/approve/complete 的真实操作人无法追溯
- **修复方案**：将真实 `user_id` 传入 `update_with_audit` 第三参数
- **登记日期**：2026-06-25
- **优先级**：P1（审计完整性）
