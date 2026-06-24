# 安全审计报告

> 审计时间：2026-06-24
> 审计范围：冰溪 ERP 系统后端（Rust + Axum）
> 审计方法：代码路径追踪 + 攻击面分析
> 审计依据：识别高危/中危/底危漏洞，必须具备可论证的端到端利用路径

---

## 审计结论

**审计完成——未发现中等或更高严重度的已确认漏洞。**

该代码库已经历 4 轮系统性安全修复（PR #240-243, #250），覆盖 14 个安全漏洞。当前代码在认证、注入防护、租户隔离、敏感数据处理等关键领域均已实现合理的安全措施。

---

## 审计范围确认

### 代码库入口点
- `backend/src/main.rs` - 后端服务入口，启动 HTTP + 中间件链路
- 中间件链路：`DefaultBodyLimit` → `audit_context` → `trace_context` → `metrics` → `TraceLayer` → `CORS` → `request_validator` → `permission` → `csrf` → `auth` → `security_headers` → `timeout`

### 信任边界
1. 公共路径（跳过认证）：`/health`, `/ready`, `/init`, `/auth/login`, `/auth/refresh`, `/auth/logout`
2. 所有业务 API 均经过 JWT + CSRF + 权限三层校验
3. 租户隔离通过 `extract_tenant_id` 实现

---

## 高风险攻击面审计结果

### 1. 认证与访问控制 ✅

| 安全机制 | 实现位置 | 验证结果 |
|---------|---------|---------|
| JWT 认证 | `auth_service.rs` | HS256 + 2小时过期 + 刷新令牌7天 |
| JTI 黑名单 | `auth_service.rs:363-380` | 进程级 HashSet，内存存储 |
| 用户级 Token 吊销 | `auth_service.rs:434-472` | 即时吊销删除/封禁用户的所有活跃 JWT |
| 用户 is_active 缓存 | `auth_middleware.rs:51-73` | 5分钟 TTL，防止禁用用户使用旧 JWT |
| API Key 黑名单 | `api_key_service.rs:88-156` | 撤销后写入 AppCache，TTL 7天 |
| Cookie/Header 双认证 | `auth_middleware.rs:108-170` | 优先 Cookie，兼容 Header |
| 密钥轮换 | `main.rs:186-192` | 支持 previous_jwt_secret 平滑过渡 |

**路径追踪示例**：
```
请求 → auth_middleware → validate_token_static → decode JWT
     → is_jti_revoked → 检查内存黑名单
     → is_user_token_revoked → 检查用户级吊销表
     → is_user_active_cached → 5分钟TTL缓存验证
     → AuthContext 注入
```

**CSRF 防护追踪**：
```
POST 请求 → csrf_middleware → extract CSRF Token
          → consume_csrf_token(token, client_ip) → IP 绑定验证
          → 一次性消费，立即从缓存移除
```

---

### 2. 注入向量防护 ✅

| 安全机制 | 实现位置 | 验证结果 |
|---------|---------|---------|
| 参数化查询 | SeaORM 默认 | 所有数据库操作使用参数化查询 |
| SQL 注入中间件 | `sql_injection_audit.rs:47-68` | 15 种危险模式黑名单兜底检测 |
| Webhook 签名 | `webhook_signature.rs:39-60` | HMAC-SHA256 + `subtle::ConstantTimeEq` 常量时间比较 |
| 文件路径校验 | `system_update_service.rs:415-418` | `outpath.starts_with(&extract_dir)` 路径遍历检查 |
| ZIP 魔数验证 | `system_update_handler.rs:10-12` | 检测 `0x50, 0x4B, 0x03, 0x04` |
| 禁止重定向 | `webhook_service.rs:203` | `redirect: Policy::none()` |

**SQL 注入防护路径追踪**：
```
用户输入 → Handler → SeaORM 参数化查询
         → sql_injection_audit_middleware（黑名单兜底）
```

**文件上传路径遍历防护**：
```
上传文件 → verify_zip_magic(data) → 魔数校验
         → canonicalize(save_path) → 解析符号链接
         → starts_with(canonical_temp_dir) → 路径边界校验
         → 写入文件
```

---

### 3. 外部交互 ✅

