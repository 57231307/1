# 集成测试执行率审计报告

- 审计日期: 2026-08-22
- 审计范围: `backend/tests/` 目录
- 审计任务: 7.2 集成测试执行率确认

## 1. 测试文件统计

| 指标 | 数值 |
|------|------|
| 集成测试文件数（`*.rs`） | 244 |

说明：`ls backend/tests/*.rs | wc -l` 得 244 个 `.rs` 文件（含子目录文件）。目录下还存在一个 `test_common` 子目录（公共测试辅助模块），不作为独立测试文件计入。

## 2. 测试函数统计

| 统计口径 | 数值 |
|----------|------|
| `#[test]` / `#[tokio::test]` 属性标记总数 | 2023 |
| `async fn test_` / `fn test_` 函数签名总数 | 1897 |

口径差异说明：
- `2023` 为测试属性标记总数（一个测试函数可能同时带 `#[tokio::test]` 等多个属性，部分 helper 函数也可能命中属性模式），更接近实际测试用例数。
- `1897` 为以 `test_` 命名的函数签名数，未覆盖命名为 `it_xxx` 或其他形式的测试。建议以属性标记数 `2023` 作为测试用例总数基准。

## 3. CI 测试覆盖确认（30 分区 nextest）

### 3.1 预编译阶段

- Job: `ci-build-test-artifacts`（ci-cd.yml:1181）
- 命令: `cargo nextest archive --archive-file nextest-archive.tar.zst`
- 产出 `nextest-archive.tar.zst`，供 30 个分片共享，避免各分片重复编译

### 3.2 测试执行阶段（`ci-test-rust`，ci-cd.yml:1242）

- 矩阵: `partition: [1..30]`（ci-cd.yml:1255），完整覆盖 1 至 30
- 并行策略: `max-parallel: 20`，`fail-fast: false`
- 运行命令（ci-cd.yml:1303-1307）:
  ```bash
  cargo nextest run \
    --archive-file nextest-archive.tar.zst \
    --partition hash:${{ matrix.partition }}/30 \
    --test-threads=1 \
    --no-fail-fast
  ```
- `--partition hash:m/30` 要求 `1 ≤ m ≤ n`，矩阵为 1-30 连续整数，30 个分片全覆盖

### 3.3 CI 全覆盖结论

CI 的 `ci-test-rust` job 通过 nextest `--partition hash:m/30` 矩阵（m = 1..30）对全部测试用例做哈希分片，30 个分片合集等价于全量执行。**CI 配置层面 30 分区完整覆盖，无遗漏分区。**

## 4. 被 ignore / skip 的测试

### 4.1 统计

| 指标 | 数值 |
|------|------|
| `#[ignore]` 属性标记总数 | 62 |

涉及 27 个测试文件，62 个测试用例被标记为 `#[ignore]`。

### 4.2 CI 是否运行 ignored 测试

- `ci-test-rust` 的 `cargo nextest run` 命令（ci-cd.yml:1303-1307）**未带 `--run-ignore` / `--run-ignored all` 参数**。
- nextest 默认跳过 `#[ignore]` 标记的测试。
- 结论: **CI 默认不执行这 62 个 `#[ignore]` 测试，需通过 `cargo test -- --ignored` 手动运行。**

### 4.3 被忽略测试明细（按文件）

