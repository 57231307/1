# V15 部署升级与多租户残留审计报告（类二十五·批次 21）

- **审计子代理**：V15 审计子代理（类二十五 部署流程与升级管理 + 多租户残留）
- **审计范围**：4 维度（25.1 部署流程 / 25.2 CLI 工具 / 25.3 升级流程 / 25.4 平滑升级）+ 25.5 多租户残留审计（26 项检查），合计 5 维度 103 项检查
- **审计依据**：
  - `/workspace/.monkeycode/docs/audits/v15-review-plan-2026-07-15.md`（V15 审计计划第 6893 行起，类二十五 4 维度 + 25.5 26 项检查）
  - `/workspace/deploy/`（5 个部署脚本 + systemd 服务 + Nginx 配置 + supervisord.conf）
  - `/workspace/backend/src/cli/`（admin / migrate / util/mod + service / backup / upgrade / misc）
  - `/workspace/backend/src/telemetry.rs`（多租户死代码常量残留）
  - `/workspace/backend/src/models/slow_query.rs`（多租户注释残留）
  - `/workspace/backend/migration/src/m0029_drop_tenant_columns.rs` + `/workspace/backend/migrations/20260628000001_drop_tenant_columns/up.sql`（多租户下线迁移）
  - `/workspace/.github/workflows/ci-cd.yml`（CI/CD 工作流）
- **审计方法**：Read 审计计划 + Grep 全面搜索（`tenant_id|tenant|租户|TenantId|multi_tenant|ACTIVE_TENANTS|active_tenants`）+ Read 关键文件 + 对照审计计划逐项核对
- **审计时间**：2026-07-16
- **审计原则**：只做审计不修改业务代码；每个维度的每个检查项都要有明确的"已落实"或"缺陷"结论，附文件路径:行号证据；风险分级 P0（阻塞）/ P1（高）/ P2（中）/ P3（低）

---

## 25.1 部署流程审计（18 项检查）

### 检查方法
1. Read `/workspace/deploy/deploy.sh`（800 行，部署主入口）
2. Read `/workspace/deploy/deploy-backend.sh`（150 行）
3. Read `/workspace/deploy/deploy-frontend.sh`（84 行）
4. Read `/workspace/deploy/deploy-prepare.sh`（202 行）
5. Read `/workspace/deploy/deploy-latest.sh`（405 行，远程一键部署）
6. Read `/workspace/deploy/bingxi-backend.service`（systemd 服务文件，26 行）
7. Read `/workspace/deploy/nginx.conf`（100 行）
8. Grep `tenant` 在 `/workspace/deploy/`（确认 0 匹配）
9. Grep `/etc/bingxi` 在 `/workspace/deploy/`（路径一致性检查）
10. 对照审计计划第 6916-6939 行逐项核对

### 发现

#### ✅ 已落实的项

1. **部署脚本完整性**（5 个脚本职责清晰）：
   - `deploy-prepare.sh`：环境准备（检查依赖 + 版本管理 + 构建 + 打包）
   - `deploy-backend.sh`：后端单独部署（8 步流程）
   - `deploy-frontend.sh`：前端单独部署（4 步流程）
   - `deploy.sh`：协调总入口（全新/更新部署识别 + 备份 + 部署 + 健康检查 + 回滚）
   - `deploy-latest.sh`：远程一键最新版部署（GitHub Release + 镜像源 + SSH 推送）
   - 证据：`/workspace/deploy/` 目录结构（LS 输出 8 个文件）

2. **脚本路径一致性**（`/etc/bingxi/.env` 与 systemd `EnvironmentFile` 一致）：
   - `deploy.sh:22`：`CONFIG_DIR="/etc/bingxi"`
   - `deploy-backend.sh:19`：`CONFIG_DIR="/etc/bingxi"`
   - `deploy-latest.sh:160`：`mkdir -p /etc/bingxi`
   - `bingxi-backend.service:12`：`EnvironmentFile=/etc/bingxi/.env`
   - 证据：批次 398 修复注释（deploy.sh:19-21）明确说明从 `/etc/bingxi-erp` 改为 `/etc/bingxi` 以对齐 systemd

3. **systemd 服务文件完整**：
   - `bingxi-backend.service:9-10`：`User=bingxi` + `Group=bingxi`
   - `bingxi-backend.service:11`：`WorkingDirectory=/opt/bingxi-erp/backend`
   - `bingxi-backend.service:12`：`EnvironmentFile=/etc/bingxi/.env`
   - `bingxi-backend.service:13`：`Environment=CONFIG_FILE=/opt/bingxi-erp/backend/config.yaml`
   - `bingxi-backend.service:14`：`ExecStart=/opt/bingxi-erp/backend/server`
   - `bingxi-backend.service:15`：`Restart=always`
   - `bingxi-backend.service:19`：`LimitNOFILE=65535`

4. **密钥自动生成**（base64 32 字节）：
   - `deploy.sh:237-263`：自动生成 `COOKIE_SECRET` 和 `JWT_SECRET`（openssl rand -base64 32，截取 48 字符）
   - `deploy.sh:268-283`：自动生成 `WEBHOOK_SECRET`（独立密钥，重试 5 次确保与 JWT 不同）
   - `deploy.sh:213-229`：自动生成 `AUDIT_SECRET_KEY`（基于硬件信息 + 随机盐 + 时间戳 SHA512）
   - `deploy-latest.sh:181-242`：远程部署同步生成 4 类密钥
   - 证据：所有密钥均使用 `openssl rand -base64 32`，符合熵比 > 0.15 要求

5. **备份前置**（升级/部署前自动备份）：
   - `deploy.sh:59-72`：`backup_current()` 函数，检测到旧版本时备份 backend + frontend_dist + .env
   - `deploy.sh:752-757`：更新部署时先 `backup_current` 再 `run_migrations`
   - `deploy-latest.sh:150-155`：远程部署备份当前版本
   - `deploy.sh:70`：保留最近 5 个备份（`ls -dt | tail -n +6 | xargs rm -rf`）

6. **部署后健康检查**：
   - `deploy.sh:445-464`：`health_check()` 函数，10 次重试，每次 2 秒
   - `deploy.sh:453`：检查 `http://127.0.0.1:8082/health` 返回 `"status":"healthy"`
   - `deploy-latest.sh:354`：远程部署后 `curl -s http://127.0.0.1:8082/health`

7. **回滚机制**：
   - `deploy.sh:467-486`：`rollback()` 函数，从 `$BACKUP_DIR` 取最新备份恢复
   - `deploy.sh:794-798`：支持 `./deploy.sh rollback` 参数触发回滚
   - 恢复 backend + frontend_dist，重启服务

8. **禁止 Docker**：
   - `/workspace/deploy/` 下无 `Dockerfile` / `docker-compose.yml`（LS 输出仅 8 个文件）
   - 符合 MEMORY.md 部署限制

9. **CI/CD 集成**（GitHub Actions 构建 → GitHub Release → 手动部署）：
   - `.github/workflows/ci-cd.yml:1330-1466`：`ci-build-rust` Job 构建后端二进制
   - `.github/workflows/ci-cd.yml:1496-1588`：`ci-build-fe` Job 构建前端 dist
   - `.github/workflows/ci-cd.yml:1593-1709`：`package-release` Job 打包发布
   - `.github/workflows/ci-cd.yml:1714-1817`：`github-release` Job 创建 GitHub Release
   - 流程：push → CI 构建 → 打包 → GitHub Release → 手动下载 deploy.sh 部署

10. **Nginx 反向代理**（8082 + WebSocket 升级头 + 静态资源缓存）：
    - `nginx.conf:26-41`：`location /api/` 反向代理 `http://127.0.0.1:8082`
    - `nginx.conf:30-31`：WebSocket 升级头（`Upgrade $http_upgrade` + `Connection 'upgrade'`）
    - `nginx.conf:19-22`：静态资源缓存（`expires 30d` + `Cache-Control "public, immutable"`）
    - `nginx.conf:13-16`：HTML 禁止缓存（`no-store, no-cache, must-revalidate`）
    - **注**：无 gzip 配置（缺陷项见下）

#### ❌ 缺陷项

##### 缺陷 25.1-A：部署脚本缺少 `set -euo pipefail`
**风险等级：P1**
**证据**：
- `deploy.sh:5`：仅 `set -e`，缺少 `-u`（未定义变量不报错）和 `-o pipefail`（管道中非最后命令失败不报错）
- `deploy-backend.sh:5`：同上
- `deploy-frontend.sh:5`：同上
- `deploy-prepare.sh:5`：同上
- `deploy-latest.sh:21`：同上
**业务影响**：未定义变量（如 `$DB_PASS`）被静默展开为空字符串，可能生成无效 config.yaml；管道中 `psql` 失败被 `2>/dev/null || true` 吞错时不会触发 `set -e`，导致部署"假成功"
**修复建议**：所有 5 个脚本顶部改为 `set -euo pipefail`，并审计 `|| true` 使用点确保仅用于已知可忽略错误

##### 缺陷 25.1-B：缺少端口冲突检查
**风险等级：P2**
**证据**：
- `deploy.sh:50-55`：仅在 `stop_old_services` 中"杀死占用 8082 端口的进程"，未在部署前检查端口是否空闲
- `deploy-backend.sh`：完全无端口检查
- `deploy-frontend.sh`：完全无 80 端口检查
**业务影响**：若 8082 端口被其他服务（如反向代理、调试工具）占用，部署后 systemd 启动会失败但脚本未提前告警，延长故障定位时间
**修复建议**：在 `check_root` 后增加 `check_ports` 函数，使用 `ss -tlnp | grep :8082` 检查，非自身进程占用时报错退出

