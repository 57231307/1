# 安全漏洞记录

> 本文件用于登记项目安全漏洞。所有已修复漏洞已迁移至 git 历史（CHANGELOG.md / PR）。
> 当前为空占位文件。审计周期内如有新漏洞发现，登记后立即启动修复流程。
> 详见 `.monkeycode/MEMORY.md` 的 Bug.md 实时漏洞管理规则。

---

## 待修复漏洞（2026-06-24 审计周期新增）

### 低危 #1：JTI 黑名单进程内存储
- **严重度**：低
- **位置**：`backend/src/services/auth_service.rs:43-49`
- **描述**：JTI 黑名单为进程级 HashSet，多实例部署时不共享
- **影响**：撤销后的旧 JWT 在其他实例最多可继续使用 2 小时（JWT 过期时间）
- **修复建议**：替换为 Redis 实现（项目已有 Redis 依赖）
- **状态**：待修复

### 低危 #2：Webhook URL 缺乏内网白名单控制（SSRF 风险）
- **严重度**：低
- **位置**：`backend/src/handlers/webhook_handler.rs:43-68`（create_webhook handler）
- **描述**：已认证用户可创建指向内网（169.254.169.254、localhost、10.x.x.x、192.168.x.x）的 Webhook URL
- **影响**：可能被滥用扫描内网或访问云元数据服务
- **修复建议**：在 `create_webhook` 时校验 URL 主机名，拦截 RFC1918/loopback/云元数据地址
- **状态**：待修复

### 低危 #3：分布式限流 `try_lock` 缺失
- **严重度**：低
- **位置**：`backend/src/middleware/rate_limit.rs:49-75`（MemoryRateLimiter::check）
- **描述**：未直接发现可利用路径，但锁中毒场景未防御
- **影响**：极端情况下可能 panic
- **修复建议**：考虑用 `try_lock` 替代或显式处理锁中毒
- **状态**：待修复

### 低危 #4：错误日志可能包含敏感上下文
- **严重度**：低
- **位置**：`backend/src/middleware/auth.rs:130-170`
- **描述**：认证失败时 Cookie 名称和头部内容在日志中可见
- **影响**：日志中可能留下 PII（username 等）
- **修复建议**：日志脱敏或降低日志级别为 debug
- **状态**：待修复

### 低危 #5：JWT 密钥硬编码风险待确认
- **严重度**：低（待确认）
- **位置**：`backend/src/utils/config.rs` / `backend/src/utils/app_state.rs`
- **描述**：未确认是否存在硬编码 fallback secret
- **影响**：若存在则配置缺失时会使用弱密钥
- **修复建议**：确认无硬编码；若存在则改为启动时强校验，缺失即 panic
- **状态**：待确认后决定是否修复

### 低危 #6：TOTP 密钥熵源待审计
- **严重度**：低（待确认）
- **位置**：`backend/src/services/totp_service.rs`
- **描述**：需确认 TOTP secret 使用密码学安全随机源
- **影响**：若使用时间戳或弱熵则 TOTP 可预测
- **修复建议**：审计生成路径，确认使用 `OsRng` 或 `rand::thread_rng()`
- **状态**：待审计后决定是否修复
