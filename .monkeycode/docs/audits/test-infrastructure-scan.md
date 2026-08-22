# 测试体系扫描报告

> 审计代理 · /workspace 仓库 · 静态扫描（不修改代码、不创建 PR、不推送）
> 扫描范围：7.1 / 7.3 / 7.6 / 7.7 / 7.8 / 7.9 / 7.10 / 7.11 / 7.12 / 7.13 共 10 项
> 数据来源：`.github/workflows/ci-cd.yml`、`.github/workflows/e2e-batch.yml`、`backend/tests/`、`backend/benches/`、`backend/migration/`、`frontend/`、`codecov.yml`、`frontend/vitest.config.ts`

## 7.1 单测覆盖率

**结论：通过（cobertura.xml 已接入 CI 与 Codecov）**

- `ci-coverage-rust` job（ci-cd.yml:1390）使用 `cargo llvm-cov nextest --cobertura --output-path coverage/cobertura.xml`（ci-cd.yml:1439-1440）产出 Cobertura XML。
- 覆盖率上传到 Codecov（`codecov/codecov-action@v4`，ci-cd.yml:1492-1500），`fail_ci_if_error: false`（信息性，不阻塞合并）。
- `codecov.yml` 配置：全项目 60% 目标、核心 service 模块 80%、前端 store 70%，均 `informational: true`。
- 前端 `vitest.config.ts` thresholds 当前仅 1%（lines/functions/branches/statements 均为 1），注释说明"当前 1.78%，逐步提升至 70%"——前端覆盖率门槛实质性失守。

## 7.3 E2E 完整通过

**结论：无法验证最近实际状态（gh CLI 未认证），工作流配置存在显著盲区**

- `gh auth status` 返回 "not logged into any GitHub hosts"，无法通过 `gh api` 查询最近 main CI 的 `ci-e2e` job 状态。建议人工确认 token 可用后补查。
- 工作流配置层面：
  - `ci-e2e`（ci-cd.yml:2370）仅在 `push` 到 main/master/tag 时触发，**PR push 不跑 E2E**（ci-cd.yml:2375 if 条件）。
  - `e2e-batch.yml` 为独立工作流，仅 `workflow_dispatch` 手动触发（每 30 批次），`concurrency.cancel-in-progress: false`。
  - E2E 仅运行 `--project=chromium` 单浏览器（e2e-batch.yml:264），firefox/webkit 不在 CI 覆盖范围。
  - `frontend/e2e/` 下有 173 个 `.spec.ts` 文件，规模可观，但实际通过率依赖手动触发批次，缺乏每次 PR 的 E2E 门禁。

## 7.6 性能基准

**结论：通过（4 项基准已接入 CI，但为信息性，不阻塞）**

- `backend/benches/` 含 4 个基准：`dye_cost_collection_bench.rs`、`inventory_calculation_bench.rs`、`voucher_generation_bench.rs`、`wage_calculation_bench.rs`。
- `ci-perf-bench` job（ci-cd.yml:1764）运行上述 4 项，使用 `cargo bench --bench <name> -- --save-baseline "main"`（ci-cd.yml:1804），criterion 缓存基线（`backend/target/criterion`）。
- 仅在 main/master/tag 触发（ci-cd.yml:1771），`continue-on-error: true`（ci-cd.yml:1772），回归 > 10% 写入报告但**不阻塞合并**。
- 无 PR 级性能回归门禁——回归仅在 main 上被记录，PR 阶段不拦截。

## 7.7 覆盖率报告

**结论：通过（artifact 已上传，但保留期仅 30 天）**

- `ci-coverage-rust` 上传 artifact `rust-coverage-report`（ci-cd.yml:1502-1510），包含 `backend/reports/` 与 `backend/coverage/`，`retention-days: 30`。
- 前端覆盖率 artifact `frontend-coverage`（ci-cd.yml:1585-1590）上传 `frontend/coverage/coverage-final.json`，`retention-days: 90`。
- Rust 报告保留期 30 天 vs 前端 90 天 vs 测试报告 90 天——Rust 覆盖率保留期偏短，长周期趋势追踪受限。
- Codecov 侧上传 `fail_ci_if_error: false`，上传失败不阻塞 CI，存在静默丢失风险。