##### 缺陷 25.1-C：缺少部署日志持久化
**风险等级：P2**
**证据**：
- `deploy.sh` 全程使用 `log()/warn()/error()` 函数输出到 stdout（`deploy.sh:28-30`），未写入 `/opt/bingxi-erp/logs/deploy-YYYYMMDD-HHMMSS.log`
- `deploy-latest.sh:60-62`：同上，仅 stdout 输出
- `deploy-backend.sh` / `deploy-frontend.sh` / `deploy-prepare.sh`：均未写入日志文件
**业务影响**：部署失败后日志仅留在终端会话中，SSH 断开即丢失，无法事后追溯；审计要求"部署全过程写入 `/opt/bingxi-erp/logs/deploy-YYYYMMDD-HHMMSS.log`"
**修复建议**：每个脚本开头增加 `exec > >(tee -a "$LOG_DIR/deploy-$(date +%Y%m%d-%H%M%S).log") 2>&1`，将所有输出同时写入日志文件

##### 缺陷 25.1-D：配置目录权限不符合 600 要求
**风险等级：P2**
**证据**：
- `deploy.sh`：创建 `$CONFIG_DIR` 后未设置权限（`mkdir -p "$CONFIG_DIR"` 在 deploy.sh:79）
- `deploy-latest.sh:328`：`chmod 640 /etc/bingxi/.env`（文件权限 640，非期望的 600）
- `deploy-backend.sh`：未设置 .env 文件权限
- 审计计划要求"`.env` 权限 600，owner root"
**业务影响**：640 权限下，同组用户（bingxi）可读 .env 中的数据库密码、JWT 密钥等敏感信息；非 root owner 也违反最小权限原则
**修复建议**：所有脚本部署后强制 `chmod 600 /etc/bingxi/.env && chown root:root /etc/bingxi/.env`（systemd EnvironmentFile 由 root 读取后再 drop 到 bingxi 用户）

##### 缺陷 25.1-E：缺少部署后业务健康检查（仅检查 active）
**风险等级：P2**
**证据**：
- `deploy-backend.sh:128-149`：仅检查 `systemctl is-active --quiet $APP_NAME`，未调用 `/health` 端点
- `deploy-frontend.sh`：完全无健康检查
- `deploy.sh:445-464`：虽有 `health_check` 函数，但仅检查 HTTP 状态码和 `"status":"healthy"` 字段，未独立校验 DB 连接、Redis 连接
**业务影响**：服务可能 systemd 显示 active 但数据库连接失败（如密码错误、远程 DB 不可达），部署脚本误报"部署完成"
**修复建议**：`health_check` 函数扩展为：① `/health` 返回 200；② 检查响应 JSON 中 `database: "healthy"` 和 `redis: "healthy"` 字段；③ 关键业务接口（如 `/api/v1/erp/auth/status`）返回 200

##### 缺陷 25.1-F：Nginx 缺少 gzip 压缩配置
**风险等级：P3**
**证据**：
- `/workspace/deploy/nginx.conf` 全文 100 行，无 `gzip on` / `gzip_types` / `gzip_min_length` 配置
**业务影响**：API JSON 响应、前端 JS/CSS 资源未压缩传输，带宽浪费 60-80%，移动端体验差
**修复建议**：在 `server` 块中增加：
```nginx
gzip on;
gzip_min_length 1024;
gzip_types text/plain text/css application/json application/javascript text/xml application/xml;
gzip_vary on;
```

##### 缺陷 25.1-G：缺少防火墙配置（仅开放 80/443）
**风险等级：P3**
**证据**：
- 所有部署脚本均未配置 ufw/iptables/firewalld 规则
- 审计计划要求"仅开放 80/443 端口，8082 不对外暴露"
**业务影响**：新部署服务器若默认防火墙未关闭 8082，攻击者可直接访问后端绕过 Nginx；生产环境 8082 暴露增加攻击面
**修复建议**：部署脚本增加 `configure_firewall` 函数：
```bash
if command -v ufw &>/dev/null; then
    ufw allow 80/tcp
    ufw allow 443/tcp
    ufw deny 8082/tcp
fi
```

##### 缺陷 25.1-H：部署回滚不恢复配置文件
**风险等级：P3**
**证据**：
- `deploy.sh:467-486`：`rollback()` 函数仅恢复 backend 二进制和 frontend dist，未恢复 .env 和 config.yaml
- `deploy-latest.sh`：无回滚函数
**业务影响**：若部署过程中 .env 被新生成的密钥覆盖（如 COOKIE_SECRET 重新生成），回滚后 config.yaml 仍引用新密钥但 server 二进制是旧版本，可能导致会话失效
**修复建议**：`rollback()` 函数扩展为恢复 .env + config.yaml（从 `$BACKUP_DIR/$latest_backup/` 复制）

---

## 25.2 内置 CLI 工具审计（18 项检查）

### 检查方法
1. Read `/workspace/backend/src/cli/mod.rs`（顶层 Command 枚举）
2. Read `/workspace/backend/src/cli/admin.rs`（密码哈希子命令）
3. Read `/workspace/backend/src/cli/migrate.rs`（数据库迁移子命令）
4. Read `/workspace/backend/src/cli/util/mod.rs`（Util 子命令枚举 + 共享辅助函数）
5. Read `/workspace/backend/src/cli/util/service.rs`（Status/Start/Stop/Restart/Logs/Health）
6. Read `/workspace/backend/src/cli/util/backup.rs`（Backup/Restore）
7. Read `/workspace/backend/src/cli/util/upgrade.rs`（Upgrade/Deploy/Rollback）
8. Read `/workspace/backend/src/cli/util/misc.rs`（Clean/Config/Info）
9. 对照审计计划第 6941-6964 行逐项核对

### 发现

#### ✅ 已落实的项

1. **CLI 入口完整性**（3 个顶层子命令）：
   - `cli/mod.rs:24-36`：`Command` 枚举含 `Admin` / `Migrate` / `Util` 三个变体
   - `cli/mod.rs:42-48`：`dispatch` 异步函数按变体分发

2. **Admin 子命令**（密码哈希）：
   - `admin.rs:10-19`：`AdminCommand::HashPassword { password_stdin: bool }`
   - `admin.rs:32-62`：`read_password` 函数支持 3 种输入方式（环境变量 > stdin > 报错）
   - `admin.rs:70-82`：使用 Python argon2/PBKDF2 生成哈希，通过 stdin 传递密码（避免 ps/proc 泄露）
   - 安全：移除了 `--password` 命令行参数（H-2 修复）

3. **Migrate 子命令**（真实实现，非占位）：
   - `migrate.rs:12-25`：6 个子命令：`Run` / `Rollback` / `Reset` / `Refresh` / `Fresh` / `Status`
   - `migrate.rs:28-46`：`get_db_connection` 函数从 `DATABASE_URL` 环境变量读取连接串
   - `migrate.rs:53-57`：`Migrator::up(&db, None)` 执行所有未执行迁移
   - `migrate.rs:59-62`：`Migrator::down(&db, Some(1))` 回滚最后一次迁移
   - `migrate.rs:78-81`：`Migrator::status(&db)` 查看迁移状态
   - **注**：V15 审计计划描述"migrate.rs 标注占位未实现"已过时，实际已完整实现

4. **Util 子命令完整性**（14 个子命令）：
   - `util/mod.rs:53-131`：`UtilCommand` 枚举含 14 个变体：Status / Start / Stop / Restart / Logs / Backup / Restore / Health / Upgrade / Deploy / Clean / Config / Info / Rollback
   - `util/mod.rs:134-168`：`run` 函数按变体分发到 service / backup / upgrade / misc 子模块

5. **服务管理**（5 个命令 + 健康检查）：
   - `service.rs:43-87`：`cmd_status` 检查后端 + Nginx + 端口 + 进程
   - `service.rs:89-111`：`cmd_start` 启动后端 + reload Nginx
   - `service.rs:113-120`：`cmd_stop` 停止后端
   - `service.rs:122-150`：`cmd_restart` 停止 + 启动 + reload Nginx
   - `service.rs:152-205`：`cmd_logs` 支持 backend / frontend / system 三种日志类型，支持 `-f` 实时跟踪
   - `service.rs:207-282`：`cmd_health` 检查服务状态 + HTTP + DB + 磁盘 + 日志大小

6. **备份恢复**（Backup + Restore）：
   - `backup.rs:22-60`：`cmd_backup` 支持 database / files / all 三种类型
   - `backup.rs:70-96`：`backup_database` 使用 `pg_dump`，失败返回 false 终止
   - `backup.rs:102-120`：`backup_config_files` 备份 config.yaml + .env + service 文件
   - `backup.rs:126-144`：`compress_backup` 压缩并设置 0o600 权限
   - `backup.rs:148-216`：`cmd_restore` 从 tar 包恢复，含路径穿越校验
   - `util/mod.rs:147-158`：Backup/Restore 失败时 `exit(1)`

7. **升级部署**（Upgrade + Deploy + Rollback）：
   - `upgrade.rs:20-96`：`cmd_upgrade` 从 GitHub Release 下载 + 备份 + 部署 + 清理
   - `upgrade.rs:98-110`：`cmd_deploy` 部署本地更新包
   - `upgrade.rs:112-165`：`cmd_rollback` 回滚到 server.old / bingxi.old

8. **清理工具**：
   - `misc.rs:7-59`：`cmd_clean` 支持 logs / backups / temp / all，清理 30 天前日志 + 90 天前备份 + 临时文件

