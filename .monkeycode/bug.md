# 安全漏洞记录

> 本文件用于登记项目安全漏洞。所有已修复漏洞已迁移至 git 历史（CHANGELOG.md / PR）。
> 审计周期内如有新漏洞发现，登记后立即启动修复流程。
> 详见 `.monkeycode/MEMORY.md` 的 Bug.md 实时漏洞管理规则。

---

## 待修复漏洞（截至 2026-06-25 第二次全面审计）

> 来源：2026-06-25 第二次全面审计报告 `.monkeycode/docs/audits/2026-06-25-full-reaudit.md`
> 审计规则：所有问题均列为错误，不区分严重度

---

### 待修复安全漏洞（7 项）

#### TS-S-1：Setup 模式 init 接口认证绕过（最高优先级）

- **文件**：[main.rs:197-206,574](file:///workspace/backend/src/main.rs#L197)
- **问题**：Setup 模式 `create_init_router()` 暴露 `/api/v1/erp/init/initialize-with-db` 等路由，但未挂载 `init_token_middleware`，攻击者可不带凭据直接 POST 完成系统初始化
- **修复**：在 `create_init_router()` 高危路由上挂载 `.layer(middleware::from_fn(init_token_middleware))`

#### TS-S-2 / BE-V-2：Webhook SSRF TOCTOU 核心漏洞

- **文件**：[webhook_service.rs:195-208](file:///workspace/backend/src/services/webhook_service.rs#L195)
- **问题**：`client.post(url)` 仍传 URL 字符串，reqwest 内部第三次解析 DNS，DNS rebinding 可绕过 SSRF
- **修复**：用 reqwest 自定义 connector 在 connect 时 pin 解析出的 IP

#### TS-S-3：测试夹具中硬编码 JWT 密钥

- **文件**：[auth_service.rs:650,713](file:///workspace/backend/src/services/auth_service.rs#L650)
- **问题**：`TEST_JWT_SECRET` 固定密钥常量，测试产物泄露后可伪造 JWT
- **修复**：改为运行时生成随机密钥

#### TS-S-4：SQL 注入审计中间件不覆盖请求体

- **文件**：[sql_injection_audit.rs:130-152](file:///workspace/backend/src/middleware/sql_injection_audit.rs#L130)
- **问题**：中间件仅审计 URL，不读取 POST/PUT 请求体
- **修复**：对文本类请求体做有限大小（1MB）的危险模式审计

#### TS-S-5：大量 handler 未调用 validator::Validate

- **文件**：[handlers/](file:///workspace/backend/src/handlers/) 多文件
- **问题**：60+ handler 未对用户输入做 `validator::Validate` 校验
- **修复**：为所有 DTO 添加 `#[derive(Validate)]` 并在 handler 入口调用 `req.validate()?`

#### TS-S-6 / BE-V-1：currency_service from_currency 未做输入校验

- **文件**：[currency_service.rs:251-264](file:///workspace/backend/src/services/currency_service.rs#L251)
- **问题**：`from_currency` 直接拼入 URL path，未校验为合法 ISO 4217 币种码
- **修复**：正则校验 `^[A-Z]{3}$`

#### TS-S-7 / BE-V-3：system_update_service 下载域名未校验

- **文件**：[system_update_service.rs:716-722](file:///workspace/backend/src/services/system_update_service.rs#L716)
- **问题**：`download_update` 未校验 `asset.browser_download_url` 域名白名单
- **修复**：校验为 `github.com`/`objects.githubusercontent.com`

---

## 已修复漏洞（2026-06-25 综合审计修复批次，PR #254 + #255）

> 以下漏洞已修复并通过 CI 验证。

### ✅ H-2：EmailConfig.api_url 死字段残留（P1-3）
### ✅ H-3：init SSRF（P1-1，5 检查点全部实现）
### ✅ P0-1：AP 发票汇率 0.01 → 1.0
### ✅ P1-11：user_id 硬编码 0 修复
### ✅ P1-10：AP 发票自动生成保留 PENDING + 传递 tax_amount
### ✅ P1-4：quotations 双重路由去重
### ✅ P1-13/14/15：audit_log/slow_query 死代码补挂载
### ✅ P2-7：custom_order_process_test.rs 编译错误

---

## 历史已修复（迁移至 CHANGELOG.md / git 历史）

> 详细修复内容见 `.monkeycode/CHANGELOG.md` 对应 PR 条目。

### 2026-06-25 PR #253 修复（9 项）
### 2026-06-25 凌晨 H-4 修复（1 项）
### 2026-06-24 PR #250 修复（8 项）
### 2026-06-24 第一轮 6 个低危漏洞
### 2026-06-25 之前修复
