# Bingxi ERP 企业级安全加固实施方案与合规评估报告

## 总体规划与分阶段路线图
本项目安全加固方案旨在将 Bingxi ERP 提升至符合 ISO 27001、GDPR 及行业零信任标准的企业级安全基线。分为三大阶段：
1. **立即实施阶段（高优先级，2周内）**：聚焦应用层高危风险闭环（XSS/CSRF、CC攻击、API泄露、SQLi、CORS提权）。
2. **短期实施阶段（中优先级，1个月内）**：聚焦基础设施防御与安全审计（WAF、DDoS清洗、IDS、自动化扫描、TOTP）。
3. **长期规划阶段（低优先级，3-6个月内）**：聚焦前沿架构演进（零信任、微服务mTLS、区块链审计、UEBA、后量子加密）。

---

## 阶段一：立即实施阶段（核心五项详情）

### 1. 将现有 JWT/Session 迁移至 HttpOnly Cookie (XSS 防护)
- **技术方案**：
  - 后端：在 `auth_handler.rs` 登录成功后，通过 `Set-Cookie` 头返回 JWT，附加 `HttpOnly=true`, `Secure=true`, `SameSite=Strict`, `Path=/`。
  - 前端：移除 `Storage::set_token` 中存入 `localStorage` 的逻辑。修改 `api.rs`，在全局请求配置中加入 `credentials: "include"` 以便浏览器自动携带 Cookie。
- **测试用例**：
  1. 成功登录后，检查浏览器 Storage 面板无 JWT 残留，Cookie 面板中存在 jwt，且勾选了 HttpOnly。
  2. 控制台执行 `document.cookie` 无法读取到 JWT。
  3. 前端跨域（或同源）发起受保护请求时，验证能否正常携带 Cookie 并通过鉴权。
- **回滚计划**：保留原有的 Header `Authorization: Bearer <token>` 解析逻辑作为后备；若前端适配失败，通过环境变量 `AUTH_MODE=header` 一键切回旧版响应体发牌模式。
- **性能影响评估**：无额外性能损耗。Cookie 头体积微小，且节省了前端每次手动读写 Storage 的开销。
- **合规性检查报告**：符合 OWASP Top 10 A03:2021-Injection (XSS 缓解) 和 A07:2021-Identification and Authentication Failures。

### 2. 基于 Redis 的双重维度速率限制 (IP + UserID)
- **技术方案**：
  - 引入 `redis` 和 `deadpool-redis` crate，在 `AppState` 中注入 Redis 连接池。
  - 重写 `middleware/rate_limit.rs`，使用 Lua 脚本实现滑动窗口（Sliding Window）或令牌桶算法。
  - Key 设计：`rate_limit:{ip}` 和 `rate_limit:{user_id}`。阈值设定为 100次/分钟。超出时拦截并返回 HTTP 429 Too Many Requests，并附带 `Retry-After` 头。
- **测试用例**：
  1. 单一 IP 匿名访问，1分钟内发送 101 次请求，断言第 101 次返回 429。
  2. 携带不同 UserID 但同一 IP，测试是否受 IP 阈值共同影响。
- **回滚计划**：在 Redis 连接失败或超时（>50ms）时，降级（Fail-open）为系统内存基础限流，或直接放行以保证可用性。
- **性能影响评估**：每次请求增加一次 Redis 网络 RTT（通常 < 2ms）。通过 Lua 脚本合并检查与递增操作，最大化性能。
- **合规性检查报告**：符合 OWASP API Security Top 10 API4:2023-Unrestricted Resource Consumption。

### 3. API 密钥生命周期管理 (32字节, 90天轮换, 自动吊销)
- **技术方案**：
  - 创建 `api_keys` 数据表，存储 `id`, `user_id`, `hashed_key`, `prefix`, `expires_at`, `is_revoked`。
  - 使用 32 字节 CSPRNG 生成密钥，只在创建时返回一次明文。数据库中存储使用 SHA-256 或 Argon2 散列后的值。
  - `expires_at` 强制设为创建后 90 天。提供定时任务（或启动时检查）标记超期密钥为 `is_revoked = true`。
- **测试用例**：
  1. 生成密钥，断言长度和随机性。
  2. 使用过期密钥调用接口，断言返回 401。
  3. 后台手动吊销密钥后，断言即时失效。
- **回滚计划**：保留系统级别的全局白名单绕过通道（仅限内部诊断IP），当新认证模块故障时可通过临时配置恢复业务。
- **性能影响评估**：API 密钥验证需查询数据库或 Redis 缓存，建议将有效密钥同步至本地 DashMap 缓存（TTL: 5分钟），性能损耗可忽略。
- **合规性检查报告**：符合 ISO 27001 密码学控制 (A.10) 及访问控制 (A.9) 的密钥管理规范。

### 4. 集成 SQLMap 到 CI/CD 流水线自动化注入测试
- **技术方案**：
  - 在 `.github/workflows/security-audit.yml` 中新增 `sqlmap-scan` 任务。
  - 启动测试环境数据库和后端服务，生成具有各类角色权限的测试 Token。
  - 使用 `sqlmap -u "http://localhost:8000/api/v1/erp/..." --headers="Cookie: jwt=..." --batch --forms --crawl=2 --level=2 --risk=2` 自动化遍历接口。
  - 通过 `grep` 或解析输出，如果发现注入点则流水线返回非 0 退出码（Fail the build）。
- **测试用例**：
  1. 在某接口故意写入拼接 SQL 漏洞，提交 PR，断言流水线红灯并拦截。
  2. 修复漏洞后，断言流水线绿灯。
- **回滚计划**：若自动化扫描误报导致阻塞发布，可在 Commit Message 中使用 `[skip sqlmap]` 标签跳过该步骤。
- **性能影响评估**：仅在 CI/CD 阶段运行，延长构建时间约 5-10 分钟，对生产环境无任何影响。
- **合规性检查报告**：符合 DevSecOps 最佳实践及 OWASP A03:2021-Injection 持续防御要求。

### 5. 实施动态 CORS 白名单验证
- **技术方案**：
  - 在 `settings.rs` 配置文件或数据库中维护合法的 `allowed_origins` 列表（例如 `https://erp.bingxi.com`, `http://localhost:3000`）。
  - 在 `main.rs` 的 CORS 中间件配置中，不再使用简单的通配符，而是利用 `tower_http::cors::AllowOrigin::predicate` 动态匹配请求的 Origin。
  - 拒绝一切非列表内的 Origin 头，避免 CORS 错误配置导致的凭证盗用。
- **测试用例**：
  1. 使用 curl 伪造 `Origin: https://evil.com` 发起预检请求 (OPTIONS)，断言返回 403 或无 CORS 头。
  2. 使用合法 Origin，断言返回正确的 `Access-Control-Allow-Origin`。
- **回滚计划**：提供紧急配置开关 `CORS_MODE=permissive`，在前端域名紧急迁移且配置未生效时，临时放宽限制。
- **性能影响评估**：内存级别的字符串匹配（或正则/HashSet匹配），耗时 < 0.1ms，无感。
- **合规性检查报告**：符合 OWASP API Security API7:2023-Security Misconfiguration 防御要求。

---
> 备注：文档由 Bingxi ERP 架构组自动生成并签署。长期及短期阶段（第6-15项）为基础设施运维与架构演进，将另行出具 Terraform 及部署工单。