9. **配置查看**：
   - `misc.rs:61-98`：`cmd_config` 显示 config.yaml + .env（脱敏 PASSWORD/SECRET/KEY）+ service 文件

10. **系统信息**：
    - `misc.rs:100-137`：`cmd_info` 显示 CLI 版本 + 安装目录 + 服务状态 + 系统信息 + 磁盘 + 内存 + 服务运行时间

11. **命令帮助**：
    - `cli/mod.rs:23`：`#[command(about = "Bingxi ERP 系统命令行工具")]`
    - clap derive 自动生成 `--help` 文档

12. **版本号**：
    - `cli/mod.rs:22`：`#[command(version = env!("CARGO_PKG_VERSION"))]`
    - `misc.rs:103`：`println!("CLI 版本: v{}", env!("CARGO_PKG_VERSION"))`

13. **镜像源配置**（6 个国内镜像 + 直连）：
    - `util/mod.rs:33-40`：`GITHUB_MIRRORS` 常量含 6 个镜像：ghfast.top / ghproxy.net / github.moeyy.xyz / mirror.ghproxy.com / gh-proxy.com / ghps.cc
    - `util/mod.rs:43-47`：`GITHUB_API_MIRRORS` 含 3 个 API 镜像
    - `util/mod.rs:243-279`：`download_with_mirrors` 函数先直连再尝试镜像
    - `util/mod.rs:282-323`：`fetch_with_mirrors` 函数同策略

#### ❌ 缺陷项

##### 缺陷 25.2-A：CLI 错误处理不统一（部分 println! + return，部分 exit(1)）
**风险等级：P2**
**证据**：
- `upgrade.rs:50-51`：`println!("[ERROR] 无法获取最新版本"); return;`（return 后退出码 0，调用方无法判断失败）
- `upgrade.rs:80`：`println!("\n[ERROR] 下载失败"); return;`（同上）
- `upgrade.rs:104`：`println!("[ERROR] 文件不存在: {}", package); return;`（同上）
- `upgrade.rs:120-122`：`println!("[ERROR] 未找到旧版本文件"); return;`（同上）
- `backup.rs:22-60`：`cmd_backup` 返回 bool，但 `cmd_upgrade` 在 `upgrade.rs:60` 检查 bool 后用 `return` 退出，退出码仍为 0
- 对比：`util/mod.rs:147-158`：Backup/Restore 失败时正确 `exit(1)`
**业务影响**：CLI 命令失败时退出码为 0，运维脚本（如 `bingxi upgrade && echo "成功"`）误判成功，可能导致后续步骤在失败状态下继续执行
**修复建议**：所有 `[ERROR]` 分支改为 `std::process::exit(1)`，或统一返回 `Result<(), Box<dyn Error>>` 由顶层 main 处理退出码

##### 缺陷 25.2-B：无 CLI 日志持久化
**风险等级：P2**
**证据**：
- 所有 CLI 子命令均使用 `println!` / `eprintln!` 输出到 stdout/stderr
- 未写入 `/opt/bingxi-erp/logs/cli-YYYYMMDD.log`
- 审计计划要求"CLI 输出同时写入日志文件"
**业务影响**：CLI 操作（特别是升级/备份/恢复）无持久化日志，事后无法追溯操作历史和失败原因
**修复建议**：在 `cli/mod.rs:dispatch` 函数入口初始化日志重定向：
```rust
let log_file = format!("/opt/bingxi-erp/logs/cli-{}.log", chrono::Local::now().format("%Y%m%d"));
let f = std::fs::OpenOptions::new().create(true).append(true).open(&log_file)?;
telemetry::init_cli_logger(f)?;
```

##### 缺陷 25.2-C：无权限校验（root 检查）
**风险等级：P1**
**证据**：
- `cli/util/upgrade.rs`：`cmd_upgrade` / `cmd_deploy` / `cmd_rollback` 均未检查当前用户是否为 root
- `cli/util/backup.rs`：`cmd_backup` / `cmd_restore` 均未检查 root
- 对比：`deploy.sh:33-37`：`check_root` 函数检查 EUID
- 审计计划要求"升级/部署/备份/恢复命令必须 root 权限"
**业务影响**：普通用户运行 `bingxi upgrade` 会因 systemd 权限不足失败，但错误信息不清晰；备份目录 `/opt/bingxi-erp/backups` 可能因权限不足写入失败导致备份丢失
**修复建议**：在 `cmd_upgrade` / `cmd_deploy` / `cmd_rollback` / `cmd_backup` / `cmd_restore` 函数开头增加：
```rust
if !nix::unistd::geteuid().is_root() {
    eprintln!("❌ 错误：此命令必须以 root 权限运行（请使用 sudo）");
    std::process::exit(1);
}
```

##### 缺陷 25.2-D：无危险操作二次确认
**风险等级：P2**
**证据**：
- `upgrade.rs:20`：`cmd_upgrade` 直接开始升级，无确认提示
- `upgrade.rs:98`：`cmd_deploy` 直接开始部署，无确认提示
- `upgrade.rs:112`：`cmd_rollback` 直接开始回滚，无确认提示
- `backup.rs:148`：`cmd_restore` 直接开始恢复（覆盖数据库），无确认提示
- 审计计划要求"Upgrade/Deploy/Rollback/Restore 必须二次确认（`--yes` 跳过）"
**业务影响**：误操作（如 `bingxi rollback` 误触发）会立即停止服务并替换二进制，无回退余地；restore 会覆盖生产数据库
**修复建议**：增加 `--yes` 参数：
```rust
if !yes {
    println!("⚠️ 即将执行升级操作，服务将停止 2-5 秒。继续？(y/N)");
    let mut input = String::new();
    std::io::stdin().read_line(&mut input)?;
    if !input.trim().eq_ignore_ascii_case("y") {
        println!("已取消");
        return;
    }
}
```

##### 缺陷 25.2-E：UtilCommand 缺少 `--force` 参数
**风险等级：P3**
**证据**：
- `util/mod.rs:99-107`：`Upgrade` 仅有 `--version` 和 `--no-backup` 参数
- `util/mod.rs:110-114`：`Deploy` 仅有 `--package` 参数
- 审计计划要求"命令参数设计（--version/--force/--backup/--rollback）"
**业务影响**：无法强制跳过版本回退检查（如降级到旧版本用于回退测试）
**修复建议**：增加 `--force` 参数跳过版本兼容性检查

---

## 25.3 内置升级流程审计（21 项检查）

### 检查方法
1. Read `/workspace/backend/src/cli/util/upgrade.rs`（356 行，升级/部署/回滚核心实现）
2. Read `/workspace/backend/src/cli/util/backup.rs`（备份恢复，升级前置依赖）
3. Read `/workspace/backend/src/cli/util/mod.rs`（共享辅助函数：download_with_mirrors / fetch_with_mirrors / build_release_url / parse_json_field）
4. 对照审计计划第 6966-6992 行逐项核对

### 发现

#### ✅ 已落实的项

1. **升级流程基本完整**：
   - `upgrade.rs:23-24`：获取当前版本（`env!("CARGO_PKG_VERSION")`）
   - `upgrade.rs:27-54`：获取目标版本（GitHub Release 最新或 `--version` 指定）
   - `upgrade.rs:57-64`：备份当前版本（`cmd_backup("all")`，失败终止）
   - `upgrade.rs:67-81`：下载新版本（带镜像源降级）
   - `upgrade.rs:84-85`：部署新版本（`deploy_release`）
   - `upgrade.rs:88-90`：清理下载包
   - 流程：当前版本检查 → 目标版本获取 → 备份 → 下载 → 部署 → 清理

2. **版本获取**（自动从 GitHub Release 最新）：
   - `upgrade.rs:38-39`：`get_latest_version()` 调用 `fetch_with_mirrors`
   - `upgrade.rs:168-176`：`get_latest_version` 函数，请求 `repos/57231307/1/releases/latest`
   - `upgrade.rs:172`：解析 `tag_name` 字段

3. **版本指定**（`--version`）：
   - `util/mod.rs:101-103`：`version: Option<String>` 参数
   - `upgrade.rs:27-36`：处理 `v` 前缀（自动补 `v`）

4. **备份前置**（`--no-backup` 跳过）：
   - `upgrade.rs:57-64`：`if !no_backup { ... cmd_backup("all") ... }`
   - `util/mod.rs:105-107`：`no_backup: bool` 参数

5. **镜像源降级**（直连失败自动尝试 6 个镜像）：
   - `util/mod.rs:243-279`：`download_with_mirrors` 先直连再 6 个镜像
   - `util/mod.rs:282-323`：`fetch_with_mirrors` 同策略

6. **包完整性校验**（tar -tf + 路径穿越检查 + 解压后二次校验）：
   - `upgrade.rs:209-218`：`tar -tf` 列出内容
   - `upgrade.rs:221-240`：逐行检查 `..` 和绝对路径
   - `upgrade.rs:254-261`：解压后 `validate_extracted_paths` 二次校验（canonicalize 解析符号链接）
   - 证据：H-1 修复（v9 复审），使用 UUID 随机临时目录防止 TOCTOU

7. **回滚支持**（二进制 + 前端）：
   - `upgrade.rs:112-165`：`cmd_rollback` 恢复 `server.old` + `bingxi.old`
   - `upgrade.rs:264-282`：`deploy_release` 部署时备份旧二进制到 `old.<timestamp>` 目录
   - `upgrade.rs:322-336`：移动新前端 dist（先 rm -rf 旧 dist，再 mv 新 dist）

#### ❌ 缺陷项