| 安全机制 | 实现位置 | 验证结果 |
|---------|---------|---------|
| HTTPS 强制 | `webhook_service.rs:169-171` | 仅允许 `https://` 协议 |
| SSRF 防护 | `webhook_service.rs:173-196` | 内网 IP 检测（私有/loopback/link-local） |
| 禁止重定向 | `webhook_service.rs:203` | `Policy::none()` |
| Webhook HMAC 签名 | `webhook_service.rs:216-219` | HMAC-SHA256 + `X-Webhook-Signature` 头 |
| 30秒超时 | `webhook_service.rs:201-202` | `timeout(30s)` + `connect_timeout(10s)` |

**Webhook 出站请求安全路径**：
```
trigger_webhook → send_http_request
  → url.starts_with("https://") → HTTPS 强制
  → DNS 反查 → IP 内网检测
  → redirect: none → 禁止重定向
  → HMAC-SHA256 签名
  → timeout(30s) → 防 SSRF 慢速攻击
```

---

### 4. 敏感数据处理 ✅

| 安全机制 | 实现位置 | 验证结果 |
|---------|---------|---------|
| 密码哈希 | `auth_service.rs:258-272` | Argon2id，64MB 内存，3次迭代 |
| 密码黑名单 | `password_validator.rs:87-205` | Top 100+ 泄露密码 + l33t 归一化 |
| 键盘序列检测 | `password_validator.rs:207-244` | 检测 qwerty/asdf/zxcv 等4+连续字符 |
| 错误响应脱敏 | `error.rs:254-267` | 生产环境移除 error_type/detail/SQL 片段 |
| Cookie Secret 校验 | `main.rs:358-375` | 启动时强制检查 ≥32 字节 |
| Webhook Secret 校验 | `main.rs:380-395` | 启动时强制检查 ≥32 字节，与 jwt_secret 互异 |
| 日志脱敏 | `auth_middleware.rs:130-170` | 认证失败日志不含敏感上下文 |

**错误响应脱敏验证**：
```
AppError → into_response → 
  match &self { ... } → (status, _log_detail) 仅日志
  → body = { code, message(脱敏), trace_id, timestamp }
  → 响应体不含 error_type/detail/原始 msg
```

---

## 低危观察（无需立即修复）

以下为低危或理论性风险，不构成可利用漏洞：

| 观察 | 位置 | 说明 |
|------|------|------|
| `unwrap()` 使用 | 多处 | 多为 fail-fast 场景，CI clippy 已监控 |
| 内存级 JWT 黑名单 | `auth_service.rs:353-354` | 多实例不共享，已在 SECURITY.md 记录 |
| SQL 审计黑名单 | `sql_injection_audit.rs` | 粗粒度兜底，主要防护依赖 SeaORM |

---

## 历史漏洞修复确认

| # | 等级 | 漏洞 | 确认状态 |
|---|------|------|---------|
| #1 | P0 | 路径遍历 | ✅ 已修复 - `system_update_service.rs:415-418` |
| #2 | P0 | WebSocket 认证绕过 | ✅ 已修复 - `ws handshake` + JWT 校验 |
| #3 | P1 | init_token 缺失 | ✅ 已修复 - `init_token.rs` 中间件 |
| #4 | P2 | 错误响应信息泄漏 | ✅ 已修复 - `error.rs` 统一脱敏 |
| #5 | P2 | API Key 撤销失效 | ✅ 已修复 - `AppCache` 黑名单 |
| #6 | P2 | 分布式限流缺失 | ✅ 已修复 - Redis INCR+EXPIRE |
| #7 | P2 | 弱密码接受 | ✅ 已修复 - Top 100+ l33t 归一化 |
| #8 | P2 | 错误响应类型泄漏 | ✅ 已修复 - `error.rs` 移除 error_type |

---

## 审计方法

本次审计采用以下方法：

1. **架构梳理**：阅读 `ARCHITECTURE.md`、`SECURITY.md` 理解信任边界
2. **入口点追踪**：检查 `main.rs` 中间件配置，确认安全链路完整性
3. **攻击面分析**：针对认证、注入、外部交互、敏感数据四大类逐一检查
4. **代码路径验证**：对每个安全机制追踪从输入到最终处理的全路径
5. **证据要求核对**：每个安全措施均验证了攻击者可控输入 → 代码路径 → 影响结果

---

## 下次审计建议

1. 定期更新 Top 100 密码黑名单（建议每季度）
2. 考虑引入 Redis 实现分布式 JTI 黑名单
3. 监控 `unwrap()` 导致的潜在 panic 场景
4. 对前端输入进行 XSS 审计（本次仅覆盖后端）
5. 审计邮件发送配额计数器（`email_send_counters`）的实际消费路径
