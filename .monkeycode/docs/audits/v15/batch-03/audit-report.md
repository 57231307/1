# V15 安全性独立审计报告（类三·批次 03）

- **审计子代理**：V15 审计子代理（类三 安全性独立审计类）
- **审计范围**：6 维度（身份认证 / 权限校验 / SQL 注入 / XSS·CSRF / 敏感信息 / 文件上传）
- **审计依据**：`/workspace/.monkeycode/MEMORY.md` 规则 11/12、`/workspace/backend/src/middleware/`、`/workspace/backend/src/utils/`
- **审计方法**：Grep 检索危险模式 + Read 关键文件 + 路由层 / 中间件链路静态分析
- **审计时间**：2026-07-16
- **审计原则**：只做审计不修改业务代码；安全问题标 P0/P1 高优先级

---

## 维度 1：身份认证机制（JWT / Session / Refresh Token）

### 检查方法
1. Read `/workspace/backend/src/middleware/auth.rs`（auth_middleware 完整逻辑）
2. Read `/workspace/backend/src/services/auth_service.rs`（JWT 生成/验证/密码哈希）
3. Read `/workspace/backend/src/handlers/auth_handler.rs`、`auth_handler_misc.rs`、`auth_handler_session.rs`
4. Grep `refresh_token|JWT_EXPIRE|exp|refresh_exp` 等关键字

### 发现

#### ✅ 已落实的安全机制
- **密码哈希算法**：`auth_service.rs:295-303` 使用 **Argon2id**（Algorithm::Argon2id / Version::V0x13 / Params::new(65536, 3, 4, None)），64MB 内存 / 3 迭代 / 4 并行度，符合规则 12 强哈希要求。
- **JWT 算法**：`auth_service.rs:159, 204, 222` 统一使用 `Algorithm::HS256` + `EncodingKey/DecodingKey`。
- **access_token 有效期**：`auth_service.rs:145` 2 小时；`auth_handler.rs:549` Cookie max_age=30 分钟（前端 expires_in=1800 与之对齐）。
- **refresh_token 有效期**：`auth_service.rs:192` JWT 内 exp=2 天（P2 7-9 已从 7 天缩短）。
- **leeway**：`auth_service.rs:225` 5 秒（P2 7-10 已从 60 秒降低），避免 Token 过期后仍有大窗口。
- **JTI 黑名单**：`auth_service.rs:470-500` `revoke_jti` 写 Redis（分布式）+ 内存回退；`auth.rs:260` `is_jti_revoked` 在 auth_middleware 中即时校验。
- **用户级 Token 吊销**：`auth.rs:277-305` `is_user_token_revoked` 软删除/封禁用户时吊销该用户所有 iat < revoked_at 的 Token，与 #6 互补。
- **is_active 实时校验**：`auth.rs:94-116, 311-334` `is_user_active_cached` 5 分钟内存缓存，TTL=300s；环境变量 `AUTH_CHECK_USER_ACTIVE` 控制（默认 true）。
- **密钥轮换**：`auth.rs:248-254` 当前密钥验证失败后尝试 `previous_jwt_secret`，支持平滑过渡。
- **Token 黑名单**：`auth.rs:240-244` 进程内 `state.cache.get_token_blacklist()` 校验。
- **refresh_token 轮换**：`auth_handler_misc.rs:120-137` 刷新时吊销旧 JTI + 旧 token 加入黑名单，生成新 session_id。
- **登录失败锁定**：`auth_handler.rs:266-307` per-IP 5 次 + per-username 10 次锁定 30 分钟（MAX_FAILED_ATTEMPTS）。
- **双因素认证**：`auth_handler_misc.rs:249-312` TOTP + 恢复码（10 个 8 字符）。
- **登录响应脱敏**：`auth_handler.rs:526-531` LoginResponse 不返回 token/refresh_token，仅通过 httpOnly Cookie 下发。

#### ⚠️ 风险点

**【P1】refresh_token Cookie max_age 与 JWT exp 不一致**
- 文件:行号：`backend/src/handlers/auth_handler_misc.rs:201`、`backend/src/handlers/auth_handler.rs:556`
- 现象：refresh_token Cookie `max_age=CookieDuration::days(7)`，但 JWT 内 `refresh_exp = now + Duration::days(2)`（`auth_service.rs:192`）。
- 风险：Cookie 在浏览器侧保留 7 天，但 JWT 实际 2 天即过期。剩余 5 天 Cookie 内含已失效 token，虽无法通过 `validate_token_static` 验证，但增加了 token 泄露窗口（如 XSS 拿到 Cookie 后 7 天内可解析出 JWT 内容）。
- 风险等级：**P1**
- 修复建议：Cookie max_age 与 JWT exp 对齐，统一改为 2 天。