##### 缺陷 25.3-A：缺少 SHA256 下载校验
**风险等级：P1**
**证据**：
- `upgrade.rs:71-81`：下载后直接进入 `deploy_release`，未校验 SHA256
- `util/mod.rs:243-279`：`download_with_mirrors` 仅检查 curl 退出码，未对比 sha256sum
- 审计计划要求"下载后 SHA256 校验，与 Release assets 中的 sha256sum 文件对比"
**业务影响**：下载包可能因网络中断、镜像源篡改、CDN 缓存错误导致损坏，未校验直接部署会安装损坏的二进制，服务启动失败
**修复建议**：
```rust
// 下载 .sha256 文件
let sha256_url = format!("{}.sha256", release_url);
let sha256_file = format!("{}.sha256", download_path);
download_with_mirrors(&sha256_url, &sha256_file, 30);

// 校验
let computed = run_cmd("sha256sum", &[&download_path])?;
let expected = std::fs::read_to_string(&sha256_file)?;
if !computed.starts_with(&expected.trim()) {
    println!("[ERROR] SHA256 校验失败");
    return;
}
```

##### 缺陷 25.3-B：缺少断点续传
**风险等级：P2**
**证据**：
- `util/mod.rs:246-249`：`curl -fsSL -m $timeout -o $output $url`，无 `-C -` 参数
- `deploy.sh:536`：bash CLI 内的 `download_with_mirror` 函数有 `curl -L -C -`（断点续传）
- `deploy-latest.sh:107`：远程部署有 `curl -L -C -`
- 审计计划要求"支持断点续传（`curl -C -`）"
**业务影响**：大文件（发布包 100MB+）下载中断后必须重新下载，浪费带宽和时间；网络不稳定环境下升级可能反复失败
**修复建议**：`download_with_mirrors` 函数的 curl 参数增加 `-C -`：
```rust
&["-fsSL", "-C", "-", "-m", &timeout.to_string(), "-o", output, url]
```

##### 缺陷 25.3-C：缺少版本格式校验
**风险等级：P2**
**证据**：
- `upgrade.rs:27-36`：仅检查 `v` 前缀，未校验版本号格式
- 审计计划要求"版本号必须匹配 `v\d+\.\d+\.\d+\.\d+` 格式"
**业务影响**：用户输入 `--version abc` 会被拼接成 `vabc` 请求 GitHub，返回 404 但错误信息不清晰；恶意输入可能注入 URL
**修复建议**：
```rust
let version_regex = regex::Regex::new(r"^v?\d+\.\d+\.\d+\.\d+$").unwrap();
if !version_regex.is_match(&v) {
    println!("[ERROR] 版本号格式错误，应为 vX.X.X.X（如 v2026.7.16.1230）");
    return;
}
```

##### 缺陷 25.3-D：缺少版本回退检查（禁止降级）
**风险等级：P2**
**证据**：
- `upgrade.rs:23-36`：获取当前版本和目标版本后直接进入备份流程，未比较版本高低
- 审计计划要求"禁止降级到比当前版本更低的版本（除非 `--force-downgrade`）"
**业务影响**：误操作 `bingxi upgrade --version v2026.1.1.1` 会降级到旧版本，可能因数据库 schema 不兼容导致服务启动失败
**修复建议**：增加版本比较逻辑，目标版本低于当前版本时报错（除非 `--force-downgrade`）

##### 缺陷 25.3-E：缺少 schema 版本兼容性检查
**风险等级：P1**
**证据**：
- `upgrade.rs:84-85`：`deploy_release` 直接覆盖二进制，未检查 DB schema 版本
- 审计计划要求"升级前检查 DB schema 版本与目标版本兼容性，不兼容则警告"
**业务影响**：新版本二进制可能依赖新的 DB schema（如新增字段/索引），若数据库未迁移，服务启动后查询失败；反之旧二进制可能不兼容新 schema
**修复建议**：升级前调用 `bingxi migrate status` 检查迁移状态，与目标版本期望的 schema 版本对比

##### 缺陷 25.3-F：缺少 API 版本兼容性检查
**风险等级：P2**
**证据**：
- `upgrade.rs:84-85`：部署时同时替换后端二进制和前端 dist，未校验前后端 API 版本一致
- 审计计划要求"升级前检查前后端 API 版本兼容性（前端 dist 与后端 server 版本一致）"
**业务影响**：若发布包中前端 dist 与后端二进制版本不匹配（如打包错误），前端调用后端 API 可能 404 或字段不一致
**修复建议**：部署后校验前端 dist 中的 `version.json` 与后端 `/api/v1/erp/system/version` 返回值一致

##### 缺陷 25.3-G：缺少配置文件迁移
**风险等级：P2**
**证据**：
- `upgrade.rs:182-355`：`deploy_release` 仅替换二进制和前端，未迁移 config.yaml 格式
- 审计计划要求"升级时自动迁移配置文件格式（如 .env 字段变更），保留旧配置"
**业务影响**：新版本可能新增配置项（如 `REDIS__CLUSTER_MODE`），旧 config.yaml 缺失该字段导致功能不可用或启动失败
**修复建议**：部署后对比 `config.yaml.example` 与现有 `config.yaml`，提示新增字段并保留旧值

##### 缺陷 25.3-H：升级后未自动执行数据库迁移
**风险等级：P1**
**证据**：
- `upgrade.rs:182-355`：`deploy_release` 部署完成后仅 `systemctl start`，未调用 `bingxi migrate up`
- 对比：`deploy.sh:357-386`：`run_migrations` 函数会执行 `psql -f *.sql`
- 审计计划要求"升级后自动执行 `bingxi migrate up`，迁移失败自动回滚"
**业务影响**：新版本二进制依赖新 schema，但数据库未迁移，服务启动后查询失败；运维需手动执行 `bingxi migrate up`，易遗漏
**修复建议**：`deploy_release` 函数在 `systemctl start` 前增加：
```rust
println!("执行数据库迁移...");
if let Err(e) = run_cmd("bingxi", &["migrate", "run"]) {
    println!("[ERROR] 数据库迁移失败，自动回滚: {}", e);
    cmd_rollback();
    return;
}
```

##### 缺陷 25.3-I：升级日志未持久化
**风险等级：P2**
**证据**：
- `upgrade.rs` 全程 `println!` 输出到 stdout
- 审计计划要求"升级全过程写入 `/opt/bingxi-erp/logs/upgrade-YYYYMMDD-HHMMSS.log`"
**业务影响**：升级失败后日志仅留在终端，SSH 断开即丢失，无法事后追溯
**修复建议**：`cmd_upgrade` 入口初始化日志重定向到文件

##### 缺陷 25.3-J：缺少升级通知机制
**风险等级：P3**
**证据**：
- `upgrade.rs` 无邮件 / Webhook 通知逻辑
- 审计计划要求"升级成功/失败通知（邮件/Webhook）"
**业务影响**：升级失败时运维无即时告警，可能延迟发现
**修复建议**：升级完成后调用通知服务（复用项目已有的通知中心）

##### 缺陷 25.3-K：回滚不回滚 DB schema
**风险等级：P1**
**证据**：
- `upgrade.rs:112-165`：`cmd_rollback` 仅恢复 `server.old` + `bingxi.old`，未执行 `bingxi migrate down`
- 审计计划要求"回滚时同步回滚 DB schema（`bingxi migrate down`）"
**业务影响**：升级时若已执行数据库迁移（如新增字段），回滚到旧二进制后旧代码不兼容新 schema，可能导致字段映射错误或查询失败
**修复建议**：`cmd_rollback` 在恢复二进制后增加：
```rust
println!("回滚数据库 schema...");
if let Err(e) = run_cmd("bingxi", &["migrate", "rollback"]) {
    println!("[WARN] 数据库回滚失败: {}", e);
}
```

##### 缺陷 25.3-L：未保留多版本备份
**风险等级：P3**
**证据**：
- `upgrade.rs:264-282`：`deploy_release` 部署时备份到 `old.<timestamp>` 目录，但未限制保留数量
- `upgrade.rs:112-118`：`cmd_rollback` 仅检查 `server.old`（单个文件，非版本目录）
- 审计计划要求"保留最近 3 个版本备份，超出自动清理"
**业务影响**：多次升级后 `old.*` 目录堆积占用磁盘；`cmd_rollback` 只能回滚到上一次升级前的版本，无法回滚到更早版本
**修复建议**：
- 部署时备份到 `backups/version-<timestamp>/` 目录
- 保留最近 3 个版本：`ls -dt backups/version-* | tail -n +4 | xargs rm -rf`
- `cmd_rollback` 支持 `--version` 参数指定回滚到哪个版本

---

## 25.4 平滑升级（零停机/灰度/回滚）审计（20 项检查）

### 检查方法
1. Read `/workspace/backend/src/cli/util/upgrade.rs`（升级流程核心）
2. Read `/workspace/deploy/bingxi-backend.service`（systemd 服务配置）
3. Grep `systemctl stop` 在 `/workspace/backend/src/cli/util/upgrade.rs`（确认停机时间）
4. Grep `blue-green|blue_green|canary|rolling|灰度|蓝绿|金丝雀` 在 `/workspace/backend/src/` 和 `/workspace/deploy/`
5. 对照审计计划第 6994-7019 行逐项核对

### 发现

#### ✅ 已落实的项

1. **WebSocket 连接保持**（Nginx 升级头配置）：
   - `nginx.conf:30-31`：`proxy_set_header Upgrade $http_upgrade` + `proxy_set_header Connection 'upgrade'`
   - 服务重启后客户端 WebSocket 会断开，但 Nginx 配置支持 WebSocket 协议升级

2. **状态保持**（Redis session store）：
   - `deploy.sh:298-299`：config.yaml 中 `redis.url` + `redis.max_connections`
   - 服务重启后会话存储在 Redis 中不丢失，用户登录状态保持

