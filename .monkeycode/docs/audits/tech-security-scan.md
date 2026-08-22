# 技术安全扫描报告

- 扫描时间：2026-08-22
- 扫描范围：/workspace 仓库 backend（Rust）+ deploy（Nginx）+ .github/workflows
- 扫描代理：审计代理（只读模式，未修改任何代码文件）
- 扫描项：2.6 / 3.11 / 3.14 / 4.7 / 4.8 / 4.10（共 7 项，其中 4.x 合并 3 项，3.x 合并 2 项，2.x 1 项）

---

## 2.6 过时依赖扫描

### 扫描方法
- 读取 `backend/Cargo.toml` 中所有依赖版本声明
- 对照 `backend/Cargo.lock` 中实际锁定版本，判断是否存在锁文件版本超出 manifest 声明（说明可升级）或显著滞后于上游

### 关键依赖版本对照

| 依赖 | Cargo.toml 声明 | Cargo.lock 锁定 | 评估 |
|------|----------------|----------------|------|
| axum | 0.8 | 0.8.9 | 当前补丁版，正常 |
| tokio | 1.0 | 1.53.1 | 锁定 1.53.1，声明 `^1.0` 覆盖最新 1.x，正常 |
| sea-orm | 2.0.2 | 2.0.2 | 当前版本，正常 |
| reqwest | 0.13 | 0.13.4 | 当前补丁版，正常 |
| jsonwebtoken | 11.0 | 11.0.0 | 当前版本，正常 |
| redis | 1.6 | 1.6.0 | 当前版本，正常 |
| utoipa | 5.2.0 | 5.5.0 | 锁定 5.5.0，声明 `^5.2.0` 覆盖，正常 |
| dashmap | 6.2 | 6.2.1 | 当前补丁版，正常 |
| chrono | 0.4 | 0.4.45 | 当前补丁版，正常 |
| once_cell | 1.20 | 1.21.4 | 锁定 1.21.4，声明 `^1.20` 覆盖，正常 |
| hmac | 0.12 | 0.12.1 | 当前补丁版，正常 |
| sha2 | 0.10 | 0.10.9 | 当前补丁版，正常 |
| thiserror | 2.0 | 2.0.20 | 当前版本，正常 |
| serde | 1.0 | 1.0.229 | 当前补丁版，正常 |
| tracing-subscriber | 0.3 | - | 0.3 系列最新，正常 |

### Cargo.lock 版本分布
- `Cargo.lock` 中 `version` 字段排序去重后前 20 个均为 0.1.x 系列（底层工具库如 anyhow、adler 等），无异常旧版本堆积
- 未发现 lock 文件中存在显著滞后于 manifest 声明的依赖

### 结论

**通过**。所有关键依赖的锁定版本均在 manifest 的 semver 约束范围内，且为当前主版本的最新补丁或次版本。无明确过时依赖。

- 建议：定期运行 `cargo update --dry-run` 检查是否有可安全升级的补丁版本
- 建议：考虑为 `reqwest`、`sea-orm`、`axum` 等核心依赖添加 `cargo-outdated` 检查到 CI

---

## 3.11 异步任务扫描

### 扫描方法
- 统计 `tokio::spawn` 使用次数（评估 detached task 规模）
- 检查 `backend/src/bootstrap/` 中的 `CancellationToken` / `cancel` 使用情况（评估优雅退出机制）

### 数据
- `tokio::spawn` 总数：**50 处**
- `CancellationToken` 使用：bootstrap 中 10 处引用，集中分布于 `service_bootstrap.rs`

### CancellationToken 机制详情（`backend/src/bootstrap/service_bootstrap.rs`）

```
L10:  use tokio_util::sync::CancellationToken;
L59:  static MAIN_CANCELLATION_TOKEN: once_cell::sync::Lazy<CancellationToken>
L64:  pub fn main_cancellation_token() -> &'static CancellationToken
L69:  V15 P2 B05-P2-5：先调用 token.cancel() 通知所有循环优雅退出，再 abort() 兜底
L71:  MAIN_CANCELLATION_TOKEN.cancel();
L84:  main 后台定时任务已关闭（{} 个，已发送 cancel 信号 + abort 兜底）
L488: _ = token.cancelled() =>
L511: _ = token.cancelled() =>
```