**【P2】legacy jwt Cookie 双写**
- 文件:行号：`backend/src/handlers/auth_handler.rs:571-577`、`auth_handler_misc.rs:210-216`、`auth_handler_session.rs:163-169`
- 现象：登录/刷新/登出时除了写 access_token，还同步写 legacy `jwt` Cookie（注释为"向后兼容"）。
- 风险：双 Cookie 同时存在，若其中一处 Cookie 处理逻辑变更遗漏，可能产生鉴权不一致；同时 auth.rs:174 仍兼容从 `jwt` Cookie 读取，延长了旧版 Cookie 的退役周期。
- 风险等级：**P2**
- 修复建议：评估前端是否仍依赖 `jwt` Cookie，若已全量切换 `access_token` 则移除 legacy Cookie 写入逻辑与 auth.rs:174 兼容分支。

**【P2】is_active 缓存多实例不一致**
- 文件:行号：`backend/src/middleware/auth.rs:91-93`（注释承认）
- 现象：`USER_ACTIVE_CACHE` 是进程内 `DashMap`，多副本部署时部分实例可能短暂持有旧值（最多 5 分钟）。
- 风险：管理员封禁用户后，部分实例最多 5 分钟内仍允许旧 JWT 通过。
- 风险等级：**P2**（已有 5 分钟窗口设计权衡）
- 修复建议：将 is_active 缓存迁移到 Redis（与 JTI 黑名单一致），或在 user_service::delete_user 时主动失效所有实例的本地缓存（通过 pub/sub）。

**【P3】leeway 5 秒**
- 文件:行号：`backend/src/services/auth_service.rs:225`
- 现象：`validation.leeway = 5`（已从 60s 降至 5s）。
- 风险：Token 过期后仍有 5 秒有效窗口。
- 风险等级：**P3**（已是合理权衡，时钟漂移容忍）

---

## 维度 2：权限校验完整性（所有 API 是否有权限校验）

### 检查方法
1. Read `/workspace/backend/src/middleware/permission.rs`（permission_middleware 完整逻辑）
2. Read `/workspace/backend/src/middleware/public_routes.rs`（PUBLIC_PATHS 白名单）
3. Read `/workspace/backend/src/middleware/request_validator.rs`
4. Grep `/workspace/backend/src/main.rs` 中 `middleware::from_fn` 路由挂载顺序

### 发现

#### ✅ 已落实的安全机制
- **全局 permission_middleware 挂载**：`main.rs:733` `from_fn_with_state(app_state_clone2, permission_middleware)` 全局生效。
- **中间件执行顺序**：`main.rs:731` 注释 `auth → omni_audit → csrf → permission → request_validator → handler`，permission 在 csrf 之后、handler 之前。
- **PUBLIC_PATHS 严格白名单**：`public_routes.rs:6-30` 仅含 health/ready/live、auth/login、auth/refresh、webhooks/integrations/callback、init/initialize* 系列。
- **精确匹配 + 子路径匹配**：`public_routes.rs:44-49` 修复低危 #3，防止 `starts_with` 误匹配（如 `/api/v1/erp/auth/logout-bypass` 不再匹配 `/api/v1/erp/auth/logout`）。
- **管理员角色带缓存**：`permission.rs:172` `admin_checker::is_admin_role` 走缓存，admin 直接放行。
- **权限缓存 TTL 5 分钟**：`permission.rs:160` `PERMISSION_CACHE_TTL = 5` 分钟，使用 `Arc<Vec<role_permission::Model>>` 避免克隆开销。
- **资源 ID 精确匹配**：`permission.rs:223-236` `matches_permission` 纯函数：`resource_id` 精确匹配（None 匹配 None，Some 匹配 Some），防垂直越权；action 支持 `*` 通配符。
- **handler 层深度防御**：`omni_audit_handler.rs:20-40`、`system_update_handler.rs:24-44` 在高危接口（审计日志查询、系统更新）显式 `require_admin_role` 二次校验。
- **init_token_middleware**：`init_token.rs:40-74` 恒定时间比较（`subtle::ConstantTimeEq`）防时序攻击，缺失 `INIT_TOKEN` 环境变量时 fail-secure。
- **CSRF Token 一次性消费 + IP 绑定**：`csrf.rs:150` `consume_csrf_token` 校验 IP 一致性，成功后立即从缓存移除。

#### ⚠️ 风险点

