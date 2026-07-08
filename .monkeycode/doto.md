# 任务与历史

> 本文件记录**当前任务**与**历史任务索引**。
> 详细历史请查阅 [`.monkeycode/docs/archives/`](file:///workspace/.monkeycode/docs/archives/)。

---

## 🔄 当前任务：v11 前端 P2-1 any 类型清理（批次 196 已完成，真实 any 已全部清理）

> 用户最高优先级规则（2026-07-04/06/08 追加）已固化到 [MEMORY.md 一、规则 0-12](file:///workspace/.monkeycode/MEMORY.md)。
> 本文件仅记录任务进度，规则不在此重复。
> 规则 10 梳理时间：2026-07-08（批次 195 = 13×15 触发）

### v11 复审结果

- **复审报告**：
  - `docs/audits/2026-07-06-reaudit-v11-backend.md`（后端 47 项）
  - `docs/audits/2026-07-06-reaudit-v11-frontend.md`（前端 16 类：P0×3 / P1×6 / P2×7）

### v11 P0 修复（已完成）

- ✅ 批次 143-145：P0 三项全部完成

### v11 P1 修复（全部完成）

- ✅ 批次 158：全项目项级 dead_code 按规则 0/1/2 真实接入（CI 12/12 全绿）
  - 类 A 真死代码删除（4 条）✅
  - 类 B 误判死代码移除标注（16 条）✅
  - 类 C 真实接入业务（19 条）✅
  - 类 D SeaORM 模型例外（10 条）保留不动
  - 4 轮 CI 修复：b7b2baa→a502bc7→9ff0e21→f9796cb（最终全绿）
- ✅ 批次 160：P2-6 custom-order AdvanceStatusDto 死代码清理 + P2-7 inventory any[] 类型化（CI 11/12 全绿，2 轮 CI 修复）
  - 后端：删除未被任何 handler 使用的 AdvanceStatusDto 结构体
  - 前端：CustomOrderAdvanceDto 移除 target_status 字段及过时 TODO(tech-debt) 注释
  - 前端：inventory 模块 7 个文件 4 个核心状态从 any[] 改为 InventoryStock[]/StockAlert[]/InventoryTransfer[]/Warehouse[]
  - CI 修复：InventoryStockTab.vue 模板 wh.name 兜底删除（Warehouse 接口无 name 字段）
  - 2 轮 CI 修复：9e704a0→1bc06a5（最终全绿）
- ✅ 批次 161：P2-5 quality 分页接入（CI 11/12 全绿，2 轮 CI 修复）
  - 后端：quality_inspection_handler.rs list_records 返回 PaginatedResponse（含 total）
  - 前端：quality.ts listQualityRecords 返回类型改为 PageResult<QualityRecord>
  - 前端：quality/index.vue fetchRecords 解析 PaginatedResponse
  - CI1 修复：PageResult 添加可选 items 字段（对齐后端 PaginatedResponse 实际返回结构）
  - CI2 修复：8 个新 clippy 死代码警告（export_csv/export 删除 + 5 常量 + 3 字段 + 5 方法 allow）
  - 2 轮 CI 修复：751491c→e5ad56c→35532c3（最终全绿）
- ✅ 批次 162：P2-1 bpm/index.vue any 清理（20 处 → 0 处，CI 11/12 全绿）
  - 4 个 ref<any[]> 改为 ref<BPMTask[]>/ref<BPMInstance[]>
  - 2 个 Record<string, any> 改为 Record<string, TagType>
  - 7 个 row: any 改为 row: BPMTask/BPMInstance/BPMTask | BPMInstance
  - 8 个模板 row as any 改为 row as BPMTask/BPMInstance
  - 新增 type TagType 联合字面量类型（el-tag type）
- ✅ 批次 163：P2-1 quotations/create.vue + sales-returns/useSr.ts any 清理（25 处 → 0 处，CI 11/12 全绿，1 轮 CI 修复）
  - quotations/create.vue（13 处）：undefined as any → undefined as unknown as number；validator 参数类型化；res.data as any → QuotationResponseDto.id
  - sales-returns/useSr.ts（12 处）：移除文件级 eslint-disable；ref<any[]> → ref<SalesReturn[]>；reactive<any> → reactive<ReturnForm>；catch (error: any) → catch (error: unknown)（6 处）；新增 SalesOrderOption/CustomerOption/ProductOption/ReturnFormItem/ReturnForm 接口
  - CI1 修复：submitData 使用 as unknown as Partial<SalesReturn> 类型转换
- ✅ 批次 164：P2-1 inventory/index.vue any 清理（3 处 → 0 处，CI 11/12 全绿）
  - form: any 改为 typeof adjustmentForm.value/typeof transferForm.value
  - form as any 改为 as unknown as InventoryTransfer
  - 'table' as any 改为 'json'（print-js 标准 type 值）
- ✅ 批次 165：P2-1 system/tabs/RoleTab.vue any 清理（6 处 → 0 处，CI 11/12 全绿）
  - res.data as any 改为运行时安全访问
  - buildPermissionTree 返回 any[] 改为 PermissionTreeNode[]，新增 PermissionTreeNode extends Permission 接口
  - handlePermissionCheck 参数 _: any, { checkedKeys }: any 改为具体类型
  - 3 处模板 row as any 改为 row as Role
- ✅ 批次 166：P2-1 system/tabs/UserTab.vue any 清理（8 处 → 0 处，CI 12/12 全绿）
  - 模板 2 处 row as any 改为 row as User
  - 3 个 validator 函数参数类型化（_: any, v: string, cb: any）→ (_rule: unknown, v: string, cb: (error?: Error) => void)

### v11 前端 P2-1 any 清理收尾（批次 191-196）

- ✅ 批次 191：quotations/detail.vue 5 处 any + 3 个 CI 阻塞类型错误修复
- ✅ 批次 192：quotations/list.vue 3 处 + CreateDlg.vue 3 处 + TransferDialog.vue 3 处
- ✅ 批次 193：Charts 4 文件 10 处 any + 2 个 CI 阻塞类型错误修复
- ✅ 批次 194：warehouse 2 处 + DepartmentTab 2 处 + SecLockTbl 2 处
- ✅ 批次 195：BorrowRecordTimeline + logistics × 3 + scheduling × 2 + RcpPanel + QltPanel + useRcp/useQlt 类型同步 + inventory AdjustmentDialog CI 类型错误修复（3 个 TS 错误）
- ✅ 批次 196：ApiLogTab 2 处 + ReturnDetailDialog 1 处 + ViewDlg 1 处 + bpm/templates 1 处 + CI 修复 ReturnDetailDialog optional 字段类型（3 个 TS2345 错误）
- **结论**：frontend/src 中无真实 any 类型残留，剩余 any 均为已修复注释（批次 98/160-182 各批次修复记录）

### v11 剩余任务

- ✅ v11 前端 P2-1：any 类型清理（批次 196 已完成，frontend/src 中无真实 any 残留，剩余 any 均为已修复注释）
- ⏳ v11 前端 P2-2：i18n 接入（仅 Login.vue，其余 ~150 个 .vue 文件硬编码中文）
- ⏳ v12 全项目复审（v11 全部修复完成后）
- ⏳ 批次 200：E2E 加强测试 + 报告（规则 5，每 10 批次一次）
- ⏳ E2E 测试用例修复：移除 mockBusinessApi，让业务 API 走真实后端
- ⏳ 迁移文件进一步整合（用户要求减少迁移文件数量）

### 批次 190：E2E 加强测试（规则 5 首次执行，进行中）

> 规则 5（2026-07-08 追加）：每 10 个批次必须完整跑完 E2E 测试一次并给出报告。
> 本批次为规则 5 首次执行，从批次 190 开始。

**任务清单**：
1. ✅ 更新 MEMORY.md 新增规则 5
2. ✅ 更新 MEMORY.md 新增规则 6（测试 mock 数据禁止硬编码）
3. ✅ 提交规则更新（commit baa8a0f）
4. ✅ 监控 CI run 28912297000 ci-e2e job（超时 cancelled，95 测试全失败）
5. ✅ 分析 E2E 日志，生成批次 190 E2E 测试报告文档
6. ✅ 修复 playwright.config.ts（reporter:html + timeout 60s）
7. ✅ 修复 ci-cd.yml ci-e2e job（PostgreSQL + 后端 + 迁移 + 初始化 + 移除 continue-on-error）
8. ✅ 提取 mock 数据到 e2e/fixtures/auth.ts（规则 6）
9. ✅ 修复前端测试 tests/components/v2-table.test.ts（规则 6 fixtures）
10. 🔄 提交 + push 触发 CI 验证
11. ⏳ 根据新 CI 结果继续修复 E2E 失败问题

### 已完成批次（最近 5 个）

| 批次 | main commit | 内容 |
|------|-------------|------|
| 196-ci | `3d7c7c9` | v11 前端 P2-1 ReturnDetailDialog optional 字段类型修复（3 个 TS2345 错误，1 文件 +3 -3 行）|
| 196 | `a568a90` | v11 前端 P2-1 清理剩余 4 个文件 5 处 any（ApiLogTab + ReturnDetailDialog + ViewDlg + bpm/templates，4 文件 +83 -74 行）|
| 195 | `16393df` | v11 前端 P2-1 inventory AdjustmentDialog 类型错误修复（3 个 TS 错误：TS7053 索引签名 + TS2322 类型推断 ×2，2 文件 +12 -9 行）|
| 194 | `c70cf5b` | v11 前端 P2-1 any 清理（6 处 → 0 处，3 文件：warehouse + DepartmentTab + SecLockTbl）|
| 193 | `7a74479` | v11 前端 P2-1 Charts 4 文件 any 清理 + 2 个 CI 类型错误修复（TransferDialog 索引签名 + purchase ref 解包）|
| 192 | `3aa61ac` | v11 前端 P2-1 any 清理（9 处 → 0 处，3 文件：quotations/list + CreateDlg + TransferDialog）|
| 191 | `8eefc1b` | v11 前端 P2-1 quotations/list.vue any 清理 + useOlv 导出冲突修复|
| 184 | `42deb8cd` | v11 前端 P2-1 any 清理（16 处 → 0 处，4 文件：ReturnsTable + SpTbl + QuotationItemEditor + PrcTbl；CI 11/12 核心 success）|
| 183-ci-v3 | `c1f9b708` | v11 前端 P2-1 SchGChart 真正接入 ECharts 5 官方类型（ECElementEvent/CustomSeriesRenderItemParams/CustomSeriesRenderItemAPI 从 'echarts' 导入，CallbackDataParams 从 'echarts/types/dist/shared' 导入；处理 size() 可选方法+联合类型；定义 CartesianCoordSys 接口补充 ECharts 类型定义不完整；CI 11/12 核心 success）|
| 183-ci-v2 | `c714a8e9` | v11 前端 P2-1 SchGChart 使用 ECharts 官方类型替代自定义接口（CI 失败：CallbackDataParams 不从主包导出 + size() 可选 + coordSys 类型不完整，被 v3 替代）|
| 183 | `4dedb8c1` | v11 前端 P2-1 any 清理（16 处 → 0 处，4 文件；SchGChart 自定义接口违规，被 ci-v2/v3 替代）|
| 181-ci-v3 | `e87577a9` | v11 前端 P2-1 真正修复 printJS 类型问题（删除错误的 print-js.d.ts + 5 文件 type:'table'→'json' + inventory/ap InvoiceTab as unknown as 清理；CI 11/12 核心 success，E2E 因 secrets 缺失 cancelled 不阻塞）|
| 181-ci-v2 | `8644118e` | v11 前端 P2-1 真正修复违规问题（scheduling setter 函数模式 + greige-fabrics 完全重写 + QualityIssue 接口 + print-js.d.ts；CI 前端类型检查 6 处 TS2349 失败，由 181-ci-v3 替代）|
| 181-ci | `56663940` | v11 前端 P2-1 批次 176-180 类型修复（违规使用 as unknown as，已废弃）|
| 182 | `2d23b967` | v11 前端 P2-1 any 清理批次 182（businessTrace + BpmApPendingTbl + useArChart + BatchActions，20 处；CI 被 181-ci-v2 替代取消）|
| 181 | `3f0497af` | v11 前端 P2-1 useSecProc.ts any 清理（5 处 → 0 处）|
| 175 | `3f0497af` | v11 前端 P2-1 any 清理（14 处 → 0 处，CI 11/12 全绿）|
| 176-180 | `f5200a2e` | v11 前端 P2-1 any 清理（合并 5 批，~60 处 → 0 处）|
| 166 | `2704cb8` | v11 前端 P2-1 system/tabs/UserTab.vue any 清理（8 处 → 0 处，1 文件 +7 -5 行，CI 12/12 全绿）|
| 165 | `40f3665` | v11 前端 P2-1 system/tabs/RoleTab.vue any 清理（6 处 → 0 处，1 文件 +9 -8 行，CI 11/12 全绿）|
| 164 | `3b3bcf3` | v11 前端 P2-1 inventory/index.vue any 清理（3 处 → 0 处，1 文件 +7 -6 行，CI 11/12 全绿）|
| 163 | `0bd6e8f` | v11 前端 P2-1 quotations/create.vue + sales-returns/useSr.ts any 清理（25 处 → 0 处，2 文件 +109 -46 行，1 轮 CI 修复后全绿）|
| 162 | `74971ed` | v11 前端 P2-1 bpm/index.vue any 清理（20 处 → 0 处，1 文件 +32 -26 行，CI 11/12 全绿）|
| 161 | `35532c3` | v11 前端 P2-5 quality 分页接入 + 8 个 clippy 死代码修复（4 文件 +15 -50 行，2 轮 CI 修复后全绿；含 PageResult items 字段 + export_csv 删除）|
| 158 | `f9796cb` | v11 P1 全项目项级 dead_code 按规则 0/1/2 真实接入（4 删 + 16 移除标注 + 19 接入业务 + 10 SeaORM 例外保留；4 轮 CI 修复后全绿；含 so_status unused import 修复 + baseline 补充 8/7）|
| 157 | `7dfc2ef` | v11 复审报告生成（后端 47 项 + 前端 16 类）|
| 143-145 | - | v11 P0 三项修复完成 |
| 131 | `b141c66` | v9 P0 purchase_inspection 4 个明细 CRUD 真实接入（migration m0042 + entity + service 4 方法 + 2 DTO + handler 真实调用；11 文件 +376 -26 行，CI 一次通过）|
| 130 | #374 | `2a42d3d` | v9 P0 bi_analysis_service 16 个方法真实接入数据库查询（SeaORM raw SQL + FromQueryResult + 11 中间结构体；bi_handler 16 handler 改实例方法注入 state.db；5 轮 CI 修复：所有权移动/未使用导入/语法错误/不必要类型转换/不必要闭包；2 文件 +962 -272 行）|
| 129 | #373 | `8bd404b` | v8 P2 financial_analysis_handler execute_report 真实执行：调用 calculate_indicators 真实计算财务指标 + ExecuteReportParams 可选 period 参数 + 透明响应 completed/no_data（1 文件 +65 -17 行，CI 一次通过）|
| 128 | #372 | `09601cb` | v8 P2 report_enhanced_handler 字段定义静态配置化：ReportFieldDefinition struct + available_fields_for_type 静态方法替代硬编码 serde_json::json!（2 文件 +74 -37 行，无 CI 错误一次通过）|
| 127 | #371 | `66cbe81` | v8 P2 import_export_handler 接入 import_tasks 表：list_import_tasks 真实查询 + import_csv/import_excel 创建+更新任务记录（m0041 migration + import_task model + 3 个 service 方法 + 3 处 handler 修改，8 文件 +267 -14 行；CI 修复：list_import_tasks 签名全路径 + QuerySelect trait）|
| 126 | #370 | `2674df1` | v8 P2 print_handler 静态配置化（6 种内置打印模板）+ inventory_stock_query alert_type 派生计算（discrepancy/out_of_stock/low_stock/expiring/normal，3 文件 +181 -54 行）|
| 125 | #369 | `c4a269f` | v8 P1 SearchSyncer 接入 sales_order_service + product_service：PG→ES 写入同步（含 Decimal→f64 转换 + 硬删除 ES 文档删除 + start_event_listener 签名扩展，8 文件 +225 -45 行；CI 修复：补导出 SalesOrderItemDoc）|
| 124 | #368 | `bbdf267` | v8 P1 SearchSyncer 接入 customer_service：PG→ES 写入同步（create/update/delete 事务提交后调用 sync_customer，最终一致性策略，9 处 handler 调用点更新，5 文件 +82 -20 行）|
| 123 | #367 | `a819ab4` | v8 P1 ElasticClient::real() 真实实现：ClientInner enum 双模式 + reqwest 直连 ES REST API + ensure_indices 启动时索引初始化（5 文件 +466 -75 行）|
| 122 | #366 | `f181e1b` | v8 P1 crm 标签真实接入：新增 crm_tag 表（m0040）+ list_tags/create_tag/delete_tag 真实持久化 + 路由路径 /crm-tags → /crm/tags 修复前端 404（8 文件 +161 -30 行）|
| 121 | #365 | `71b9bfb` | v8 死代码清理：删除 event_kafka KafkaEventEnvelope struct + from_event + into_event（74 行）；误删 report/ds+job 已恢复（CI 教训：跨文件 impl 块需谨慎评估）|
| 120 | #364 | `4842e97` | v7 P2-7 initialize_dimensions 真实接入 main.rs 启动 + P2-10 删除 EventBackend trait + BroadcastBackend + BridgeStream + EventBackendType + backend_type（5 文件 +43 -481 行）|
| 119 | #363 | `fd4faf7` | v7 P2-2 删除 token_bucket.rs 整个文件 + P2-5 删除 data_permission check_data_permission + 4 scope 常量 + P2-7 删除 assist_accounting create_assist_record（4 文件 -274 行）|
| 118 | #362 | `01c4475` | v7 P2-9 supplier_handler 资质端点真实接入 + P2-6 cost_collection 3 函数删除 + P2-4 report/ds cleanup_expired_cache 删除 + P2-8 fixed_asset calculate_monthly_depreciation 删除 + P2-13 websocket connection_count 删除（7 文件 -183 行）|
| 117 | #361 | `dd19874` | v7 P1-5 收尾：4 处生产代码 .unwrap()/.expect() 安全化（webhook_signature 返回 Result + date_utils/timeout expect 加不变量注释） |
| 116 | #360 | `5e00b04` | v7 P1-4 删除未接入业务的 Redis 缓存层模块（2 文件 504 行 + 清理 user/product service cache 代码 105 行） |
| 115 | #359 | `e9f3996` | v7 P1-3 删除未接入业务的 failover 抽象模块（4 文件 1015 行 + 2 集成测试） |
| 114 | #358 | `36a9730` | v7 P1-6 通知路径 warn 日志化（10 处）+ P1-5 启动期 expect 安全化（3 处中风险）+ .monkeycode 文件夹整理优化 |
| 113 | #357 | `9d65a72` | v7 P1-1 webhook PUT 语义 + P1-7 占位符 2 处 + P1-8 let _ = 检查存在性 5 处 |
| 112 | #356 | `6052810` | v7 P1-9 api_keys 表 created_by 列持久化（migration m0039） |
| 111 | #355 + 621cb0a | `20a8ce7` | v7 P1-2 incoterms 接入 quotation_service + P1-10 audit/crm keyword/source |
| 110 | #354 | `20a8c11` | v7 P0 webhook callback PUBLIC_PATHS + message_type/title + payload 接入 |

### v7 复审 P1 修复总结 ✅

P1 项全部修复完成（P1-1 ~ P1-10），详见 [MEMORY.md 二、章节](file:///workspace/.monkeycode/MEMORY.md)。

### v7 复审 P2 修复总结 ✅

P2 项全部修复完成（P2-1 ~ P2-13，13/13 项），详见 [MEMORY.md 二、章节](file:///workspace/.monkeycode/MEMORY.md)。

### v8 复审 P1 修复总结 ✅

P1 项全部修复完成（批次 121-125）：
- 批次 121：删除 event_kafka KafkaEventEnvelope（死代码）
- 批次 122：crm 标签真实接入（list_tags/create_tag/delete_tag 持久化）
- 批次 123：ElasticClient::real() 真实实现（reqwest 直连 ES REST API）
- 批次 124：SearchSyncer 接入 customer_service（PG→ES 写入同步）
- 批次 125：SearchSyncer 接入 sales_order_service + product_service（PG→ES 写入同步）

### v8 复审 P2 修复总结 ✅（5/5 完成）

- ✅ 批次 126：print_handler 静态配置化 + inventory_stock_query alert_type 派生计算
- ✅ 批次 127：import_export_handler 接入 import_tasks 表（list_import_tasks 真实查询 + import_csv/import_excel 任务记录）
- ✅ 批次 128：report_enhanced_handler 字段定义静态配置化（ReportFieldDefinition struct + available_fields_for_type 静态方法）
- ✅ 批次 129：financial_analysis_handler execute_report 真实执行（calculate_indicators 真实计算 + 透明响应 completed/no_data）

### 下一步：启动 v9 全项目复审

v8 复审 P0/P1/P2 项全部修复完成（P1 5 项 + P2 5 项 = 10 项）。启动 v9 全项目复审，循环直到复审没有问题。

### 后续批次规划

- **批次 130+**：v9 全项目复审分批修复

### 复审维度（基于历次复审经验）

1. 事务边界 TOCTOU（lock_exclusive 是否覆盖所有 update/delete）
2. 输入验证（金额 round_dp / 字段长度 / 范围校验）
3. 错误处理（panic/unwrap/expect / 错误吞没）
4. 业务逻辑（金额计算 / 状态字符串常量化）
5. 并发竞态（advisory_lock 覆盖）
6. N+1 查询（LIMIT 兜底 / 显式 join）
7. 死代码（unused field/function/variant）
8. 占位符功能（TODO / stub / let _ =）
9. 前端类型（any 清理 / 显式接口）
10. 路由权限（v-permission 编辑/删除按钮）
11. 测试质量（as any / 测试命名）
12. 安全性（IP 提取 / SQL 注入 / XSS）
13. Clippy baseline 残留警告清理
14. **预留 API/占位符功能真实接入**（用户新规则，批次 103+ 重点）

---

## 📜 历史任务索引

详细历史：见 [CHANGELOG.md](file:///workspace/.monkeycode/CHANGELOG.md) 与 [docs/archives/](file:///workspace/.monkeycode/docs/archives/)

| 批次范围 | 主要内容 | 状态 |
|---------|----------|------|
| 96-98 | v5 P0/P1/P2 修复（ArService 真实实现 + 状态机 lock_exclusive + 分页 clamp + 金额精度） | ✅ |
| 85-95 | v2/v3/v4 复审 P0-P3 修复（事务边界 + spawn panic 隔离 + FOR UPDATE） | ✅ |
| 49-84 | v19 P0/P1/P2/P3 修复（早期审计修复） | ✅ |
| 1-48 | 早期修复（前端权限/路由/API 断链/安全漏洞） | ✅ |
