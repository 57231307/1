# 纺织面料二批系统（BingXi ERP）代码态全流程详细验证方案（门禁式）

本方案用于“代码已开发完成、待全量验证 → 通过后才能打包 → 打包后部署 → 初始化 → 业务验收 → 运维演练”的全链路验证。方案严格对齐本仓库的实际技术栈与脚本实现：后端 Rust/Axum/SeaORM，前端 Rust/Yew/Trunk，数据库 PostgreSQL，发布与部署通过 GitHub Actions 产物 + systemd + Nginx。

## 0. 验证总则

### 0.1 适用场景

- 代码托管于 GitHub，处于开发完成待验证状态。
- 本方案不包含“服务器重装、清零重装”类内容，只覆盖从源码拉取开始的验证与交付流程。
- 验证流程固定为：源码拉取 → 代码全维度验证 → GitHub 打包 → 部署上线 → 初始化 → 业务使用 → 运维维护；不允许跳步。

### 0.2 硬性门禁（G0 ~ G9）

任一门禁未通过：立即停止，记录问题清单（含日志与复现步骤），回到对应门禁修复后重新验证，禁止进入后续流程。

- **G0 源码与版本合规**
- **G1 仓库合规与静态质量**
- **G2 后端/前端测试**
- **G3 编译构建验证（本地）**
- **G4 数据库脚本落库验证**
- **G5 接口连通与安全验证（Smoke + 权限）**
- **G6 面料二批核心业务 E2E 验收剧本**
- **G7 GitHub Actions 打包与产物校验**
- **G8 部署上线与回归**
- **G9 运维维护演练（监控、更新、回滚、备份恢复策略）**

### 0.3 项目架构一致性校验（贯穿所有门禁）

在进入 G1 前必须先做一次“架构一致性快照”，确保验证方案与项目实际一致，避免用错技术栈/命令导致的误判：

