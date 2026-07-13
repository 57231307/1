# 未完成任务（详细）

> 本文件**详细**记录未完成的任务（问题描述、影响范围、修复方案、技术要点），禁止简化。
> 已完成任务见 [doto-su.md](file:///workspace/.monkeycode/doto-su.md)，一句话总结见 [CHANGELOG.md](file:///workspace/.monkeycode/CHANGELOG.md)，规则见 [MEMORY.md](file:///workspace/.monkeycode/MEMORY.md)。
> 最近一次整理：2026-07-13（批次 375 规则 10 整理，归档批次 356-374 详细记录到 doto-su.md）。

---

## ✅ 历史任务：v8-v12 复审问题修复（全部完成）

- **v8 复审**（批次 290-308）：21 项问题全部修复 ✅
- **v9 复审**（批次 317-323）：16 项问题全部修复 ✅
- **sea-orm 版本调研**（批次 324）：确认使用 1.1.20 稳定版正确 ✅
- **规则 14 新增**（批次 324）：移除所有警告抑制 ✅
- **v10 复审**（批次 325-339）：53 项问题全部修复 ✅
- **v11 复审**（批次 340-346）：27 项问题全部修复 ✅
- **v12 复审**（批次 347-355）：15 项问题全部修复 ✅

> 详细记录已归档到 [doto-su.md](file:///workspace/.monkeycode/doto-su.md)，一句话总结见 [CHANGELOG.md](file:///workspace/.monkeycode/CHANGELOG.md)。
> 安全漏洞（7 项，批次 290-296）已全部修复，详见 [bug.md](file:///workspace/.monkeycode/bug.md)（当前为空）。

---

## 🔄 当前任务：v13 复审 + 业务/财务场景闭环 + 运行逻辑环流程闭环（2026-07-13 启动）

> **v13 复审报告**（2026-07-13，规则 15 启动，详见 [v13-review-2026-07-13.md](file:///workspace/.monkeycode/docs/audits/v13-review-2026-07-13.md)）：v12 复审全部完成后启动，新增"业务场景闭环""财务场景闭环""运行逻辑环流程闭环"等复审维度。
> **核心目标**：baseline 213 条摘要行（~993 个警告）全部清零 + 业务/财务/逻辑闭环问题修复 + v14 历史遗留任务合并处理。
> **执行策略**：规则 13+14+15 联动，复审完成后自动连续修复，每批 5-6 文件，CI 全绿后自动进入下一批，无需用户确认。
> **v14 历史遗留合并**：v14 深度调研报告（2026-07-09）的高风险 6/6 ✅ 已完成，中风险剩余 3 项 + 低风险 74 项 + v13 前端/后端 P2 共 9 项全部合并到 v13 复审修复队列。

### 进度总览

| 维度 | 总数 | 已完成 | 剩余 | 状态 |
|------|------|--------|------|------|
| 🟢 baseline 警告清零 | 213 摘要 / 89 位置 / 135 文件 | 11 | 202 | 🔄 批次 357 完成 11 项 unused import |
| 🟢 业务场景闭环 | 21 | 13 | 8 | 🔄 P0 6 项 + P1 7 项已完成（批次 356/358/359/360/361/364/365/366，B-P1-8 完整闭环 6 个高风险变体全部接入幂等） |
| 🟢 财务场景闭环 | 16 | 7 | 9 | 🔄 P0 2 项 + P1 5 项已完成（批次 356/358/359/360/362/363，F-P1-2 完整闭环） |
| 🟢 运行逻辑环流程闭环（5 子维度） | 45 | 40 | 5 | ✅ P1 6 项 + P2 13 项全部完成（批次 367-374）+ P3 21/26 完成（批次 375-379：L-5/L-7/L-8/L-9/L-10 + L-12/L-13/L-14/L-15 + L-17/L-18/L-19/L-20 + L-16/L-24 + L-37/L-39/L-40/L-41/L-44 + 已验证 L-25/L-33/L-34/L-35/L-45） |
| 🟢 v14 中风险遗留（测试覆盖 + useTableApi） | 3 大类 | 0 | 3 大类 | ⏳ 待修复 |
| 🟢 v14 低风险遗留 | 74 | 0 | 74 | ⏳ 后续迭代 |
| 🟢 v13 前端 P2 + 后端 P2 + 其他遗留 | 9 | 0 | 9 | ⏳ 待修复 |
| **合计** | **~378** | **70** | **~308** | — |

### v13 复审修复队列（按优先级排序，详见复审报告）

- **P0 级（14 项 - 阻塞）**：业务场景 6 项 + 财务场景 8 项
- **P1 级（19 项 - 高）**：业务场景 9 项 + 财务场景 4 项 + 运行逻辑 6 项
- **P2 级（24 项 - 中）**：业务场景 6 项 + 财务场景 4 项 + 运行逻辑 14 项
- **P3 级（25 项 - 低）**：运行逻辑 25 项
- **baseline 清零（213 项）**：dead_code 193 + unused_import 15 + 其他 5

### 已完成批次归档（批次 356-374，详见 doto-su.md）

> 批次 356-374 的详细记录已归档到 [doto-su.md](file:///workspace/.monkeycode/doto-su.md)，一句话总结见 [CHANGELOG.md](file:///workspace/.monkeycode/CHANGELOG.md)。

**批次 356-374 修复完成项汇总**：
- **业务场景 P0（6 项 ✅）**：B-P0-1 订单审批→库存预留 + B-P0-2 销售出库→库存流水 + B-P0-3 生产订单→成本核算 + B-P0-4 采购退货凭证 + B-P0-5 销售退货凭证 + B-P0-6 生产领退料凭证兼容
- **财务场景 P0（2 项 ✅）**：F-P0-1 科目余额回写 + F-P0-2 库存桥接凭证自动过账
- **业务场景 P1（7 项 ✅）**：B-P1-1 销售退货事务边界 + B-P1-2 盘点完成事件 + B-P1-4 销售订单状态变更事件 + B-P1-5 采购订单审批事件 + B-P1-6 删除孤岛事件 + B-P1-8 事件幂等（6 个高风险变体全部接入）+ B-P1-9 生产订单 BPM 回写
- **财务场景 P1（5 项 ✅）**：F-P1-1 试算平衡校验 + F-P1-2 三大报表走凭证体系 + F-P1-3 辅助核算记录写入 + F-P1-4 科目余额刷新方法
- **运行逻辑环 P1（6 项 ✅）**：L-1 CLI 吞错 + L-21 MatchStatus 缺终态 + L-26 后台任务缺 cancellation + L-27/28/29 事件总线 spawn 句柄丢失
- **运行逻辑环 P2（12 项 ✅）**：L-2/L-3 脚本吞错 + L-4 回滚吞错 + L-6 事件发送吞错 + L-11 静态正则 expect + L-22 BorrowStatus 缺取消态 + L-23 DyeBatchStatus 缺异常态 + L-30 OmniAudit 句柄丢失 + L-31 WebSocket 句柄泄漏 + L-36/L-38/L-42 配置项 silent default + L-43 .env.example 缺失声明
- **baseline 清零（11 项 ✅）**：批次 357 清理 11 个 unused import warning

### 已完成批次归档（批次 375-379）

**批次 375（PR #547 已合并）**：
- L-9 health_handler.rs：移除 `let _ = start_time_init()` 吞错模式（函数返回 Instant，非 Result，无需错误处理）
- L-5/L-7/L-8/L-10：4 个文件移除 `let _ =` 吞错模式（spawn 句柄不需要保存、AtomicUsize fetch_* 返回值不需要使用）

**批次 376（PR #548 已合并）**：
- L-12/L-13/L-14/L-15：4 个测试文件移除 `let _ = result` 吞错模式（改为 `assert!(result.is_err(), "...")`）

**批次 377（PR #549 已合并）**：
- L-17/L-18/L-19/L-20：7 个文件 12 处测试 `let _ = result` 吞错修复
- 文件：ap_reconciliation_service.rs、mrp_engine_service.rs、bom_service.rs、production_order_service.rs、customer_credit_limit.rs、voucher_service.rs、ar/recon.rs

**批次 378（PR #550 已合并）**：
- L-16 middleware/csrf.rs：9 处测试 `expect` 消除（4 个测试函数改为 `Result<(), Box<dyn std::error::Error>>` 返回 + `?` 操作符）
- L-24 services/init_service.rs：InitTaskStatus 枚举补充终态完整性文档

**批次 379（PR #552 已合并）**：
- L-37 main.rs AUDIT_RETENTION_DAYS：消除 silent default，改为 match + is_production() 区分 warn/info
- L-39 main.rs ELASTICSEARCH_URL：消除 silent default，改为 match + is_production() 区分 warn/info
- L-40 telemetry.rs：3 处 silent default（ENV/OTEL_EXPORTER_OTLP_ENDPOINT/OTEL_ENABLED），使用 LazyLock + is_production()
- L-41 cli/util/service.rs：SERVER__HOST/SERVER__PORT silent default，改为 match + eprintln 提示
- L-44 .env.example：BINGXI_ENV_FILE/BINGXI_SYSTEMD_DIR 取消注释，显式声明

### v14 历史遗留任务（合并到 v13 修复队列）

#### 1. 测试覆盖补测（7 项 - 中风险）

**问题背景**：项目测试覆盖率严重不足，关键模块零测试或低覆盖，无法保证代码质量和重构安全性。

**影响范围**：
- handlers 层：100+ 文件覆盖率仅 10%
- services 层：107 个 service 无任何测试
- frontend api 层：覆盖率 4.4%
- ai 算法层：零测试
- store 层：覆盖率低
- middleware 层：覆盖率低（permission.rs 已在批次 240 补测 23 个）

**修复方案**：
- 按模块优先级分批补测：先补核心业务 service（auth/user/order/inventory），再补 handler，最后补前端
- 每个 service 至少覆盖：正常路径 + 边界条件 + 错误处理
- 测试 mock 数据遵循规则 6（禁止硬编码，使用 fixtures 工厂函数）
- 使用 `tokio::test` + `testcontainers` 或内存数据库进行 service 集成测试

**技术要点**：
- service 测试需 mock DatabaseConnection（使用 `sea-orm-mock` 或自建 trait + mock 实现）
- handler 测试使用 `axum::test::TestServer` + 内存路由
- 前端测试使用 `vitest` + `@testing-library/vue`
- AI 算法测试使用固定输入 + 期望输出对比（Golden Master 模式）

#### 2. view 表格逻辑接入 useTableApi（剩余 10 个 - 中风险）

**当前进度**：46/56 完成（批次 267-289 已处理 46 个文件）

**待修复文件清单**（剩余 10 个 ⏳）：
- `frontend/src/views/finance/voucher/*`（财务凭证模块剩余文件）
- `frontend/src/views/data-import/*`（数据导入模块剩余文件）
- `inventory/tabs/InventoryStockTab`（1-based 分页）
- `inventoryAdjustment/AdjustmentListTab`
- `inventoryTransfer/TransferListTab`
- `barcodeScanner`（0-based 分页需特殊处理）
- `assistAccounting`（0-based 分页需特殊处理）
- 其他待扫描发现的遗漏文件

**修复方案**：
- 扫描所有使用 `el-table` + 分页的 view 文件
- 评估每个 view 的特殊逻辑（如有自定义排序/筛选需保留）
- 接入 `useTableApi` composable，删除重复的表格逻辑代码
- 保持 view 的业务逻辑不变，只替换通用表格逻辑

**技术要点**：
- `useTableApi` 已封装：分页参数管理 / 数据加载 / loading 状态 / 错误处理
- 接入时需保留 view 特有的查询参数构建逻辑（如日期范围/多字段搜索）
- 部分 view 有自定义列配置/导出功能，需评估是否纳入 composable
- **测试 mock 适配**：view 接入后不再 import `listXxx`，测试 mock 需从 `@/api/xxx` 改为 `@/api/request`，mock 返回 `{ code, message, data: { items/list, total } }`，断言 `mock.calls[0][1].params`

#### 3. v13 前端 P2（3 项 ⏳）

- FE-P2-1：前端类型定义完善（any 类型清理已完成，剩余 unknown 类型细化）
- FE-P2-2：前端组件 props 类型强化
- FE-P2-3：i18n 覆盖率（200+ 视图，后续迭代）— 大量 view 未接入 i18n，硬编码中文文本

#### 4. v13 后端 P2（3 项 ⏳）

- P2-1：后端错误处理统一（部分 handler 仍直接返回字符串而非 AppError）
- P2-2：后端日志规范（部分模块日志级别不当）
- P2-3：后端配置项完善

#### 5. 其他遗留（3 项 ⏳）

- FE-P2-6：大列表虚拟化（966 处 el-table，后续迭代）— 引入 `el-table-v2` 或 `vue-virtual-scroller`
- P2-8：剩余 143 个无测试 service（后续迭代）— 分批补测
- E2E 失败排查（已知问题，待规则 5 节点）— 下载 playwright-report 分析失败用例

### v14 低风险修复队列（74 项 - 后续迭代）

**占位符/Mock 存根（21 项）**：逐个评估，合理保留的加注释说明，不合理的真实实现
**项目规则符合性（11 项）**：评估是否符合规则 0-13，不符合的修正
**死代码（8 项）**：逐个评估是否接入业务或删除（与 v13 baseline 清零合并处理）
**其他（34 项）**：命名规范/注释完善/代码风格等，后续迭代统一处理

### v13 复审剩余项（按优先级排序）

#### 业务场景 P1 剩余（2 项 ⏳）

- **B-P1-3**：客户/供应商主数据变更未同步关联单据 — 发布 `CustomerUpdated`/`SupplierUpdated` 事件，监听器异步刷新关联单据（改动面大，需评估冗余字段范围）
- **B-P1-7**：事件处理失败无重试 + 死信队列 + 告警 — 引入重试机制（指数退避）+ 死信队列 + 告警

#### 业务场景 P2 剩余（6 项 ⏳）

- B-P2-1：ar_service create_payment 与 mark_as_paid 状态更新重复
- B-P2-2：customer_credit_evaluate 孤岛 service（评估后删除或接入业务）
- B-P2-3：CostCollectionService 仅 HTTP 调用，无业务联动
- B-P2-4：MrpEngineService 仅 HTTP 调用，无业务联动
- B-P2-5：CapacityService 仅 HTTP 调用，无业务联动
- B-P2-6：InventoryReservationService 仅 HTTP 调用，销售流程未集成

#### 财务场景 P0 剩余（6 项 ⏳）

- **F-P0-3**：销售出库缺收入凭证 — 在销售出库时同步生成收入凭证 + 成本凭证
- **F-P0-4**：AR 收款未生成凭证 — 在 create_payment 中调用 voucher_service 生成核销凭证
- **F-P0-5**：AP 付款未生成凭证 — 在 confirm 中调用 voucher_service 生成核销凭证
- **F-P0-6**：销售→应收链路断开 — 在销售发货后生成应收发票
- **F-P0-7**：采购→应付链路断开 — 在采购入库后生成应付发票
- **F-P0-8**：AR/AP 核销未生成凭证 — 在核销时生成核销凭证

#### 财务场景 P1 剩余（1 项 ⏳）

- **F-P1-1 剩余**：期末结转逻辑（下期期初余额写入）— close_period 新增期末结转，将本期期末余额写入下期期初余额

#### 财务场景 P2 剩余（4 项 ⏳）

- F-P2-1：无期末调整（暂估/摊销/预提）机制
- F-P2-2：报表无穿透追溯功能
- F-P2-3：销售成本按 product.cost_price 计算未与采购实际单价联动
- F-P2-4：AR/AP 对账单生成不触发凭证

#### 运行逻辑环 P3 剩余（6 项 ⏳ - 低优先级）

详见 [v13-review-2026-07-13.md](file:///workspace/.monkeycode/docs/audits/v13-review-2026-07-13.md) 第四节。

**已完成 P3 项（20 项 ✅）**：
- 批次 375（5 项）：L-5 system_update_handler 3 处 let _ = remove_file 吞错 + L-7 websocket broadcast let _ = tx.send 吞错 + L-8 cli/admin stdin.write_all 吞错 + L-9 main.rs start_time_init 吞错 + L-10 init_service 冗余 let _ = query_result 死代码
- 批次 376（4 项）：L-12 email_service hmac_sha256 expect 消除 + L-13 hash_password Params::new expect 消除 + L-14 date_utils 2 处 expect 消除（utc_offset/today_start_utc）+ L-15 middleware/timeout fallback expect 消除
- 批次 377（4 项）：L-17 ap_reconciliation_service 2 处 + voucher_service 1 处 + ar/recon 1 处 测试 let _ = result 吞错 + L-18 mrp_engine_service 3 处测试 let _ = result 吞错 + L-19 bom_service 2 处 + customer_credit_limit 1 处 测试 let _ = result 吞错 + L-20 production_order_service 测试 let _ = service 占位抑制 dead_code
- 批次 378（2 项）：L-16 CSRF 中间件测试 9 处 expect 改为 ? 操作符 + Result 返回类型 + L-24 InitTaskStatus 枚举补充终态完整性文档
- 已验证通过（5 项）：L-25 process_state_machine 完整闭环 + L-33 数据库事务 commit/rollback 路径完备 + L-34 Arc<Mutex<T>> 锁中毒处理统一降级 + L-35 文件句柄 Drop 闭环 + L-45（已验证）

**剩余 P3 项（6 项 ⏳）**：
- L-32：审计日志 spawn 句柄丢失（改为 mpsc channel + 单消费者模式，需调研）
- L-37/L-39/L-40/L-41：silent default 项（配置依赖闭环）
- L-44：silent default 项（配置依赖闭环）

---

## 规则节点提醒

- **规则 5（E2E 独立工作流，每 30 批次）**：批次 330 已到期需触发（403 权限不足，需用户手动触发）
  - 批次 N（30 倍数）：触发 e2e-batch.yml workflow_dispatch
  - 批次 N+20：第 1 次监控（GitHub API 查询 run 状态）
  - 批次 N+28：第 2 次监控（若 N+20 未完成）
  - 批次 N+29：最后监控，未完成则跳过 N+30 的 E2E 周期
  - **注意**：E2E 已从 ci-cd.yml 独立到 e2e-batch.yml，不阻塞主 CI
- **规则 10（每 15 批次记忆整理）**：批次 375 已完成（归档批次 356-374 详细记录到 doto-su.md + 精简 doto.md + 新增 v13 剩余项清单）
  - 下次整理：批次 390
- **规则 13（修复流程自动化与连续执行）**：CI 全绿后自动开始下一批，无需用户确认
- **规则 14（移除所有警告抑制）**：所有警告视为错误需修复
- **规则 15（v13 复审严格规范 + 业务/财务场景闭环 + 运行逻辑环流程闭环）**：2026-07-13 新增
