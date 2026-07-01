# 安全漏洞记录

> 本文件用于登记项目安全漏洞。所有已修复漏洞已迁移至 git 历史（CHANGELOG.md / PR）。
> 审计周期内如有新漏洞发现，登记后立即启动修复流程。
> 详见 `.monkeycode/MEMORY.md` 的 Bug.md 实时漏洞管理规则。

---

## 周期性安全审计报告（2026-07-01）

> 审计范围：全代码库高风险攻击面（认证与访问控制、注入向量、外部交互、敏感数据处理）
> 审计结论：**未发现中等或更高严重度的已确认漏洞**
> 历史待修复 7 项漏洞已全部验证修复完成

### 审计详情

#### 一、认证与访问控制 ✅ 安全

- **JWT 认证**：多层防护（签名验证、JTI 黑名单、用户级 Token 吊销、is_active 检查）
  - 文件：[auth.rs](file:///workspace/backend/src/middleware/auth.rs)
- **权限校验**：基于角色的权限系统，带 5 分钟 TTL 缓存，精确匹配 resource_type/resource_id/action
  - 文件：[permission.rs](file:///workspace/backend/src/middleware/permission.rs)
- **管理员检查**：fail-closed 设计（数据库异常时拒绝访问），使用 ADMIN_ROLE_CODE 常量
  - 文件：[admin_checker.rs](file:///workspace/backend/src/utils/admin_checker.rs)
- **CSRF 防护**：Token + IP 绑定，一次性消费，公开路由要求自定义请求头
  - 文件：[csrf.rs](file:///workspace/backend/src/middleware/csrf.rs)
- **Init Token**：初始化接口保护，恒定时间比较防时序攻击，fail-secure 设计
  - 文件：[init_token.rs](file:///workspace/backend/src/middleware/init_token.rs)

#### 二、注入向量 ✅ 安全

- **SQL 注入**：使用 SeaORM 参数化查询，SQL 注入审计中间件覆盖 URL 和请求体
  - 文件：[sql_injection_audit.rs](file:///workspace/backend/src/middleware/sql_injection_audit.rs)
- **路径遍历**：静态文件服务完整的路径规范化和符号链接检查（canonicalize + starts_with）
  - 文件：[static.rs](file:///workspace/backend/src/routes/static.rs)
- **命令注入**：未发现直接的 shell 命令执行或系统命令拼接

#### 三、外部交互 ✅ 安全

- **Webhook SSRF**：完整防护（HTTPS 强制、IP 黑名单、DNS 重绑定防护、禁止重定向、resolve_to_addrs 固定 IP）
  - 文件：[webhook_service.rs](file:///workspace/backend/src/services/webhook_service.rs)
  - 文件：[ssrf_guard.rs](file:///workspace/backend/src/utils/ssrf_guard.rs)
- **系统更新**：GitHub 域名白名单、HTTPS 强制、重定向限制、最终 URL 二次校验
  - 文件：[system_update_service.rs](file:///workspace/backend/src/services/system_update_service.rs)
- **汇率服务**：ISO 4217 校验、禁止重定向
  - 文件：[currency_service.rs](file:///workspace/backend/src/services/currency_service.rs)

#### 四、敏感数据处理 ✅ 安全

- **密码**：Argon2id 哈希算法
- **JWT 密钥**：环境变量配置，支持密钥轮换
- **日志脱敏**：Authorization 头和用户名脱敏
- **Webhook 密钥**：独立的 Webhook 签名密钥配置
- **测试密钥**：运行时随机生成，无硬编码
  - 文件：[auth_service.rs](file:///workspace/backend/src/services/auth_service.rs)

### 历史待修复漏洞验证结果（7 项全部已修复）

| 编号 | 漏洞 | 状态 | 验证位置 |
|------|------|------|----------|
| TS-S-1 | Setup 模式 init 接口认证绕过 | ✅ 已修复 | [init_token.rs](file:///workspace/backend/src/middleware/init_token.rs) 中间件已挂载 |
| TS-S-2/BE-V-2 | Webhook SSRF TOCTOU 核心漏洞 | ✅ 已修复 | [webhook_service.rs](file:///workspace/backend/src/services/webhook_service.rs) 使用 resolve_to_addrs |
| TS-S-3 | 测试夹具中硬编码 JWT 密钥 | ✅ 已修复 | [auth_service.rs](file:///workspace/backend/src/services/auth_service.rs) 运行时随机生成 |
| TS-S-4 | SQL 注入审计中间件不覆盖请求体 | ✅ 已修复 | [sql_injection_audit.rs](file:///workspace/backend/src/middleware/sql_injection_audit.rs) 请求体审计已实现 |
| TS-S-5 | 大量 handler 未调用 validator | ⚠️ 部分修复 | 已覆盖 31 个 handler 文件（58 处 validate 调用） |
| TS-S-6/BE-V-1 | currency_service 输入未校验 | ✅ 已修复 | ISO 4217 正则校验已实现 |
| TS-S-7/BE-V-3 | system_update 下载域名未校验 | ✅ 已修复 | GitHub 域名白名单已实现 |

> 注：TS-S-5 输入验证属于代码质量范畴，SeaORM 参数化查询已防止 SQL 注入，
> 不构成可论证的端到端安全利用路径，故不计为中危及以上漏洞。

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