- **数据库**：本项目为 PostgreSQL，配置样例见 [backend/.env.example](file:///workspace/backend/.env.example)；部署脚本与 systemd service 亦依赖 PostgreSQL，见 [deploy/bingxi-backend.service](file:///workspace/deploy/bingxi-backend.service)。
- **缓存**：仓库提供内存缓存实现（DashMap），见 [cache.rs](file:///workspace/backend/src/utils/cache.rs)；未发现必须依赖 Redis 的连接实现，因此 Redis 只能作为“可选项/规划项”对待，不能作为硬性依赖写入门禁。
- **监控**：仓库存在 Prometheus 指标服务与中间件实现，见 [metrics_service.rs](file:///workspace/backend/src/services/metrics_service.rs) 与 [metrics.rs](file:///workspace/backend/src/middleware/metrics.rs)；但当前主程序路由中未发现挂载 `/metrics` 的逻辑，需在 G9 中明确“是否启用、如何启用、证据是什么”。

## G0. GitHub 源码拉取与版本合规验证

### G0-1 源码拉取操作验证

操作命令（示例）：

```bash
git clone <GitHub 仓库地址>
cd <repo>
git fetch --all --tags
git checkout <待验证分支/发布分支>
git pull --rebase
```

验证点：

- 分支切换无报错；拉取无冲突；工作区干净（或仅包含本次验证产生的本地日志，不提交）。

合格标准：

- `git status` 显示无冲突、无未提交的业务改动。

### G0-2 源码完整性校验

必须包含目录/文件（抽检清单）：

- 前端：`frontend/`（Yew + Trunk）
- 后端：`backend/`（Axum + SeaORM）
- 数据库脚本：`backend/database/migration/`（至少包含 [001_consolidated_schema.sql](file:///workspace/backend/database/migration/001_consolidated_schema.sql)）
- CI/CD：`.github/workflows/ci-cd.yml`
- 部署脚本：`deploy/` 与 `快速部署/install.sh`
- 监控配置：`monitoring/`（Prometheus/Grafana/Alertmanager）

合格标准：

- 关键目录存在且非空；无明显占位空目录。

### G0-3 版本一致性验证（代码 → 打包 → 部署）

本项目涉及多个“版本来源”，必须在验证前确认并形成记录：

- **后端构建版本**：`backend/Cargo.toml` 的 `package.version`
- **前端构建版本**：`frontend/Cargo.toml` 的 `package.version`
- **发布包版本**：根目录 [VERSION](file:///workspace/VERSION)（部署/更新逻辑会读取部署目录中的 `VERSION` 文件，见 [system_update_service.rs](file:///workspace/backend/src/services/system_update_service.rs)）
- **CI 打包版本**：GitHub Actions 在 [.github/workflows/ci-cd.yml](file:///workspace/.github/workflows/ci-cd.yml) 中生成（tag `v*` 取 tag，否则基于最新 tag 与 commit “打分”计算）

合格标准：

- 本次验证记录中明确：本次验收对应哪个 commit（`git rev-parse HEAD`）、对应哪个 tag（如有）、对应发布包版本号策略（走 tag 或走 CI 算法）。

## G1. 仓库合规与静态质量验证（打包前硬门禁）

### G1-1 敏感信息与调试残留排查

建议命令（示例）：

```bash
git grep -nE "(password\\s*=|jwt_secret|AKIA|BEGIN RSA PRIVATE KEY|postgres://[^\\s]+:[^\\s]+@)" -- . || true
git grep -nE "(TODO\\(remove\\)|FIXME\\(remove\\)|console\\.log\\(|println!\\(\"debug\"|debug!\\()" -- . || true
```

验证点：

- 不允许提交明文数据库账号密码、Token、私钥等；不允许遗留临时调试逻辑影响生产行为。

合格标准：

- 无敏感信息命中；如命中必须确认是否为 `.env.example` 这类样例文件且无真实凭证。

### G1-2 Rust 格式与静态检查（后端 + 前端）

后端：

```bash
cd backend
cargo fmt --all -- --check
cargo clippy --all -- -D warnings
```

前端：

```bash
cd frontend
cargo fmt --all -- --check
cargo clippy --all -- -D warnings
```

合格标准：

- `cargo fmt` 全量通过；`cargo clippy` 不允许出现 warning（本门禁按 `-D warnings` 执行）。

## G2. 测试验证（打包前硬门禁）

### G2-1 单元/集成测试

本仓库 CI 的后端测试命令为：

```bash
cd backend
cargo test --all --jobs 2
```

建议本地与 CI 完全一致；必要时补充：

```bash
cd backend
cargo test --tests
```

合格标准：

- 测试全部通过；如存在 `ignored` 测试，必须在验收报告中列出原因与是否影响上线。

### G2-2 API 回归用例（可选增强）

仓库包含 Postman 集合：

- [postman_collection.json](file:///workspace/backend/docs/postman_collection.json)

合格标准：

- 关键接口（登录、档案查询、库存查询/更新、销售开单、应收/对账）至少完成一次回归调用并留存导出结果。

## G3. 编译构建验证（本地，打包前硬门禁）

### G3-1 后端 Release 构建

```bash
cd backend
cargo build --release
test -f target/release/server
```

合格标准：

- `target/release/server` 生成成功。

### G3-2 前端 Release 构建（Trunk）

```bash
rustup target add wasm32-unknown-unknown
cargo install --locked trunk
cd frontend
trunk build --release
test -d dist
```

合格标准：

- `frontend/dist` 生成成功，且包含 `index.html` 等静态资源。

## G4. 数据库脚本验证（打包前硬门禁）

### G4-1 脚本落库验证（全新库）

本仓库整合迁移脚本为：

- [001_consolidated_schema.sql](file:///workspace/backend/database/migration/001_consolidated_schema.sql)

验证步骤：

1) 准备全新 PostgreSQL 数据库（空库）。  
2) 以“单文件落库”的方式执行：

```bash
psql "postgres://<user>:<password>@<host>:<port>/<db>" -f backend/database/migration/001_consolidated_schema.sql
```

验证点：

- 执行过程无报错；核心表/视图/索引/触发器创建成功；基础字典数据（若脚本包含）写入成功。

合格标准：

- 无 SQL error；关键业务表存在并可查询。

### G4-2 与应用初始化逻辑一致性验证

后端初始化服务会在初始化流程中按文件名排序执行迁移目录下的 `.sql` 文件，执行实现见：

- [run_migrations](file:///workspace/backend/src/services/init_service.rs#L164-L228)

验证点：

- 发布包中必须包含 `database/migration/` 目录（CI 打包会拷贝 `backend/database`，见 [.github/workflows/ci-cd.yml](file:///workspace/.github/workflows/ci-cd.yml)）。

合格标准：

- 初始化流程可找到迁移目录并执行成功（无 “migration dir not found” / “execute_unprepared failed”）。

## G5. 接口连通与安全验证（打包前硬门禁）

### G5-1 启动模式验证：初始化模式 vs 完整模式

后端在数据库连接失败时会启动初始化路由（setup mode），相关路由见 [main.rs](file:///workspace/backend/src/main.rs) 的 `create_init_router()`：

- `GET /api/v1/erp/init/status`
- `POST /api/v1/erp/init/test-database`
- `POST /api/v1/erp/init/initialize-with-db`

验证点：

- 当 DB 不可用时：初始化接口可用，且业务接口不可用/返回合理错误。
- 当 DB 可用时：系统进入完整模式（full mode），业务接口可用。

合格标准：

- 两种模式切换符合预期；初始化流程可完成建表与管理员初始化（若初始化流程包含该逻辑）。

### G5-2 核心接口 Smoke Test（必须）

建议最小集：

- 登录：`POST /api/v1/erp/login`（见 [routes/mod.rs](file:///workspace/backend/src/routes/mod.rs)）
- 基础连通性：Nginx 提供静态健康检查端点 `GET /health`（直接返回 `OK`，见 [nginx.conf](file:///workspace/deploy/nginx.conf)）
- 后端健康接口：`GET /api/v1/erp/health`（见 [routes/mod.rs](file:///workspace/backend/src/routes/mod.rs) 与 [health_handler.rs](file:///workspace/backend/src/handlers/health_handler.rs)）

合格标准：

- 无 404/500；返回结构稳定且可被前端解析。

### G5-3 权限与越权验证（RBAC）

验证点：

- 未登录调用受保护接口：应被拦截（401/403）。
- 低权限角色调用敏感接口（如删除、审核、核销等）：应被拦截（403）。

合格标准：

- 权限拦截始终生效，无“绕过路由”。

## G6. 面料二批核心业务 E2E 验收（部署后必须）

本门禁要求“用真实 UI/接口走完整业务闭环”，并且数据库层面可核对结果。

### G6-1 验收数据准备（最小数据集）

- 供应商 1 个、客户 2 个（含赊账客户）
- 面料档案：至少 2 个品名，每个品名至少 2 个色号；缸号/色号/批次组合覆盖“多缸号/多色号”
- 仓库 1 个、库位若干（如系统启用库位）

合格标准：

- 可完成采购入库、销售开单、库存扣减、客户欠款、回款核销的全链路。

### G6-2 多缸号/色号管理（档案）

验证点：

- 产品/色号新增、编辑、查询；
- 针对“缸号+色号”相关约束：是否存在唯一性规则、是否有重复提示（以系统实际实现为准）。

合格标准：

- 档案管理无异常；重复数据的处理符合业务预期（允许/禁止/提示必须明确）。

### G6-3 整匹销售 + 散剪销售

验证点（按实际 UI 字段/接口字段核对）：

- 整匹销售：库存按匹/米扣减准确；
- 散剪销售：按米扣减准确，库存不出现负数或错批次扣减；
- 作废/回滚：作废后库存回滚准确。

合格标准：

- 库存台账与销售单据对得上；无“负库存/错批次/重复扣减”。

### G6-4 批次库存核算

验证点：

- 入库后批次库存增加；
- 销售出库后批次库存减少；
- 盘点盈亏后库存正确调整（如系统启用盘点模块）。

合格标准：

- 任意时刻可通过库存查询定位到“缸号/色号/批次”的可用量，且与单据一致。

### G6-5 下游客户赊账对账（应收）

验证点：

- 赊账销售产生应收；
- 回款核销应收减少；
- 对账单/报表可导出并核对明细。

合格标准：

- 欠款统计、核销记录、对账明细一致，且无重复计算。

## G7. GitHub Actions 打包构建验证（硬门禁）

本仓库 GitHub Actions 工作流为：

- [.github/workflows/ci-cd.yml](file:///workspace/.github/workflows/ci-cd.yml)

### G7-1 打包门禁校验（必须与 CI 一致）

验证点：

- `test` job：后端 `cargo test --all`
- `build-backend` job：后端 `cargo build --release` 并校验 `backend/target/release/server`
- `build-frontend` job：前端 `trunk build --release` 并校验 `frontend/dist`
- `package-release` job：组装 `release/bingxi-erp/`，并打包为 `bingxi-erp-<version>.zip`

合格标准：

- Actions 全绿；产物 zip 可下载并正常解压。

### G7-2 产物清单校验

必须包含（按 CI 组装逻辑抽检）：

- `backend/server`
- `frontend/` 静态资源
- `deploy/`（部署脚本、systemd、nginx 配置）
- `database/`（迁移脚本目录）

合格标准：

- 产物目录结构与 CI 一致，关键文件齐全。

## G8. 部署上线验证（硬门禁）

### G8-1 离线发布包部署（deploy.sh）

发布包内置部署脚本：

- [deploy.sh](file:///workspace/deploy/deploy.sh)

验证点（按脚本行为逐项验收）：

- 解压到 `/opt/bingxi-erp`
- 环境变量文件 `/etc/bingxi/.env` 存在且可被 systemd 读取
- systemd 服务 [bingxi-backend.service](file:///workspace/deploy/bingxi-backend.service) 正常启动
- Nginx 配置 [nginx.conf](file:///workspace/deploy/nginx.conf) 生效，静态资源与 `/api/` 反代可用

合格标准：

- `systemctl status bingxi-backend` 正常；页面可访问；核心接口 smoke test 通过。

### G8-2 在线一键安装/更新（install.sh）

一键脚本：

- [install.sh](file:///workspace/快速部署/install.sh)

验证点：

- 能正确拉取 GitHub latest release 资产并解压；
- 能调用发布包内 `deploy/deploy.sh` 完成安装/更新；
- 安装后 `bingxi` CLI 可用（start/stop/restart/status/update）。

合格标准：

- 安装与 update 均能完成，且服务自动重启成功。

## G9. 运维维护演练（硬门禁）

### G9-1 监控可用性验证（Prometheus/Grafana/Alertmanager）

仓库提供监控配置：

- Prometheus： [prometheus.yml](file:///workspace/monitoring/prometheus/prometheus.yml)、[alert_rules.yml](file:///workspace/monitoring/prometheus/alert_rules.yml)
- Grafana Dashboard： [bingxi-erp-overview.json](file:///workspace/monitoring/grafana/dashboards/bingxi-erp-overview.json)
- Alertmanager： [alertmanager.yml](file:///workspace/monitoring/alertmanager/alertmanager.yml)

注意：当前后端虽实现了 `/metrics` 路由与指标采集，但主程序路由中未发现挂载该路由的逻辑（仅实现于 [metrics_service.rs](file:///workspace/backend/src/services/metrics_service.rs)）。因此本门禁需要先确认：

- 是否要求上线时暴露 Prometheus 指标？
- 若要求：需按“变更流程”将 metrics router 挂载到主路由后，再做联调验证与留存证据。

合格标准：

- 监控项“有证据”：要么确认不启用并形成记录；要么启用后能被 Prometheus 抓取并在 Grafana 展示核心指标。

### G9-2 更新、回滚演练

验证点：

- 更新：通过 `bingxi update` 或重新执行 `install.sh update` 能完成平滑更新并重启服务；
- 回滚：需要明确“回滚策略与回滚包来源”（例如固定保留上一个 release zip/tar，或 `/opt/bingxi-erp` 版本目录备份机制）。

合格标准：

- 更新可重复、可追溯（记录版本号与 commit）；回滚有可执行步骤且演练成功。

### G9-3 数据备份与恢复策略（必须明确，不允许空白）

仓库当前未发现“应用内置备份/恢复命令”的实现，因此本门禁按 PostgreSQL 标准方式验收：

- 备份：`pg_dump`（全库或按 schema）
- 恢复：`pg_restore` 或 `psql -f`

合格标准：

- 在测试环境完成一次“备份 → 删除库 → 恢复 → 业务回归”闭环，并留存命令与日志。

## 附录 A：验证证据留存模板（建议）

- 验证批次：`YYYY-MM-DD`
- Git 信息：branch、commit SHA、tag（如有）
- 门禁结论：G0~G9 全部 PASS
- 关键日志：
  - CI 链接与 run id（GitHub Actions）
  - 部署日志（deploy.sh 输出、systemctl status）
  - 数据库落库日志（psql 输出）
  - 关键接口回归结果（Postman/脚本输出）
