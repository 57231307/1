# 安全审计报告

> 审计时间：2026-06-24
> 审计范围：冰溪 ERP 系统后端（Rust + Axum）
> 审计方法：代码路径追踪 + 攻击面分析

---

## 审计结论

**审计完成——未发现中等或更高严重度的已确认漏洞。**

该代码库已经历 4 轮系统性安全修复（PR #240-243, #250），覆盖 14 个安全漏洞。当前代码在认证、注入防护、租户隔离、敏感数据处理等关键领域均已实现合理的安全措施。

---

## 已确认的安全措施

### 1. 认证与访问控制 ✅

| 安全机制 | 实现位置 | 评估 |
|---------|---------|------|
| JWT 认证 | `auth_service.rs` | HS256 + 2小时过期 + 刷新令牌7天 |
| JWT 黑名单 | `auth_service.rs` (JTI) | 进程级 HashSet，内存存储 |
| API Key 黑名单 | `api_key_service.rs` | 撤销后写入 AppCache，TTL 7天 |
| 用户活跃状态缓存 | `middleware/auth.rs` | 5分钟 TTL，防止禁用用户继续使用旧 JWT |
| 密钥轮换 | `auth_service.rs` | 支持 previous_jwt_secret 平滑过渡 |

**路径追踪示例**：
```
请求 → auth_middleware → validate_token_static → decode JWT
     → is_user_active_cached → DB 查询 is_active
     → is_jti_revoked → 检查内存黑名单
     → AuthContext 注入
```

### 2. CSRF 防护 ✅

| 安全机制 | 实现位置 | 评估 |
|---------|---------|------|
| CSRF Token | `middleware/csrf.rs` | 一次性使用，消费后立即删除 |
| IP 绑定 | `middleware/csrf.rs:139` | Token 与请求 IP 绑定，防盗链 |
| 公开路径白名单 | `middleware/public_routes.rs` | 登录/健康检查等跳过校验 |

### 3. 注入向量防护 ✅

| 安全机制 | 实现位置 | 评估 |
|---------|---------|------|
| 参数化查询 | SeaORM 默认 | 所有数据库操作使用参数化查询 |
| SQL 注入中间件 | `middleware/sql_injection_audit.rs` | 15 种危险模式兜底检测 |
| Webhook 签名 | `webhook_signature.rs` | HMAC-SHA256，常量时间比较 |
| HTML 转义 | `email_service.rs:11-25` | 邮件模板用户输入自动转义 |

**路径追踪示例（Webhook）**：
```
trigger_webhook → send_http_request
  → redirect: none（禁止重定向）
  → HMAC-SHA256 签名
  → verify_webhook_signature
    → ct_eq 常量时间比较
```

### 4. 租户隔离 ✅

| 安全机制 | 实现位置 | 评估 |
|---------|---------|------|
| 租户 ID 提取 | `middleware/tenant.rs` | `extract_tenant_id` 返回错误而非 unwrap_or(0) |
| 查询过滤 | 各 service 层 | `filter(Column::TenantId.eq(tenant_id))` |
| 权限校验 | `middleware/permission.rs` | 角色-资源-操作三级校验 |

### 5. 敏感数据处理 ✅

| 安全机制 | 实现位置 | 评估 |
|---------|---------|------|
| 密码哈希 | `auth_service.rs` | Argon2id，64MB 内存，3次迭代 |
| 密码黑名单 | `password_validator.rs` | Top 100+ 泄露密码 + l33t 归一化 |
| 键盘序列检测 | `password_validator.rs` | 检测 qwerty/asdf/zxcv 等4+连续字符 |
| 错误响应脱敏 | `utils/error.rs` | release 模式移除 SQL 片段/堆栈 |

### 6. 限流与 Bruteforce 防护 ✅

| 安全机制 | 实现位置 | 评估 |
|---------|---------|------|
| 分布式限流 | `middleware/rate_limit.rs` | Redis INCR+EXPIRE，多实例共享 |
| 内存回退 | `middleware/rate_limit.rs` | Redis 不可用时优雅降级 |
| 登录限流 | `middleware/rate_limit.rs` | 5次/5分钟，防止暴力破解 |

### 7. 文件操作安全 ✅

| 安全机制 | 实现位置 | 评估 |
|---------|---------|------|
| ZIP 魔数验证 | `system_update_handler.rs:10-12` | 检测 0x50, 0x4B, 0x03, 0x04 |
| 路径遍历检查 | `system_update_service.rs:415-418` | `outpath.starts_with(&extract_dir)` |
| 规范化路径 | `system_update_handler.rs:178-194` | `canonicalize()` 解析符号链接 |
| 文件大小限制 | `system_update_handler.rs:145` | 100MB 上限 |

---

## 低危观察（无需立即修复）

以下为低危或理论性风险，不构成可利用漏洞：

| 观察 | 位置 | 说明 |
|------|------|------|
| `unwrap()` 使用 | 多处 | 多为 fail-fast 场景，CI clippy 已监控 |
| 内存级 JWT 黑名单 | `auth_service.rs` | 多实例不共享，已在 SECURITY.md 记录 |
| SQL 审计黑名单 | `sql_injection_audit.rs` | 粗粒度兜底，主要防护依赖 SeaORM |

---

## 安全已知的限制（文档记录）

| 限制 | 说明 | 建议方案 |
|------|------|---------|
| JTI 黑名单内存存储 | 多实例不共享 | 未来替换为 Redis |
| 分布式追踪未对接 OTel | 当前仅 W3C traceparent 透传 | 未来按需引入 |
| ErrorResponse.trace_id 孤立 | 每次错误独立生成 UUID | 后续与 trace_context 关联 |

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
