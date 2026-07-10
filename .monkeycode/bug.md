# 安全审计漏洞报告

## 审计日期
2026-06-27

## 审计范围
- 认证与访问控制
- 注入向量（SQL、命令、路径）
- 外部交互（Webhook、出站请求）
- 敏感数据处理

---

## 一、高危漏洞

### 1.1 SQL 注入风险 - tracking_service.rs LIMIT 未参数化

**严重度**: 高危

**位置**: `/workspace/backend/src/services/tracking_service.rs:258-259`

**代码路径**:
```rust
sql.push_str(" GROUP BY path ORDER BY view_count DESC LIMIT ");
sql.push_str(&limit.to_string());
```

**攻击者画像**: 已认证用户

**可控输入向量**: 通过 `/api/v1/erp/analytics/popular-pages` 端点的 `limit` 查询参数

**利用路径**:
1. 用户发送请求 `GET /api/v1/erp/analytics/popular-pages?limit=1%20%3B%20DROP%20TABLE%20page_views--`
2. `tracking_handler.rs:147` 对 limit 进行 clamp(1, 100)，但没有验证是否包含 SQL 注入字符
3. `tracking_service.rs:259` 将 limit 直接拼接到 SQL 字符串中
4. 构造的恶意 SQL 被执行

**影响**: 数据库表删除、数据泄露、数据篡改

**修复建议**:
将 LIMIT 也改为参数化绑定：
```rust
sql.push_str(" GROUP BY path ORDER BY view_count DESC LIMIT $1");
params.push(limit.into());
```

---

### 1.2 命令注入风险 - backup.rs cmd_restore

**严重度**: 高危

**位置**: `/workspace/backend/src/cli/util/backup.rs:149`

**代码路径**:
```rust
run_cmd("cp", &[&src, &dst])
```

**攻击者画像**: 本地用户（具有执行 CLI 权限）

**可控输入向量**: 通过 CLI 参数 `bingxi restore --file=恶意路径`

**利用路径**:
1. 攻击者构造恶意备份文件，包含路径如 `../../../etc/passwd`
2. 执行 `bingxi restore --file=malicious_backup.tar.gz`
3. `cmd_restore` 解压文件后，将其中的文件复制到目标路径
4. 如果备份文件中包含指向敏感文件的路径，可能覆盖系统文件

**影响**: 系统文件覆盖、权限提升、数据泄露

**修复建议**:
在解压后验证所有文件路径是否在允许的目录范围内，使用 `canonicalize` 和 `starts_with` 检查。

---

### 1.3 SSRF 防护不完整 - currency_service.rs

**严重度**: 高危

**位置**: `/workspace/backend/src/services/currency_service.rs:301-305`

**代码路径**:
```rust
let client = reqwest::Client::builder()
    .redirect(reqwest::redirect::Policy::none())
    .timeout(std::time::Duration::from_secs(10))
    .build()?;
```

**攻击者画像**: 已认证用户

**可控输入向量**: 通过 `/api/v1/erp/currency/sync` 端点的币种参数

**利用路径**:
1. 虽然代码中有 `validate_currency_code` 校验，但如果验证不充分
2. 攻击者可能通过 DNS Rebinding 攻击，使域名在验证时解析为公网 IP
3. 实际请求时解析为内网 IP（如 192.168.1.1）
4. 绕过 SSRF 防护，访问内网服务

**影响**: 访问内网服务、云元数据泄露

**修复建议**:
使用 `ssrf_guard::validate_url_and_resolve` 获取安全 IP 列表，然后用 `resolve_to_addrs` 固定连接：
```rust
let (host, safe_addrs) = validate_url_and_resolve(&url)?;
let client = reqwest::Client::builder()
    .redirect(reqwest::redirect::Policy::none())
    .resolve_to_addrs(&host, &safe_addrs)
    .build()?;
```

---

## 二、中危漏洞

### 2.1 日志信息泄露 - webhook_service.rs

**严重度**: 中危

**位置**: `/workspace/backend/src/services/webhook_service.rs:235`

**代码路径**:
```rust
tracing::warn!(error = %e, webhook_url = %url, "Webhook 签名计算失败，跳过签名头");
```

**攻击者画像**: 已认证用户（创建 Webhook 的用户）

**可控输入向量**: 创建 Webhook 时的 URL 参数

**利用路径**:
1. 用户创建 Webhook 时设置包含敏感信息的 URL，如 `https://example.com/webhook?secret=abc123`
2. 如果签名计算失败，完整 URL 会被记录到日志中
3. 日志可能被攻击者获取，泄露敏感参数

**影响**: 敏感信息泄露

**修复建议**:
日志中不记录完整 URL，只记录主机名：
```rust
tracing::warn!(error = %e, webhook_host = %host, "Webhook 签名计算失败，跳过签名头");
```

---

### 2.2 缺少速率限制 - Webhook 测试端点

**严重度**: 中危

**位置**: `/workspace/backend/src/handlers/webhook_handler.rs:114-135`

**代码路径**:
```rust
pub async fn test_webhook(
    State(state): State<AppState>,
    _auth: AuthContext,
    Path(id): Path<i32>,
) -> Result<Json<ApiResponse<WebhookDeliveryResult>>, AppError> {
    // 无速率限制
}
```