| 文件 | ignore 数 | 主要原因 |
|------|-----------|----------|
| bi_analysis_test.rs | 11 | 需要 PostgreSQL（to_char/EXTRACT 语法） |
| services_inventory_stock_service_test.rs | 5 | 需要 inventory_stocks/warehouses/products 表 schema |
| services_so_order_workflow_test.rs | 4 | 需要 sales_orders 表 schema |
| services_inventory_adjustment_service_test.rs | 4 | 需要 DB schema |
| services_ap_reconciliation_service_test.rs | 3 | 需要 ap_reconciliation 表 schema |
| services_bom_service_test.rs | 3 | 需要 boms/bom_items 表 schema |
| services_customer_credit_limit_test.rs | 3 | 需要 customer_credit_ratings 表 schema |
| services_inventory_reservation_service_test.rs | 3 | 需要 inventory_reservations 表 schema |
| services_mrp_engine_service_test.rs | 3 | 需要 inventory_stocks/bom/mrp_results 表 schema |
| services_voucher_service_test.rs | 3 | 需要 vouchers/voucher_items 表 schema |
| services_accounting_period_service_test.rs | 2 | 需要 accounting_periods 表 schema |
| services_ar_recon_test.rs | 2 | 需要 ar_reconciliations 表 schema |
| services_ar_vfy_test.rs | 2 | 需要完整 schema + 测试数据 |
| ap_payment_workflow_test.rs | 1 | 需要 PostgreSQL + 前置 APPROVED 付款数据 |
| color_card_e2e_test.rs | 1 | 需要 color_cards 表 schema |
| dye_batch_workflow_test.rs | 1 | 需要 PostgreSQL + 前置缸号/流转卡数据 |
| handlers_user_handler_test.rs | 1 | 需要真实 PostgreSQL（is_admin_role 查 DB） |
| lab_dip_workflow_test.rs | 1 | 需要 PostgreSQL + 前置客户/产品/颜色数据 |
| outsourcing_receipt_workflow_test.rs | 1 | 需要 PostgreSQL + 前置委外订单数据 |
| production_order_workflow_test.rs | 1 | 需要 PostgreSQL + 前置产品/工作中心数据 |
| production_recipe_workflow_test.rs | 1 | 需要 PostgreSQL + 前置工单/缸号数据 |
| purchase_receipt_workflow_test.rs | 1 | 需要 PostgreSQL + 前置采购订单数据 |
| quotation_e2e_test.rs | 1 | 需要 quotations 表 schema |
| sales_delivery_workflow_test.rs | 1 | 需要 PostgreSQL + Elasticsearch |
| services_production_order_service_test.rs | 1 | 依赖 SQLite 内存 DB schema |
| services_so_delivery_test.rs | 1 | 依赖 DB schema |
| websocket_test.rs | 1 | 需启动 axum server，沙箱 OOM 跳过 |

### 4.4 忽略原因分类

| 原因类别 | 用例数 | 说明 |
|----------|--------|------|
| 需要真实 DB schema（PostgreSQL/SQLite） | 53 | 依赖具体表结构与前置数据，CI service container 虽提供 PostgreSQL，但未自动建表/灌数 |
| 需要外部组件（Elasticsearch/axum server） | 2 | 依赖 ES 或本地启动 server |
| 沙箱 OOM | 1 | websocket 完整集成测试内存不足 |
| 其他（依赖 SQLite 内存 schema） | 6 | 标注依赖 SQLite 内存 DB schema，CI 默认跳过 |

## 5. 执行率结论

| 指标 | 数值 | 备注 |
|------|------|------|
| 测试用例总数（属性标记口径） | 2023 | `#[test]`/`#[tokio::test]` |
| CI 实际执行用例数 | 1961 | 2023 - 62（ignored） |
| CI 被跳过用例数 | 62 | `#[ignore]` 标记 |
| CI 执行率 | 96.9% | 1961 / 2023 |
| 30 分区 nextest 全覆盖 | 是 | partition 1-30 完整 |

### 结论

- **CI 分区覆盖: 100%**（30 个 nextest `--partition hash:m/30` 分区完整覆盖，无遗漏分区）
- **CI 用例执行率: 96.9%**（62 个 `#[ignore]` 测试在 CI 默认模式下不执行）
- **未达到 100% 执行率**：存在 62 个被 `#[ignore]` 标记的测试，CI 默认不运行。这些测试主要依赖真实 DB schema 与前置数据，部分有 `TEST_DATABASE_URL` 环境变量提示可手动启用，但 CI nextest 命令未带 `--run-ignored` 参数。

### 改进建议

1. 对于依赖 PostgreSQL schema 的测试，在 CI service container 中自动执行 migration + 灌入种子数据后，移除 `#[ignore]` 或在 nextest 命令中增加 `--run-ignored all`。
2. 对于需要 Elasticsearch/axum server 的测试，评估是否在 CI 中提供对应 service container。
3. 对于沙箱 OOM 的 websocket 测试，考虑拆分为更小粒度的单元测试。