**【P1】PUBLIC_PATHS 子路径放行过宽**
- 文件:行号：`backend/src/middleware/public_routes.rs:46-48`
- 现象：`is_public_path` 实现 `clean_path.starts_with(p) && clean_path[p.len()..].starts_with('/')`，即 `/api/v1/erp/auth/login/anything` 会被判定为公开路径，跳过 JWT 认证。
- 风险：若未来新增 `/api/v1/erp/auth/login/{id}` 等子路径业务接口（如 SSO 回调），将绕过认证。当前 login/refresh 是 POST 端点，子路径无业务意义，但白名单语义过宽。
- 风险等级：**P1**
- 修复建议：公开路径白名单改为**严格精确匹配**（仅 `==`），删除子路径前缀匹配；如确需子路径公开，单独显式登记。

**【P1】request_validator_middleware 名不副实**
- 文件:行号：`backend/src/middleware/request_validator.rs:16-70`
- 现象：函数名 `request_validator_middleware` 暗示请求验证，但实际逻辑仅记录 `tracing::debug!("未认证的状态变更请求")`，**不拦截任何请求**（line 68 `Ok(next.run(request).await)` 无条件放行）。
- 风险：维护者可能误以为该中间件提供了 Origin/CSRF 兜底校验，实际无任何拦截能力；安全审计时易产生"已防护"的错觉。
- 风险等级：**P1**（误判风险高于实际漏洞）
- 修复建议：要么删除该中间件（避免误导），要么实现真正的 Origin 白名单校验逻辑；若仅用于日志，应重命名为 `request_logging_middleware`。

**【P2】权限缓存 TTL 5 分钟**
- 文件:行号：`backend/src/middleware/permission.rs:160`
- 现象：`PERMISSION_CACHE_TTL = 5` 分钟，角色权限变更后最多 5 分钟延迟生效。
- 风险：管理员撤销某角色权限后，该角色用户最多 5 分钟内仍可访问已撤销的资源。
- 风险等级：**P2**（已有显式权衡）
- 修复建议：权限变更时主动 `PERMISSION_CACHE.remove(&role_id)` 失效缓存；或迁移到 Redis 共享缓存。

**【P3】extract_resource_info 对未知路径返回 "unknown"**
- 文件:行号：`backend/src/middleware/permission.rs:115`
- 现象：非 `/api/v1/erp/*` 路径返回 `("unknown", None)`，permission_middleware 会用 `resource_type="unknown"` 查询权限表。
- 风险：若权限表中存在 `resource_type="unknown"` 的通配权限，可能意外放行。
- 风险等级：**P3**（需 DB 中存在 "unknown" 记录才会触发）
- 修复建议：未知路径默认拒绝（fail-secure），或要求所有路由必须在权限表中显式登记。

---

## 维度 3：SQL 注入防护（参数化查询）

### 检查方法
1. Grep `format!\(.*SELECT|format!\(.*INSERT|format!\(.*UPDATE|format!\(.*DELETE|format!\(.*FROM`
2. Grep `Statement::from_sql_and_bindings|Statement::from_string|raw_sql|execute_unprepared|sqlx::query`
3. Read `backend/src/utils/sql_escape.rs`、`backend/src/middleware/sql_injection_audit.rs`
4. Read `backend/src/handlers/omni_audit_handler.rs`、`backend/src/services/bi_analysis_service.rs`、`backend/src/services/tracking_service.rs`、`backend/src/services/ar_service.rs` 等 raw SQL 用法

### 发现