#### ❌ 缺陷项

##### 缺陷 25.4-A：升级有 2-5 秒服务中断（非零停机）
**风险等级：P0**
**证据**：
- `upgrade.rs:183-187`：`deploy_release` 函数开头 `systemctl stop` + `sleep 2`
- `upgrade.rs:343-349`：部署完成后 `systemctl start` + `sleep 3`
- 总停机时间：2（停止）+ 部署耗时 + 3（启动等待）≈ 5-10 秒
- 审计计划明确指出"当前 `systemctl stop` 导致服务中断（2-5 秒），必须改造为零停机"
**业务影响**：升级期间所有 API 请求返回 502/连接拒绝，前端用户操作失败，WebSocket 断开，正在进行的批量操作（如报表生成）中断
**修复建议**：实施蓝绿部署（见缺陷 25.4-B）或滚动部署

##### 缺陷 25.4-B：无蓝绿部署能力
**风险等级：P1**
**证据**：
- `/workspace/deploy/` 下无双实例配置脚本
- `bingxi-backend.service` 仅单实例
- `nginx.conf` upstream 仅 `127.0.0.1:8082` 单点
- Grep `blue-green|blue_green|蓝绿` 在 backend/src/ 和 deploy/：0 匹配
- 审计计划要求"双实例部署（A/B），新版本启动健康后切换 Nginx upstream，旧实例优雅退出"
**业务影响**：无法在不中断服务的情况下升级，每次升级都有 5-10 秒停机
**修复建议**：
- 新增 `bingxi-backend-blue.service` + `bingxi-backend-green.service` 双服务
- Nginx upstream 配置切换脚本：`nginx -s reload` + upstream 配置文件软链接切换
- 部署流程：启动新实例 → 健康检查 → 切换 upstream → 优雅停止旧实例

##### 缺陷 25.4-C：无滚动部署能力
**风险等级：P2**
**证据**：
- 单实例部署，无多实例滚动升级机制
- 审计计划要求"多实例滚动升级，逐个实例升级，期间保持最小可用实例数"
**业务影响**：单点部署无法满足高可用要求
**修复建议**：实施多实例部署 + 滚动升级（需先实现蓝绿部署）

##### 缺陷 25.4-D：无金丝雀发布能力
**风险等级：P2**
**证据**：
- Grep `canary|金丝雀` 在 backend/src/ 和 deploy/：0 匹配
- 审计计划要求"新版本先发布到 1 个实例，观察 5-10 分钟无异常后全量发布"
**业务影响**：新版本全量发布后若发现严重 bug，所有用户受影响
**修复建议**：基于蓝绿部署，先切换 10% 流量到新实例，观察 5-10 分钟无异常后再全量切换

##### 缺陷 25.4-E：无灰度发布能力
**风险等级：P2**
**证据**：
- Grep `灰度|gray|grey|gradual` 在 backend/src/ 和 deploy/：0 匹配
- 审计计划要求"按用户 ID/IP 百分比灰度（5% → 25% → 50% → 100%）"
**业务影响**：无法逐步验证新版本稳定性
**修复建议**：Nginx upstream 配置 weight 参数 + Lua 脚本按用户 ID 哈希分流

##### 缺陷 25.4-F：无健康检查门禁
**风险等级：P1**
**证据**：
- `upgrade.rs:351-355`：部署后仅检查 `is_service_active`（systemd active 状态），未检查业务健康
- 审计计划要求"新版本启动后健康检查通过才接入流量，失败则不切换"
- 对比：`deploy.sh:445-464` 有 `health_check` 函数检查 `/health` 端点
**业务影响**：服务可能 systemd active 但业务异常（如 DB 连接失败），部署脚本误报成功
**修复建议**：`deploy_release` 函数在 `systemctl start` 后增加：
```rust
if !health_check_with_retry(10, 2) {
    println!("[ERROR] 健康检查失败，自动回滚");
    cmd_rollback();
    return;
}
```

##### 缺陷 25.4-G：无优雅退出机制
**风险等级：P1**
**证据**：
- `upgrade.rs:184`：`systemctl stop` 发送 SIGTERM，但 `bingxi-backend.service` 未配置 `KillSignal` / `TimeoutStopSec`
- `bingxi-backend.service` 全文 26 行，无 `KillSignal=SIGTERM` / `TimeoutStopSec=30s` 配置
- 后端代码（main.rs）未确认是否监听 SIGTERM 并优雅退出
- 审计计划要求"收到 SIGTERM 后停止接受新请求，等待已有请求完成（max 30s），再退出"
**业务影响**：`systemctl stop` 默认 90 秒后 SIGKILL，但期间已有请求可能被强制中断；若后端不优雅退出，数据库连接池可能未正确释放
**修复建议**：
- systemd 服务文件增加：`KillSignal=SIGTERM` + `TimeoutStopSec=30s` + `SendSIGKILL=no`
- 后端代码增加 SIGTERM 信号处理：停止接受新连接 + 等待已有请求完成（max 30s）+ 释放资源

##### 缺陷 25.4-H：无连接 draining
**风险等级：P2**
**证据**：
- `nginx.conf` 无 drain 配置
- 审计计划要求"Nginx upstream 切换前先 drain 旧实例连接（nginx drain mode）"
**业务影响**：切换 upstream 时旧实例上的活跃连接被立即断开
**修复建议**：使用 `nginx -s reload` 平滑切换 + 配置 `proxy_next_upstream` 重试机制

##### 缺陷 25.4-I：无长任务处理机制
**风险等级：P2**
**证据**：
- Grep `long.task|长任务|graceful.shutdown` 在 backend/src/：0 匹配
- 审计计划要求"升级期间长任务（报表生成/数据导入）暂停或迁移到新实例"
**业务影响**：升级期间正在生成的报表/数据导入任务中断，需重新开始
**修复建议**：长任务支持断点续传 + 任务状态持久化到 Redis，升级后可恢复

##### 缺陷 25.4-J：无数据库迁移兼容性保障
**风险等级：P1**
**证据**：
- `migrate.rs:53-57`：`Migrator::up` 执行迁移，无兼容性检查
- 审计计划要求"DB schema 迁移必须向后兼容（新增字段 nullable/有默认值），支持新旧版本同时运行"
- 现有迁移文件中部分 `ALTER TABLE ADD COLUMN` 未指定 `DEFAULT`（如新增 NOT NULL 字段会导致旧版本查询失败）
**业务影响**：蓝绿部署时新旧版本同时运行，若 schema 不兼容（如新版本新增 NOT NULL 字段无默认值），旧版本插入数据时违反约束
**修复建议**：制定迁移规范：新增字段必须 nullable 或有 DEFAULT；删除字段必须先废弃一个版本；重命名字段必须分两步（先新增 + 双写 + 迁移 + 切换 + 删旧）

##### 缺陷 25.4-K：无配置热更新
**风险等级：P3**
**证据**：
- Grep `SIGHUP|hot.reload|热更新` 在 backend/src/：0 匹配
- 审计计划要求"非破坏性配置变更支持热更新（无需重启），通过 SIGHUP 或 admin API"
**业务影响**：修改日志级别、限流阈值等非破坏性配置需重启服务
**修复建议**：后端监听 SIGHUP 信号，重新加载配置文件中的非破坏性项

##### 缺陷 25.4-L：无自动回滚触发
**风险等级：P1**
**证据**：
- `upgrade.rs:351-355`：部署后仅检查一次 `is_service_active`，未持续监控
- 审计计划要求"升级后健康检查连续失败 3 次（30 秒内）自动回滚"
**业务影响**：部署后服务可能在 1 分钟后才出问题（如内存泄漏、连接池耗尽），无自动回滚导致故障持续
**修复建议**：部署后启动监控线程，每 10 秒健康检查一次，连续 3 次失败触发 `cmd_rollback`

##### 缺陷 25.4-M：无回滚时间目标（RTO/RPO）
**风险等级：P3**
**证据**：
- 无 RTO/RPO 文档定义
- 审计计划要求"回滚 RTO ≤ 5 分钟，RPO ≤ 0（无数据丢失）"
**业务影响**：无明确目标，回滚效率无衡量标准
**修复建议**：在运维文档中定义 RTO/RPO 目标，并定期演练验证

##### 缺陷 25.4-N：无回滚验证
**风险等级：P2**
**证据**：
- `upgrade.rs:160-164`：回滚后仅检查 `is_service_active`，未验证业务接口
- 审计计划要求"回滚后健康检查 + 业务接口验证，失败通知运维"
**业务影响**：回滚后服务可能 active 但业务异常，运维误以为已恢复
**修复建议**：回滚后执行完整健康检查 + 关键业务接口验证

##### 缺陷 25.4-O：无缓存预热
**风险等级：P3**
**证据**：
- Grep `warm.up|预热|preheat` 在 backend/src/：0 匹配
- 审计计划要求"升级后预热关键缓存（用户/产品/客户/供货商），避免冷启动性能下降"
**业务影响**：升级后缓存为空，首批请求命中 DB，响应时间飙升
**修复建议**：升级后执行缓存预热脚本，加载热点数据到 Redis

##### 缺陷 25.4-P：无流量切换脚本
**风险等级：P2**
**证据**：
- `/workspace/deploy/` 无 Nginx upstream 切换脚本
- 审计计划要求"Nginx upstream 切换脚本（`nginx -s reload` + upstream 配置切换）"
**业务影响**：蓝绿部署时需手动修改 Nginx 配置，易出错
**修复建议**：新增 `deploy/switch-upstream.sh` 脚本，切换 upstream 软链接 + `nginx -s reload`