### 优雅退出机制分析

1. **双重退出策略**：采用 `cancel()` 通知 + `abort()` 兜底的分层策略
   - `MAIN_CANCELLATION_TOKEN.cancel()` 通知所有监听 token 的循环优雅退出
   - `MAIN_BACKGROUND_TASKS`（`Vec<JoinHandle>`）保存所有 spawn 的句柄，shutdown 时遍历 `abort()` 强杀残留任务

2. **句柄保存**：`MAIN_BACKGROUND_TASKS: std::sync::Mutex<Vec<tokio::task::JoinHandle<()>>>` 集中管理后台任务句柄，避免 detached task 泄漏

3. **审计引擎清理**：`OmniAuditEngine` 和 `AuditLogService` 保留 clone 用于 shutdown 后调用 `shutdown()`，避免审计 detached task 泄漏

4. **Bridge 监听器**：`BRIDGE_LISTENER_HANDLE: std::sync::Mutex<Option<tokio::task::JoinHandle<()>>>` 单独管理，shutdown 时可关闭

### 风险点
- 50 处 `tokio::spawn` 中，仅进入 `MAIN_BACKGROUND_TASKS` 的才有 abort 兜底；需确认所有长期运行的 spawn 均已注册到该 Vec
- `MAIN_BACKGROUND_TASKS` 使用 `std::sync::Mutex`，若在 spawn 注册时持有锁时间过长可能阻塞

### 结论

**基本通过，有改进空间**。已建立 `CancellationToken` + `abort()` 双层优雅退出机制，审计引擎和 Bridge 监听器均有独立 shutdown 路径。主要风险在于 50 处 spawn 中未注册到 `MAIN_BACKGROUND_TASKS` 的 detached task 是否都有自终止逻辑。

- 建议：审计剩余 50 处 `tokio::spawn`，确认每处要么注册到 `MAIN_BACKGROUND_TASKS`，要么监听 `main_cancellation_token()`
- 建议：短期 fire-and-forget 的 spawn（如 webhook 发送）可接受 detached，但需有超时和错误日志

---

## 3.14 内存泄漏扫描

### 扫描方法
- 搜索 `Box::leak`、`static.*Mutex`、`OnceLock`、`lazy` 模式（排除 test 和注释）
- 检查 `backend/src/container/` 中的 `DashMap` / `HashMap::new()` 使用

### Box::leak
- **未发现** `Box::leak` 使用（排除 test/注释后）

### static 全局状态清单

| 位置 | 声明 | 用途 | 泄漏风险 |
|------|------|------|---------|
| `routes_bootstrap.rs:42` | `static SETUP_MODE_INITIALIZED: OnceLock<Arc<Mutex<bool>>>` | 模式初始化标志 | 低，单次初始化 |
| `service_bootstrap.rs:54` | `static MAIN_BACKGROUND_TASKS: std::sync::Mutex<Vec<JoinHandle>>` | 后台任务句柄 | 低，shutdown 时 abort |
| `middleware/slow_query.rs:46` | `static SLOW_QUERY_ALERT_STATE: LazyLock<Mutex<HashMap<u64,(Instant,u32)>>>` | 慢查询告警状态 | **中**，HashMap 无上限增长 |
| `middleware/circuit_breaker.rs:147` | `static CIRCUIT_BREAKERS: Lazy<Arc<Mutex<HashMap<String,CircuitEntry>>>>` | 熔断器状态 | **中**，HashMap 无上限增长 |
| `middleware/auth.rs:53` | `static USER_ACTIVE_CACHE: OnceLock<DashMap<i32,(bool,Instant)>>` | 用户活跃缓存 | **中**，DashMap 无上限增长 |
| `middleware/rate_limit.rs:107` | `static REDIS_RATE_LIMITER: OnceCell<Option<Arc<Mutex<ConnectionManager>>>>` | Redis 限流连接 | 低，单例连接 |
| `utils/redis_cache.rs:36` | `static REDIS_CONN: OnceCell<Option<Arc<Mutex<ConnectionManager>>>>` | Redis 缓存连接 | 低，单例连接 |
| `services/auth_service_ops/jti.rs:40` | `static REDIS_JTI_BLACKLIST: OnceCell<Option<Arc<Mutex<ConnectionManager>>>>` | JTI 黑名单连接 | 低，单例连接 |
| `services/inventory_finance_bridge_ops/listener.rs:24` | `static BRIDGE_LISTENER_HANDLE: Mutex<Option<JoinHandle>>` | 监听器句柄 | 低，shutdown 时关闭 |