## 7.8 测试有效性

**结论：断言数量充足，但需警惕空壳测试**

- `backend/tests/` 共 245 个 `.rs` 文件，`assert` 相关匹配共 4716 行。
- 断言类型分布：`assert!` 2393、`assert_eq!` 2281、`assert_ne!` 44、`assert` 1。
- 单文件平均约 19 行断言，总量健康。
- 风险点：4716 行断言中含 `assert!(result.is_ok())` / `assert_eq!(resp.code, 200)` 等浅断言，无法识别"断言恒真"或"断言空集合"等空壳测试。建议抽样审查高断言密度文件的断言质量（而非仅计数）。

## 7.9 测试隔离

**结论：部分通过（夹具已抽取，但 sqlite 回退破坏隔离）**

- 公共夹具 `backend/tests/test_common/mod.rs` 提供 `setup_test_db()`，被 27/245 个测试文件引用。
- 夹具优先读 `TEST_DATABASE_URL`，**未设置时回退到 `sqlite::memory:`**（test_common/mod.rs:25）并打印警告。
- CI 中 `DATABASE_URL` 已指向 PG16 service container，CI 隔离 OK。
- **本地开发隔离缺陷**：未设 `TEST_DATABASE_URL` 的开发者默认跑 sqlite，与生产 PG 方言不一致（见 7.10），测试结果可能误导。`sqlite::memory:` 每个连接独立实例，跨测试共享状态需显式传递，存在隐式状态泄漏风险。
- 207 处 `setup_test_db` / `test_common` / `fixture` 引用，夹具复用率高。

## 7.10 环境一致性

**结论：不通过（CI PG16 / 生产 PG18 / 本地 SQLite 三方不一致）**

- CI：`postgres:16-alpine`（ci-cd.yml:1259、1398、e2e-batch.yml:72 共 5 处）。
- 生产：`backend/src/lib.rs:4` 注释明确"数据库：PostgreSQL 18"。
- 本地回退：`sqlite::memory:`（test_common/mod.rs:25）。
- **三版本不一致**：CI(PG16) ← 生产(PG18) ← 本地(SQLite)。
- test_common/mod.rs:14-15 已自知差距：JSONB、部分索引、DO 块、RLS 在 sqlite 上语义不同。
- `rls_context_test.rs:106` 注释"SQLite 不支持 SET LOCAL（PostgreSQL 事务级会话变量）"——RLS 相关测试在 sqlite 回退时进入降级分支，实际未覆盖生产路径。
- 迁移文件 128 个（`backend/migration/`），CI 仅在 `ci-e2e` 与 `ci-coverage-rust` 跑 `bingxi migrate run`，单测分片 job（ci-test-rust）未显式跑迁移，依赖 schema 自动同步或测试内建表。

## 7.11 契约测试

**结论：不通过（无契约测试工具，无快照测试）**

- `backend/Cargo.toml` 未引入 `insta`、`schemathesis`、`pact`、`openapi` 任何契约/快照工具。
- `rg "snapshot|insta" backend/tests/` 返回 0 真匹配（命中的 `snapshot` 字段如 `before_snapshot`/`after_snapshot` 是业务字段，非快照测试）。
- `rg "contract" backend/tests/` 57 处命中均为业务模块名（`labor_contract`、`sales_contract`），非契约测试。
- `utils_migration_jump_detector_test.rs` 检测迁移编号跳跃，属迁移完整性测试，非 API 契约。
- 前端无 mock server 契约测试（`frontend/tests/fixtures/` 为静态 mock 数据，非 pact 契约）。
- API 层（Axum handler）与前端调用方之间无契约约束，schema 漂移只能靠 E2E 暴露。

