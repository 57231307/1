# 安全审计漏洞登记

> 实时检测与修复（修复后删除条目）。所有漏洞修复完成后保留本文件为空。
> 最近一次整理：2026-07-11（批次 289 后规则 10 整理），已删除已修复的 3.1 请求体限制条目。

## 审计来源

- 2026-06-27 安全审计报告（剩余 7 项未修复）
- v14 深度调研报告（2026-07-09，114 个问题，大部分已在批次 237-289 修复，剩余待修复项见 [doto.md](file:///workspace/.monkeycode/doto.md)）

---

## 一、高危漏洞（3 项 ⏳ 未修复）

### 1.1 SQL 注入风险 - tracking_service.rs LIMIT 未参数化

**严重度**：高危
**状态**：⏳ 未修复
**位置**：[tracking_service.rs:258-259](file:///workspace/backend/src/services/tracking_service.rs#L258-L259)

**问题**：`limit` 参数直接拼接到 SQL 字符串，攻击者可通过 `/api/v1/erp/analytics/popular-pages?limit=` 注入 SQL。

**修复方案**：改为参数化绑定 `LIMIT $1` + `params.push(limit.into())`。

---

### 1.2 命令注入风险 - backup.rs cmd_restore

**严重度**：高危
**状态**：⏳ 未修复
**位置**：[backup.rs:149](file:///workspace/backend/src/cli/util/backup.rs#L149)

**问题**：`cmd_restore` 解压后直接 `cp`，未校验文件路径范围，恶意备份文件可覆盖系统文件。

**修复方案**：解压后用 `canonicalize` + `starts_with` 校验所有文件路径在允许目录范围内。

---

### 1.3 SSRF 防护不完整 - currency_service.rs

**严重度**：高危
**状态**：⏳ 未修复
**位置**：[currency_service.rs:301-305](file:///workspace/backend/src/services/currency_service.rs#L301-L305)

**问题**：仅禁止重定向，未使用 `resolve_to_addrs` 固定 IP，存在 DNS Rebinding 攻击风险。

**修复方案**：使用 `ssrf_guard::validate_url_and_resolve` 获取安全 IP 列表，用 `resolve_to_addrs` 固定连接。

---

## 二、中危漏洞（3 项 ⏳ 未修复）

### 2.1 日志信息泄露 - webhook_service.rs

**严重度**：中危
**状态**：⏳ 未修复
**位置**：[webhook_service.rs:235](file:///workspace/backend/src/services/webhook_service.rs#L235)

**问题**：签名计算失败时日志记录完整 webhook URL，可能泄露 URL 中的敏感参数。

**修复方案**：日志只记录主机名，不记录完整 URL。

---

### 2.2 缺少速率限制 - Webhook 测试端点

**严重度**：中危
**状态**：⏳ 未修复
**位置**：[webhook_handler.rs:114-135](file:///workspace/backend/src/handlers/webhook_handler.rs#L114-L135)

**问题**：`test_webhook` 端点无速率限制，攻击者可频繁调用探测内网服务。

**修复方案**：添加速率限制中间件，限制单个用户每分钟最多 10 次。

---

### 2.3 文件权限安全 - system_update_service.rs

**严重度**：中危
**状态**：⏳ 未修复
**位置**：[system_update_service.rs:438](file:///workspace/backend/src/services/system_update_service.rs#L438)

**问题**：解压时保留原始 `unix_mode`，恶意更新包可设置 SUID/SGID 位导致权限提升。

**修复方案**：解压时重置权限 `mode & 0o755`，移除 SUID/SGID/粘性位。

---

## 三、低危漏洞（1 项 ⏳ 未修复）

### 3.1 ~~缺少全局请求体大小限制~~ ✅ 已修复（2026-07-11 确认）

**状态**：✅ 已修复
**位置**：[main.rs:556](file:///workspace/backend/src/main.rs#L556)

**说明**：已在 main.rs 中配置 `DefaultBodyLimit::max(MAX_HTTP_BODY_BYTES)`，全局请求体大小限制已生效。本条目已从待修复队列删除。

---

### 3.2 备份文件权限

**严重度**：低危
**状态**：⏳ 未修复
**位置**：[backup.rs:54-62](file:///workspace/backend/src/cli/util/backup.rs#L54-L62)

**问题**：备份文件包含 `.env`（可能含数据库密码），但未设置安全文件权限。

**修复方案**：备份完成后设置文件权限为 `0o600`（仅所有者可读）。

---

## 四、已确认的安全实践（通过审计）

### 认证与访问控制
- ✅ JWT 验证使用 HS256 算法，包含过期时间检查
- ✅ 密码使用 Argon2id 哈希（64MB 内存，3 次迭代）
- ✅ 角色权限校验在 middleware 层实现
- ✅ CSRF 防护包含 token 验证和 IP 绑定

### SQL 注入防护
- ✅ 大部分 SQL 查询使用参数化绑定
- ✅ LIKE 模式使用 `escape_like_pattern` 转义特殊字符
- ✅ 分页参数使用 clamp 限制范围

### SSRF 防护
- ✅ Webhook URL 验证包含：协议白名单、主机名黑名单、IP 范围检查
- ✅ 使用 `resolve_to_addrs` 消除 DNS Rebinding TOCTOU 漏洞
- ✅ 禁止跟随重定向

### 路径遍历防护
- ✅ 静态文件服务使用 `sanitize_static_path` 过滤路径
- ✅ 使用 `canonicalize` 解析符号链接，防止逃逸

### Webhook 安全
- ✅ 出站签名使用 HMAC-SHA256
- ✅ 测试接口不回显响应内容，防止 SSRF 信息泄露
- ✅ 重试次数限制（MAX_RETRY_COUNT=5）

---

## 五、修复优先级建议

| 优先级 | 漏洞 | 状态 |
|--------|------|------|
| P0 | 1.1 SQL 注入 (LIMIT) | ⏳ 未修复 |
| P0 | 1.2 命令注入 (backup) | ⏳ 未修复 |
| P0 | 1.3 SSRF (currency) | ⏳ 未修复 |
| P1 | 2.1 日志泄露 | ⏳ 未修复 |
| P1 | 2.2 速率限制 | ⏳ 未修复 |
| P1 | 2.3 文件权限 | ⏳ 未修复 |
| P2 | 3.2 备份权限 | ⏳ 未修复 |