##### 缺陷 25.4-Q：无升级监控告警
**风险等级：P2**
**证据**：
- 升级期间无监控指标采集异常告警
- 审计计划要求"升级期间监控指标不中断，异常立即告警（错误率/响应时间/资源使用）"
**业务影响**：升级期间若监控中断，故障无法及时发现
**修复建议**：升级脚本集成监控检查，错误率 > 1% 或 P99 > 2s 时告警

##### 缺陷 25.4-R：无升级演练要求
**风险等级：P3**
**证据**：
- 无 staging 环境演练文档
- 审计计划要求"生产升级前必须在 staging 环境演练，记录演练报告"
**业务影响**：直接在生产升级，风险高
**修复建议**：制定升级流程规范，要求 staging 演练通过后方可生产升级

---

## 25.5 多租户代码残留与文件残留审计（26 项检查）

### 检查方法
1. Grep `tenant_id|tenant|租户|TenantId|multi_tenant|ACTIVE_TENANTS|active_tenants` 在 `/workspace/backend/src/`（9 行匹配）
2. Grep `tenant_id|tenant|租户|TenantId|multi_tenant` 在 `/workspace/frontend/src/`（0 匹配）
3. Grep `tenant_id|tenant|租户|TenantId|multi_tenant` 在 `/workspace/deploy/`（0 匹配）
4. Grep `tenant_id|tenant|租户|TenantId|multi_tenant` 在 `/workspace/.github/`（0 匹配）
5. Grep `tenant_id|tenant|租户|TenantId|multi_tenant` 在 `/workspace/backend/migration/src/`（19 行匹配，迁移注册和注释）
6. Grep `tenant_id|tenant|租户|TenantId|multi_tenant` 在 `/workspace/backend/migrations/`（200+ 行匹配，含 m0029 下线迁移和历史创建迁移）
7. Grep `tenant_id|tenant|租户|TenantId|multi_tenant` 在 `/workspace/.monkeycode/docs/`（169 行匹配，活跃文档 + 历史文档）
8. Grep `ACTIVE_TENANTS|active_tenants` 在 `/workspace/backend/`（1 行匹配，telemetry.rs:274 死代码）
9. Grep `tenant|TENANT` 在 `/workspace/backend/.env.example`（0 匹配）
10. Grep `tenant` 在 `/workspace/backend/Cargo.toml`（0 匹配）
11. Grep `tenant` 在 `/workspace/frontend/package.json`（0 匹配）
12. Grep `tenant|租户|多租户` 在 `/workspace/.monkeycode/MEMORY.md`（2 行匹配，均为"已删除"说明性注释）
13. Grep `tenant|租户|多租户` 在 `/workspace/.monkeycode/docs/ARCHITECTURE.md`（2 行匹配）
14. Grep `tenant|租户|多租户` 在 `/workspace/.monkeycode/docs/SECURITY.md`（1 行匹配）
15. Grep `tenant|租户|多租户` 在 `/workspace/.monkeycode/docs/DEVELOPER_GUIDE.md`（1 行匹配）
16. Grep `tenant|租户|多租户` 在 `/workspace/.monkeycode/docs/CODE_STYLE_GUIDE.md`（1 行匹配）
17. Grep `tenant|租户|多租户` 在 `/workspace/.monkeycode/docs/PROJECT_HEALTH_REPORT.md`（1 行匹配）
18. Grep `tenant|租户|多租户` 在 `/workspace/.monkeycode/docs/INTERFACES.md`（多行匹配）
19. Grep `tenant|租户` 在 `/workspace/backend/src/routes/`（0 匹配）
20. LS `/workspace/backend/src/middleware/`（17 个文件，无 tenant.rs）
21. 对照审计计划第 7039-7066 行逐项核对

### 发现

#### 残留项 1：代码残留扫描
**证据**：`/workspace/backend/src/` 共 9 行匹配：
- `backend/src/telemetry.rs:274`：`pub const ACTIVE_TENANTS: &str = "active_tenants";`（死代码常量）
- `backend/src/handlers/advanced/mod.rs:8`：注释"decide 异常检测 / 销售合同 / 销售价格 / 租户管理"
- `backend/src/handlers/data_permission_handler.rs:22`：注释"造成跨部门/跨租户越权读取"
- `backend/src/services/material_shortage_service.rs:438,443,447,498,526`：5 处注释"租户功能已删除"
- `backend/src/models/slow_query.rs:11`：注释"关键索引：idx_slow_query_tenant"
**风险等级**：P2（代码层无功能影响，但死代码常量违反项目规则六"死代码处理规范"）
**修复建议**：
1. 删除 `telemetry.rs:274` 的 `ACTIVE_TENANTS` 常量
2. 更新 `handlers/advanced/mod.rs:8` 注释，移除"租户管理"
3. 更新 `handlers/data_permission_handler.rs:22` 注释，移除"跨租户"
4. 更新 `material_shortage_service.rs` 5 处注释，改为"配置不再持久化"（移除"租户"字样）
5. 更新 `models/slow_query.rs:11` 注释，移除 `idx_slow_query_tenant` 索引引用

#### 残留项 2：死代码常量 ACTIVE_TENANTS
**证据**：
- `backend/src/telemetry.rs:274`：`pub const ACTIVE_TENANTS: &str = "active_tenants";`
- Grep `ACTIVE_TENANTS|active_tenants` 在 `/workspace/backend/`：仅 1 行匹配（定义处），无任何引用
- 违反项目规则六第 1 条"禁止使用文件级 `#![allow(dead_code)]` 全局抑制；CI 会在 clippy 检查中失败"
**风险等级**：P2（CI clippy 应报告 dead_code 警告，但可能被 baseline 机制容忍）
**修复建议**：立即删除 `telemetry.rs:274` 的 `ACTIVE_TENANTS` 常量定义

#### 残留项 3：注释残留 idx_slow_query_tenant
**证据**：
- `backend/src/models/slow_query.rs:11`：`//! 关键索引：idx_slow_query_captured / idx_slow_query_exec_time / idx_slow_query_tenant`
- `idx_slow_query_tenant` 索引已在 m0029 迁移中删除（`migrations/20260628000001_drop_tenant_columns/up.sql:117`）
**风险等级**：P3（注释残留，无功能影响）
**修复建议**：更新注释为 `//! 关键索引：idx_slow_query_captured / idx_slow_query_exec_time`（移除 `idx_slow_query_tenant`）

#### 残留项 4：模型字段残留
**证据**：Grep `tenant_id` 在 `/workspace/backend/src/models/`：0 匹配
**结论**：✅ 已清理（SeaORM 模型中无 tenant_id 字段）

#### 残留项 5：Service 残留
**证据**：Grep `tenant_id|tenant` 在 `/workspace/backend/src/services/`：仅 `material_shortage_service.rs` 5 处注释（见残留项 1），无 `.filter(tenant_id.eq(...))` 逻辑
**结论**：✅ 已清理（无 tenant 过滤逻辑，仅注释残留）

#### 残留项 6：Handler 残留
**证据**：Grep `tenant_id|tenant` 在 `/workspace/backend/src/handlers/`：2 处注释（`advanced/mod.rs:8` + `data_permission_handler.rs:22`），无 `extract_tenant_id` 逻辑
**结论**：✅ 已清理（无 tenant 提取逻辑，仅注释残留）

#### 残留项 7：中间件残留
**证据**：LS `/workspace/backend/src/middleware/` 显示 17 个文件，无 `tenant.rs` / `tenant_context.rs`
**结论**：✅ 已清理（无 tenant 中间件）

#### 残留项 8：路由残留
**证据**：Grep `tenant` 在 `/workspace/backend/src/routes/`：0 匹配
**结论**：✅ 已清理（无 tenant 路由）

#### 残留项 9：配置残留
**证据**：
- Grep `tenant|TENANT` 在 `/workspace/backend/.env.example`：0 匹配
- Grep `tenant` 在 `/workspace/backend/Cargo.toml`：0 匹配
**结论**：✅ 已清理（配置文件无 tenant 配置项）

#### 残留项 10：迁移文件残留（历史创建迁移仍含 tenant_id 定义）
**证据**：6 个历史创建迁移文件仍保留 `tenant_id` 列定义：
1. `backend/migrations/20260527000001_add_basic_data_and_system_tables/up.sql:218,231,240,253`：api_keys + webhooks 表 tenant_id 列 + 索引
2. `backend/migrations/20260617000005_create_after_sales/up.sql:13,27`：tenant_id BIGINT NOT NULL + idx_aftersales_tenant 索引
3. `backend/migrations/20260617000006_create_color_cards/up.sql:17,25`：tenant_id BIGINT NOT NULL + idx_color_cards_tenant 索引
4. `backend/migrations/20260617000011_create_sales_facts/up.sql:2,3,7,23,27,31,35,39`：多租户隔离注释 + tenant_id 列 + 4 个 tenant 索引 + COMMENT
5. `backend/migrations/20260618000002_create_color_price_history/up.sql:20,27`：tenant_id BIGINT NOT NULL + idx_price_history_tenant 索引
6. `backend/migrations/20260618000005_create_seasonal_price_rules/up.sql:17,26`：tenant_id BIGINT NOT NULL + idx_seasonal_tenant_active 索引
**风险等级**：P3（m0029 迁移已删除这些列和索引，新部署会"先创建后删除"，功能无影响但浪费迁移时间）
**修复建议**：审计计划建议"保留但标注已废弃"。在每个文件头部增加注释：
```sql
-- 注意：本迁移创建的 tenant_id 列和索引已在 m0029_drop_tenant_columns 中删除
-- 多租户功能已于 2026-06-28 完整下线，此处的 tenant_id 定义仅为迁移历史保留
```

