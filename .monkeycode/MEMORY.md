# 项目规则记忆

> 本文件是项目的**规则记忆**，记录必须遵守的规则、指令、偏好和工作流规范。
> 历史归档与详细内容请查阅 [`.monkeycode/docs/archives/`](file:///workspace/.monkeycode/docs/archives/)。

---

## 文件定义

| 文件 | 用途 | 说明 |
|------|------|------|
| `MEMORY.md` | 项目规则记忆 | 规则、规范、关键经验（必须遵守） |
| `doto.md` | 任务与历史 | 当前任务 + 历史归档索引（实时更新） |
| `CHANGELOG.md` | 任务精简总结 | 任务一句话摘要列表（PR 完成后更新） |
| `docs/archives/` | 历史归档 | 已优化前的完整内容（按日期保留） |

---

## 一、格式说明

### 用户指令条目
```
[用户指令摘要]
- Date: YYYY-MM-DD
- Context: 提及的场景或时间
- Instructions:
  - 具体的知识点
```

### 项目知识条目
```
[项目知识摘要]
- Date: YYYY-MM-DD
- Context: Agent 在执行 [具体任务] 时发现
- Category: 运维部署|构建方法|测试方法|排错调试|工作流协作|环境配置
- Instructions:
  - 具体的知识点
```

---

## 二、基础规范

[沟通语言]
- Date: 2026-06-19
- Category: 基础偏好
- 使用中文进行回复和沟通

[编码规范]
- Date: 2026-06-19
- Category: 开发规范
- 禁止硬编码，所有文本需使用中文
- 代码注释必须使用中文

[项目标识]
- Date: 2026-06-19
- Category: 基础偏好
- 项目名称统一（以 main 仓库 README 为准），所有文档/界面/输出信息一致

[开发辅助]
- Date: 2026-06-19
- Category: 工作流协作
- 每次新增或修改功能时，必须调用合适的技能或 MCP 工具
- 严格按照技能规范进行开发

[任务管理]
- Date: 2026-06-19
- Category: 工作流协作
- 使用中文建立待办任务（doto.md）
- 每完成一个待办任务，立即标记为"已完成"

[记忆管理]
- Date: 2026-06-19
- Category: 工作流协作
- 实时查看和更新 `MEMORY.md` 规则记忆文档
- 关键内容存储在 `MEMORY.md`，变更记录到 `CHANGELOG.md`
- **路径策略（2026-06-19 确认）**：test 分支合并入 main 时 `-X theirs` 会覆盖 `.monkeycode/`，必须以 main 版本为准；test 自己的 `.monkeycode/docs/` 不应混入 main

[死代码与未使用文件处理]
- Date: 2026-06-24
- Category: 开发规范
- **不使用的文件/代码/文件夹必须删除**（删除前评估影响范围，删除后更新受影响文件）
- 修改文件后保存前**必须交叉自审**（检查引用、配置、文档是否同步）
- **功能必须接入项目**（尽可能减少 TODO，禁止遗留占位代码）

[Bug.md 实时漏洞管理]
- Date: 2026-06-24
- Category: 工作流协作
- **实时检测** `.monkeycode/bug.md` 漏洞文件
- 发现漏洞 → 立即启动修复（按 P0/P1/P2 优先级）
- **修复一个漏洞后立即从 bug.md 删除对应条目**（避免重复处理）
- 所有漏洞修复完成后保留 `bug.md` **空文件**（不删除，作为漏洞登记占位）
- **完成状态 (2026-06-24)**：bug.md 全部 8 个漏洞已修复（PR #250），
  bug.md 已简化为占位文件

[任务规划管理]
- Date: 2026-06-19
- Category: 工作流协作
- 所有任务规划文件保存在 `.monkeycode/docs/` 下

[数据库配置]
- Date: 2026-06-19
- Category: 环境配置
- 数据库类型：PostgreSQL
- 连接方式：远程数据库连接模式

[功能实现依据]
- Date: 2026-06-19
- Category: 开发规范
- 新增功能接口、数据库操作需遵循现有规范

[打包与发布要求]
- Date: 2026-06-19
- Category: 运维部署
- 打包时必须进行全面测试：功能测试、兼容性测试、稳定性测试

---

## 三、安全规范

[租户隔离]
- Date: 2026-06-19
- Category: 安全规范
- **严禁**使用 `auth.tenant_id.unwrap_or(0)` 获取租户ID
- 必须使用 `extract_tenant_id(&auth)?` 进行租户ID提取
- 所有涉及租户数据的操作都需严格的租户隔离验证

[敏感信息保护]
- Date: 2026-06-19
- Category: 安全规范
- 禁止硬编码敏感信息（密码、密钥、令牌等）
- 使用环境变量或配置管理工具

[输入验证]
- Date: 2026-06-19
- Category: 安全规范
- 所有用户输入必须验证和清理
- 使用参数化查询防止 SQL 注入
- 对输出进行编码防止 XSS 攻击

---

## 四、CI/CD 强制（2026-06-20 用户强调）

[本地编译禁止]
- Date: 2026-06-20
- Category: 运维部署
- **禁止**本地编译验证（`cargo build` / `cargo check` / `cargo test` / `cargo fmt -- --check` / `cargo clippy` / `npm run build` / `vue-tsc` / `pnpm typecheck` 等）
- **禁止**本地启动服务做端到端验证
- 所有验证走 GitHub Actions CI：修改代码 → commit → push → 监控 run → 失败拉 logs → 修复 → 重 push
- **唯一允许的本地操作**：文件 diff、语法、文本类（git status、cat、grep、sed、Edit、Write）

[CI 监控 API]
- Date: 2026-06-23
- Category: 排错调试
- `/repos/{owner}/{repo}/commits/{sha}/check-runs` —— 查询 check run 状态
- `/repos/{owner}/{repo}/actions/runs/{id}/logs` —— 下载 logs zip
- `/repos/{owner}/{repo}/check-runs/{id}/annotations` —— 错误标注
- `/repos/{owner}/{repo}/actions/runs/{id}/jobs` —— 查询 job 列表

[服务器环境]
- Date: 2026-05-27
- Category: 运维部署
- 服务名称：bingxi-backend（systemd），安装目录：`/opt/bingxi-erp`
- 后端端口：8082，日志目录：`/opt/bingxi-erp/backend/logs`，备份目录：`/opt/bingxi-erp/backups`
- 环境配置：`/etc/bingxi-erp/.env`
- 部署命令：`bingxi update`（CLI 工具）
- 部署方式：CICD 构建 → GitHub Release → 手动部署到生产服务器
- **禁止** Docker 容器部署（不得创建 Dockerfile、docker-compose.yml）

[部署限制]
- Date: 2026-05-29
- Category: 运维部署
- 不安装 PostgreSQL 客户端（用远程数据库 39.99.34.194:5432）
- 不安装 Redis（用远程 Redis 服务器）
- 只需安装 Nginx、curl

---

## 五、核心经验（关键排错与开发经验）

[集成测试跨 crate 调用私有函数]
- Date: 2026-06-24
- Context: commit `e8e69a52` 修复 14 个 E0624 编译错误时发现
- Category: 排错调试
- `tests/` 目录下的集成测试编译为**独立二进制 crate**，`fn foo()` 对集成测试 crate 不可见
- 修复：`fn foo()` → `pub fn foo()`（或使用 `pub(crate)` 限制可见性）
- 错误模式：`error[E0624]: associated function compose_color_no is private`
- 决策原则：内部实现细节稳定可作为测试入口时用 `pub`；否则考虑重构暴露更窄的公共 API

[沙箱网络限制]
- Date: 2026-06-24
- Context: SSH 切换后尝试在沙箱内推送测试时发现
- Category: 环境配置
- **限制**：沙箱环境出站 22 端口（github.com）和 443 端口（ssh.github.com）均被防火墙阻断
- **可用**：443 端口（github.com HTTPS API/页面）正常
- **影响**：沙箱内无法 `git push` 或 `ssh -T git@github.com`，所有推送操作必须在**用户本地终端**执行
- **应对策略**：
  - 沙箱内仅做：代码编辑、git commit、文档更新
  - 推送前：通过 GitHub HTTPS API 检查远程状态
  - 用户本地：执行 `git push` / `gh pr create`
- **验证方法**：`nc -zv github.com 22` 返回 "Connection timed out" 即确认

[.monkeycode 目录 gitignore 规则]
- Date: 2026-06-24
- Context: ssh-public-key 文档创建后 `git add` 失败时发现
- Category: 环境配置
- `.gitignore` 默认忽略 `.monkeycode/`，仅白名单：`MEMORY.md` / `doto.md` / `bug.md` / `CHANGELOG.md`
- `.monkeycode/docs/` 子目录不在白名单
- **添加新归档文件**必须用 `git add -f` 强制添加
- 已有 71 个 `.monkeycode/docs/*.md` 文件被追踪（历史均用 `-f` 添加）

[集成测试 `crate` 语义]
- Date: 2026-06-24
- Context: PR #247 批次 C 修复时发现
- Category: 排错调试
- `tests/` 目录下的集成测试编译为独立二进制，`crate` 关键字指向**测试二进制本身**
- 引用 lib.rs 暴露的模块必须用 `Cargo.toml` 中的 `name` 字段（连字符 `-` 转下划线 `_`），即 `bingxi_backend`
- 单元测试（`src/` 内的 `#[cfg(test)]`）中 `crate` 指向 lib，两者语义不同
- 错误模式：`use crate::services::...` → 修复：`use bingxi_backend::services::...`

[Clippy Baseline 脆弱性]
- Date: 2026-06-24
- Context: PR #247 + #248 CI 失败时发现；PR #250 再次出现
- Category: 排错调试
- `backend/.clippy-baseline.txt` 用 `comm -23` 精确行比较检测"新警告"
- CI 脚本（`.github/workflows/ci-cd.yml:405-416`）用 `sort -u` 处理多行 `rendered` 字段，导致基线只包含 `= help:`、`= note:` 等辅助文本而非警告摘要行
- **症状**：CI 误报数百到上千个"新警告"（实际为 0）；PR #250 编译成功后 baseline 441 → 当前 1539，差 1113 全是误报
- **修复**：删除 `backend/.clippy-baseline.txt`，让 CI 在 bootstrap 模式下重建
- **快速诊断**：CI 误报"大量新警告"时，先 `head backend/.clippy-baseline.txt` 检查首行内容（应为警告摘要而非辅助文本）
- **长期方案（TODO）**：改用 `jq` 提取结构化标识符（`code` + `message` + `span`）进行去重

[Cache::get 返回值语义]
- Date: 2026-06-24
- Context: PR #250 修复 #5 API Key 黑名单 CI 失败时发现
- Category: 排错调试
- `backend/src/utils/cache.rs` 的 `Cache` trait 定义 `fn get(&self, key: &K) -> Option<V>`，返回值已 **Clone**（不是 `Option<&V>`）
- 不能在结果上调用 `.copied()`（仅 `Option<&T>` 或迭代器支持）
- 错误模式：`cache.get(&key).copied().unwrap_or(false)` → 修复：`cache.get(&key).unwrap_or(false)`

[JTI 黑名单→Redis 迁移设计]
- Date: 2026-06-24
- Context: 修复低危 #1 JTI 黑名单进程内存储时设计
- Category: 安全 / 性能
- **现状**：`auth_service.rs` 用 `static JTI_BLACKLIST: LazyLock<RwLock<HashMap<String, i64>>>`，多实例不共享
- **风险**：撤销后的旧 JWT 在其他实例最多可继续使用 2 小时（JWT 过期时间）
- **迁移方案**：
  - 优先用 Redis SETEX（`SET key value EX <ttl>`），TTL 到期自动清理，零维护成本
  - 环境变量 `JTI_REDIS_URL` 或回退 `REDIS_URL` 启用
  - **失败回退**：Redis 不可用时降级到原 HashMap（避免阻塞业务）
  - **清理**：`cleanup_expired_jti` 在 Redis 模式下为 noop（TTL 自动清理）
- **关键 API**：
  - 写：`SET key value EX <ttl_secs>` → `redis::AsyncCommands::set_ex`
  - 读：`EXISTS key` → `redis::AsyncCommands::exists`
- **优雅降级模式**：与 `rate_limit.rs` 的 `REDIS_RATE_LIMITER` 设计一致
- **测试覆盖**：未配置 Redis 时回退路径的行为（`is_jti_revoked` 一致性、清理逻辑）

[SSRF 防护双重校验必要性]
- Date: 2026-06-24
- Context: 修复低危 #2 Webhook SSRF 时设计
- Category: 安全
- **单次校验的弱点**：create 时校验 `url` 指向公网，但攻击者可注册合法公网域名后修改 DNS 记录为内网 IP（DNS Rebinding）
- **必须双重校验**：
  1. `create_webhook` 时校验（防滥用：阻止用户保存内网 URL）
  2. `trigger_webhook` 发送前**再次**校验（防 DNS Rebinding：每次重新解析）
- **校验内容**（`backend/src/utils/ssrf_guard.rs`）：
  - 协议白名单：`http://` / `https://`（拒 `file://`、`gopher://` 等）
  - 主机名黑名单：`localhost` / `*.local` / `*.internal`
  - IP 黑名单：解析为 IP 后校验
    - IPv4：RFC1918（10/8、172.16/12、192.168/16）、loopback（127/8）、link-local（169.254/16 含云元数据 169.254.169.254）
    - IPv6：`::1`、`::`、`fe80::/10`、ULA（fc00::/7）、IPv4-mapped 内部 IPv4
- **错误传播**：校验失败时 `WebhookDeliveryResult` 返回 `success: false, error: "SSRF 防护拦截：..."`（不 panic 业务）

[DashMap vs std::sync::Mutex 选型]
- Date: 2026-06-24
- Context: 修复低危 #3 限流器 try_lock 时决策
- Category: 代码规范
- **DashMap**（分片无锁 HashMap）：
  - 优点：高并发读、API 简洁（无 unwrap）
  - 缺点：API 不暴露 `PoisonError`，极端 panic 场景下不可恢复
- **std::sync::Mutex<HashMap>**：
  - 优点：`try_lock()` 显式处理锁不可用
  - 缺点：单锁并发受限
- **决策原则**：
  - 高频/性能关键：DashMap
  - 安全关键 + 锁中毒需防御：std::sync::Mutex + try_lock
- **项目实践**（限流器 `MemoryRateLimiter`）：
  - 改用 `std::sync::Mutex<HashMap>` + `try_lock`
  - 锁失败时 **fail-open**（默认放行） + `warn!` 日志
  - 性能可接受：180 req/min/user 是常见限流阈值，单锁不构成瓶颈
- **关键模式**：`let Ok(mut g) = self.storage.try_lock() else { return; };`（Rust 1.65+ let-else）

[日志脱敏按字符而非字节]
- Date: 2026-06-24
- Context: 修复低危 #4 认证失败日志脱敏时实现
- Category: 安全 / 国际化
- **风险**：截断 UTF-8 字符串用字节切片 `&s[..n]` 可能切到字符中间，panic（`byte index N is not a char boundary`）
- **正确做法**：用 `chars().take(n)` 按 Unicode 字符截断
- **项目实践**（`auth.rs::mask_username`）：
  ```rust
  let chars: Vec<char> = username.chars().collect();
  if chars.len() <= 2 { "***".to_string() }
  else { format!("{}***", chars[..2].iter().collect::<String>()) }
  ```
- **测试覆盖**：中文用户名 `"管理员"` → `"管***"`（3 字符按字符截断）
- **Authorization 头脱敏**：保留前缀 `"Bearer "` + Token 前几位 + `(len=N)` 供排错，截断 Token 部分

[totp-rs 5.5 熵源确认]
- Date: 2026-06-24
- Context: 审计低危 #6 TOTP 熵源时确认
- Category: 安全 / 依赖审计
- `totp-rs = { version = "5.5", features = ["qr", "gen_secret"] }` 启用 `gen_secret` feature
- `Secret::generate_secret()` 源码（`constantoine/totp-rs@v5.5.0/src/secret.rs`）：
  ```rust
  pub fn generate_secret() -> Secret {
      use rand::Rng;
      let mut rng = rand::thread_rng();
      let mut secret: [u8; 20] = Default::default();
      rng.fill(&mut secret[..]);
      Secret::Raw(secret.to_vec())
  }
  ```
- **熵源链**：`rand::thread_rng()` → 内部用 `OsRng`（rand 0.8+）→ 操作系统 CSPRNG（Linux: `getrandom(2)`）
- **安全等级**：密码学安全（160 bits 熵，符合 RFC 4226 推荐）
- **审计结论**：✅ 无需修改，TOTP 密钥生成路径已是密码学最佳实践

[GitHub Token 安全存储]
- Date: 2026-06-24
- Context: 用户提供 fine-grained PAT 用于推送
- Category: 安全 / 凭证管理
- **绝不写入任何 git 跟踪文件**（.git/config / MEMORY.md / doto.md / CHANGELOG.md / commit message）
- **存储位置**：沙箱本地 `~/.git-credentials`（600 权限，git credential helper = store 自动读取）
- **类型**：fine-grained PAT（`github_pat_` 前缀，90 天有效期，用户提供）
- **沙箱网络限制**：SSH 22 端口被防火墙阻断，必须用 HTTPS push
- **推送诊断流程**（PAT 403 必走）：
  1. 立即用 PAT 测 issue 创建（`POST /repos/.../issues`）
  2. 403 `Resource not accessible by personal access token` = 缺写权限
  3. 不是 token 错误，是 fine-grained PAT 权限未勾选
  4. 引导用户去 https://github.com/settings/pats 给 token 勾选 `Contents: Read and write`
- **推送命令**：
  ```bash
  git credential fill <<< $'protocol=https\nhost=github.com'  # 验证 token 读回
  git push -u origin <branch>  # 自动从 ~/.git-credentials 读取
  ```
- **SSH 22 端口 vs HTTPS 443**：沙箱 raw TCP 22/443 阻断，但 git/curl 高层 HTTPS 透通（透明代理）

[分布式限流回退必须真实回退]
- Date: 2026-06-24
- Context: PR #250 #6 修复后 CI 单元测试 `test_check_rate_limit_falls_back_to_memory` 失败时发现
- Category: 排错调试
- 错误设计：`check_redis_rate_limit` 返回 `Ok(true)`（未配置 Redis），`check_rate_limit` 直接放行
- 正确设计：返回 `Result<Option<bool>>`：
  - `Ok(Some(allowed))`：Redis 判定结果
  - `Ok(None)`：未配置 Redis（应回退）
  - `Err(_)`：Redis 错误（应回退）
- 调用方（`check_rate_limit`）在 `Ok(None)` 和 `Err(_)` 两种情况下都必须调用 `memory_limiter.check(key)`
- **测试断言**：`assert!(!check_rate_limit(...))` 第 N 次（max 限流上限）应被拒绝，验证真正回退到内存

[Cargo build --release vs cargo test 编译差异]
- Date: 2026-06-24
- Context: PR #250 #5 修复在 release build 才暴露 `.copied()` 编译错误
- Category: 排错调试
- 某些编译错误在 `cargo test`（dev build）中不会触发，但在 `cargo build --release`（`opt-level=2`）会触发
- **CI 防护**：依赖 `🏗️ Rust 后端构建` job 跑 `cargo build --release` 早期发现问题
- **本地验证**（非 CI）：`cargo check --release --all-targets` 可提前暴露此类问题

[`|| true` 反模式]
- Date: 2026-06-24
- Context: PR #248 修复 `color_price_crud_test.rs:90` 的 E0599 时发现
- Category: 排错调试
- `assert!(some_expr.is_ok() || true)` 是恒真式断言，无测试价值却能**掩盖编译错误**
- CI 中应使用 `cargo check --tests` 或 `cargo test --no-run` 提前发现编译错误

[SeaORM Trait 必导]
- Date: 2026-06-23
- Context: PR #242 clippy 防御性 allow 误报清理时发现
- Category: 排错调试
- `Entity::find()` → 需 `use sea_orm::EntityTrait;`
- `.filter()` → 需 `use sea_orm::QueryFilter;`
- `.gte()/.lt()/.gt()/.lte()/.eq()` → 需 `use sea_orm::ColumnTrait;`
- `.count()/.all()/.paginate()` → 需 `use sea_orm::PaginatorTrait;`
- 清理 sea_orm trait 导入时**不能批量删**，必须**逐个静态验证**（`grep -n "Entity::find\|\.filter\|\.gte\|\.lt"`）
- CI E0599 的 help 提示会明确指出需要的 trait 名（如 `trait EntityTrait which provides find is implemented but not in scope`）

[Clippy Lint 名规范]
- Date: 2026-06-23
- Context: PR #242 修复 useless_attribute 警告时发现
- Category: 排错调试
- rustc builtin lint：`unused_variables` / `unused_imports` / `dead_code`（不带 `clippy::` 前缀）
- clippy 内置 lint：`clippy::redundant_clone` / `clippy::too_many_arguments` / `clippy::needless_pass_by_value` / `clippy::useless_attribute` 等
- `clippy::unused_variables` 是**无效 lint 名**，触发 `unknown_lints` 警告
- 标记**实际被使用项**的 `#[allow(...)]` 触发 rustc 1.94 `useless_attribute` 警告（CI `-D warnings` 升级为 error）

[Validator 限制]
- Date: 2026-06-23
- Context: PR #242 修复 CSV 导入大小限制时发现
- Category: 排错调试
- `#[validate(length(max = X))]` 只支持**整数字面量**
- 不支持 Rust 表达式：`length(max = 10 * 1024 * 1024)` ❌
- 必须用：`length(max = 10_485_760)` ✅

[子代理协作模式]
- Date: 2026-06-24
- Context: 批次 B/C 死代码清理 8 轮并行时总结
- Category: 工作流协作
- 大批量相似任务（如 40 个文件清理）使用 8 轮 × 5 个子代理的并行结构
- 子代理仅**编辑文件**，不直接推 PR；主代理汇总后开 1 个 PR
- 子代理不得操作 `.monkeycode/` 目录或 `CHANGELOG.md`（避免污染记忆）

[子代理 sea_orm 清理警示]
- Date: 2026-06-23
- Context: 批次 B 子代理误删 `inventory_stock_txn.rs` 的 `QueryFilter`/`UpdateMany` 导入
- Category: 排错调试
- 子代理清理 sea_orm trait 导入时**必须**先 grep 使用点，再决定是否删除
- 历史教训：批次 B 经历 2 次 fixup 才恢复

---

## 六、工作流协作

[工作角色定位]
- Date: 2026-05-27
- Category: 工作流协作
- 主代理角色：总控（项目经理/架构师）
- 子代理（Task 工具）= 员工，负责具体执行
- 主代理职责：分析任务 → 拆解 → 分配 → 总结成果 → 推 PR
- 不直接写代码，而是分配给员工执行

[GitHub 分支策略]
- Date: 2026-06-16
- Category: 版本控制
- `main` 为主分支（正式版），不允许删除
- `test` 为测试分支，不允许删除
- 所有修复/功能变更在 test 分支进行
- 验证后自动合并入 main
- 修复分支合并后自动删除

[提交信息规范]
- Date: 2026-06-19
- Category: 版本控制
- 使用中文编写提交信息
- 描述"做了什么"和"为什么"

[代码审查]
- Date: 2026-06-19
- Category: 版本控制
- 所有代码变更需经过审查
- 审查重点：代码质量、安全性、性能、测试覆盖

[日志诊断技能自动触发]
- Date: 2026-06-07
- Category: 工作流协作
- 技能名：`/log-diagnosis` 日志诊断技能（自动触发）
- 触发关键词：日志、错误日志、异常日志、崩溃日志、服务器日志、traceId、错误码、异常堆栈
- 核心规则：全量原则、上下文原则、代码验证原则、报告原则、配置优先原则
- 报告保存：`.diagnosis/reports/{YYYY-MM-DD}_{问题描述}.md`

---

## 七、代码规范

[命名约定]
- Date: 2026-06-19
- Category: 开发规范
- 使用有意义、描述性的名称
- 遵循项目或语言的命名规范
- 避免缩写和单字母变量（除约定俗成的，如循环中的 `i`）

[代码组织]
- Date: 2026-06-19
- Category: 开发规范
- 相关代码放在一起
- 保持适当的抽象层次
- 函数只做一件事，保持单一职责原则

[注释与文档]
- Date: 2026-06-19
- Category: 开发规范
- 注释解释"为什么"而不是"做什么"
- 为公共 API 提供清晰的文档
- 保持文档与代码同步更新

[死代码处理规范]
- Date: 2026-06-19
- Category: 开发规范
- **禁止**文件级 `#![allow(dead_code)]` 全局抑制（CI 会失败）
- **禁止**crate 级 `#![allow(unused_imports)]` / `#![allow(unused_variables)]`
- 真正未使用项**显式删除**（git 保留历史）；保留项加 `pub` 修饰或 `#[allow(dead_code)]` + TODO
- **例外**：`backend/src/models/` 下的 SeaORM 自动生成模型可保留文件级 `#![allow(dead_code)]`
- 详细规范：见 `docs/superpowers/plans/2026-06-23-clippy-deadcode-cleanup-plan.md`

[CI 死代码强制]
- Date: 2026-06-19
- Category: 开发规范
- 配置：`backend/.clippy.toml` `warn` 段开启 `dead_code`/`unused_imports`/`unused_variables`
- 工作流：`.github/workflows/ci-cd.yml` `cargo clippy --all-targets -- -D warnings`
- 任何死代码警告都会让 CI 失败

---

## 八、性能与错误处理

[数据库查询]
- Date: 2026-06-19
- Category: 性能规范
- 优化查询，避免 N+1
- 使用适当索引
- 大数据量查询分页处理

[缓存策略]
- Date: 2026-06-19
- Category: 性能规范
- 合理使用缓存，明确失效策略
- 避免缓存过期数据

[资源管理]
- Date: 2026-06-19
- Category: 性能规范
- 及时释放不再使用的资源
- 避免内存泄漏
- 合理控制并发数量

[错误处理]
- Date: 2026-06-19
- Category: 开发规范
- 业务错误：返回友好提示
- 系统错误：记录详细日志，返回通用错误
- 验证错误：明确指出失败原因
- 尽可能实现优雅降级，提供重试机制

---

## 九、文档与持续改进

[API 文档]
- Date: 2026-06-19
- Category: 文档规范
- 所有 API 接口必须有文档：接口路径、请求参数、响应格式、示例

[代码文档]
- Date: 2026-06-19
- Category: 文档规范
- 复杂逻辑必须有注释说明
- 公共函数必须有文档注释
- 保持文档与代码同步更新

[持续改进]
- Date: 2026-06-19
- Category: 开发规范
- 定期审查代码质量，及时重构
- 记录技术债务，制定偿还计划
- 关注新技术发展，定期团队分享

---

## 十、近期关键 PR 索引（2026-06-23 ~ 2026-06-24）

| PR | 标题 | 合并 commit | 状态 |
|----|------|-------------|------|
| #245 | 批次 A dead_code 清理（20 高频文件） | a3f6a978 | ✅ |
| #246 | 批次 B dead_code 清理（30 中频文件） | c274a5c4 | ✅ |
| #247 | 批次 C dead_code 清理（40 低频文件 + 12 测试导入） | f524dad7 | ✅ |
| #248 | CI 错误修复（E0599 + clippy baseline） | cd7f6b5e | ✅ |

### 安全漏洞修复总览（4 waves / 14 漏洞）

| Wave | 等级 | 漏洞 | PR | commit |
|------|------|------|----|--------|
| Wave 1 | P0 | #1 #2 | #240 | b298c99 |
| Wave 2 | P1 | #3 #4 #6 #9 | #241 | cdb2ada |
| Wave 3 | P2 | #7 #8 | #242 | 2ab793c |
| Wave 4 | P3 | #5 #10 #11 #12 #13 #14 | #243 | 37ce64e |

详细修复内容：见 `docs/archives/`

---

## 十一、最近 PR 经验要点

[PR #245 批次 A 经验]
- 20 个高频 dead_code 文件清理
- `backend/src/services/enhanced_logger.rs` 从 401 行减至 122 行
- 删除旧 `backend/.clippy-baseline.txt`（行号偏移失效）

[PR #246 批次 B 经验]
- 30 个中高频文件清理
- 修复集成测试编译错误：`PricingContext` 加 `Serialize` 派生、`match_tier_for_unit_test` 改 `pub`
- 误删 `inventory_stock_txn.rs` 的 `QueryFilter`/`UpdateMany` → 2 次 fixup 恢复
- 删除损坏的 clippy baseline（246 个"新警告"误报）

[PR #247 批次 C 经验]
- 40 个低频文件 + 12 个集成测试导入修复（`use crate::` → `use bingxi_backend::`，共 20 处）
- 8 轮 × 5 子代理并行结构
- 再次发现并删除损坏的 clippy baseline（970 个"新警告"误报）

[PR #248 CI 错误修复经验]
- `color_price_crud_test.rs:90` 错误调用 `active.is_active.is_ok()`（类型是 `ActiveValue<bool>`，不是 `Result`）
- 修复：`match &active.is_active { sea_orm::ActiveValue::Set(v) => assert_eq!(*v, false), _ => panic!(...) }`
- 删除损坏的 clippy baseline（基线 441 行只有辅助文本，无警告摘要行）
- 根本原因：CI 脚本 `sort -u` 处理多行 `rendered` 字段失效
- **TODO 改进**：CI 改用 `jq` 提取结构化标识符（`code` + `message` + `span`）作为基线条目

[14 个安全漏洞修复总览]
- 见 `docs/archives/CHANGELOG-2026-06-24-pre-optimization.md` 详细修复内容
- 关键经验：CSRF Token 需 IP 绑定 + 强制轮换；错误响应体生产环境脱敏（移除 `error_type`/`detail`）

---

## 十二、归档索引

完整历史内容（优化前的详细记录）：

- 完整 MEMORY：`.monkeycode/docs/archives/MEMORY-2026-06-24-pre-optimization.md`
- 完整 doto：`.monkeycode/docs/archives/doto-2026-06-24-pre-optimization.md`
- 完整 CHANGELOG：`.monkeycode/docs/archives/CHANGELOG-2026-06-24-pre-optimization.md`

历史审计报告：
- `.monkeycode/docs/audits/2026-06-19-*.md` —— 路由/API 审计
- `.monkeycode/docs/audits/2026-06-19-modern-code-audit.md` —— 现代代码质量审计（73/100）
- `.monkeycode/docs/audits/2026-06-19-clippy-deep-prediction.md` —— Clippy 死代码深度预判
- `.monkeycode/docs/audits/2026-06-22-runtime-issues-detection.md` —— 项目真实运行问题检测（80/100）

历史规划：
- `.monkeycode/docs/superpowers/plans/2026-06-23-clippy-deadcode-cleanup-plan.md`
- `.monkeycode/docs/superpowers/plans/2026-06-24-clippy-deadcode-batch-bc-plan.md`

[GitHub Token 安全]
- Date: 2026-06-24
- Context: 健康检查发现 Token（`ghu_` 前缀）明文存储在 .git/config
- Category: 环境配置
- **风险**：该 Token 拥有 57231307/1 与 57231307/2 两个仓库的 **admin 权限**，泄露可推送任意代码
- **违规**：违反项目安全规范"禁止在代码中硬编码敏感信息"
- **修复指南**：见 `.monkeycode/docs/archives/2026-06-24/token-rotation-2026-06-24.md`
- **推荐方式**：SSH Key 认证（`git@github.com:57231307/1.git`）优于 HTTPS + Token
- **降级方案**：环境变量 `GITHUB_TOKEN` + 启动脚本加载
- **重要提醒**：仓库中**严禁**提交真实 Token 字符串（GitHub Secret Scanning 会阻止 push）
- **检查方法**：`git remote -v` 不应出现 token 字符串
- **沙箱执行记录（2026-06-24 14:10 UTC）**：
  - 已生成专用 SSH key `/root/.ssh/github_bingxi`（ed25519，fingerprint `SHA256:lWfrC60FouzfR7pF9KHnHjutL1S5WTpQW+gQTdFhdbw`）
  - `/root/.ssh/config` 已配置：限定 github.com 使用专用 key（`IdentitiesOnly yes`）
  - .git/config remote URL 已从 `https://x-access-token:...@github.com/...` 切换到 `git@github.com:57231307/1.git`
  - 明文 Token 已从 .git/config 移除（本地暴露风险已消除）
  - 公钥位置：`.monkeycode/docs/archives/2026-06-24/ssh-public-key-2026-06-24.md`
  - 待用户操作：注册公钥到 GitHub + 撤销旧 Token