**攻击者画像**: 已认证用户

**可控输入向量**: 重复调用 `/api/v1/erp/webhooks/:id/test`

**利用路径**:
1. 攻击者创建指向内网服务的 Webhook（虽然有 SSRF 防护，但可用于探测）
2. 频繁调用测试端点，获取响应状态码
3. 推断内网服务是否存在、服务状态等信息

**影响**: 信息泄露、资源消耗

**修复建议**:
添加速率限制中间件，限制单个用户每分钟最多调用 10 次。

---

### 2.3 文件权限安全 - system_update_service.rs

**严重度**: 中危

**位置**: `/workspace/backend/src/services/system_update_service.rs:436-448`

**代码路径**:
```rust
if let Some(mode) = file.unix_mode() {
    if let Ok(metadata) = fs::metadata(&outpath) {
        let mut perms = metadata.permissions();
        perms.set_mode(mode);
        if let Err(e) = fs::set_permissions(&outpath, perms) {
            tracing::warn!("设置文件权限失败 {:?}: {}", outpath, e);
        }
    }
}
```

**攻击者画像**: 更新包提供者（如果更新源被劫持）

**可控输入向量**: 更新包中的文件权限位

**利用路径**:
1. 更新包中包含设置了 SUID/SGID 位的恶意文件
2. 更新服务解压时保留原始权限
3. 恶意文件获得特殊权限，可能导致权限提升

**影响**: 权限提升、代码执行

**修复建议**:
解压时重置文件权限，不允许 SUID/SGID 位：
```rust
let mut perms = fs::Permissions::default();
#[cfg(unix)]
{
    use std::os::unix::fs::PermissionsExt;
    let safe_mode = mode & 0o755; // 移除 SUID/SGID/粘性位
    perms.set_mode(safe_mode);
}
```

---

## 三、低危漏洞

### 3.1 缺少全局请求体大小限制

**严重度**: 低危

**位置**: 全局配置

**问题描述**:
虽然 `crm_handler.rs` 的 `import_leads` 有 10MB 文件大小限制，但缺少全局的请求体大小限制。

**影响**: 潜在的资源耗尽攻击

**修复建议**:
在 axum 应用配置中设置全局请求体大小限制：
```rust
Router::new()
    .layer(axum::extract::DefaultBodyLimit::max(10 * 1024 * 1024))
```

---

### 3.2 备份文件权限

**严重度**: 低危

**位置**: `/workspace/backend/src/cli/util/backup.rs:54-62`

**代码路径**:
```rust
run_cmd("cp", &["-r", &config_dir, &backup_dir])
run_cmd("cp", &["-r", env_file, &backup_dir])
run_cmd("cp", &["-r", &service_file, &backup_dir])
```

**问题描述**:
备份文件包含 `.env`（可能包含数据库密码），但没有设置安全的文件权限。

**影响**: 敏感信息可能被其他用户读取

**修复建议**:
备份完成后设置文件权限为 0o600（仅所有者可读）：
```rust
run_cmd("chmod", &["-R", "600", &backup_dir])
```

---

## 四、已确认的安全实践（通过审计）

### 4.1 认证与访问控制
- ✅ JWT 验证使用 HS256 算法，包含过期时间检查
- ✅ 密码使用 Argon2id 哈希（64MB内存，3次迭代）
- ✅ 角色权限校验在 middleware 层实现
- ✅ CSRF 防护包含 token 验证和 IP 绑定

### 4.2 SQL 注入防护
- ✅ 大部分 SQL 查询使用参数化绑定
- ✅ LIKE 模式使用 `escape_like_pattern` 转义特殊字符
- ✅ 分页参数使用 clamp 限制范围

### 4.3 SSRF 防护
- ✅ Webhook URL 验证包含：协议白名单、主机名黑名单、IP 范围检查
- ✅ 使用 `resolve_to_addrs` 消除 DNS Rebinding TOCTOU 漏洞
- ✅ 禁止跟随重定向

### 4.4 路径遍历防护
- ✅ 静态文件服务使用 `sanitize_static_path` 过滤路径
- ✅ 使用 `canonicalize` 解析符号链接，防止逃逸

### 4.5 Webhook 安全
- ✅ 出站签名使用 HMAC-SHA256
- ✅ 测试接口不回显响应内容，防止 SSRF 信息泄露
- ✅ 重试次数限制（MAX_RETRY_COUNT=5）

---

## 五、修复优先级建议

| 优先级 | 漏洞 | 预计修复时间 |
|--------|------|-------------|
| P0 | 1.1 SQL注入 (LIMIT) | 1小时 |
| P0 | 1.2 命令注入 (backup) | 2小时 |
| P0 | 1.3 SSRF (currency) | 1小时 |
| P1 | 2.1 日志泄露 | 30分钟 |
| P1 | 2.2 速率限制 | 1小时 |
| P1 | 2.3 文件权限 | 1小时 |
| P2 | 3.1 请求体限制 | 30分钟 |
| P2 | 3.2 备份权限 | 30分钟 |