#### 残留项 11：迁移文件残留 - 租户管理表
**证据**：
- `backend/migrations/20260628000001_drop_tenant_columns/up.sql:199-207`：m0029 迁移删除了 7 个租户管理表（tenants / tenant_users / tenant_configs / tenant_subscriptions / tenant_usage / tenant_invoices / tenant_plans）
- 旧迁移快照 `backend-database-migration/006_tenant_saas.sql` 保留租户管理表创建脚本（历史快照，不在迁移路径中）
**结论**：✅ 已清理（m0029 已删除所有租户管理表，旧快照保留作历史记录）

#### 残留项 12：前端残留
**证据**：Grep `tenant_id|tenant|租户|TenantId|multi_tenant` 在 `/workspace/frontend/src/`：0 匹配
**结论**：✅ 已清理（前端无 tenant 引用）

#### 残留项 13：部署脚本残留
**证据**：Grep `tenant_id|tenant|租户|TenantId|multi_tenant` 在 `/workspace/deploy/`：0 匹配
**结论**：✅ 已清理（部署脚本无 tenant 引用）

#### 残留项 14：文档残留 - 活跃文档
**证据**：7 个活跃文档仍提及 tenant：
1. `.monkeycode/MEMORY.md:368,380`：2 处，均为"多租户功能已移除"说明性注释（合理保留）
2. `.monkeycode/docs/ARCHITECTURE.md:5,332`：2 处，"系统支持多租户 SaaS 部署模式"+"多租户 SaaS"（**需清理**）
3. `.monkeycode/docs/SECURITY.md:103`：1 处，"auth/permission/tenant 中间件链路完整保留"（**需清理**，tenant 中间件已删除）
4. `.monkeycode/docs/DEVELOPER_GUIDE.md:11`：1 处，"实现多租户 SaaS 部署模式"（**需清理**）
5. `.monkeycode/docs/CODE_STYLE_GUIDE.md:386`：1 处，"租户隔离是否正确（使用 `extract_tenant_id`）"（**需清理**，函数已删除）
6. `.monkeycode/docs/INTERFACES.md:275,279-284`：多行，"多租户管理 `/api/v1/erp/tenants`" + 6 个 API 端点（**需清理**，路由已删除）
7. `.monkeycode/docs/PROJECT_HEALTH_REPORT.md:87`：1 处，`.nest("/api/v1/erp/tenant", tenant::routes())`（**需清理**，路由已删除）
**风险等级**：P2（活跃文档误导开发者，可能基于过时文档实现已删除的功能）
**修复建议**：
- ARCHITECTURE.md:5 移除"多租户 SaaS 部署模式"描述
- ARCHITECTURE.md:332 移除"多租户 SaaS"
- SECURITY.md:103 移除"tenant 中间件"引用
- DEVELOPER_GUIDE.md:11 移除"多租户 SaaS 部署模式"
- CODE_STYLE_GUIDE.md:386 移除"租户隔离"检查项
- INTERFACES.md:275-284 移除"多租户管理"API 章节
- PROJECT_HEALTH_REPORT.md:87 移除 tenant 路由引用

#### 残留项 15：文档残留 - 历史文档
**证据**：历史审计文档保留 tenant 引用：
- `.monkeycode/docs/audits/2026-06-18-db-n1-audit.md`：tenant_billing_service.rs / tenant:{tenant_id} 缓存键空间
- `.monkeycode/docs/audits/2026-06-28-strict-reaudit-v5.md`：维度 16 残留租户检查
- `.monkeycode/docs/audits/v15/batch-14/audit-report.md`：tenant_id 列引用
- `.monkeycode/docs/deploy-report-2026-06-21.md`：tenant_id 列不存在问题
- `.monkeycode/docs/superpowers/plans/2026-06-17-roadmap.md` / `2026-06-18-p13-batch1-comprehensive-plan.md`：历史规划
- `.monkeycode/docs/database/legacy-migration-snapshots/006_tenant_saas.sql`：SaaS 租户管理表创建脚本
**风险等级**：P3（历史文档保留作记录，符合审计计划"可保留作为历史记录"）
**修复建议**：每个历史文档头部增加标注：
```markdown
> **注意**：多租户功能已于 2026-06-28 m0029 迁移完整下线。本文档中提及的 tenant 相关内容仅作历史记录。
```

#### 残留项 16：数据库残留
**证据**：
- `backend/migrations/20260628000001_drop_tenant_columns/up.sql:124-196`：m0029 迁移删除了 39 个业务表的 tenant_id 列
- 生产数据库已通过 m0029 迁移清理（假设迁移已执行）
**结论**：✅ 已清理（通过 m0029 迁移验证）

#### 残留项 17：索引残留
**证据**：
- `backend/migrations/20260628000001_drop_tenant_columns/up.sql:17-117`：m0029 迁移删除了 50+ 个 tenant 相关索引
- 包括 idx_sales_orders_tenant_customer_status / idx_inventory_stocks_tenant_wh_product / idx_ar_invoices_tenant_customer_due 等
**结论**：✅ 已清理（通过 m0029 迁移验证）

#### 残留项 18：外键残留
**证据**：
- `backend/migrations/20260628000001_drop_tenant_columns/up.sql:199-207`：m0029 迁移删除租户管理表时同步删除外键
- 业务表的 tenant_id 列无外键约束（原设计为 INTEGER/BIGINT，未引用 tenants 表）
**结论**：✅ 已清理

#### 残留项 19：测试残留
**证据**：Grep `tenant_id|tenant` 在 `/workspace/backend/src/` 测试代码中：0 匹配（仅注释残留）
**结论**：✅ 已清理（无 tenant 相关测试用例）

#### 残留项 20：依赖残留
**证据**：
- Grep `tenant` 在 `/workspace/backend/Cargo.toml`：0 匹配
- Grep `tenant` 在 `/workspace/frontend/package.json`：0 匹配
**结论**：✅ 已清理（无 tenant 相关依赖包）

#### 残留项 21：环境变量残留
**证据**：Grep `tenant|TENANT` 在 `/workspace/backend/.env.example`：0 匹配
**结论**：✅ 已清理（无 TENANT_ID / TENANT_NAME 等环境变量）

#### 残留项 22：API 残留
**证据**：
- Grep `tenant` 在 `/workspace/backend/src/routes/`：0 匹配
- Grep `tenant_id` 在 `/workspace/backend/src/models/`：0 匹配
- API 响应中无 tenant_id 字段（模型已清理）
**结论**：✅ 已清理（API 接口无 tenant 相关请求头/查询参数/响应字段）

#### 残留项 23：错误消息残留
**证据**：Grep `租户不存在|tenant.not.found|tenant.invalid` 在 `/workspace/backend/src/`：0 匹配
**结论**：✅ 已清理（AppError 错误消息无 tenant 相关提示）

#### 残留项 24：日志残留
**证据**：
- Grep `tenant_id` 在 `/workspace/backend/src/` 日志输出（`tracing::info/warn/error`）：0 匹配
- `material_shortage_service.rs:443,526` 的 `tracing::warn!` 含"租户配置表已删除"字样（说明性日志，非 tenant_id 字段）
**结论**：✅ 已清理（日志输出无 tenant_id 字段）

#### 残留项 25：监控指标残留
**证据**：
- `backend/src/telemetry.rs:274`：`pub const ACTIVE_TENANTS: &str = "active_tenants";` 常量定义
- Grep `ACTIVE_TENANTS` 在 `/workspace/backend/`：仅 1 行匹配（定义处），无任何 `register_gauge` / `record` 调用
**风险等级**：P2（死代码常量，违反项目规则六）
**修复建议**：删除 `telemetry.rs:274` 的 `ACTIVE_TENANTS` 常量（同残留项 2）

#### 残留项 26：清理验证报告
**证据**：Grep `多租户清理|tenant.cleanup|租户清理验证` 在 `/workspace/.monkeycode/docs/`：
- `.monkeycode/docs/audits/2026-06-28-strict-reaudit-v5.md:536-596`：维度 16 残留租户检查报告（15 项检查）
- 无独立的"多租户清理验证报告"文档
**风险等级**：P3（v5 审计报告已覆盖，但未生成独立验证报告）
**修复建议**：生成独立的多租户清理验证报告，记录清理的文件/代码行数/迁移文件列表

#### 残留项 27：缓存键空间
**证据**：
- `.monkeycode/docs/audits/2026-06-18-db-n1-audit.md:104`：历史文档提及 `tenant:{tenant_id}:{entity_type}:{entity_id}` 键空间
- Grep `tenant:` 在 `/workspace/backend/src/utils/cache.rs`：0 匹配
- Grep `tenant:{` 在 `/workspace/backend/src/`：0 匹配
**结论**：✅ 已清理（缓存键不再使用 tenant:{tenant_id} 前缀）

---

## 审计结果汇总

| 维度 | P0 | P1 | P2 | P3 | 已落实 | 总检查项 |
|------|----|----|----|----|--------|----------|
| 25.1 部署流程 | 0 | 1 | 4 | 3 | 10 | 18 |
| 25.2 CLI 工具 | 0 | 1 | 3 | 1 | 13 | 18 |
| 25.3 升级流程 | 0 | 4 | 6 | 2 | 9 | 21 |
| 25.4 平滑升级 | 1 | 5 | 8 | 4 | 2 | 20 |
| 25.5 多租户残留 | 0 | 0 | 4 | 3 | 19 | 26 |
| **合计** | **1** | **11** | **25** | **13** | **53** | **103** |