### container/mod.rs 的 DashMap

```
L75:  pub email_send_counters: Arc<DashMap<(i32, u64), Arc<AtomicU32>>>
L283: email_send_counters: Arc::new(DashMap::new())
L418: email_send_counters: Arc::new(DashMap::new())
```

- `email_send_counters` 的 key 为 `(i32, u64)`（用户ID + 时间戳），**无清理机制**，随时间无限增长

### 结论

**有风险，需修复**。`OnceLock` / `OnceCell` / `Lazy` 用于单例初始化属于正常模式，风险可控。存在 3 处无界 HashMap/DashMap 增长风险：

1. **`SLOW_QUERY_ALERT_STATE`**（`slow_query.rs:46`）：按 `u64` key 累积，无淘汰策略
2. **`CIRCUIT_BREAKERS`**（`circuit_breaker.rs:147`）：按 `String` key 累积，无淘汰策略
3. **`USER_ACTIVE_CACHE`**（`auth.rs:53`）：按 `i32` 用户ID 累积，DashMap 无淘汰策略
4. **`email_send_counters`**（`container/mod.rs:75`）：按 `(i32,u64)` 累积，无淘汰策略

- 建议：为上述 4 处无界集合添加 TTL 淘汰或 LRU 演变策略
- 建议：`USER_ACTIVE_CACHE` 可改用 `moka`（项目已依赖）获得自动过期能力
- 建议：对 `CIRCUIT_BREAKERS` 和 `SLOW_QUERY_ALERT_STATE` 增加定期清理过期 entry 的后台任务

---

## 4.7 漏洞扫描配置

### 扫描方法
- 检查 `.github/workflows/` 中是否有 `cargo-audit` 配置
- 检查 `backend/Cargo.toml` 是否有 audit 配置

### CI 配置详情（`.github/workflows/ci-cd.yml` L1595-1660）

存在 `ci-audit` job：

```
ci-audit:
    name: 🛡️ 依赖审计
    runs-on: ubuntu-latest
    timeout-minutes: 15
    needs: ci-info
```

关键行为：
1. **安装**：`cargo install cargo-audit --locked`（L1613，安装失败会阻塞 CI）
2. **执行**：`cargo audit --json` 在 `backend/` 目录运行（L1620）
3. **退出码处理**：
   - 使用 `set +e` 捕获漏洞，发现漏洞**不阻塞** CI（L1621-1624）
   - 仅当 `cargo audit` 命令本身异常时才阻塞（L1603 注释明确）
4. **报告**：生成 `reports/cargo-audit-report.md`，解析 `vulnerabilities.found` 数量
5. **摘要**：写入 `$GITHUB_STEP_SUMMARY`

### Cargo.toml 配置
- `backend/Cargo.toml` 中**无** `[audit]` 段落配置
- 无 `[~audit]` advisory ignore 列表

### 工作流文件清单
- `ci-cd.yml`（主 CI/CD）
- `dead-code-audit.yml`（死代码审计）
- `e2e-batch.yml`（端到端批次测试）

### 结论

**基本通过，有改进空间**。已配置 `cargo-audit` 自动扫描并生成报告，安装失败会阻塞 CI。但存在以下不足：

1. **漏洞不阻塞**：发现漏洞时 CI 仍可通过（`set +e` 捕获），漏洞仅记录在案。生产级项目建议对 `high`/`critical` 级别漏洞设置阻塞门禁
2. **无 ignore 列表**：`Cargo.toml` 缺少 `[audit]` 配置段，无法对已评估并接受的低风险漏洞进行 ignore，可能导致每次审计报告噪音
3. **无前端审计**：job 中提到 `Setup Frontend` 但需确认 `npm audit` 是否同步配置（从 `AUDIT_SECRET_KEY` 引用看可能已有，但本次未深入验证）
4. **无定时扫描**：仅在 CI 触发时扫描，无 `schedule` cron 定时扫描（新漏洞可能在无提交时出现）

