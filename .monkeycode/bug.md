# 安全漏洞记录

> 本文件用于登记项目安全漏洞。所有已修复漏洞已迁移至 git 历史（CHANGELOG.md / PR）。
> 当前为空占位文件。审计周期内如有新漏洞发现，登记后立即启动修复流程。
> 详见 `.monkeycode/MEMORY.md` 的 Bug.md 实时漏洞管理规则。

---

## 待修复漏洞（2026-06-24 审计周期新增）

> 所有 6 个低危漏洞已在 2026-06-24 当日完成处理：
> - #1 JTI 黑名单→Redis：已修复（auth_service.rs 切换 Redis SETEX + 内存回退）
> - #2 Webhook URL 内网白名单（SSRF）：已修复（新建 ssrf_guard.rs）
> - #3 分布式限流 try_lock：已修复（rate_limit.rs 改用 std Mutex + try_lock）
> - #4 认证失败日志脱敏：已修复（auth.rs 新增 mask_auth_header / mask_username）
> - #5 JWT 密钥硬编码：审计无问题（main.rs 启动时强制校验 + Default::default 在生产 panic）
> - #6 TOTP 熵源：审计无问题（totp-rs 5.5 Secret::generate_secret 内部用 rand::thread_rng → OsRng）
>
> 详细修复说明与 PR 见 git log / CHANGELOG.md。