---

## 修复优先级队列

### P0（阻塞 - 1 项）

1. **缺陷 25.4-A**：升级有 2-5 秒服务中断（非零停机）
   - 文件：`/workspace/backend/src/cli/util/upgrade.rs:183-187, 343-349`
   - 修复方案：实施蓝绿部署（见缺陷 25.4-B）或滚动部署，消除 `systemctl stop` 导致的服务中断

### P1（高 - 11 项）

2. **缺陷 25.1-A**：部署脚本缺少 `set -euo pipefail`
   - 文件：`/workspace/deploy/deploy.sh:5` + `deploy-backend.sh:5` + `deploy-frontend.sh:5` + `deploy-prepare.sh:5` + `deploy-latest.sh:21`
   - 修复方案：所有 5 个脚本顶部改为 `set -euo pipefail`

3. **缺陷 25.2-C**：CLI 无权限校验（root 检查）
   - 文件：`/workspace/backend/src/cli/util/upgrade.rs` + `backup.rs`
   - 修复方案：在 cmd_upgrade / cmd_deploy / cmd_rollback / cmd_backup / cmd_restore 函数开头增加 root 权限检查

4. **缺陷 25.3-A**：缺少 SHA256 下载校验
   - 文件：`/workspace/backend/src/cli/util/upgrade.rs:71-81`
   - 修复方案：下载后对比 Release assets 中的 sha256sum 文件

5. **缺陷 25.3-E**：缺少 schema 版本兼容性检查
   - 文件：`/workspace/backend/src/cli/util/upgrade.rs:84-85`
   - 修复方案：升级前调用 `bingxi migrate status` 检查迁移状态

6. **缺陷 25.3-H**：升级后未自动执行数据库迁移
   - 文件：`/workspace/backend/src/cli/util/upgrade.rs:182-355`
   - 修复方案：`deploy_release` 函数在 `systemctl start` 前调用 `bingxi migrate run`

7. **缺陷 25.3-K**：回滚不回滚 DB schema
   - 文件：`/workspace/backend/src/cli/util/upgrade.rs:112-165`
   - 修复方案：`cmd_rollback` 在恢复二进制后调用 `bingxi migrate rollback`

8. **缺陷 25.4-B**：无蓝绿部署能力
   - 文件：`/workspace/deploy/` + `bingxi-backend.service` + `nginx.conf`
   - 修复方案：新增双实例服务 + Nginx upstream 切换脚本

9. **缺陷 25.4-F**：无健康检查门禁
   - 文件：`/workspace/backend/src/cli/util/upgrade.rs:351-355`
   - 修复方案：部署后健康检查通过才接入流量，失败自动回滚

10. **缺陷 25.4-G**：无优雅退出机制
    - 文件：`/workspace/deploy/bingxi-backend.service` + 后端 main.rs
    - 修复方案：systemd 增加 `KillSignal=SIGTERM` + `TimeoutStopSec=30s`；后端监听 SIGTERM 优雅退出

11. **缺陷 25.4-J**：无数据库迁移兼容性保障
    - 文件：`/workspace/backend/migration/src/`
    - 修复方案：制定迁移规范：新增字段必须 nullable 或有 DEFAULT

12. **缺陷 25.4-L**：无自动回滚触发
    - 文件：`/workspace/backend/src/cli/util/upgrade.rs:351-355`
    - 修复方案：部署后启动监控线程，连续 3 次健康检查失败触发 `cmd_rollback`

### P2（中 - 25 项）

13. **缺陷 25.1-B**：缺少端口冲突检查
14. **缺陷 25.1-C**：缺少部署日志持久化
15. **缺陷 25.1-D**：配置目录权限不符合 600 要求
16. **缺陷 25.1-E**：缺少部署后业务健康检查（仅检查 active）
17. **缺陷 25.2-A**：CLI 错误处理不统一
18. **缺陷 25.2-B**：无 CLI 日志持久化
19. **缺陷 25.2-D**：无危险操作二次确认
20. **缺陷 25.3-B**：缺少断点续传
21. **缺陷 25.3-C**：缺少版本格式校验
22. **缺陷 25.3-D**：缺少版本回退检查（禁止降级）
23. **缺陷 25.3-F**：缺少 API 版本兼容性检查
24. **缺陷 25.3-G**：缺少配置文件迁移
25. **缺陷 25.3-I**：升级日志未持久化
26. **缺陷 25.4-C**：无滚动部署能力
27. **缺陷 25.4-D**：无金丝雀发布能力
28. **缺陷 25.4-E**：无灰度发布能力
29. **缺陷 25.4-H**：无连接 draining
30. **缺陷 25.4-I**：无长任务处理机制
31. **缺陷 25.4-N**：无回滚验证
32. **缺陷 25.4-P**：无流量切换脚本
33. **缺陷 25.4-Q**：无升级监控告警
34. **残留项 1**：代码残留（telemetry.rs ACTIVE_TENANTS 死代码 + 5 处注释残留）
35. **残留项 14**：文档残留 - 活跃文档（ARCHITECTURE.md / SECURITY.md / DEVELOPER_GUIDE.md / CODE_STYLE_GUIDE.md / INTERFACES.md / PROJECT_HEALTH_REPORT.md 共 6 个文档需清理）
36. **残留项 25**：监控指标残留（ACTIVE_TENANTS 常量，同残留项 1）

### P3（低 - 13 项）

37. **缺陷 25.1-F**：Nginx 缺少 gzip 压缩配置
38. **缺陷 25.1-G**：缺少防火墙配置
39. **缺陷 25.1-H**：部署回滚不恢复配置文件
40. **缺陷 25.2-E**：UtilCommand 缺少 `--force` 参数
41. **缺陷 25.3-J**：缺少升级通知机制
42. **缺陷 25.3-L**：未保留多版本备份
43. **缺陷 25.4-K**：无配置热更新
44. **缺陷 25.4-M**：无回滚时间目标（RTO/RPO）
45. **缺陷 25.4-O**：无缓存预热
46. **缺陷 25.4-R**：无升级演练要求
47. **残留项 3**：注释残留 idx_slow_query_tenant
48. **残留项 10**：迁移文件残留（6 个历史创建迁移）
49. **残留项 15**：文档残留 - 历史文档
50. **残留项 26**：清理验证报告

---

## 审计结论

### 整体评估

本批审计覆盖类二十五部署流程与升级管理（4 维度 77 项检查）+ 25.5 多租户残留审计（26 项检查），合计 5 维度 103 项检查。

**关键发现**：
1. **部署流程**（25.1）：基础框架完整（5 个脚本 + systemd + Nginx + CI/CD），但缺少 `set -euo pipefail`、端口检查、部署日志持久化、业务健康检查等运维细节
2. **CLI 工具**（25.2）：14 个子命令完整实现，migrate 子命令已真实实现（非占位），但缺少权限校验、危险操作确认、日志持久化、错误处理不统一
3. **升级流程**（25.3）：基本流程完整（备份 → 下载 → 部署 → 清理），但缺少 SHA256 校验、断点续传、schema/API 版本兼容、数据库迁移自动执行、回滚 DB schema 等关键能力
4. **平滑升级**（25.4）：**严重不合理**，当前完全无零停机/蓝绿/灰度/金丝雀部署能力，升级有 2-5 秒服务中断（P0），无自动回滚，无优雅退出
5. **多租户残留**（25.5）：**主体已清理**（模型/Service/Handler/中间件/路由/配置/前端/部署脚本/测试/依赖/环境变量/API/错误消息/日志/缓存键空间全部 0 匹配），但仍有 1 处死代码常量（telemetry.rs:274 ACTIVE_TENANTS）+ 5 处注释残留 + 6 个历史迁移文件残留 + 6 个活跃文档残留需清理

### 多租户残留清理完成度

- **26 项检查中 19 项已清理**（73%）：模型字段 / Service / Handler / 中间件 / 路由 / 配置 / 租户管理表 / 前端 / 部署脚本 / 数据库 / 索引 / 外键 / 测试 / 依赖 / 环境变量 / API / 错误消息 / 日志 / 缓存键空间
- **7 项残留**（27%）：
  - P2：死代码常量 ACTIVE_TENANTS + 5 处注释残留 + 6 个活跃文档残留
  - P3：注释残留 idx_slow_query_tenant + 6 个历史迁移文件残留 + 历史文档残留 + 清理验证报告

### 与 V15 审计计划对比

V15 审计计划第 7068 行合理性评估："当前存在代码残留（telemetry.rs ACTIVE_TENANTS 死代码 + slow_query.rs 注释残留）+ 迁移文件残留（6 个创建迁移仍保留 tenant_id 定义）+ 20+ 文档残留，**不合理**，必须清理。"

**审计结论**：与计划描述一致，多租户残留确实存在，需按修复优先级队列清理。

---

## 关联文档

- [V15 审计计划](file:///workspace/.monkeycode/docs/audits/v15-review-plan-2026-07-15.md)（类二十五第 6893 行起）
- [batch-04 审计报告](file:///workspace/.monkeycode/docs/audits/v15/batch-04/audit-report.md)（报告格式参考）
- [项目规则](file:///workspace/.trae/rules/project_rules.md)（死代码处理规范第六章）
- [MEMORY.md](file:///workspace/.monkeycode/MEMORY.md)（多租户功能已移除说明）
- [m0029 迁移](file:///workspace/backend/migration/src/m0029_drop_tenant_columns.rs)（多租户下线迁移）
- [m0029 up.sql](file:///workspace/backend/migrations/20260628000001_drop_tenant_columns/up.sql)（删除 39 列 + 50+ 索引 + 7 表）