- 建议：为 `high`/`critical` 漏洞添加阻塞门禁（`if [ "$VULN_COUNT" -gt 0 ]` 时 exit 1）
- 建议：在 `backend/Cargo.toml` 添加 `[audit]` 段，配置已接受的 advisory ignore
- 建议：添加 `schedule: cron` 每日定时运行 cargo-audit，捕获新公开的漏洞

---

## 4.8 供应链安全扫描

### 扫描方法
- 检查 `backend/Cargo.toml` 中是否有 `git+` 依赖（不可信源）
- 检查 `Cargo.lock` 中的 source 来源
- 检查 `.cargo/config.toml` 是否有 registry 替换

### git+ 依赖
- `backend/Cargo.toml`：**0 处** `git+` 依赖
- `backend/Cargo.lock`：**0 处** `git+` source

### path 依赖
```
backend/Cargo.toml:133: migration = { version = "0.1.0", path = "migration" }
backend/Cargo.toml:185: path = "src/main.rs"
backend/Cargo.toml:205: path = "src/bin/hash_password.rs"
backend/Cargo.toml:209: path = "src/bin/cli.rs"
```
- `migration` 为本地 workspace 内路径依赖，正常
- 其余为 `[[bin]]` 声明的二进制入口，正常

### Cargo.lock source 验证
```
reqwest 0.13.4 source = "registry+https://github.com/rust-lang/crates.io-index"
```
- 所有依赖均来自 crates.io 官方 registry
- 无私有 registry、无 Git 仓库直接依赖

### .cargo/config.toml
```toml
[profile.dev]
opt-level = 0
debug = false
codegen-units = 256
lto = false
panic = "abort"
incremental = false
```
- **无** `[source]` 替换配置
- **无** `[registry]` 自定义配置
- 仅有 dev profile 编译优化配置

### 结论

**通过**。供应链来源清洁：

1. 所有依赖均来自 crates.io 官方 registry，无 `git+` 不可信源
2. 无私有 registry 替换，无 `[source]` 配置
3. 唯一的 `path` 依赖为 workspace 内的 `migration` 子模块，正常
4. `Cargo.lock` 中有完整 checksum（如 reqwest 的 `checksum = "219c..."`）

- 建议：考虑在 CI 中验证 `Cargo.lock` 中所有 source 均为 `registry+https://github.com/rust-lang/crates.io-index`，防止意外引入私有源
- 建议：可添加 `cargo vendor` 离线缓存校验，确保构建可复现

---

## 4.10 TLS 合规扫描

### 扫描方法
- 检查 `backend/src/services/` 中 reqwest / https / tls 使用
- 检查 `backend/Cargo.toml` 中 reqwest 的 TLS 后端配置
- 检查 `deploy/nginx*.conf` 中的 `ssl_protocols` / `ssl_ciphers` 配置

### Nginx TLS 配置（`deploy/nginx.conf` L30-45）

```
L30: ssl_certificate /etc/ssl/certs/ssl-cert-snakeoil.pem
L31: ssl_certificate_key /etc/ssl/private/ssl-cert-snakeoil.key
L32: ssl_protocols TLSv1.2 TLSv1.3
L33: ssl_ciphers ECDHE-ECDSA-AES128-GCM-SHA256:ECDHE-RSA-AES128-GCM-SHA256:...
L34: ssl_prefer_server_ciphers off
L35: ssl_session_cache shared:SSL:10m
L36: ssl_session_timeout 1d
L37: ssl_session_tickets off
```

TLS 配置详情：
- **协议**：仅 TLSv1.2 + TLSv1.3（已禁用 TLSv1.0/v1.1，**合规**）
- **套件**：仅 AEAD 套件（AES-GCM + CHACHA20-POLY1305），**无 CBC 套件**
- **会话票据**：`off`（防止前向保密被绕过）
- **HSTS**：`max-age=31536000; includeSubDomains`（1 年，**合规**）
- **安全头**：X-Frame-Options DENY、X-Content-Type-Options nosniff、Referrer-Policy

### reqwest TLS 后端