## 7.12 混沌测试

**结论：不通过（零混沌测试覆盖）**

- `rg "chaos|fault.*inject|toxiproxy|failpoint|fail_point" backend/` 返回 **0 匹配**。
- `backend/Cargo.toml` 未引入 `toxiproxy`、`failpoint` 等故障注入依赖。
- 无网络分区、数据库故障、依赖超时等混沌场景测试。
- `failover_metrics_test.rs` 存在，但属指标计算单测，非故障注入。
- 生产环境依赖（PostgreSQL、Kafka、Elasticsearch）的故障恢复路径无测试覆盖，仅靠 E2E happy path 验证。

## 7.13 flaky 率

**结论：不通过（无系统化 flaky 治理，仅 clippy 有基础设施重试）**

- `ci-cleanup-retry` job（ci-cd.yml:2636）名为 "retry" 实为"收尾清理"，注释明确"仅报告，无自动重试"（ci-cd.yml:2634），收集各 job 结果后提示"人工排查后手动重试"（ci-cd.yml:2677）。
- 唯一的重试逻辑在 clippy（ci-cd.yml:437-455，`MAX_ATTEMPTS=3`），仅针对退出码 143/137/1（基础设施问题），**不重试测试失败**。
- `ci-test-rust` 30 分片使用 `fail-fast: false`（ci-cd.yml:1249），单分片失败不取消其他分片，但分片内失败即阻塞，无重试。
- 无 `--retries` / `rerun-failed` / `flaky` 标记机制，无 flaky 测试隔离/quarantine 列表。
- nextest 支持 `--retries` 参数但未在 CI 中启用。
- 集成测试 `--test-threads=1`（ci-cd.yml:1306）串行执行，降低竞态 flaky，但无法消除数据库状态泄漏导致的 flaky。

## 10 项评估总结

| 编号 | 检查项 | 结论 | 关键风险 |
|------|--------|------|----------|
| 7.1 | 单测覆盖率 | 通过 | 前端门槛实质性失守（1%），Rust 信息性不阻塞 |
| 7.3 | E2E 完整通过 | 无法验证 | gh 未认证；PR 不跑 E2E，仅 main 手动触发 |
| 7.6 | 性能基准 | 通过 | 仅 main 触发、不阻塞合并，PR 无性能门禁 |
| 7.7 | 覆盖率报告 | 通过 | Rust artifact 保留期 30 天偏短，上传失败静默 |
| 7.8 | 测试有效性 | 部分通过 | 断言数充足，但浅断言/空壳测试无法自动识别 |
| 7.9 | 测试隔离 | 部分通过 | 夹具已抽取，但 sqlite 回退破坏隔离 |
| 7.10 | 环境一致性 | 不通过 | CI PG16 / 生产 PG18 / 本地 SQLite 三方不一致 |
| 7.11 | 契约测试 | 不通过 | 零契约工具、零快照测试 |
| 7.12 | 混沌测试 | 不通过 | 零故障注入、零混沌覆盖 |
| 7.13 | flaky 率 | 不通过 | 无测试重试、无 flaky 隔离机制 |

**高优先级整改项**：
1. 7.10 环境一致性：CI 升级到 PG18 对齐生产，或生产降级到 PG16 对齐 CI；本地 sqlite 回退应改为拒绝运行（fail-fast）而非降级。
2. 7.13 flaky 率：nextest 启用 `--retries 2`，建立 flaky 测试 quarantine 列表。
3. 7.11 契约测试：引入 `insta` 快照测试 + OpenAPI schema 生成与校验。
4. 7.12 混沌测试：引入 `toxiproxy` 或 `failpoint`，覆盖 DB/网络故障恢复路径。
5. 7.3 E2E：配置 gh token 后补查最近 main CI 实际状态；评估 PR 级 smoke E2E 可行性。
6. 7.1 前端覆盖率：vitest thresholds 从 1% 提升至 60%，对齐 codecov.yml 目标。