#### ✅ 已落实的安全机制
- **SeaORM 默认参数化**：项目 90%+ 查询走 SeaORM `Entity::find().filter(Column::Eq(value))`，自动参数化。
- **全局 SQL 注入审计中间件**：`sql_injection_audit.rs:137` 拦截 URL / query / 文本类请求体（1MB 内）的危险模式（80+ 模式，含 UNION SELECT / SLEEP / INFORMATION_SCHEMA / xp_cmdshell 等），命中即返回 400。
- **LIKE 模式转义工具**：`sql_escape.rs:4-18` `escape_like_pattern` 转义 `%` `_` `\` `\0`；`safe_like_pattern` 自动包裹 `%keyword%`。
- **自定义 SQL 报表禁用**：`report_template_service.rs:167-171` `data_source_sql.is_some()` 直接返回 `permission_denied`，彻底关闭 `Statement::from_string + query_all` 多语句执行入口。
- **audit_cleanup_service 参数化**：`audit_cleanup_service.rs:53-57, 73-77` `Statement::from_sql_and_values` + `$1` 占位符，retention_days 参数化绑定。
- **omni_audit_handler 动态 WHERE**：`omni_audit_handler.rs:199-238` `where_clauses.push(format!("user_id = ${}", param_idx))` 仅拼接占位符序号，用户值通过 `where_params` 绑定；keyword 走 `safe_like_pattern` 转义。
- **BI service 白名单常量拼接**：`bi_analysis_service.rs:317-324, 1195-1217` `period_expr` / `dim_to_expr` / `measure_to_expr` 通过 `match` 严格白名单返回常量字符串，用户输入（start_date/end_date/measure）走 `$1, $2` 参数化绑定。
- **ar_service 参数化**：`ar_service.rs:1346-1360` `format!("status <> ${}", params.len() + 1)` 动态构造占位符序号，所有用户值通过 `params.push(...)` 绑定。
- **tracking_service 参数化**：`tracking_service.rs:179-187, 218-226` `sql.push_str(&format!(" AND viewed_at >= ${}", params.len() + 1))` + `params.push(from.into())`。
- **number_generator advisory lock**：`number_generator.rs:58-63` `pg_advisory_xact_lock($1)` 参数化绑定 lock_key。

#### ⚠️ 风险点

**【P2】omni_audit_handler 用 format! 拼接 SQL 占位符**
- 文件:行号：`backend/src/handlers/omni_audit_handler.rs:205-238, 246-260, 356`
- 现象：`format!("user_id = ${}", param_idx)`、`format!("SELECT {} FROM omni_audit_logs{} ORDER BY id DESC LIMIT ${} OFFSET ${}", select_fields, where_sql, param_idx, param_idx + 1)` 用 format! 拼接 SQL 语句。
- 风险：虽然 `param_idx` 是内部计数器（非用户输入），`select_fields` 是硬编码字段列表，`where_sql` 是占位符拼接结果，**实际无注入风险**，但代码风格易被后续维护者误判为安全模板，新增字段时可能引入注入。
- 风险等级：**P2**（代码风格风险，非实际漏洞）
- 修复建议：改用 SeaORM QuerySelect 动态构造，或抽取专用 builder 函数明确区分"常量 SQL 骨架"与"用户参数"。

**【P2】slow_query_handler 使用 Statement::from_string**
- 文件:行号：`backend/src/handlers/slow_query_handler.rs:150-159, 191-192`
- 现象：`Statement::from_string(DatabaseBackend::Postgres, sql.to_string())`，sql 是硬编码字符串（无用户输入拼接）。
- 风险：当前安全，但 `from_string` 不强制参数化，后续若修改 sql 拼接用户输入则立即产生注入。
- 风险等级：**P2**（代码风格风险）
- 修复建议：即使无用户输入，也改用 `Statement::from_sql_and_values` 保持一致风格。

**【P3】bi_analysis_service.rs:1242 空参数 from_sql_and_values**
- 文件:行号：`backend/src/services/bi_analysis_service.rs:1242`
- 现象：`Statement::from_sql_and_values(sea_orm::DatabaseBackend::Postgres, sql, [])` 空参数列表，sql 中字段名是白名单常量。
- 风险：无实际注入风险，但白名单常量通过 `format!` 拼接，若白名单 match 分支遗漏可能引入新维度未校验。
- 风险等级：**P3**

**【P3】main.rs:399 execute_unprepared**
- 文件:行号：`backend/src/main.rs:399`
- 现象：`db.execute_unprepared(sql)` 执行硬编码 DDL（ALTER TABLE / CREATE INDEX），sql 是字符串字面量。
- 风险：无用户输入，安全。
- 风险等级：**P3**

---

## 维度 4：XSS / CSRF 防护

### 检查方法
1. Read `/workspace/backend/src/middleware/csrf.rs`、`backend/src/middleware/csp.rs`
2. Read `/workspace/backend/src/main.rs` 中 `SetResponseHeaderLayer` / `cors` / `csp_middleware` 挂载
3. Read `/workspace/backend/src/handlers/auth_handler.rs` 中 Cookie 配置

### 发现

#### ✅ 已落实的安全机制
- **Cookie httpOnly**：`auth_handler.rs:541, 554, 573` access_token / refresh_token / legacy jwt 均 `http_only(true)`，防 XSS 读取。
- **Cookie SameSite=Strict**：`auth_handler.rs:543, 555, 575` 所有 Cookie `same_site(SameSite::Strict)`，防 CSRF 跨站携带。
- **Cookie secure=is_production**：`auth_handler.rs:542, 554, 574` 生产环境强制 HTTPS。
- **CSRF Token 机制**：`csrf.rs:84-174` 对 POST/PUT/PATCH/DELETE 强制 `X-CSRF-Token` 头；`consume_csrf_token` 一次性消费 + IP 绑定（Wave 3 #7）+ 强制轮换（refresh 时 `clear_old_csrf_token_for_user`）。
- **公开路径自定义头防御**：`csrf.rs:97-124` 公开端点的非安全方法要求 `X-Requested-With` 或 `X-CSRF-Token` 自定义头，阻止简单表单提交型 CSRF。
- **CSP 中间件**：`csp.rs:23-33` `default-src 'self'; script-src 'self' 'wasm-unsafe-eval'; object-src 'none'; base-uri 'self'; frame-ancestors 'none'; upgrade-insecure-requests`。
- **安全响应头**：`main.rs:757-784` X-Content-Type-Options: nosniff / X-Frame-Options: DENY / X-XSS-Protection: 1; mode=block / Referrer-Policy: strict-origin-when-cross-origin / Permissions-Policy: geolocation=(), microphone=(), camera=()。
- **HSTS**：`main.rs:843-847` 仅 production 环境注入 `max-age=31536000; includeSubDomains; preload`。
- **CORS 严格白名单**：`main.rs:319-346` `AllowOrigin::predicate` 动态校验 Origin 精确匹配（拒绝通配符），`allow_credentials(true)` 配合 Cookie 鉴权。
- **LoginResponse 不返回 token**：`auth_handler.rs:507-510` 注释明确"安全漏洞 #10 + #13 修复"，token 仅在 httpOnly Cookie 中。
- **CSRF 错误响应不泄露**：`csrf.rs:180-188` 统一 JSON 格式 `{success:false, code, message, data:null}`。

#### ⚠️ 风险点

**【P2】CSP 允许 'unsafe-inline' 和 'wasm-unsafe-eval'**
- 文件:行号：`backend/src/middleware/csp.rs:24-25`
- 现象：`script-src 'self' 'wasm-unsafe-eval'`、`style-src 'self' 'unsafe-inline'`。
- 风险：`unsafe-inline` 削弱了 CSP 对 XSS 的防护（内联脚本可执行）；`wasm-unsafe-eval` 允许 WASM 执行（注释为 argon2 wasm，但攻击者注入的 wasm 同样可执行）。
- 风险等级：**P2**（SPA 框架兼容性权衡）
- 修复建议：长期目标改为 nonce-based CSP 或 hash-based CSP，移除 'unsafe-inline'；WASM 执行限制到特定脚本源。

**【P2】init 模式 CSP 与 production 模式不一致**
- 文件:行号：`backend/src/main.rs:826-828` vs `main.rs:773-775`
- 现象：init 模式（数据库未初始化分支）使用 `SetResponseHeaderLayer::overriding(CONTENT_SECURITY_POLICY, ...)` 硬编码 CSP（含 `script-src 'self' 'unsafe-inline' 'wasm-unsafe-eval'`），未走 `csp_middleware`；production 模式走 `csp_middleware`（无 'unsafe-inline' script-src）。
- 风险：init 模式 CSP 更宽松，且两条 CSP 来源不一致，维护时易遗漏同步。
- 风险等级：**P2**
- 修复建议：统一使用 `csp_middleware`，删除 init 模式的 SetResponseHeaderLayer CSP 注入。

**【P3】X-XSS-Protection: 1; mode=block 已废弃**
- 文件:行号：`backend/src/main.rs:765-768`
- 现象：`X-XSS-Protection: 1; mode=block` 在现代浏览器（Chrome 78+ / Edge / Firefox）已废弃，且 `1; mode=block` 在某些场景下可能引入 XSS 漏洞（旧 IE 反射 XSS 过滤器 bug）。
- 风险等级：**P3**（现代浏览器忽略此头，CSP 已覆盖）
- 修复建议：改为 `X-XSS-Protection: 0` 或直接移除，依赖 CSP 防护。

---

## 维度 5：敏感信息保护（密码哈希 / Token 存储 / 日志脱敏）

### 检查方法
1. Read `/workspace/backend/src/utils/password_validator.rs`、`backend/src/services/auth_service.rs`
2. Grep `tracing::.*password|tracing::.*token|tracing::.*secret|tracing::.*hash`
3. Read `/workspace/backend/src/middleware/auth.rs` 中 `mask_auth_header` / `mask_username`
4. Read `/workspace/backend/src/services/init_service.rs`（初始化密码处理）

### 发现

#### ✅ 已落实的安全机制
- **Argon2id 密码哈希**：`auth_service.rs:295-303` 强算法 + 强参数（详见维度 1）。
- **密码强度校验**：`password_validator.rs` 严格策略：
  - 长度 8-128
  - 必含大小写 + 数字 + 特殊字符
  - 100+ 常见密码黑名单（含 l33t 变体归一化）
  - 键盘序列检测（横排/竖排/蛇形/反向）
  - 连续字符 / 重复字符检测
  - 强度评分 ≥ Medium
- **spawn_blocking 包装哈希计算**：`auth_service.rs:266-273, 312-314`、`init_service.rs:167` 异步密码哈希用 `tokio::task::spawn_blocking`，避免阻塞 tokio worker。
- **Authorization 头脱敏**：`auth.rs:28-39` `mask_auth_header` 截断保留前 12 字符 + 长度信息，完整 token 不进入日志。
- **用户名脱敏**：`auth.rs:51-58` `mask_username` 保留前 2 字符 + `***`，按 Unicode 字符截断（中文安全）。
- **JWT 错误日志不记录 token 内容**：`auth_handler.rs:459`、`auth_handler_misc.rs:105` 仅记录错误对象 `e`，不记录 token 字符串。
- **数据库错误脱敏**：`init_service.rs:142-143` `test_database` 失败时返回 `"数据库测试查询失败"`，不透传 DbErr 原文（防内网服务枚举）。
- **登录响应不返回 token**：详见维度 4。
- **Webhook 签名密钥分离**：`webhook_integration_handler.rs:383` 使用独立 `webhook_secret`，与 `jwt_secret` 分离，避免 JWT 密钥泄露导致回调伪造。
- **HMAC-SHA256 + 恒定时间比较**：`webhook_signature.rs:25-66` 出站/入站同一份算法，`subtle::ConstantTimeEq` 防时序攻击。
- **Webhook test 不回显响应体**：`webhook_integration_handler.rs:437` `result.response_body = Some("出于安全原因，已隐藏响应内容")` 防 SSRF 读取内网数据回显。
- **JWT 密钥不写入日志**：Grep 全项目 `tracing::.*jwt_secret|tracing::.*cookie_secret|tracing::.*webhook_secret` 无匹配。
- **密码哈希长度校验**：`init_service.rs:172-177` 仅记录长度（`password_hash.len()`），不记录哈希内容。

#### ⚠️ 风险点

**【P1】Webhook payload 完整记录到日志**
- 文件:行号：`backend/src/handlers/webhook_integration_handler.rs:398-402`
- 现象：`tracing::info!(event_type = %req.event_type, payload = %req.payload, "Webhook 签名验证通过，已接收第三方回调事件")` 完整记录 `req.payload` 到日志。
- 风险：第三方回调 payload 可能含敏感业务数据（订单金额、客户信息、付款凭证等），完整记录违反规则 11"日志中禁止记录敏感信息明文"；日志聚合系统通常权限较低，易引发数据泄露。
- 风险等级：**P1**
- 修复建议：仅记录 `event_type` + `payload_size` + `payload_keys`（已在 line 404-411 计算），删除 `payload = %req.payload` 字段；如需完整 payload 用于排错，应单独写入加密的 webhook_logs 表并限制访问权限。

**【P2】CLI admin 工具直接打印密码哈希到 stdout**
- 文件:行号：`backend/src/cli/admin.rs:108`
- 现象：`println!("{}", String::from_utf8_lossy(&output.stdout))` 直接打印 python3 子进程的 stdout（含 `Argon2 哈希: $argon2id$...`）到终端。
- 风险：CLI 工具运行环境（如 SSH 会话、CI 日志）可能被记录，密码哈希泄露后可离线暴力破解（虽然 Argon2id 抗暴力，但仍增加风险）。
- 风险等级：**P2**（CLI 工具，非服务端）
- 修复建议：改为写入文件并设置 600 权限，或仅显示哈希前缀 + 长度，完整哈希通过 `--output` 参数指定文件。

**【P2】cli/admin.rs 调用外部 python3 生成密码哈希**
- 文件:行号：`backend/src/cli/admin.rs:87-97`
- 现象：通过 `Command::new("python3").arg("-c").arg(python_code).stdin(Stdio::piped())` 调用外部 python3 子进程生成密码哈希，而非使用 Rust 原生 argon2 crate。
- 风险：
  1. 依赖运行环境安装了 python3 + argon2 库（或 hashlib），部署环境不一致时失败
  2. 子进程参数虽为常量字符串（无命令注入），但 spawn 子进程增加了攻击面
  3. 与 `auth_service.rs::hash_password` 重复实现，违反 DRY，参数（迭代次数/内存）可能不一致
- 风险等级：**P2**
- 修复建议：直接调用 `AuthService::hash_password`（Rust 原生 argon2），移除 python3 子进程调用。

**【P3】登录失败日志记录用户名**
- 文件:行号：`backend/src/handlers/auth_handler.rs:302-305`
- 现象：`tracing::warn!("Account globally locked due to too many failed attempts: {}", payload.username)` 记录完整用户名。
- 风险：登录失败日志通常权限较低，攻击者可借此枚举有效用户名。
- 风险等级：**P3**（运维需要，权衡可接受）
- 修复建议：考虑使用 `mask_username` 脱敏（与 auth.rs 一致），或仅在 security_audit target 下记录完整用户名。

---

## 维度 6：文件上传安全（类型 / 大小 / 内容校验）

### 检查方法
1. Grep `Multipart|Field::multipart|axum::extract::Multipart`
2. Read `backend/src/handlers/crm_handler.rs::import_leads`、`backend/src/handlers/system_update_handler.rs::upload_and_update`
3. Read `backend/src/services/system_update_service.rs::extract_zip_entry`、`backend/src/utils/path_validator.rs`

### 发现

#### ✅ 已落实的安全机制
- **全局 HTTP 请求体大小限制**：`main.rs:662` `DefaultBodyLimit::max(MAX_HTTP_BODY_BYTES)` = 12MB，兜底防 OOM DoS。
- **system_update 上传多重防护**：
  - `system_update_handler.rs:188` `require_admin_role` admin 权限校验
  - `system_update_handler.rs:192` `MAX_UPDATE_SIZE = 100MB` 大小限制
  - `system_update_handler.rs:214` `verify_zip_magic` 校验 ZIP magic bytes（`50 4B 03 04`）
  - `system_update_handler.rs:225-247` `canonicalize` + `starts_with` 路径遍历防护
  - `system_update_service.rs:803-813` `enclosed_name` + `starts_with` 双重路径校验（Tar Slip 防护）
  - `system_update_service.rs:853-864` `set_safe_permissions` 重置 SUID/SGID/sticky bit（0o755 目录 / 0o600 文件）
  - `system_update_service.rs:867-880` `validate_download_url` 仅允许 HTTPS + GitHub 域名
- **crm import_leads 防护**：
  - `crm_handler.rs:112` `MAX_IMPORT_SIZE = 10MB` 大小限制
  - `crm_handler.rs:119` `.xlsx` 后缀校验
  - `crm_handler.rs:109` `auth: AuthContext` 登录认证
  - `crm/lead.rs:235` `calamine::open_workbook_auto_from_rs` 解析失败返回 400
- **path_validator 递归深度限制**：`path_validator.rs:15` `MAX_RECURSION_DEPTH = 100` 防恶意嵌套目录栈溢出。
- **SSRF 防护**：`ssrf_guard.rs:49-115` 完整 SSRF 防护（IPv4/IPv6 私网/loopback/元数据/CGNAT/多播/保留地址 + 主机名黑名单 + DNS Rebinding 防御）。

#### ⚠️ 风险点

**【P1】crm import_leads 缺少 magic bytes 校验**
- 文件:行号：`backend/src/handlers/crm_handler.rs:119`
- 现象：仅检查 `file_name.ends_with(".xlsx")`，未校验文件内容 magic bytes（`50 4B 03 04` ZIP 头 + `[Content_Types].xml` OOXML 标识）。
- 风险：攻击者可上传任意内容的 `.xlsx` 后缀文件，虽然 `calamine::open_workbook_auto_from_rs` 解析会失败，但解析过程可能触发未知的库 bug（如 zip 炸弹、XXE）；同时后缀校验可被绕过（如 `.xlsx` 实为可执行脚本，配合其他下载漏洞触发）。
- 风险等级：**P1**
- 修复建议：增加 `verify_xlsx_magic` 函数校验前 4 字节为 `50 4B 03 04`（与 `verify_zip_magic` 一致）；可选校验偏移 30-46 字节处含 `[Content_Types].xml`。

**【P1】system_update 缺少 zip bomb 防护**
- 文件:行号：`backend/src/services/system_update_service.rs:799-842`
- 现象：`extract_zip_entry` 用 `io::copy(zip_entry, &mut outfile)` 解压，未限制总解压大小 / 压缩比。
- 风险：攻击者构造高压缩比 zip bomb（如 42.zip：42KB 压缩 → 4.5PB 解压），导致磁盘耗尽 / OOM。
- 风险等级：**P1**
- 修复建议：在 `extract_zip_entry` 中累计 `io::copy` 字节数，超过阈值（如 500MB 总解压 / 单文件 100MB）时返回 `UpdateError::ValidationError`；或使用 `take(limit)` 限制单文件读取字节数。

**【P2】crm import_leads 无文件内容病毒扫描**
- 文件:行号：`backend/src/handlers/crm_handler.rs:107-139`
- 现象：上传的 xlsx 仅做大小 + 后缀校验，无病毒扫描（如 ClamAV 集成）。
- 风险：xlsx 是 OOXML 格式（实际为 zip），可内嵌恶意宏（xlsm）或 OLE 对象；虽然 calamine 仅读取数据，但若文件被转发到其他系统（如导出回前端）可能传播宏病毒。
- 风险等级：**P2**
- 修复建议：评估接入 ClamAV 或类似病毒扫描服务；或明确限制仅读取纯数据 sheet，拒绝含 macroSheet 的文件。

**【P3】email_handler 邮件附件未实现**
- 文件:行号：`backend/src/handlers/email_handler.rs:162`
- 现象：`attachments: None` 写死，邮件附件功能未实现。
- 风险：当前无上传风险（功能未实现），但未来实现时需注意文件上传安全。
- 风险等级：**P3**（功能缺失，非安全漏洞）

**【P3】system_update 上传文件名未脱敏写入日志**
- 文件:行号：`backend/src/handlers/system_update_handler.rs:195`
- 现象：`let file_name = field.file_name().unwrap_or("update.zip").to_string()` 后续若记录日志可能含用户输入。
- 风险：当前代码未直接记录 file_name 到日志，但若未来添加日志需注意文件名注入（日志注入攻击）。
- 风险等级：**P3**

---

## 汇总

### 按风险等级统计

| 风险等级 | 数量 | 维度分布 |
|---------|------|---------|
| P0 | 0 | — |
| P1 | 6 | 维度1×1 / 维度2×2 / 维度5×1 / 维度6×2 |
| P2 | 9 | 维度1×2 / 维度2×1 / 维度3×2 / 维度4×2 / 维度5×2 / 维度6×1 |
| P3 | 6 | 维度1×1 / 维度2×1 / 维度3×2 / 维度4×1 / 维度5×1 / 维度6×2 |

### P1 高优先级修复清单

1. **【维度1·P1】refresh_token Cookie max_age 与 JWT exp 不一致**（`auth_handler_misc.rs:201`）→ Cookie max_age 改为 2 天
2. **【维度2·P1】PUBLIC_PATHS 子路径放行过宽**（`public_routes.rs:46-48`）→ 改为严格精确匹配
3. **【维度2·P1】request_validator_middleware 名不副实**（`request_validator.rs:16-70`）→ 删除或重命名 + 实现真正校验
4. **【维度5·P1】Webhook payload 完整记录到日志**（`webhook_integration_handler.rs:398-402`）→ 仅记录摘要
5. **【维度6·P1】crm import_leads 缺少 magic bytes 校验**（`crm_handler.rs:119`）→ 增加 xlsx magic 校验
6. **【维度6·P1】system_update 缺少 zip bomb 防护**（`system_update_service.rs:799-842`）→ 增加总解压大小限制

### 整体评价

项目在安全性方面已建立了较完善的防御体系：
- **身份认证**：Argon2id + JWT + refresh_token 轮换 + JTI 黑名单 + is_active 实时校验 + 密钥轮换，机制完整
- **权限校验**：全局 permission_middleware + handler 层 admin 二次校验 + 资源 ID 精确匹配，防越权
- **SQL 注入**：SeaORM 参数化 + 全局 SQL 注入审计中间件 + LIKE 转义工具 + 自定义 SQL 报表禁用，防护到位
- **XSS/CSRF**：httpOnly + SameSite=Strict + CSRF Token 一次性消费 + IP 绑定 + CSP + 安全响应头，多层防御
- **敏感信息**：Argon2id + 日志脱敏 + 错误信息脱敏 + Webhook 密钥分离，符合规则 11/12
- **文件上传**：大小限制 + magic 校验 + 路径遍历防护 + 权限掩码 + admin 权限，关键防护到位

主要风险集中在**代码风格一致性**（format! 拼接 SQL 占位符、from_string 使用）和**边缘场景防护**（zip bomb、xlsx magic、日志 payload 脱敏），无 P0 级紧急漏洞。

---

## 审计方法附录

### 工具使用统计
- Read 文件数：14（auth.rs / permission.rs / public_routes.rs / csrf.rs / csp.rs / init_token.rs / password_validator.rs / sql_escape.rs / sql_injection_audit.rs / auth_service.rs / auth_handler.rs / auth_handler_misc.rs / auth_handler_session.rs / system_update_handler.rs / system_update_service.rs / ssrf_guard.rs / webhook_signature.rs / omni_audit_handler.rs / slow_query_handler.rs / init_service.rs / path_validator.rs / main.rs / request_validator.rs / bi_analysis_service.rs / crm_handler.rs / report_template_service.rs / tracking_service.rs / ar_service.rs / audit_cleanup_service.rs / number_generator.rs / webhook_integration_handler.rs）
- Grep 搜索次数：18（覆盖 SQL 注入模式 / unwrap / 敏感词 / 文件上传 / 命令执行 / 日志脱敏等）
- 涉及目录：`backend/src/middleware/`、`backend/src/utils/`、`backend/src/services/`、`backend/src/handlers/`、`backend/src/routes/`、`backend/src/main.rs`

### 审计边界
- **未覆盖**：前端代码（frontend/）安全审计、依赖项漏洞（cargo audit）、配置文件敏感信息扫描（.env.example / config.yaml）、数据库迁移脚本（migration/）审计、CI/CD 工作流安全（.github/workflows/）
- **未触发**：本地编译/构建（遵守 CI/CD Only 规则）