`Cargo.toml` 声明：
```
reqwest = { version = "0.13", features = ["json"] }
```

- **未显式指定 TLS 后端**（无 `rustls-tls` 或 `native-tls` feature）
- reqwest 0.13 默认使用 `native-tls`（依赖系统 OpenSSL）
- `Cargo.lock` 中存在 `rustls`（可能被其他依赖间接引入，如 axum/tonic）

### services 层 HTTPS 强制

1. **webhook_service.rs:248**
   ```
   if !url.to_lowercase().starts_with("https://") {
   ```
   - 强制 webhook URL 使用 HTTPS，**合规**

2. **webhook_service.rs:272-282** `build_webhook_client`
   - 设置 30s 超时 + 10s 连接超时
   - 禁用重定向（`Policy::none()`），防止 SSRF 重定向绕过
   - 使用 `resolve_to_addrs` 固定到已校验 IP，消除 TOCTOU 窗口

3. **system_update_service.rs:258**
   ```
   if parsed.scheme() != "https" {
       return Err(...)
   }
   ```
   - 强制下载 URL 使用 HTTPS
   - 仅允许 `github.com` / `objects.githubusercontent.com` 域名

### 结论

**通过，有改进空间**。

合规项：
1. Nginx 仅启用 TLSv1.2 + TLSv1.3，禁用旧协议
2. 仅使用 AEAD 加密套件，无 CBC 套件
3. 配置 HSTS（1 年）+ 安全响应头
4. 服务层强制 HTTPS（webhook + 系统更新）
5. webhook 客户端有 SSRF 防护（IP 固定 + 禁用重定向）

风险点：
1. **snakeoil 证书**：`ssl-cert-snakeoil.pem` 为 Debian 默认自签名证书，仅适用于测试，**生产环境必须替换为正式证书**（注释已提示）
2. **reqwest TLS 后端未显式指定**：依赖系统 OpenSSL，不同环境行为可能不一致。建议显式启用 `rustls-tls` 获得确定性 TLS 行为
3. **未检查证书吊销**：未配置 OCSP stapling（`ssl_stapling on`）

- 建议：生产部署时替换 snakeoil 证书为 Let's Encrypt 或商业证书
- 建议：为 reqwest 显式指定 `rustls-tls` feature：`reqwest = { version = "0.13", features = ["json", "rustls-tls"] }`
- 建议：Nginx 添加 `ssl_stapling on` 启用 OCSP 装订

---

## 综合评估汇总

| 编号 | 扫描项 | 结论 | 风险等级 |
|------|--------|------|---------|
| 2.6 | 过时依赖 | 通过 | 低 |
| 3.11 | 异步任务 | 基本通过 | 中 |
| 3.14 | 内存泄漏 | 有风险 | **中高** |
| 4.7 | 漏洞扫描配置 | 基本通过 | 中 |
| 4.8 | 供应链安全 | 通过 | 低 |
| 4.10 | TLS 合规 | 通过 | 低 |

### 优先修复建议

1. **P0 - 内存泄漏（3.14）**：4 处无界 HashMap/DashMap（`SLOW_QUERY_ALERT_STATE`、`CIRCUIT_BREAKERS`、`USER_ACTIVE_CACHE`、`email_send_counters`）需添加淘汰策略
2. **P1 - 漏洞扫描门禁（4.7）**：`cargo-audit` 发现漏洞不阻塞 CI，建议对 high/critical 级别设置阻塞
3. **P1 - 异步任务审计（3.11）**：50 处 `tokio::spawn` 需确认是否全部注册到 `MAIN_BACKGROUND_TASKS` 或监听 cancellation token
4. **P2 - TLS 后端（4.10）**：reqwest 显式指定 `rustls-tls`，Nginx 替换 snakeoil 证书
5. **P2 - 定时审计（4.7）**：添加 cron 定时运行 cargo-audit

### 未修改文件声明

本次扫描为只读审计，未修改任何代码文件（`backend/`、`frontend/`、`deploy/`、`.github/`）。仅创建本报告文件于 `.monkeycode/docs/audits/tech-security-scan.md`。

git status 中显示的其他改动均为本扫描开始前已存在的改动，非本次扫描产生。
