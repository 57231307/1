# 安全漏洞记录

> 本文件用于登记项目安全漏洞。所有已修复漏洞已迁移至 git 历史（CHANGELOG.md / PR）。
> 审计周期内如有新漏洞发现，登记后立即启动修复流程。
> 详见 `.monkeycode/MEMORY.md` 的 Bug.md 实时漏洞管理规则。

---

## 待修复漏洞（截至 2026-06-25 综合审计修复批次）

> 来源：2026-06-25 综合审计报告 `.monkeycode/docs/audits/2026-06-25-comprehensive-audit.md`
> 修复批次：9 个 commit（待推送 CI 验证）

---

### 🟡 残留漏洞（1 项，待最终修复）

#### H-1：Webhook 发送 SSRF TOCTOU（🟡 接近完成，仅剩 connector 改造）

- **已修复部分**（2026-06-25 修复批次）：
  - ✅ `validate_url` 已调用 `ssrf_guard::validate_url`
  - ✅ `ssrf_guard.rs` 已补齐 CGNAT / IPv4-mapped IPv6 / ULA fc00::/7 / link-local fe80::/10
  - ✅ secret 已改用 HMAC-SHA256（`webhook_signature::sign_webhook_payload`）
  - ✅ **删除 L187-211 内联 IP 校验逻辑**，统一调用 `ssrf_guard::validate_url`（P1-2 修复）
- **未修复部分**（仅剩 1 项）：
  - ❌ `client.post(url)` 仍传 URL 字符串 → reqwest 内部第三次解析 DNS → TOCTOU 窗口仍可被低 TTL DNS 利用
- **修复方案**：在 connect 时强制使用解析的 IP（reqwest 自定义 connector + `tokio::net::TcpStream::connect_timeout` 指定 IP 而非域名）
- **风险等级**：已从"部分修复"降级为"接近完成"（仅剩 connector 改造，攻击窗口已大幅收敛）

---

## 已修复漏洞（2026-06-25 综合审计修复批次，9 commits）

> 以下漏洞已在 2026-06-25 修复批次中修复，共 9 个 commit，待推送 CI 验证。

### ✅ H-2：Email Service `EmailConfig.api_url` 死字段残留（P1-3）

- **状态**：✅ 已修复
- **修复**：删除 `EmailConfig.api_url: Option<String>` 字段定义 + `from_env()` 中 `api_url: None` 赋值
- **残留**：`Authorization: Bearer` 明文传递属 SendGrid 第三方 API 约束，不可独立修复

### ✅ H-3：init 子系统 test_database_connection SSRF（P1-1）

- **状态**：✅ 已修复（5 个检查点全部实现）
- **修复内容**：
  1. ✅ port 范围校验（解析为 u16，仅允许 1-65535）
  2. ✅ 内网 IP 白名单（实现 `is_internal_ip` 函数，仅允许 RFC1918 + loopback + link-local + ULA）
  3. ✅ 错误消息脱敏（不透传底层 DbErr 原文，详细错误通过 tracing::warn 记录）
  4. ✅ 初始化模式约束（系统已初始化后拒绝调用）
  5. ✅ TODO(ssrf) 注释已移除（逻辑已实现）

### ✅ P0-1：AP 发票默认汇率 0.01（应为 1.0）

- **状态**：✅ 已修复
- **修复**：抽取常量 `DEFAULT_BASE_CURRENCY_EXCHANGE_RATE = Decimal::new(1, 0)`，替换 L91/L154/L200 三处汇率赋值，补单元测试覆盖
- **残留**：历史数据订正脚本需另起任务处理

### ✅ P1-11：销售订单/AP 发票审批 user_id 硬编码 0

- **状态**：✅ 已修复
- **修复**：so/order_workflow.rs L142/L194/L223 + ap_invoice_service.rs L278/L345/L430 的 `Some(0)` → `Some(user_id)`
- **残留**：ap_invoice_service.rs `mark_as_paid`（事件驱动场景无用户上下文）保留 `Some(0)` + TODO

### ✅ P1-10：AP 发票自动生成跳过审批 + 税额丢失

- **状态**：✅ 已修复
- **修复**：`invoice_status` AUDITED → PENDING（两处）；return 分支从 purchase_return_item 汇总 tax_amount
- **残留**：receipt 分支 tax_amount 保持 ZERO（receipt 模型无 tax_amount 字段，待模型补充后传递）

### ✅ P1-4：quotations 双重路由注册

- **状态**：✅ 已修复
- **修复**：移除 sales.rs 中 `.nest("/quotations", quotations())` + `quotations()` 函数 + `quotation_handler` import

### ✅ P1-13/14/15：audit_log_handler / slow_query_handler 死代码

- **状态**：✅ 已修复
- **修复**：system::routes() 补挂载 `.merge(audit_logs()).merge(slow_queries())`；移除两个文件共 14 处 `#[allow(dead_code)]` 标记

### ✅ P2-7：custom_order_process_test.rs crate:: 编译错误

- **状态**：✅ 已修复
- **修复**：`crate::utils::process_state_machine::CustomOrderStatus::Xxx` → `bingxi_backend::utils::process_state_machine::CustomOrderStatus::Xxx`

---

## 历史已修复（迁移至 CHANGELOG.md / git 历史）

> 详细修复内容见 `.monkeycode/CHANGELOG.md` 对应 PR 条目。

### 2026-06-25 PR #253 修复（9 项）
- ✅ M-1 ~ M-7、L-1、L-2（详见 CHANGELOG.md）

### 2026-06-25 凌晨 H-4 修复（1 项）
- ✅ H-4 静态资源路径不 canonicalize → 符号链接任意文件读取（commit 5e6e1ac0）

### 2026-06-24 PR #250 修复（8 项）
- ✅ P0 路径遍历 / P0 WebSocket 认证绕过 / P1 init_token / P2 错误响应信息泄漏 / P2 API Key 撤销失效 / P2 分布式限流 / P2 弱密码 / P2 错误响应类型泄漏

### 2026-06-24 第一轮 6 个低危漏洞
- ✅ #1 ~ #6（详见 CHANGELOG.md）

### 2026-06-25 之前修复
- ✅ L-3 / L-4 / 优化 1 / 优化 2（详见 CHANGELOG.md）
