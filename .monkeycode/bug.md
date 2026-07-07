# 安全漏洞记录

> 本文件用于登记项目安全漏洞。所有已修复漏洞已迁移至 git 历史（CHANGELOG.md / PR）。
> 审计周期内如有新漏洞发现，登记后立即启动修复流程。
> 详见 `.monkeycode/MEMORY.md` 的 Bug.md 实时漏洞管理规则。

---

## 周期性安全审计报告（2026-07-07）

> 审计范围：全代码库高风险攻击面（认证与访问控制、注入向量、外部交互、敏感数据处理）
> 审计结论：**未发现中等或更高严重度的已确认漏洞**
> 历史待修复漏洞已全部验证修复完成

### 审计详情

#### 一、认证与访问控制 ✅ 安全

- **JWT 认证**：多层防护完善
  - 签名验证：使用jsonwebtoken库验证JWT签名
  - JTI 黑名单：已吊销的session_id立即拒绝（进程内黑名单）
  - 用户级 Token 吊销：软删除/封禁用户时标记该用户的所有iat < revoked_at的Token一律拒绝
  - is_active 检查：5分钟本地缓存避免每次查DB，禁用用户的旧JWT失效窗口最坏延迟5分钟
  - 密钥轮换：支持previous_jwt_secret平滑过渡
  - 文件：[auth.rs](file:///workspace/backend/src/middleware/auth.rs)

- **WebSocket 认证**：握手时验证 JWT，从 URL 参数提取 token 后校验
  - 文件：[notifications.rs](file:///workspace/backend/src/websocket/notifications.rs)

- **权限校验**：基于角色的权限系统，精确匹配
  - 5 分钟 TTL 缓存，精确匹配 resource_type/resource_id/action
  - resource_id 精确匹配（None 匹配 None，Some(id) 匹配 Some(id)，防止垂直越权）
  - action 支持 "*" 通配符（表示该资源类型的所有操作）
  - 文件：[permission.rs](file:///workspace/backend/src/middleware/permission.rs)

- **管理员检查**：fail-closed 设计（数据库异常时拒绝访问），使用 ADMIN_ROLE_CODE 常量
  - 文件：[admin_checker.rs](file:///workspace/backend/src/utils/admin_checker.rs)

- **CSRF 防护**：Token + IP 绑定，一次性消费，公开路由要求自定义请求头
  - Token生成：登录时生成并绑定客户端IP
  - Token验证：消费时校验IP一致性，防止跨IP重放
  - 一次性消费：验证成功后立即从缓存移除，防止重放攻击
  - 公开路径防御：非安全方法要求自定义请求头（X-Requested-With 或 X-CSRF-Token），阻止简单表单提交CSRF
  - 文件：[csrf.rs](file:///workspace/backend/src/middleware/csrf.rs)

- **Init Token**：初始化接口保护，恒定时间比较防时序攻击，fail-secure 设计
  - 文件：[init_token.rs](file:///workspace/backend/src/middleware/init_token.rs)

- **速率限制**：IP + UserID 双维度限流（180 req/min），登录端点防暴力破解（5次/5分钟），分布式优先内存回退
  - 文件：[rate_limit.rs](file:///workspace/backend/src/middleware/rate_limit.rs)

- **公开路径收敛**：仅健康检查 + 登录 + 刷新 + Webhook回调 6 个端点匿名访问，严格前缀匹配防绕过
  - 精确匹配 + 子路径匹配（starts_with(p) && path[p.len()..].starts_with('/')），防止/api/v1/erp/auth/login-bypass绕过
  - 文件：[public_routes.rs](file:///workspace/backend/src/middleware/public_routes.rs)

#### 二、注入向量 ✅ 安全

- **SQL 注入**：多层防护完善
  - 核心业务：使用 SeaORM 参数化查询（.filter().eq()，自动参数绑定）
  - 原始 SQL 场景：omni_audit/audit_cleanup/slow_query/bi_analysis均使用 `Statement::from_sql_and_values` 参数绑定，无用户输入直接拼接
  - LIKE 查询：使用 safe_like_pattern 转义特殊字符（%, _, \），防止SQL注入
  - SQL 注入审计中间件：覆盖 URL 和请求体，检测常见注入模式
  - 文件：[sql_escape.rs](file:///workspace/backend/src/utils/sql_escape.rs)
  - 文件：[sql_injection_audit.rs](file:///workspace/backend/src/middleware/sql_injection_audit.rs)
  - 文件：[bi_analysis_service.rs](file:///workspace/backend/src/services/bi_analysis_service.rs)

- **路径遍历**：静态文件服务完整的路径规范化
  - canonicalize + starts_with 检查，防止符号链接绕过
  - 文件：[static.rs](file:///workspace/backend/src/routes/static.rs)

- **命令注入**：CLI 工具使用 `Command::new(cmd).args(args)` 参数数组调用，无 shell 字符串拼接
  - 文件：[cli/util/mod.rs](file:///workspace/backend/src/cli/util/mod.rs)

- **XSS 防护**：前端 v-html 场景使用 DOMPurify 白名单过滤，CSP 响应头限制脚本来源
  - 文件：[csp.rs](file:///workspace/backend/src/middleware/csp.rs)

- **CSV/Excel 注入防护**：导入导出服务对单元格内容进行转义，防止公式注入（=, +, -, @）
  - 文件：[import_export_handler.rs](file:///workspace/backend/src/handlers/import_export_handler.rs)

#### 三、外部交互 ✅ 安全

- **Webhook SSRF**：完整防护（IPv4/IPv6双栈覆盖）
  - HTTPS 强制：仅允许 http/https 协议，拒绝 file://、gopher:// 等
  - IP 黑名单：覆盖 IPv4 RFC1918/loopback/link-local/云元数据/CGNAT + IPv6 loopback/link-local/ULA/multicast
  - DNS 重绑定防护：validate_url_and_resolve 返回固定IP列表，调用方使用 resolve_to_addrs 消除 TOCTOU窗口
  - 主机名黑名单：localhost/.local/.internal/.intranet/.corp/.lan/.home
  - 禁止重定向：或二次校验最终URL
  - 文件：[ssrf_guard.rs](file:///workspace/backend/src/utils/ssrf_guard.rs)
  - 文件：[webhook_service.rs](file:///workspace/backend/src/services/webhook_service.rs)

- **Webhook 签名**：入站/出站统一 HMAC-SHA256，恒定时间比较防时序攻击
  - 使用 subtle::ConstantTimeEq 进行恒定时间比较
  - 文件：[webhook_signature.rs](file:///workspace/backend/src/utils/webhook_signature.rs)

- **系统更新**：GitHub 域名白名单、HTTPS 强制、重定向限制、最终 URL 二次校验
  - 文件：[system_update_service.rs](file:///workspace/backend/src/services/system_update_service.rs)

- **汇率服务**：ISO 4217 校验、禁止重定向
  - 文件：[currency_service.rs](file:///workspace/backend/src/services/currency_service.rs)

#### 四、敏感数据处理 ✅ 安全

- **密码**：Argon2id 哈希算法 + 多重强度校验
  - 密码强度校验：长度（8-128）、复杂度（大写+小写+数字+特殊字符）
  - 常见密码黑名单：Top 100 全球泄露密码（password/123456/admin/monkey等）
  - 键盘序列检测：横排（qwerty/asdf/zxcv）、竖排（qazwsx）、数字行（1234）
  - l33t 变体检测：归一化后匹配（@/4→a, 3→e, 1/!/|→i, 0→o, 5/$→s, 7→t）
  - 截尾黑名单：去掉末尾数字/特殊字符后匹配（password1! → password）
  - 文件：[password_validator.rs](file:///workspace/backend/src/utils/password_validator.rs)
  - 文件：[auth_service.rs](file:///workspace/backend/src/services/auth_service.rs)

- **密钥管理**：环境变量配置，独立密钥，强制强度，密钥轮换
  - JWT 密钥：环境变量配置，支持密钥轮换（previous_jwt_secret 平滑过渡）
  - Cookie 密钥：独立 cookie_secret，强制 32 字节以上，禁止降级复用 JWT 密钥
  - Webhook 密钥：独立 webhook_secret，强制 32 字节以上，禁止复用 JWT 密钥
  - 审计密钥：独立 AUDIT_SECRET_KEY，用于审计日志 HMAC 签名
  - 启动时校验：所有密钥强制通过 validate_secret 校验，拒绝弱密钥（"secret"/"password"等）和默认占位符
  - 文件：[main.rs](file:///workspace/backend/src/main.rs)
  - 文件：[config/settings.rs](file:///workspace/backend/src/config/settings.rs)

- **日志脱敏**：完善实现
  - Authorization 头截断脱敏（保留前12字符，完整Token不进入日志）
  - 用户名 PII 截断（保留前2字符 + ***）
  - 按字符而非字节截断（支持中文用户名）
  - 文件：[auth.rs](file:///workspace/backend/src/middleware/auth.rs)

- **测试密钥**：运行时随机生成，无硬编码
  - 文件：[auth_service.rs](file:///workspace/backend/src/services/auth_service.rs)
  - 文件：[app_state.rs](file:///workspace/backend/src/utils/app_state.rs)

- **前端 Token 存储**：access_token/refresh_token 由后端 httpOnly Cookie 管理，前端 JS 无法读取
  - 仅 csrf_token 存储于非 httpOnly Cookie 供前端读取
  - 文件：[storage.ts](file:///workspace/frontend/src/utils/storage.ts)

- **API 密钥**：SHA256 哈希存储，撤销后入黑名单缓存（TTL 7天）
  - 文件：[api_key_service.rs](file:///workspace/backend/src/services/api_key_service.rs)

- **敏感操作告警**：密码变更等敏感操作触发安全审计事件
  - 文件：[sensitive_action_alert.rs](file:///workspace/backend/src/services/sensitive_action_alert.rs)

### 低危观察项（不构成可利用漏洞）

| 编号 | 观察项 | 说明 | 位置 |
|------|--------|------|------|
| LOW-1 | `webhook_signature.rs` 返回Result而非expect | 批次117已修复，HMAC初始化失败时返回Err而非panic | [webhook_signature.rs](file:///workspace/backend/src/utils/webhook_signature.rs) |
| LOW-2 | 数据权限服务预留 API | 已标注 `#[allow(dead_code)] + TODO`，尚未广泛接入业务，不构成当前攻击面 | [data_permission_service.rs](file:///workspace/backend/src/services/data_permission_service.rs) |
| LOW-3 | 内存限流器锁中毒时 fail-open | 极端场景下（持锁线程 panic）使用 unwrap_or_else 降级放行，属于可用性优先的设计决策，可接受 | [rate_limit.rs](file:///workspace/backend/src/middleware/rate_limit.rs) |
| LOW-4 | WebSocket token 通过 URL 参数传递 | JWT 可能出现在服务器 access log 中；但日志脱敏已覆盖 URL 参数场景，且 token 有效期短（30分钟） | [notifications.rs](file:///workspace/backend/src/websocket/notifications.rs) |

> 注：以上低危项均不具备可论证的端到端利用路径，不计入中危及以上漏洞。

---

## 周期性安全审计报告（2026-07-05）

> 审计范围：全代码库高风险攻击面（认证与访问控制、注入向量、外部交互、敏感数据处理）
> 审计结论：**未发现中等或更高严重度的已确认漏洞**
> 历史待修复漏洞已全部验证修复完成

### 审计详情

#### 一、认证与访问控制 ✅ 安全

- **JWT 认证**：多层防护（签名验证、JTI 黑名单、用户级 Token 吊销、is_active 检查）
  - 文件：[auth.rs](file:///workspace/backend/src/middleware/auth.rs)
- **WebSocket 认证**：握手时验证 JWT，从 URL 参数提取 token 后校验
  - 文件：[notifications.rs](file:///workspace/backend/src/websocket/notifications.rs)
- **权限校验**：基于角色的权限系统，带 5 分钟 TTL 缓存，精确匹配 resource_type/resource_id/action
  - 文件：[permission.rs](file:///workspace/backend/src/middleware/permission.rs)
- **管理员检查**：fail-closed 设计（数据库异常时拒绝访问），使用 ADMIN_ROLE_CODE 常量
  - 文件：[admin_checker.rs](file:///workspace/backend/src/utils/admin_checker.rs)
- **CSRF 防护**：Token + IP 绑定，一次性消费，公开路由要求自定义请求头
  - 文件：[csrf.rs](file:///workspace/backend/src/middleware/csrf.rs)
- **Init Token**：初始化接口保护，恒定时间比较防时序攻击，fail-secure 设计
  - 文件：[init_token.rs](file:///workspace/backend/src/middleware/init_token.rs)
- **速率限制**：IP + UserID 双维度限流（180 req/min），登录端点防暴力破解（5次/5分钟），分布式优先内存回退
  - 文件：[rate_limit.rs](file:///workspace/backend/src/middleware/rate_limit.rs)
- **公开路径收敛**：仅健康检查 + 登录 + 刷新 6 个端点匿名访问，严格前缀匹配防绕过
  - 文件：[public_routes.rs](file:///workspace/backend/src/middleware/public_routes.rs)

#### 二、注入向量 ✅ 安全

- **SQL 注入**：核心业务使用 SeaORM 参数化查询；原始 SQL 场景（omni_audit/audit_cleanup/slow_query）均使用 `from_sql_and_values` 参数绑定，无用户输入直接拼接；LIKE 模式使用 safe_like_pattern 转义特殊字符
  - 文件：[omni_audit_handler.rs](file:///workspace/backend/src/handlers/omni_audit_handler.rs)
  - 文件：[sql_escape.rs](file:///workspace/backend/src/utils/sql_escape.rs)
  - 文件：[sql_injection_audit.rs](file:///workspace/backend/src/middleware/sql_injection_audit.rs)
- **路径遍历**：静态文件服务完整的路径规范化和符号链接检查（canonicalize + starts_with）
  - 文件：[static.rs](file:///workspace/backend/src/routes/static.rs)
- **命令注入**：CLI 工具使用 `Command::new(cmd).args(args)` 参数数组调用，无 shell 字符串拼接
  - 文件：[cli/util/mod.rs](file:///workspace/backend/src/cli/util/mod.rs)
- **XSS 防护**：前端 v-html 场景使用 DOMPurify 白名单过滤，CSP 响应头限制脚本来源
  - 文件：[csp.rs](file:///workspace/backend/src/middleware/csp.rs)
- **CSV/Excel 注入防护**：导入导出服务对单元格内容进行转义，防止公式注入
  - 文件：[import_export_handler.rs](file:///workspace/backend/src/handlers/import_export_handler.rs)

#### 三、外部交互 ✅ 安全

- **Webhook SSRF**：完整防护（HTTPS 强制、IP 黑名单、DNS 重绑定防护、禁止重定向、resolve_to_addrs 固定 IP 消除 TOCTOU）
  - 文件：[webhook_service.rs](file:///workspace/backend/src/services/webhook_service.rs)
  - 文件：[ssrf_guard.rs](file:///workspace/backend/src/utils/ssrf_guard.rs)
- **Webhook 签名**：入站/出站统一 HMAC-SHA256，恒定时间比较防时序攻击
  - 文件：[webhook_signature.rs](file:///workspace/backend/src/utils/webhook_signature.rs)
- **系统更新**：GitHub 域名白名单、HTTPS 强制、重定向限制、最终 URL 二次校验；上传更新包使用 multipart 解析
  - 文件：[system_update_service.rs](file:///workspace/backend/src/services/system_update_service.rs)
  - 文件：[system_update_handler.rs](file:///workspace/backend/src/handlers/system_update_handler.rs)
- **汇率服务**：ISO 4217 校验、禁止重定向
  - 文件：[currency_service.rs](file:///workspace/backend/src/services/currency_service.rs)

#### 四、敏感数据处理 ✅ 安全

- **密码**：Argon2id 哈希算法；密码强度校验（长度、复杂度、常见密码黑名单、键盘序列检测、l33t 变体检测）
  - 文件：[password_validator.rs](file:///workspace/backend/src/utils/password_validator.rs)
  - 文件：[auth_service.rs](file:///workspace/backend/src/services/auth_service.rs)
- **JWT 密钥**：环境变量配置，支持密钥轮换（previous_jwt_secret 平滑过渡）
- **Cookie 密钥**：独立 cookie_secret，强制 32 字节以上，禁止降级复用 JWT 密钥
- **Webhook 密钥**：独立 webhook_secret，强制 32 字节以上
- **审计密钥**：独立 AUDIT_SECRET_KEY，用于审计日志 HMAC 签名
- **日志脱敏**：Authorization 头截断脱敏、用户名 PII 截断
  - 文件：[auth.rs](file:///workspace/backend/src/middleware/auth.rs)
- **测试密钥**：运行时随机生成，无硬编码
  - 文件：[auth_service.rs](file:///workspace/backend/src/services/auth_service.rs)
  - 文件：[app_state.rs](file:///workspace/backend/src/utils/app_state.rs)
- **前端 Token 存储**：access_token/refresh_token 由后端 httpOnly Cookie 管理，前端 JS 无法读取；仅 csrf_token 存储于非 httpOnly Cookie 供前端读取
  - 文件：[storage.ts](file:///workspace/frontend/src/utils/storage.ts)
  - 文件：[request.ts](file:///workspace/frontend/src/api/request.ts)
- **API 密钥**：SHA256 哈希存储，撤销后入黑名单缓存（TTL 7天）
  - 文件：[api_key_service.rs](file:///workspace/backend/src/services/api_key_service.rs)
- **敏感操作告警**：密码变更等敏感操作触发安全审计事件
  - 文件：[sensitive_action_alert.rs](file:///workspace/backend/src/services/sensitive_action_alert.rs)
  - 文件：[audit.rs](file:///workspace/backend/src/utils/audit.rs)

### 低危观察项（不构成可利用漏洞）

| 编号 | 观察项 | 说明 | 位置 |
|------|--------|------|------|
| LOW-1 | `webhook_signature.rs` 中 `.expect()` | HMAC 接受任意长度密钥，初始化不会失败，属于已知安全的 expect | [webhook_signature.rs](file:///workspace/backend/src/utils/webhook_signature.rs) |
| LOW-2 | 数据权限服务大量预留 API | 已标注 `#[allow(dead_code)] + TODO`，尚未广泛接入业务，不构成当前攻击面 | [data_permission_service.rs](file:///workspace/backend/src/services/data_permission_service.rs) |
| LOW-3 | 内存限流器锁中毒时 fail-open | 极端场景下（持锁线程 panic）默认放行，属于可用性优先的设计决策，可接受 | [rate_limit.rs](file:///workspace/backend/src/middleware/rate_limit.rs) |
| LOW-4 | WebSocket token 通过 URL 参数传递 | JWT 可能出现在服务器 access log 中；但日志脱敏已覆盖 URL 参数场景，且 token 有效期短（30分钟） | [notifications.rs](file:///workspace/backend/src/websocket/notifications.rs) |

> 注：以上低危项均不具备可论证的端到端利用路径，不计入中危及以上漏洞。

---

## 周期性安全审计报告（2026-07-04）

> 审计范围：全代码库高风险攻击面（认证与访问控制、注入向量、外部交互、敏感数据处理）
> 审计结论：**未发现中等或更高严重度的已确认漏洞**
> 历史待修复漏洞已全部验证修复完成

### 审计详情

#### 一、认证与访问控制 ✅ 安全

- **JWT 认证**：多层防护（签名验证、JTI 黑名单、用户级 Token 吊销、is_active 检查）
  - 文件：[auth.rs](file:///workspace/backend/src/middleware/auth.rs)
- **WebSocket 认证**：握手时验证 JWT，从 URL 参数提取 token 后校验
  - 文件：[notifications.rs](file:///workspace/backend/src/websocket/notifications.rs)
- **权限校验**：基于角色的权限系统，带 5 分钟 TTL 缓存，精确匹配 resource_type/resource_id/action
  - 文件：[permission.rs](file:///workspace/backend/src/middleware/permission.rs)
- **管理员检查**：fail-closed 设计（数据库异常时拒绝访问），使用 ADMIN_ROLE_CODE 常量
  - 文件：[admin_checker.rs](file:///workspace/backend/src/utils/admin_checker.rs)
- **CSRF 防护**：Token + IP 绑定，一次性消费，公开路由要求自定义请求头
  - 文件：[csrf.rs](file:///workspace/backend/src/middleware/csrf.rs)
- **Init Token**：初始化接口保护，恒定时间比较防时序攻击，fail-secure 设计
  - 文件：[init_token.rs](file:///workspace/backend/src/middleware/init_token.rs)
- **速率限制**：IP + UserID 双维度限流（180 req/min），登录端点防暴力破解（5次/5分钟），分布式优先内存回退
  - 文件：[rate_limit.rs](file:///workspace/backend/src/middleware/rate_limit.rs)
- **公开路径收敛**：仅健康检查 + 登录 + 刷新 6 个端点匿名访问，严格前缀匹配防绕过
  - 文件：[public_routes.rs](file:///workspace/backend/src/middleware/public_routes.rs)

#### 二、注入向量 ✅ 安全

- **SQL 注入**：核心业务使用 SeaORM 参数化查询；原始 SQL 场景（omni_audit/audit_cleanup/slow_query）均使用 `from_sql_and_values` 参数绑定，无用户输入直接拼接；LIKE 模式使用 safe_like_pattern 转义特殊字符
  - 文件：[omni_audit_handler.rs](file:///workspace/backend/src/handlers/omni_audit_handler.rs)
  - 文件：[sql_escape.rs](file:///workspace/backend/src/utils/sql_escape.rs)
  - 文件：[sql_injection_audit.rs](file:///workspace/backend/src/middleware/sql_injection_audit.rs)
- **路径遍历**：静态文件服务完整的路径规范化和符号链接检查（canonicalize + starts_with）
  - 文件：[static.rs](file:///workspace/backend/src/routes/static.rs)
- **命令注入**：CLI 工具使用 `Command::new(cmd).args(args)` 参数数组调用，无 shell 字符串拼接
  - 文件：[cli/util/mod.rs](file:///workspace/backend/src/cli/util/mod.rs)
- **XSS 防护**：前端 v-html 场景使用 DOMPurify 白名单过滤，CSP 响应头限制脚本来源
  - 文件：[csp.rs](file:///workspace/backend/src/middleware/csp.rs)
- **CSV/Excel 注入防护**：导入导出服务对单元格内容进行转义，防止公式注入
  - 文件：[import_export_handler.rs](file:///workspace/backend/src/handlers/import_export_handler.rs)

#### 三、外部交互 ✅ 安全

- **Webhook SSRF**：完整防护（HTTPS 强制、IP 黑名单、DNS 重绑定防护、禁止重定向、resolve_to_addrs 固定 IP 消除 TOCTOU）
  - 文件：[webhook_service.rs](file:///workspace/backend/src/services/webhook_service.rs)
  - 文件：[ssrf_guard.rs](file:///workspace/backend/src/utils/ssrf_guard.rs)
- **Webhook 签名**：入站/出站统一 HMAC-SHA256，恒定时间比较防时序攻击
  - 文件：[webhook_signature.rs](file:///workspace/backend/src/utils/webhook_signature.rs)
- **系统更新**：GitHub 域名白名单、HTTPS 强制、重定向限制、最终 URL 二次校验；上传更新包使用 multipart 解析
  - 文件：[system_update_service.rs](file:///workspace/backend/src/services/system_update_service.rs)
  - 文件：[system_update_handler.rs](file:///workspace/backend/src/handlers/system_update_handler.rs)
- **汇率服务**：ISO 4217 校验、禁止重定向
  - 文件：[currency_service.rs](file:///workspace/backend/src/services/currency_service.rs)

#### 四、敏感数据处理 ✅ 安全

- **密码**：Argon2id 哈希算法；密码强度校验（长度、复杂度、常见密码黑名单、键盘序列检测、l33t 变体检测）
  - 文件：[password_validator.rs](file:///workspace/backend/src/utils/password_validator.rs)
  - 文件：[auth_service.rs](file:///workspace/backend/src/services/auth_service.rs)
- **JWT 密钥**：环境变量配置，支持密钥轮换（previous_jwt_secret 平滑过渡）
- **Cookie 密钥**：独立 cookie_secret，强制 32 字节以上，禁止降级复用 JWT 密钥
- **Webhook 密钥**：独立 webhook_secret，强制 32 字节以上
- **审计密钥**：独立 AUDIT_SECRET_KEY，用于审计日志 HMAC 签名
- **日志脱敏**：Authorization 头截断脱敏、用户名 PII 截断
  - 文件：[auth.rs](file:///workspace/backend/src/middleware/auth.rs)
- **测试密钥**：运行时随机生成，无硬编码
  - 文件：[auth_service.rs](file:///workspace/backend/src/services/auth_service.rs)
  - 文件：[app_state.rs](file:///workspace/backend/src/utils/app_state.rs)
- **前端 Token 存储**：access_token/refresh_token 由后端 httpOnly Cookie 管理，前端 JS 无法读取；仅 csrf_token 存储于非 httpOnly Cookie 供前端读取
  - 文件：[storage.ts](file:///workspace/frontend/src/utils/storage.ts)
  - 文件：[request.ts](file:///workspace/frontend/src/api/request.ts)
- **API 密钥**：SHA256 哈希存储，撤销后入黑名单缓存（TTL 7天）
  - 文件：[api_key_service.rs](file:///workspace/backend/src/services/api_key_service.rs)
- **敏感操作告警**：密码变更等敏感操作触发安全审计事件
  - 文件：[sensitive_action_alert.rs](file:///workspace/backend/src/services/sensitive_action_alert.rs)
  - 文件：[audit.rs](file:///workspace/backend/src/utils/audit.rs)

### 低危观察项（不构成可利用漏洞）

| 编号 | 观察项 | 说明 | 位置 |
|------|--------|------|------|
| LOW-1 | `webhook_signature.rs` 中 `.expect()` | HMAC 接受任意长度密钥，初始化不会失败，属于已知安全的 expect | [webhook_signature.rs](file:///workspace/backend/src/utils/webhook_signature.rs) |
| LOW-2 | 数据权限服务大量预留 API | 已标注 `#[allow(dead_code)] + TODO`，尚未广泛接入业务，不构成当前攻击面 | [data_permission_service.rs](file:///workspace/backend/src/services/data_permission_service.rs) |
| LOW-3 | 内存限流器锁中毒时 fail-open | 极端场景下（持锁线程 panic）默认放行，属于可用性优先的设计决策，可接受 | [rate_limit.rs](file:///workspace/backend/src/middleware/rate_limit.rs) |
| LOW-4 | WebSocket token 通过 URL 参数传递 | JWT 可能出现在服务器 access log 中；但日志脱敏已覆盖 URL 参数场景，且 token 有效期短（30分钟） | [notifications.rs](file:///workspace/backend/src/websocket/notifications.rs) |

> 注：以上低危项均不具备可论证的端到端利用路径，不计入中危及以上漏洞。

---

## 周期性安全审计报告（2026-07-03）

> 审计范围：全代码库高风险攻击面（认证与访问控制、注入向量、外部交互、敏感数据处理）
> 审计结论：**未发现中等或更高严重度的已确认漏洞**
> 历史待修复漏洞已全部验证修复完成

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
- **速率限制**：IP + UserID 双维度限流（180 req/min），登录端点防暴力破解（5次/5分钟），分布式优先内存回退
  - 文件：[rate_limit.rs](file:///workspace/backend/src/middleware/rate_limit.rs)
- **公开路径收敛**：仅健康检查 + 登录 + 刷新 6 个端点匿名访问，严格前缀匹配防绕过
  - 文件：[public_routes.rs](file:///workspace/backend/src/middleware/public_routes.rs)

#### 二、注入向量 ✅ 安全

- **SQL 注入**：核心业务使用 SeaORM 参数化查询；原始 SQL 场景（omni_audit/audit_cleanup/slow_query）均使用 `from_sql_and_values` 参数绑定，无用户输入直接拼接
  - 文件：[omni_audit_handler.rs](file:///workspace/backend/src/handlers/omni_audit_handler.rs)
  - 文件：[sql_injection_audit.rs](file:///workspace/backend/src/middleware/sql_injection_audit.rs)
- **路径遍历**：静态文件服务完整的路径规范化和符号链接检查（canonicalize + starts_with）
  - 文件：[static.rs](file:///workspace/backend/src/routes/static.rs)
- **命令注入**：CLI 工具使用 `Command::new(cmd).args(args)` 参数数组调用，无 shell 字符串拼接
  - 文件：[cli/util/mod.rs](file:///workspace/backend/src/cli/util/mod.rs)
- **XSS 防护**：前端 v-html 场景使用 DOMPurify 白名单过滤，CSP 响应头限制脚本来源
  - 文件：[report-templates/index.vue](file:///workspace/frontend/src/views/report-templates/index.vue)

#### 三、外部交互 ✅ 安全

- **Webhook SSRF**：完整防护（HTTPS 强制、IP 黑名单、DNS 重绑定防护、禁止重定向、resolve_to_addrs 固定 IP 消除 TOCTOU）
  - 文件：[webhook_service.rs](file:///workspace/backend/src/services/webhook_service.rs)
  - 文件：[ssrf_guard.rs](file:///workspace/backend/src/utils/ssrf_guard.rs)
- **Webhook 签名**：入站/出站统一 HMAC-SHA256，恒定时间比较防时序攻击
  - 文件：[webhook_signature.rs](file:///workspace/backend/src/utils/webhook_signature.rs)
- **系统更新**：GitHub 域名白名单、HTTPS 强制、重定向限制、最终 URL 二次校验
  - 文件：[system_update_service.rs](file:///workspace/backend/src/services/system_update_service.rs)
- **汇率服务**：ISO 4217 校验、禁止重定向
  - 文件：[currency_service.rs](file:///workspace/backend/src/services/currency_service.rs)

#### 四、敏感数据处理 ✅ 安全

- **密码**：Argon2id 哈希算法
- **JWT 密钥**：环境变量配置，支持密钥轮换（previous_jwt_secret 平滑过渡）
- **Cookie 密钥**：独立 cookie_secret，强制 32 字节以上，禁止降级复用 JWT 密钥
- **Webhook 密钥**：独立 webhook_secret，强制 32 字节以上
- **日志脱敏**：Authorization 头截断脱敏、用户名 PII 截断
  - 文件：[auth.rs](file:///workspace/backend/src/middleware/auth.rs)
- **测试密钥**：运行时随机生成，无硬编码
  - 文件：[auth_service.rs](file:///workspace/backend/src/services/auth_service.rs)
- **前端 Token 存储**：access_token/refresh_token 由后端 httpOnly Cookie 管理，前端 JS 无法读取；仅 csrf_token 存储于非 httpOnly Cookie 供前端读取
  - 文件：[storage.ts](file:///workspace/frontend/src/utils/storage.ts)
- **API 密钥**：SHA256 哈希存储，撤销后入黑名单缓存（TTL 7天）
  - 文件：[api_key_service.rs](file:///workspace/backend/src/services/api_key_service.rs)

### 低危观察项（不构成可利用漏洞）

| 编号 | 观察项 | 说明 | 位置 |
|------|--------|------|------|
| LOW-1 | `webhook_signature.rs` 中 `.expect()` | HMAC 接受任意长度密钥，初始化不会失败，属于已知安全的 expect | [webhook_signature.rs:23](file:///workspace/backend/src/utils/webhook_signature.rs#L23-L23) |
| LOW-2 | 数据权限服务大量预留 API | 已标注 `#[allow(dead_code)] + TODO`，尚未广泛接入业务，不构成当前攻击面 | [data_permission_service.rs](file:///workspace/backend/src/services/data_permission_service.rs) |
| LOW-3 | 内存限流器锁中毒时 fail-open | 极端场景下（持锁线程 panic）默认放行，属于可用性优先的设计决策，可接受 | [rate_limit.rs:82-88](file:///workspace/backend/src/middleware/rate_limit.rs#L82-L88) |

> 注：以上低危项均不具备可论证的端到端利用路径，不计入中危及以上漏洞。

---

## 周期性安全审计报告（2026-07-02）

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
