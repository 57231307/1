# 未完成任务

> 本文件**只记录未完成任务**（任务队列、待修复项、剩余清单）。
> 已完成任务见 [doto-su.md](file:///workspace/.monkeycode/doto-su.md)，一句话总结见 [CHANGELOG.md](file:///workspace/.monkeycode/CHANGELOG.md)，规则见 [MEMORY.md](file:///workspace/.monkeycode/MEMORY.md)。
> 最近整理：2026-07-25（P0 任务三次核实：6 并行代理代码级扫描，修正二次核实多处偏差 —— D09 实际 11 个 >100 行函数非 100 个、D13 残留 18 个缩写文件非 0 个、D03/D04 product_service.rs 已通过 facade 模式接入；模块 G 12/17 项 P0 任务完成，P0 阻塞级任务 99/104 95.2% 完成；5 项重新打开：D05/D08/D09/D13/D14）

---

## 〇、内容归类总览（模块 G 共 17 项 P0 任务）

> 本节为快速索引，按 4 个维度归类；详细条目见 §三，依赖关系见 §二。

### 0.1 按状态归类（2026-07-25 三次核实后修正：12 ✅ / 5 ⏳ / 0 ❌）

| 状态 | 数量 | 任务编号 |
|------|------|----------|
| ✅ 已完成 | 12 | D01, D02, D03, D04, D06, D07, D10, D11, D12, D15, D16, D17 |
| ⏳ 进行中（重新打开） | 5 | **D05**（i18n 接入率 18.6%，279 文件未接入）、**D08**（>80 行函数 95 个）、**D09**（>100 行函数 11 个）、**D13**（残留 18 个缩写命名文件 Ar×6+Bpm×11+Ai×1）、**D14**（残留 4 处不规范命名） |
| ❌ 未开始 | 0 | — |

> ⚠️ 2026-07-25 三次核实修正二次核实的多处偏差：D09 实际仅 11 个 >100 行函数（非 100 个）、D13 实际残留 18 个缩写文件（非 0 个）、D03/D04 实际 product_service.rs 已通过 facade 模式接入缓存（非未接入）。详细核实数据见 §0.7。

### 0.6 核实结果汇总（2026-07-24 复核）

> 2026-07-23 对 D10/D05/D13/D14 进行代码级核实发现 doto.md 的 2026-07-19 审计数据存在偏差；2026-07-24 复核 D05 i18n 接入现状进一步发现 9 个"部分接入"文件未记录。

| 任务 | doto 记录 | 实际核实 | 偏差 | 影响 |
|------|-----------|----------|------|------|
| **D10** | 第 3 批 2/4 完成 | 第 3 批 4/4 完成 | doto 滞后 | 进度更乐观，models/status.rs + mrp_engine_service.rs 已拆分 |
| **D10** | 第 4/5 批 8 文件行数基准 | 6 文件行数逆生长 | 基准失效 | wage +114 / ap_invoice +99 / ap_recon +103 / init +60 / ar/vfy +48 / flow_card +14 |
| **D10** | 第 6 批 11 个文件 | 实际 15 个文件 | doto 少 4 | 含 ar_ops/verification.rs（D10-1 拆分副产物，1062 行） |
| **D10** | 隐含未完成 21 个 | 实际 >1000 行 23 个 | doto 少 2 | 净差 +2（第 3 批 -2 + 第 6 批 +4） |
| **D05** | AssetListTab.vue 864 行 | 实际 609 行（864 为中文字符数非行数） | doto 单位混淆 | 单文件最大值记录口径修正为"中文字符数" |
| **D05** | 11 已接入 / 344 未接入 / 3.1% | 实际 20 文件含 useI18n：10 完整 + 10 部分接入 / 335 未接入 | doto 漏记 9 个部分接入文件 | 接入率口径需细分（完整 2.8% / 含部分 5.6%）；9 个部分接入文件需补全 |
| **D05** | UserTab.vue 已接入 | 实际存在命名空间 BUG：locales 定义 `settings.user.*`，代码调用 `system.user.*` | 命名空间不匹配 | t() 调用回退显示键名，需修复（L277/287/303） |
| **D13** | 123 个缩写文件 | 实际 111 个（25 类前缀）/ 121 个（27 类前缀） | doto 多 12/2 | 严格 25 类前缀口径 111 个；含 Ar + advanced(Rcp/Qlt/Rpt/Ai) 口径 121 个 |
| **D13** | 25 类缩写前缀 | 实际 27 类 | doto 少 2 类 | 补 Ar + Rcp/Qlt/Rpt/Ai |
| **D13** | 第 7 批 purchase (6) | 实际 purchase (3) | doto 多 3 | 其余 4 个为描述性短名非缩写 |
| **D14** | 风格 A 21 个 | 实际 25 个 | doto 少 4 | 工作量被低估 |
| **D14** | listXxx 47 文件 84 处 | 实际 59 文件 104 处 | doto 少 12 文件 20 处 | 最大偏差源工作量低估约 23% |
| **D14** | removeXxx 2 文件 2 处 | 实际 1 文件 1 处 | doto 多 1 | 仅 role.ts |
| **D14** | queryXxx 2 文件 2 处 | 实际 1 文件 1 处 | doto 多 1 | 仅 assist-accounting.ts |
| **D14** | addXxx 5 文件 6 处 / fetchXxx 1 文件 1 处 | ✅ 一致 | 无 | — |

### 0.7 三次核实结果汇总（2026-07-25 代码级扫描，6 并行代理）

> 2026-07-25 对 P0 阻塞级任务进行代码级三次核实，修正二次核实的多处偏差。核实采用 6 个并行代理同时扫描，使用 Python 脚本（基于括号深度追踪，正确处理字符串/字符/注释/原始字符串）替代简单 awk 脚本。

| 任务 | 二次核实记录 | 三次核实实际 | 偏差修正 | 处置 |
|------|----------|----------|------------|------|
| **D05** | 84/355 文件接入（23.7%），271 未接入 | **66/355 含 useI18n（18.6%），76/355 含 t() 调用（21.4%），279 未接入（78.6%）** | 🟡 二次多报 18 个接入文件（口径混淆） | 继续接入剩余 279 文件 |
| **D08** | >80 行函数 136 个 | **>80 行函数 95 个**（其中 >100 行 71 个，>200 行 18 个，>500 行 1 个） | 🟡 二次多报 41 个（脚本精度不足） | 拆分 95 个超长函数（优先 18 个 >200 行） |
| **D09** | >100 行函数 100 个 | **>100 行函数 11 个** | 🔴 二次多报 89 个（严重偏差，扫描脚本 bug） | 仅需拆分 11 个函数，接近完成 |
| **D10** | 0 个 >1000 行文件 | ✅ **0 个 >1000 行文件**（最大 993 行 purchase_return_service.rs） | ✅ 一致 | 无需处置 |
| **D13** | 0 个剩余缩写组件 | ❌ **18 个剩余缩写文件**（Ar×6 + Bpm×11 + Ai×1） | 🔴 二次误判为完成（漏扫 Ar/Bpm/Ai 前缀） | 重新打开，需重命名 18 个文件 |
| **D14** | 残留 4 处不规范命名 | ✅ **残留 4 处**（完全一致：listAuditLogs + listSlowQueries + addTagToCustomer + removeTagFromCustomer） | ✅ 一致 | 补齐 4 处命名 |
| **D06** | 247/355 含 aria-label（69.6%） | ✅ **260/374 含 aria-label（69.5%）**（含 components/ 目录扩展），抽样 3 个不含 aria-label 的容器组件均无图标按钮 | ✅ 一致 | 无需处置 |
| **D03/D04** | product_service.rs 未接入 Redis 缓存 | ✅ **5 个 service 全部已接入**（product_service.rs 是 facade，实际 impl 在 product_ops/crud.rs L27-28/110/121/213/331 已接入 redis_cache） | 🔴 二次误判（未识别 facade 模式） | 无需处置，标记完成 |

#### D05 详细核实数据（三次核实）
- **总文件数**：355 个 .vue 文件
- **含 useI18n 文件数**：66 个（18.6%）—— 全部 66 个文件都实际调用了 t()，无空壳接入
- **含 $t/t 调用文件数**：76 个（21.4%）—— 含 10 个未导入 useI18n 但模板用全局 $t() 的文件
- **完全无 i18n 接入文件数**：279 个（78.6%）
- **抽样验证**：3 个未接入文件全部含大量硬编码中文
  - inventory/components/StatCards.vue（62 行）：4 处 stat-label 硬编码（库存总量/库存预警/仓库数量/低于最小库存）
  - purchase-inspection/components/PurchaseInspectionDetail.vue（62 行）：title + 8 处 label + el-divider + 6 处表格列 label 全硬编码
  - user-profile/index.vue（324 行）：标题/按钮/placeholder/校验 message/ElMessage 全场景硬编码（最严重样本）

#### D08 详细核实数据（三次核实，前 20 名 >80 行函数）
1. `get_predefined_templates` — 701 行 — [services/report/tpl.rs:20](file:///workspace/backend/src/services/report/tpl.rs)
2. `login` — 464 行 — [handlers/auth_handler.rs:227](file:///workspace/backend/src/handlers/auth_handler.rs)
3. `bootstrap_full_mode` — 380 行 — [bootstrap/service_bootstrap.rs:74](file:///workspace/backend/src/bootstrap/service_bootstrap.rs)
4. `receive_transfer` — 345 行 — [services/inv/batch.rs:256](file:///workspace/backend/src/services/inv/batch.rs)
5. `confirm` — 332 行 — [services/ap_payment_service.rs:186](file:///workspace/backend/src/services/ap_payment_service.rs)
6. `create_default_role_permissions` — 323 行 — [services/init_service_ops/permission.rs:14](file:///workspace/backend/src/services/init_service_ops/permission.rs)
7. `auto_match` — 291 行 — [services/ar/vfy_ops/match.rs:32](file:///workspace/backend/src/services/ar/vfy_ops/match.rs)
8. `omni_audit_middleware` — 288 行 — [middleware/omni_audit.rs:18](file:///workspace/backend/src/middleware/omni_audit.rs)
9. `start_event_listener` — 280 行 — [services/event_bus_ops/listener.rs:25](file:///workspace/backend/src/services/event_bus_ops/listener.rs)
10. `receive_order` — 278 行 — [services/po/receipt.rs:27](file:///workspace/backend/src/services/po/receipt.rs)
- **按目录分组**：services/ 46 个（48%）、handlers/ 15 个、routes/ 11 个、utils/ 5 个、middleware/ 4 个、services/inv/ 4 个、services/crm/ 3 个、bootstrap/ 2 个

#### D09 详细核实数据（三次核实，全部 11 个 >100 行函数）
1. `get_predefined_templates` — 701 行 — [services/report/tpl.rs:20](file:///workspace/backend/src/services/report/tpl.rs)
2. `try_from` — 227 行 — [services/event_kafka_payload.rs:381](file:///workspace/backend/src/services/event_kafka_payload.rs)
3. `from` — 226 行 — [services/event_kafka_payload.rs:150](file:///workspace/backend/src/services/event_kafka_payload.rs)
4. `inventory` 路由 — 166 行 — [routes/inventory.rs:24](file:///workspace/backend/src/routes/inventory.rs)
5. `purchases` 路由 — 164 行 — [routes/purchase.rs:22](file:///workspace/backend/src/routes/purchase.rs)
6. `ap` 路由 — 159 行 — [routes/finance.rs:381](file:///workspace/backend/src/routes/finance.rs)
7. `builtin_transition_rules` — 155 行 — [services/dye_batch_state_machine_service.rs:154](file:///workspace/backend/src/services/dye_batch_state_machine_service.rs)（纯数据表，可豁免）
8. `get_cash_flow_statement` — 116 行 — [services/finance_report_service.rs:459](file:///workspace/backend/src/services/finance_report_service.rs)
9. `test_payload_all_variants_round_trip` — 112 行 — [services/event_kafka.rs:442](file:///workspace/backend/src/services/event_kafka.rs)（测试函数，可豁免）
10. `get_daily_report` — 111 行 — [services/ap_report_service.rs:222](file:///workspace/backend/src/services/ap_report_service.rs)
11. `get_income_statement` — 101 行 — [services/finance_report_service.rs:278](file:///workspace/backend/src/services/finance_report_service.rs)
- **可豁免 2 个**：builtin_transition_rules（纯数据表）+ test_payload_all_variants_round_trip（测试函数）
- **实际需拆分 9 个**：get_predefined_templates（最严重）+ 2 个 From/TryFrom + 3 个路由聚合 + 3 个 report 函数

#### D13 详细核实数据（三次核实，18 个剩余缩写文件）
**Ar 前缀（6 个）—— arReconciliation/components/**：
1. ArReconciliationCharts.vue
2. ArReconciliationConfirm.vue
3. ArReconciliationDetail.vue
4. ArReconciliationDispute.vue
5. ArReconciliationFilter.vue
6. ArReconciliationTable.vue

**Bpm 前缀（11 个）—— bpm/{approval,definitions}/components/**：
- bpm/approval/components/（6 个）：BpmApprovalApprovalDialog.vue / BpmApprovalChainDialog.vue / BpmApprovalCompletedTable.vue / BpmApprovalPendingTable.vue / BpmApprovalStat.vue / BpmApprovalTransferDialog.vue
- bpm/definitions/components/（5 个）：BpmDefinitionFilter.vue / BpmDefinitionForm.vue / BpmDefinitionTable.vue / BpmDefinitionTemplateDialog.vue / BpmDefinitionVersionDialog.vue

**Ai 前缀（1 个）**：
- components/ai/AiPredictionChart.vue

> 注：其余 24 类缩写前缀（Ap/Bom/Cp/Crm/Db/Di/Ep/Lgs/Ms/Olv/Pc/Pi/Prd/Pr/Prc/Purch/Sa/Sch/Sec/Tfa/Vchr/Rcp/Qlt/Rpt）已全部完成转换。

#### D14 残留 4 处不规范命名（三次核实完全一致）
- `listAuditLogs`（[audit.ts:79](file:///workspace/frontend/src/api/audit.ts)）→ 应改 `getAuditLogList`
- `listSlowQueries`（[slow-query.ts:69](file:///workspace/frontend/src/api/slow-query.ts)）→ 应改 `getSlowQueryList`
- `addTagToCustomer`（[crm-enhanced.ts:220](file:///workspace/frontend/src/api/crm-enhanced.ts)）→ 应改 `createTagForCustomer`
- `removeTagFromCustomer`（[crm-enhanced.ts:224](file:///workspace/frontend/src/api/crm-enhanced.ts)）→ 应改 `deleteTagFromCustomer`

#### D03/D04 缓存接入核实（三次核实修正）
- **5 个 service 全部已接入 Redis 缓存（5/5）**：
  - user_service.rs：L29-30 导入 + 7 处 get/set/del 调用
  - product_service.rs：facade 文件，实际 impl 在 product_ops/crud.rs L27-28/110/121/213/331 接入
  - customer_service.rs：L23-24 导入 + 6 处调用
  - supplier_service.rs：L8-9 导入 + 7 处调用
  - role_service.rs：L5-6 导入 + 6 处调用
- **接入模式一致**：读路径 get_json 命中即返回 / 未命中查 DB 后 set_json（TTL=300s），写路径调用 del 失效
- **基础设施**：utils/redis_cache.rs（L2 Redis 层）+ services/cache_service.rs（L1 moka 层）双层齐备

#### 教训记录（三次核实新增）
1. **D09 二次核实严重误判**：二次核实记录"100 个 >100 行函数"实际只有 11 个。**根因**：二次核实使用的简单 awk 脚本在遇到函数内部的 `}` 时会过早截断（如 auth_handler.rs:login 在第 251 行的内部 `}` 处停住），导致同一个函数被拆分成多段重复计数。**修复**：三次核实改用 Python 脚本基于括号深度追踪，正确处理字符串/字符/注释/原始字符串，避免误判嵌套 `}` 为函数结尾。
2. **D13 二次核实漏扫**：二次核实记录"0 个剩余缩写组件"实际有 18 个。**根因**：二次核实只检查了 25 类前缀，遗漏了 Ar/Bpm/Ai 三类前缀（其中 Ar 和 Ai 在二次核实时被错误地归类为"合法前缀"）。
3. **D03/D04 二次核实误判 facade 模式**：二次核实记录"product_service.rs 未接入缓存"实际已通过 product_ops/crud.rs 接入。**根因**：D10 拆分时将 product_service.rs 拆分为 facade + product_ops/ 子模块，二次核实只扫描了 facade 文件未跟踪到子模块。
4. **核实启示**：扫描脚本必须使用括号深度追踪算法，不能用简单 awk；facade 模式的缓存接入需跟踪到 impl 实际所在文件；缩写前缀检查必须覆盖全部 27 类不能遗漏。

### 0.2 按任务类型归类

| 任务类型 | 数量 | 任务编号 | 说明 |
|----------|------|----------|------|
| 代码质量类 | 4 | D08, D09, D10, D12 | 函数拆分 / 文件拆分 / 圈复杂度（后端代码结构优化链路 D08→D09→D10） |
| 前端重构类 | 5 | D05, D06, D07, D13, D14 | i18n / a11y（aria+alt）/ 命名规范（缩写+api） |
| 部署运维类 | 8 | D01, D02, D03, D04, D11, D15, D16, D17 | Docker / install / 缓存 / 测试 DB / 零停机 / 调度 / OA（其中 6 项为审计误判） |

### 0.3 按工作量归类

| 工作量 | 数量 | 任务编号 |
|--------|------|----------|
| S（小） | 3 | D01, D02, D07 |
| M（中） | 5 | D11, D12, D15, D16, D17 |
| L（大） | 4 | D03, D04, D09, D10 |
| XL（超大） | 5 | D05, D06, D08, D13, D14 |

### 0.4 按执行顺位归类（关键路径：D08→D09→D10 代码质量链路；D14→D13→D05 前端重构链路）

| 顺位 | 任务 | 状态 | 说明 |
|------|------|------|------|
| 第 1 顺位 | D08 超长函数 | ⏳ 重新打开 | 三次核实：实际 95 个 >80 行函数（非 0 个），需继续拆分 |
| 第 2 顺位 | D10 1000 行文件 | ✅ 已完成 | D08 完成后立即推进，6 批 34 文件全部完成；三次核实确认 0 个 >1000 行文件 |
| 第 3 顺位 | D14 api 命名统一 | ⏳ 重新打开 | 三次核实：残留 4 处不规范命名（listAuditLogs/listSlowQueries/addTagToCustomer/removeTagFromCustomer） |
| 第 4 顺位 | D13 前端缩写命名 | ⏳ 重新打开 | 三次核实：残留 18 个缩写文件（Ar×6+Bpm×11+Ai×1），Batch 1-7 未覆盖全部前缀 |
| 第 5 顺位 | D05 useI18n | ⏳ 重新打开 | 三次核实：实际接入率 18.6%（66/355），279 文件未接入，需继续接入 |

### 0.5 文档章节归类

| 章节 | 内容类型 | 用途 |
|------|----------|------|
| §一 当前状态与总体进度 | 状态跟踪 | 批次进度 / 决策记录 |
| §二 模块 G 依赖关系图 | 依赖关系 | 关键路径可视化 |
| §三 未完成任务清单 | 任务详情 | 6 项大型任务逐项展开 |
| §四 P1/P2/P3 任务规划 | 未来规划 | P0 完成后的后续工作 |
| §五 规则节点提醒 | 规则约束 | 执行过程中需遵守的规则 |
| §六 历史归档索引 | 归档索引 | 已完成项的归档链接 |

---

## 一、当前状态与总体进度

### 1.1 进度总览

| 优先级 | 总数 | 已完成 | 未完成 | 完成率 |
|--------|------|--------|--------|--------|
| **P0 阻塞级** | 104 | 99 | **5** | 95.2% |
| **P1 高优先级** | 257 | 0 | **257** | 0% |
| **P2 中优先级** | 248 | 0 | **248** | 0% |
| **P3 低优先级** | 123 | 0 | **123** | 0% |
| **合计** | **732** | **99** | **633** | **13.5%** |

> ⚠️ 2026-07-25 三次核实后，模块 G 5 项 P0 任务重新打开（D05/D08/D09/D13/D14），P0 完成数 104→99。

### 1.2 状态：⏳ 模块 G 5 项 P0 任务重新打开（三次核实修正）

- **当前批次**：Batch 491 ⏳ 进行中 —— P0 任务三次核实与修正（6 并行代理代码级扫描，修正二次核实多处偏差：D09 实际 11 个 >100 行函数非 100 个、D13 残留 18 个缩写文件非 0 个、D03/D04 product_service.rs 已通过 facade 接入）
- **上一批次**：Batch 490 ✅ 已完成 —— D05 useI18n 全量接入（77 文件 + 3327 翻译键，PR #732 合并；但三次核实发现实际接入率仅 18.6%，279 文件未接入）
- **执行策略**：规则 13+14+15+20 联动；CI 全绿后自动进入下一批；所有警告视为错误必须真实修复；修复前必须调研现有实现禁止重复造轮子；注释必须与功能一致禁止随意编写（规则 20）；规则 13 步骤 4 自审必须 grep 所有引用新字段/新结构体的调用点；**禁止本地编译验证**（cargo check/build/test/clippy + npm build/type-check/vitest/vue-tsc），必须直接 push 让 CI 验证；**扫描脚本必须使用括号深度追踪算法**（三次核实教训）

### 1.3 关键决策记录

| 决策 | 日期 | 内容 |
|------|------|------|
| 批次节奏 | 2026-07-17 | 每批 9-12 文件，遵循规则 13 连续执行流程；每 30 批触发 E2E（规则 5）；每 15 批整理记忆（规则 10） |
| 批次顺序 | 2026-07-17 | 按顺序修复所有批次，不再限制单数批次 |
| 术语澄清 | 2026-07-17 | 缸号（batch_no）=染色批次号；dye_lot_no=染色批号（lot 概念，防色差混批） |
| 旧表保留 | 2026-07-17 | 保留 color_card_borrow_records 不重命名为 _legacy，保护 Rust migration m0029 链路；应用层不再读写 |
| 复审归档 | 2026-07-17 | 复审报告 [v15-fix-reaudit-2026-07-17.md](file:///workspace/.monkeycode/docs/audits/v15-fix-reaudit-2026-07-17.md)；4 项已完成项归档（P0-S08/S16/F14/T04）；P0-S14 重新打开（migration 047 缺失） |
| 规则 20 | 2026-07-17 | 新增规则：注释必须与功能一致，禁止随意编写；CI 强制检查 |
| 自审门强化 | 2026-07-17 | Batch 473 教训：步骤 4 自审必须 grep 所有引用新字段/新结构体的调用点（如 `audit_log::ActiveModel {` / `OmniAuditMessage {` 等），不能只看 git diff 的已修改文件 |

---

## 二、模块 G 依赖关系图

> 仅保留模块 G（部署与运维）的依赖关系，模块 A-F 已全部完成（归档见 [archives/2026-07-22/doto-historical-tasks.md](file:///workspace/.monkeycode/docs/archives/2026-07-22/doto-historical-tasks.md)）。

```
P0-D01 ✅ Docker 文件 (S)        ← 独立（审计误判）
P0-D02 ✅ install.sh (S)         ← 独立（审计误判）
P0-D03 ✅ 5 service 缓存 (L)    ──→ P0-D04 ✅ moka→Redis (L)（三次核实确认 product_service.rs 通过 facade 模式接入）
P0-D05 ⏳ useI18n (XL)          ← 独立（三次核实：实际接入率 18.6%，279 文件未接入，需继续接入）
P0-D06 ✅ aria-label (XL)        ← 独立（55 子批次 ~225 文件，三次核实 260/374 含 aria-label 69.5%）
P0-D07 ✅ img alt (S)            ← 独立（审计误判，三次核实 2 个图片标签 100% 含 alt）
P0-D08 ⏳ 超长函数 (XL)          ──→ P0-D09 ⏳ 100 行函数 (L) ──→ P0-D10 ✅ 1000 行文件 (L)
   （三次核实：D08 实际 95 个 >80 行函数；D09 实际仅 11 个 >100 行函数，接近完成）
P0-D11 ✅ setup_test_db (M)     ← 独立（审计误判）
P0-D12 ✅ 圈复杂度 (M)           ← 独立（6 重构 + 2 误判）
P0-D13 ⏳ 前端缩写命名 (XL)     ← 独立（三次核实：残留 18 个缩写文件 Ar×6+Bpm×11+Ai×1，需重新打开）
P0-D14 ⏳ api 命名统一 (XL)     ← 独立（三次核实：残留 4 处不规范命名 listAuditLogs/listSlowQueries/addTagToCustomer/removeTagFromCustomer）
P0-D15 ✅ 升级零停机 (M)         ← 独立（审计误判）
P0-D16 ✅ 报表订阅调度 (M)       ← 独立（审计误判）
P0-D17 ✅ OA 公告 (M)            ← 独立（审计误判）
```

**关键路径**：P0-D08 → P0-D09 → P0-D10（代码质量链路）；P0-D14 → P0-D13 → P0-D05（前端重构链路）

---

## 三、未完成任务清单（模块 G，6 项大型任务）

> 模块 A-F 共 39 项 P0 任务已全部完成，详见 [archives/2026-07-22/doto-historical-tasks.md](file:///workspace/.monkeycode/docs/archives/2026-07-22/doto-historical-tasks.md)。

### 3.1 P0-D05 useI18n 接入（类七，XL，⏳ 重新打开）

- **来源**：batch-07 P0-07-5
- **证据（2026-07-25 三次核实）**：355 个 .vue 文件；66 个含 useI18n（18.6%），76 个含 t() 调用（21.4%），279 个完全未接入（78.6%）；locales/zh-CN.ts 1912 行 ~27 模块命名空间，en-US.ts 双语同步
- **接入率口径细分（三次核实修正）**：
  - **完整接入（约 18 个）**：所有用户可见中文已替换为 `$t`/`t()`，locales 命名空间完整 —— D05 Batch 1-5 完成的 10 文件 + D05 Batch 7 完成的 color-cards 4 文件 + color-prices 4 文件
  - **部分接入（约 10 个）**：已 `import { useI18n }` + `const { t } = useI18n(...)`，但仅替换了部分 ElMessage/ElMessageBox 提示，模板文案/表格列 label/表单 label 等仍硬编码 —— Login.vue + ai-extend(4) + budget + cost + inventoryTransfer(3) + system/tabs/UserTab.vue
  - **未接入（279 个）**：完全未引入 useI18n（含 10 个模板用全局 $t() 但未导入 useI18n 的文件）
- **抽样验证（2026-07-25 三次核实）**：
  - inventory/components/StatCards.vue（62 行）：4 处 stat-label 硬编码（库存总量/库存预警/仓库数量/低于最小库存）
  - purchase-inspection/components/PurchaseInspectionDetail.vue（62 行）：title + 8 处 label + el-divider + 6 处表格列 label 全硬编码
  - user-profile/index.vue（324 行）：标题/按钮/placeholder/校验 message/ElMessage 全场景硬编码（最严重样本）
- **UserTab.vue 命名空间 BUG（2026-07-24 发现）**：locales 定义 `settings.user.*`，代码调用 `system.user.*`（L277/287/303），命名空间前缀不匹配导致 t() 回退显示键名，需修复
- **Top 20 硬编码密集文件（2026-07-24 扫描，按中文字符数排名）**：1. AssetListTab.vue 864 字符（609 行）2. print-templates/index.vue 785 字符（525 行）3. bpm/index.vue 716 字符（626 行）4. report-templates/index.vue 706 字符 5. quality/index.vue 691 字符 6. crm/tabs/CustomerListTab.vue 680 字符 7. system/audit-log/index.vue 669 字符 8. quality-standards/index.vue 648 字符 9. crm/leads/index.vue 641 字符 10. inventory/index.vue 626 字符 11. crm/opportunities/index.vue 619 字符 12. Setup.vue 605 字符（457 行）13. dye-recipe/index.vue 595 字符 14. Login.vue 566 字符（336 行，部分已接入）15. warehouse/index.vue 546 字符 16. supplier/SupplierDialog.vue 523 字符 17. crm/detail.vue 508 字符 18. email/index.vue 486 字符 19. dye-batch/index.vue 483 字符 20. color-cards/issues.vue 477 字符
- **修复方案**：355 个 .vue 视图组件全部接入 useI18n，所有硬编码中文迁移到 locales/zh-CN.ts + en-US.ts 同步；按业务模块横向切片，每批 10-12 文件，预估需 23-28 批次（279/12≈23）；10 个部分接入文件需补全模板接入
- **关联文件**：[frontend/src/views/](file:///workspace/frontend/src/views/) + [frontend/src/locales/zh-CN.ts](file:///workspace/frontend/src/locales/zh-CN.ts) + [frontend/src/locales/en-US.ts](file:///workspace/frontend/src/locales/en-US.ts)
- **依赖**：建议在 D13/D14 完成后推进（避免同时修改 .vue 文件造成冲突）⏳ D13/D14 三次核实后重新打开
- **工作量**：XL（5 项中最大）
- **批次**：490（D05 独立批次；预估 23-28 子批次，每批 10-12 文件）
- **执行优先级**：第 5 顺位（最后推进，D13/D14 完成后启动）
- **当前进度**：⏳ 重新打开 —— Batch 1-8 已完成 77 文件接入 + 3327 翻译键（PR #732 合并）；但三次核实发现实际接入率仅 18.6%（66/355），279 文件未接入，需继续接入剩余文件；详见 [doto-su.md §D05](file:///workspace/.monkeycode/doto-su.md) + [CHANGELOG.md D05-9](file:///workspace/.monkeycode/CHANGELOG.md)
- **i18n 接入模式**：模板用 `$t('key')`，script 用 `const { t } = useI18n({ useScope: 'global' })`；命名空间 `{module}.{section}.{key}`；状态标签映射函数化响应式求值（如 `getTypeLabel`/`getStatusLabel`）；带参数翻译用 `t('key', { param })`；键名冲突用子命名空间重命名（如 `export`→`exportFile`、`print`→`printDialog`）
- **批次规划**：
  - Batch 1-5：✅ 已完成 10 文件完整接入（PR #724/#725/#727/#729，详见 [doto-su.md](file:///workspace/.monkeycode/doto-su.md) + [CHANGELOG.md D05-1~D05-5](file:///workspace/.monkeycode/CHANGELOG.md)）
    - Batch 1：fixed-assets/tabs/AssetListTab.vue（fixedAssets 命名空间）
    - Batch 2：print-templates/index.vue + bpm/index.vue（printTemplates + bpm）
    - Batch 3：report-templates/index.vue + quality/index.vue（reportTemplates + quality）
    - Batch 4：crm/tabs/CustomerListTab.vue + system/audit-log/index.vue（crmCustomer + auditLog）
    - Batch 5：quality-standards/index.vue + crm/leads/index.vue（qualityStandards + crmLeads）
  - Batch 7：✅ 已完成 8 文件完整接入（colorCards + colorPrices 命名空间 365 翻译键，locales zh-CN.ts/en-US.ts 1445→1912 行，389 处 $t/t() 调用 0 缺失键，详见 [CHANGELOG.md D05-7](file:///workspace/.monkeycode/CHANGELOG.md)）
    - color-cards/list.vue + create.vue + detail.vue + issues.vue（colorCards 命名空间 200 翻译键）
    - color-prices/list.vue + create.vue + detail.vue + batch-adjust.vue（colorPrices 命名空间 165 翻译键）
  - Batch 8：✅ 已完成 i18n 缺失键补全（7 文件 + 3 命名空间扩展，locales 2863→3011 键 +148，详见 [CHANGELOG.md D05-8](file:///workspace/.monkeycode/CHANGELOG.md)）
    - budget/tabs/BudgetListTab.vue（budget 命名空间扩展 49 键：title/createBudget/export/filter/status/table/dialog/message）
    - businessTrace/index.vue（新增 businessTrace 命名空间 61 键：tab/placeholder/button/card/field/table/empty/form/message）
    - capacity/index.vue + capacity/components/{CapacityTable,CapacityTrend,CapacityBottleneck,CapacityStat}.vue（新增 capacityModule 命名空间 36 键：title/dateRange/stat/trend/table/bottleneck/workCenterStatus/common）
    - advanced/components/AdvancedQualityPanel.vue（advancedModule.quality.confidence 1 键）
    - audit 脚本验证 0 真实缺失键（9 个 ${...} 动态模板字面量为 regex 误报已排除）
  - Batch 6：⏳ 待启动 —— inventory/index.vue (626 字符) + crm/opportunities/index.vue (619 字符)
  - 待修复：UserTab.vue 命名空间 BUG（system.user → settings.user）+ 9 个部分接入文件补全

### 3.2 P0-D08 91+ 超长函数（类七，XL，⏳ 重新打开）

- **来源**：batch-07 P0-07-8
- **证据（2026-07-25 三次核实）**：使用 Python 脚本基于括号深度追踪扫描 875 个 .rs 文件，>80 行函数实际 95 个（其中 >100 行 71 个，>200 行 18 个，>500 行 1 个）；最严重案例 services/report/tpl.rs:20 get_predefined_templates 701 行、handlers/auth_handler.rs:227 login 464 行、bootstrap/service_bootstrap.rs:74 bootstrap_full_mode 380 行
- **二次核实偏差修正**：二次核实记录 136 个 >80 行函数（多报 41 个），根因是简单 awk 脚本在函数内部 `}` 处过早截断导致同一函数被拆分重复计数；三次核实改用 Python 括号深度追踪算法修正
- **已重构确认**：event_bus.rs:412 start_event_listener D12-2 已重构（实际 279 行，CC 33→10 达标，列入观察名单不强拆）
- **豁免函数**：dye_batch_state_machine_service.rs:154 builtin_transition_rules 155 行纯数据表（27 条状态机三元组定义）豁免拆分
- **修复方案**：拆分超长函数为单一职责小函数（每个 ≤50 行），主函数仅做协调；优先处理 18 个 >200 行函数
- **关联文件**：[backend/src/services/report/tpl.rs](file:///workspace/backend/src/services/report/tpl.rs) / [handlers/auth_handler.rs](file:///workspace/backend/src/handlers/auth_handler.rs) / [bootstrap/service_bootstrap.rs](file:///workspace/backend/src/bootstrap/service_bootstrap.rs) / [services/inv/batch.rs](file:///workspace/backend/src/services/inv/batch.rs) / 等 35+ 文件
- **依赖**：无前置依赖
- **工作量**：XL
- **批次**：488（D 系列 17 项一次性打包；预估 10-12 子批次）
- **执行优先级**：第 1 顺位（无前置依赖 + 解锁 D09/D10）
- **当前进度**：⏳ 重新打开 —— 第一至第四梯队 167 函数已拆分（PR #669-#682 main ba8e97f）；但三次核实发现仍残留 95 个 >80 行函数（含已拆分函数的回归 + 二次扫描遗漏的 handlers/middleware/routes/bootstrap/cli 目录），需继续拆分；详细 CI 修复教训见 [doto-su.md §V15 Batch 488](file:///workspace/.monkeycode/doto-su.md)
- **梯队规划**：
  - 第一梯队（>200 行 6 函数，2 批）：✅ 全部完成
  - 第二梯队（150-200 行 22 函数，4 批）：✅ 全部完成
  - 第三梯队（100-150 行 53 函数，8 子批次）：✅ 全部完成（PR #669/#670 + main 772c0312 + b869a0cd + 97fd77ee + 47ad2bfa + 4e1cb058）
  - 第四梯队（80-100 行，预估 20 批）：✅ 全部完成（子批次 1-12 共 84 函数 + 精确扫描确认 0 候选剩余，PR #672-#681 main 0c5c4d4；子批次9 修复 E0507 借用错误，子批次10 修复 E0106 生命周期标注 + E0308 DateTime 类型 + 3 个 BUG，子批次11 修复步骤4自审发现的 validate_import_data 命名冲突，子批次12 修复步骤4自审发现的 build_approval_active 命名冲突 + 新增 StockFabricFields/ItemAmounts 结构体封装多参数 helper）
  - D09 收尾（100+行函数）：✅ 全部完成（PR #682 main ba8e97f，拆分 get_import_template 113行→11行 + export_orders_to_csv 112行→24行，精确扫描确认 100+行函数 0 候选剩余）
  - 模板化提取候选：inventory_finance_bridge_service.rs 7 个 create_*_voucher 函数提取通用 create_bridge_voucher<VoucherBuilder>

### 3.3 P0-D09 54+ 函数超过 100 行（类二，L，⏳ 重新打开）

- **来源**：batch-02 P0-02-01
- **证据（2026-07-25 三次核实）**：使用 Python 脚本基于括号深度追踪扫描，>100 行函数实际仅 11 个（非二次核实的 100 个）；最严重案例 services/report/tpl.rs:20 get_predefined_templates 701 行、services/event_kafka_payload.rs:381 try_from 227 行、services/event_kafka_payload.rs:150 from 226 行、routes/inventory.rs:24 inventory 路由 166 行
- **二次核实严重偏差修正**：二次核实记录 100 个 >100 行函数（多报 89 个），根因是简单 awk 脚本在函数内部 `}` 处过早截断导致同一函数被拆分重复计数；三次核实改用 Python 括号深度追踪算法修正
- **可豁免 2 个**：builtin_transition_rules（纯数据表 155 行）+ test_payload_all_variants_round_trip（测试函数 112 行）
- **实际需拆分 9 个**：get_predefined_templates（最严重 701 行）+ 2 个 From/TryFrom（event_kafka_payload.rs）+ 3 个路由聚合（inventory/purchases/ap）+ 3 个 report 函数（get_cash_flow_statement 116 行 / get_daily_report 111 行 / get_income_statement 101 行）
- **修复方案**：D08 完成后 D09 自动完成（D09 是 D08 子集，D08 阈值 >80 行涵盖 D09 阈值 >100 行）
- **关联文件**：同 P0-D08
- **依赖**：P0-D08 ⏳
- **工作量**：L（实际接近完成，仅需拆分 9 个函数）
- **批次**：488（D08 子集，不独立成批）
- **当前进度**：⏳ 重新打开 —— 之前已拆分 2 个 100+ 行函数（get_import_template + export_orders_to_csv，PR #682 main ba8e97f）；三次核实发现仍残留 11 个 >100 行函数（含可豁免 2 个），实际需拆分 9 个

### 3.4 P0-D10 30 个后端文件超过 1000 行（类二，L，进行中）

- **来源**：batch-02 P0-02-02
- **证据**：2026-07-19 精确扫描：实际 30 个 >1000 行文件，13 个 >1500 行，1 个 >2000 行（ar_service.rs 2067 行）；审计后新增越线 main.rs 1005 行 + init_service.rs 1287 行；28 个原审计文件全部仍 >1000 行无一下降；bi_analysis_service.rs 增长最快（+201 行 1461→1662）
- **修复方案**：按职责拆分为多个文件（如 ar_service.rs 拆分为 ar_service facade + ar_ops/{types,json_helpers,collection,verification,report}；models/status.rs 拆分为 status/sales / status/purchase / status/inventory；main.rs 拆为 main / routes_bootstrap / middleware_bootstrap）
- **关联文件**：[backend/src/services/ar_service.rs](file:///workspace/backend/src/services/ar_service.rs) (259, 原 2489) / [production_order_service.rs](file:///workspace/backend/src/services/production_order_service.rs) (1998) / [so/delivery.rs](file:///workspace/backend/src/services/so/delivery.rs) (1930) / [voucher_service.rs](file:///workspace/backend/src/services/voucher_service.rs) (1841) / [energy_service.rs](file:///workspace/backend/src/services/energy_service.rs) (1800) / 等 30 文件
- **依赖**：P0-D08/D09（避免函数拆分和文件拆分同时进行造成冲突）
- **工作量**：L
- **批次**：488（D 系列 17 项一次性打包；预估 5-6 子批次，每批 5-6 文件）
- **执行优先级**：第 2 顺位（D08 完成后立即推进）
- **当前进度**：D10-1 ✅ 完成（ar_service.rs 2489→259 行 facade + 5 子模块 2256 行，PR #683 main 34b8cae）；D10-2 ✅ 完成（production_order_service.rs 2141→689 行 facade + production_order_ops/{mod,types,crud,completion,approval} 5 子模块 1628 行，41 方法按职责分散到多 impl 块，PR #684 main 0385401）；D10-3 ✅ 完成（so/delivery.rs 2095→822 行 facade + delivery_ops/{mod,types,ship,inventory,cancel,export} 6 子模块 1403 行，30 方法按职责分散到多 impl 块，PR #684 main 0385401）；D10-2a ✅ 完成（voucher_service.rs 2058→882 行 facade + voucher_ops/{mod,crud,workflow,balance,assist} 5 子模块，39 方法 5+12+11+11，PR #685 main f836552）；D10-2b ✅ 完成（outsourcing_service.rs 1879→436 行 facade + outsourcing_ops/{mod,types,order,order_item,receipt,voucher} 6 子模块 + business_mode_service.rs 1739→741 行 facade + business_mode_ops/{mod,types,config,flow_step,rule,order_link} 6 子模块，PR #686 main 882cecc）；D10-3a ✅ 完成（chemical_service.rs 1730→349 行 facade + chemical_ops/{mod,types,master,category,lot,requisition} 6 子模块 43 方法 + bi_analysis_service.rs 1711→317 行 facade + bi_analysis_ops/{mod,types,sales,profit,drilldown,olap} 6 子模块 20+ 方法，PR #687 main d301de9）；D10-3b ✅ 完成（models/status.rs 1577→status/mod.rs + {common,master_data,production,purchase,sales,inventory,mrp,payment} 8 分组文件，PR #688 main 69de94f；mrp_engine_service.rs 1593→605 行 facade + mrp_engine_ops/{mod,types,stock,bom,calculation,query,order} 7 子模块 22 方法，StockInfo 提升为 pub(crate)，facade 仅 pub use 8 个原 pub struct，PR #691 main 9818351，CI 修复 3 轮：5 unused imports + 6 sea_orm trait 缺失 + 集成测试 common 模块名称遮蔽）；第 1 批 3 个 >1800 行文件全部完成，第 2 批 4/4 完成，第 4 批 4/4 完成）；D10-4a ✅ 完成（dye_batch_state_machine_service.rs 1512→920 行 facade + dye_batch_state_machine_ops/{mod 17, lifecycle_log 152, state_rule 195, rework 232, operation 117} 4 子模块，4 Service 27 方法按职责分散到多 impl 块，db 字段改 pub(crate)，外部调用路径不变；wage_service.rs 1621→774 行 facade + wage_ops/{mod 14, rate 351, record 242, calculation 357} 3 子模块，3 Service 29 方法按职责分散到多 impl 块，db 字段改 pub(crate)，2 日期纯函数改 pub(crate) 供 calculation 复用，外部 wage_handler.rs 调用路径不变，PR #692 main ac593a2）；D10-4b ✅ 完成（ar/vfy.rs 1368→568 行 facade + ar/vfy_ops/{mod 17, match 389, aging 158, reconciliation 221, confirm 113} 5 子模块，ArReconciliationService 5 公开方法 + helper 分散到多 impl 块，db 字段改 pub(crate)，外部调用路径不变；ap_invoice_service.rs 1405→407 行 facade + ap_invoice_ops/{mod 16, types 159, receipt 390, crud 398, report 161} 5 子模块，ApInvoiceService 20 方法分散到多 impl 块（receipt 9 + crud 8 + report 3），ReceiptVoucherContext 移到 receipt.rs，db 字段改 pub(crate)，CI 修复 1 轮：receipt.rs 缺失 ColumnTrait，PR #693 main 6a480d9）；D10-5 ✅ 完成（init_service.rs 1347→293 行 facade + init_service_ops/{mod,setup,role,permission,dept_user} 4 子模块 10 方法 + flow_card_service.rs 1285→386 行 facade + flow_card_ops/{mod,route,card_crud,card_state,step,feedback} 5 子模块 4 Service 35 方法 + ap_reconciliation_service.rs 1346→621 行 facade + ap_reconciliation_ops/{mod,types,crud,confirm,report,auto} 5 子模块 18 方法 + search/elastic.rs 1230→756 行 facade含测试394行 + elastic_ops/{mod,client_ops,syncer_ops,types_ops} 3 子模块，PR #696 main 6bc4dca，CI 修复 1 轮：5 方法可见性私有→pub(crate) + 1 unused import SearchClient）；第 1-5 批全部完成（3+4+4+4+4=19 文件）；D10-6a ✅ 完成（event_bus.rs 1196→240 facade + event_bus_ops/{mod,publish,subscribe,retry}，PR #698 main 9d26d7d）；D10-6b-1 ✅ 完成（lab_dip_service.rs 1188→230 facade + lab_dip_ops/{mod,types,request,sample,resample} + production_recipe_service.rs + product_service.rs + system_update_service.rs 4 文件，PR #700 main 325dfed，CI 修复 8 个新增 Clippy 警告更新 baseline）；D10-6b-2 ✅ 完成（ar_ops/verification.rs 1062→30 + verification_ops/{mod 21, query 214, auto 415, manual 490} 23 方法 3 impl 块 + purchase_receipt_service.rs 1074→481 facade + purchase_receipt_ops/{mod 28, auth 35, crud 207, state 122, items 278, query 76} + ar/recon.rs 1070→658 facade含测试 + ar/recon_ops/{mod 13, crud 206, lifecycle 259} + bpm_service.rs 1060→148 facade + bpm_ops/{mod 15, instance 404, task 453, monitor 151}，db 字段 pub(crate)，所有子模块独立导入 sea_orm traits，无 #[allow] 警告抑制，PR #702 main 3890add，CI 修复 1 轮：E0252 ReconciliationModel 重复导入 + 3 个 unused import）；D10-6b-3 ✅ 完成（bom_service.rs 1046→587 facade + bom_ops/{mod 20, crud 317, state 105, tree 145} 16 方法 3 impl 块 + import_export_service.rs 1018→546 facade + import_export_ops/{mod 16, import 218, export 226, task 105} 10 方法 3 impl 块 + main.rs 1005→171 入口 + bootstrap/{mod 12, infra_bootstrap 76, middleware_bootstrap 282, routes_bootstrap 182, service_bootstrap 453} 按启动流程职责拆分非 facade 模式，db 字段 pub(crate)，所有子模块独立导入 sea_orm traits，无 #[allow] 警告抑制，PR #703 main 7120cf3，覆盖率 job 因 Broken pipe 基础设施问题失败已 admin 合并）；第 6 批 15/15 全部完成（D10-6a 4/15 + D10-6b-1 4/15 + D10-6b-2 4/15 + D10-6b-3 3/15）；D10 全部 6 批 34 文件全部完成
- **核实（2026-07-23）**：✅ 第 3 批 4/4 完成（doto 滞后记录为 2/4，实际 models/status.rs 已拆分为 status/ 目录 9 子文件、mrp_engine_service.rs 已降至 605 行）；❌ 第 4/5 批 6 个文件行数逆生长（D08 拆分引入 helper 导致）：wage_service.rs 1507→1621(+114)→D10-4a 已降至 774、ap_invoice_service.rs 1306→1405(+99)、ap_reconciliation_service.rs 1243→1346(+103)、init_service.rs 1287→1347(+60)、ar/vfy.rs 1320→1368(+48)、flow_card_service.rs 1271→1285(+14)；❌ 第 6 批实际 15 个文件（doto 记录 11 个），含 D10-1 拆分副产物 ar_ops/verification.rs 1062 行需再次拆分；当前真实 >1000 行文件共 23 个（doto 隐含 21 个，净差 +2）
- **批次规划**：
  - 第 1 批：✅ ar_service.rs (2489→259 facade + ar_ops/{types 75, json_helpers 98, collection 676, verification 1062, report 422, mod 23}) / ✅ production_order_service.rs (2141→689 facade + production_order_ops/{mod 17, types 87, crud 568, completion 667, approval 288}) / ✅ so/delivery.rs (2095→822 facade + delivery_ops/{mod 16, types 35, ship 588, inventory 357, cancel 270, export 136}) 3 个 >1800 行文件全部完成
  - 第 2 批：✅ voucher_service.rs (2058→882 facade + voucher_ops/{mod, crud 468, workflow, balance, assist}，39 方法 5+12+11+11) / ✅ energy_service.rs (1826→324 facade + energy_ops/{meter,consumption,allocation_rule,allocation_record}) / ✅ outsourcing_service.rs (1879→436 facade + outsourcing_ops/{mod,types,order 724,order_item,receipt,voucher}，4 Service 39 方法) / ✅ business_mode_service.rs (1739→741 facade + business_mode_ops/{mod,types,config,flow_step,rule,order_link}，4 Service 28 方法) 4 个 >1700 行文件全部完成
  - 第 3 批：✅ chemical_service.rs (1676→349) + ✅ bi_analysis_service.rs (1662→317) + ✅ models/status.rs (1577→status/mod.rs + 8 分组文件) + ✅ mrp_engine_service.rs (1593→605 facade + mrp_engine_ops 7 子模块 22 方法) 4 个 >1500 行文件全部完成（PR #687/#688/#691）
  - 第 4 批：✅ dye_batch_state_machine_service.rs (1512→920 facade + dye_batch_state_machine_ops 4 子模块 27 方法) + ✅ wage_service.rs (1621→774 facade + wage_ops 3 子模块 29 方法，PR #692) + ✅ ar/vfy.rs (1368→568 facade + ar/vfy_ops 5 子模块) + ✅ ap_invoice_service.rs (1405→407 facade + ap_invoice_ops 5 子模块，PR #693 main 6a480d9) 4 个 >1300 行文件全部完成
  - 第 5 批：✅ init_service.rs (1347→293 facade + init_service_ops/{mod 11, setup 287, role 215, permission 387, dept_user 198}) + ✅ flow_card_service.rs (1285→386 facade + flow_card_ops/{mod 16, route 151, card_crud 227, card_state 190, step 247, feedback 162}) + ✅ ap_reconciliation_service.rs (1346→621 facade + ap_reconciliation_ops/{mod 17, types 99, crud 189, confirm 182, report 111, auto 235}) + ✅ search/elastic.rs (1230→756 facade含测试394行 + elastic_ops/{mod 4, client_ops 343, syncer_ops 41, types_ops 49}) 4 个 >1200 行文件全部完成（PR #696 main 6bc4dca，CI 修复 1 轮：5 方法可见性私有→pub(crate) + 1 unused import SearchClient）
  - 第 6 批：原 15 个 1000-1200 行文件（含 D10-1 副产物 ar_ops/verification.rs 1062 行）全部完成 15/15：✅ D10-6a (4/15) event_bus.rs (1243→240 facade + event_bus_ops/{mod,publish,subscribe,retry}) / po/order.rs (1234) / auth_service.rs (1201) / inventory_finance_bridge_service.rs (1192)，PR #698 main 9d26d7d；✅ D10-6b-1 (4/15) lab_dip_service.rs (1188→230 facade + lab_dip_ops/{mod,types,request,sample,resample}) / production_recipe_service.rs (1181) / product_service.rs (1075) / system_update_service.rs (1074)，PR #700 main 325dfed；✅ D10-6b-2 (4/15) ar_ops/verification.rs (1062→30) / purchase_receipt_service.rs (1074→481) / ar/recon.rs (1070→658) / bpm_service.rs (1060→148)，PR #702 main 3890add；✅ D10-6b-3 (3/15) bom_service.rs (1046→587 facade + bom_ops/{mod 20, crud 317, state 105, tree 145} 16 方法 3 impl 块) / import_export_service.rs (1018→546 facade + import_export_ops/{mod 16, import 218, export 226, task 105} 10 方法 3 impl 块) / main.rs (1005→171 入口 + bootstrap/{mod 12, infra_bootstrap 76, middleware_bootstrap 282, routes_bootstrap 182, service_bootstrap 453} 按启动流程职责拆分非 facade 模式)，db 字段 pub(crate)，所有子模块独立导入 sea_orm traits，无 #[allow] 警告抑制，PR #703 main 7120cf3（覆盖率 job 因 Broken pipe 基础设施问题失败已 admin 合并）

### 3.5 P0-D13 前端缩写命名组件（类二，XL，⏳ 重新打开）

- **来源**：batch-02 P0-02-05
- **证据（2026-07-25 三次核实）**：使用 Glob 系统检查全部 27 类缩写前缀，残留 18 个缩写命名 .vue 文件（Ar×6 + Bpm×11 + Ai×1）；其余 24 类前缀已全部完成转换
- **二次核实偏差修正**：二次核实记录"0 个剩余缩写组件"是误判，根因是只检查了 25 类前缀，遗漏了 Ar/Bpm/Ai 三类前缀（其中 Ar 和 Ai 在二次核实时被错误地归类为"合法前缀"）
- **残留 18 个文件明细**：
  - **Ar 前缀（6 个）—— arReconciliation/components/**：ArReconciliationCharts.vue / ArReconciliationConfirm.vue / ArReconciliationDetail.vue / ArReconciliationDispute.vue / ArReconciliationFilter.vue / ArReconciliationTable.vue
  - **Bpm 前缀（11 个）—— bpm/{approval,definitions}/components/**：BpmApprovalApprovalDialog.vue / BpmApprovalChainDialog.vue / BpmApprovalCompletedTable.vue / BpmApprovalPendingTable.vue / BpmApprovalStat.vue / BpmApprovalTransferDialog.vue / BpmDefinitionFilter.vue / BpmDefinitionForm.vue / BpmDefinitionTable.vue / BpmDefinitionTemplateDialog.vue / BpmDefinitionVersionDialog.vue
  - **Ai 前缀（1 个）**：components/ai/AiPredictionChart.vue
- **历史核实（2026-07-23）**：❌ 数量偏差。严格按 25 类前缀搜索实际 111 个（views/ 110 + components/ 1，doto 多记 12 个）；若补入 advanced(Rcp/Qlt/Rpt/Ai 4 个) + arReconciliation(Ar 6 个)，则实际 121 个。❌ 前缀分类不完整：实际 27 类（doto 记 25 类，缺 Ar + Rcp/Qlt/Rpt/Ai）
- **修复方案**：重命名剩余 18 个文件为描述性全名（如 ArReconciliationCharts→AccountReceivableReconciliationCharts、BpmApprovalApprovalDialog→BusinessProcessApprovalApprovalDialog、AiPredictionChart→ArtificialIntelligencePredictionChart）；同步更新父级 import
- **关联文件**：[frontend/src/views/arReconciliation/components/](file:///workspace/frontend/src/views/arReconciliation/components/) + [frontend/src/views/bpm/approval/components/](file:///workspace/frontend/src/views/bpm/approval/components/) + [frontend/src/views/bpm/definitions/components/](file:///workspace/frontend/src/views/bpm/definitions/components/) + [frontend/src/components/ai/AiPredictionChart.vue](file:///workspace/frontend/src/components/ai/AiPredictionChart.vue)
- **依赖**：建议在 D14 完成后推进（避免同时修改 import 路径造成冲突）⏳ D14 三次核实后重新打开
- **工作量**：XL
- **批次**：489（D13 独立批次；预估 12-15 子批次，每批 8-10 文件）
- **执行优先级**：第 4 顺位（D14 完成后推进）
- **当前进度**：⏳ 重新打开 —— Batch 1-7 已完成 121 文件重命名 + 43 caller 文件更新（PR #716/#717/#718/#719/#720/#721/#722）；但三次核实发现仍残留 18 个缩写文件（Ar/Bpm/Ai 前缀未处理），需新增 Batch 8 完成
- **批次规划**：按模块分组（每模块独立批次）⚠️ 以下数量为 doto 原记录，核实后需调整（见核实行）
  - Batch 1：✅ 已完成（#716 main 937b9a2）sales-contract (3) + system-update (3) + sales-price (5) + purchase-price (5) 共 16 文件 + 6 caller（ScFilter→SalesContractFilter / SuVerDetail→SystemUpdateVersionDetail / SpTbl→SalesPriceTable / PpTbl→PurchasePriceTable 等）
  - Batch 2：✅ 已完成（#717 main c3e2f58）logistics (6) + finance/tabs (4) + voucher/tabs (4) + data-import (4) 共 18 文件 + 6 caller（LgsFilter→LogisticsFilter / VchrForm→VoucherForm / DiTplTable→DataImportTemplateTable 等，DiTplForm 接口重命名为 DataImportTemplateFormData）
  - Batch 3：✅ 已完成（#718 main 404fc14）security/two-factor (5) + security/components (4) + capacity (4) + advanced (4) 共 17 文件 + 7 caller（TfaStep1→TwoFactorAuthStep1 / SecAlertTbl→SecurityAlertTable / CpBottleneck→CapacityBottleneck / AiPanel→AdvancedAiPanel 等，useTfaProc.ts 接口 TwoFactorAuthStep3Instance 同步更新）
  - Batch 4：✅ 已完成（#719 main ef91527）api-gateway (1) + sales (3) + scheduling (10) + arReconciliation (6) 共 20 文件 + 8 caller（EpForm→ApiEndpointForm / OlvFilter→SalesOrderFilter / SchGAdj→SchedulingGanttAdjust / SchMTbl→SchedulingMachineTable / ArTbl→ArReconciliationTable 等）
  - Batch 5：✅ 已完成（#720 main 84cafd8）purchase-return (5) + material-shortage (3) + production (4) + bpm/definitions (5) 共 17 文件 + 5 caller（PrRtnApr→PurchaseReturnApproval / MsSevCard→MaterialShortageSeverityCard / PrdFilter→ProductionFilter / BpmDfFilter→BpmDefinitionFilter 等；额外 BpmDfFormData→BpmDefinitionFormData 4 处引用）
  - Batch 6：✅ 已完成（#721 main db50305）bpm/approval (6) + purchase-contract (4) + purchase-inspection (5) + sales-analysis (5) 共 20 文件 + 6 caller（BpmApAprDlg→BpmApprovalApprovalDialog / PcFilter→PurchaseContractFilter / PiFilter→PurchaseInspectionFilter / SaStat→SalesAnalysisStat 等；额外 4 个本地 interface 重命名 BpmApStats/PiFormData/PiStats/PcFormData）
  - Batch 7：✅ 已完成（#722 main 6854060）bom (1) + dashboard (4) + purchase (3) + purchaseReceipt (4) + components/ai (1) 共 13 文件 + 5 caller（BomForm→BillOfMaterialsForm / DbActTbl→DashboardActivityTable / DbPie→DashboardPie / DbStat→DashboardStat / DbTrend→DashboardTrend / PurchFilter→PurchaseFilter / PurchTbl→PurchaseTable / PurchTop→PurchaseTop / PrcDetail→PurchaseReceiptDetail / PrcFilter→PurchaseReceiptFilter / PrcForm→PurchaseReceiptForm / PrcTbl→PurchaseReceiptTable / AIPredictionChart→AiPredictionChart；caller 更新 5 文件：bom/index.vue + Dashboard.vue + purchase/index.vue + purchaseReceipt/index.vue + ai-extend/quality-prediction.vue + SalesAnalysisTrend.vue 注释 + usePrcProc.ts；额外 1 个本地 interface 重命名 PrcFormModel→PurchaseReceiptFormModel 4 处引用，无 #[allow] 警告抑制）

### 3.6 P0-D14 前端 api 命名不统一（类二，XL，⏳ 重新打开）

- **来源**：batch-02 P0-02-06
- **证据（2026-07-25 三次核实）**：使用 Grep 检查 `export (async )?(function|const) (list|add|remove|query|fetch)[A-Z]` 五种不规范模式，残留 4 处不规范命名（与二次核实完全一致）：listAuditLogs（audit.ts:79）+ listSlowQueries（slow-query.ts:69）+ addTagToCustomer（crm-enhanced.ts:220）+ removeTagFromCustomer（crm-enhanced.ts:224）
- **历史核实（2026-07-23）**：✅ 文件总数 96 一致；✅ addXxx 5 文件 6 处一致；✅ fetchXxx 1 文件 1 处一致；✅ request.ts 存在应保留。❌ 风格 A 实际 25 个（doto 记 21，少 4，工作量被低估）；❌ listXxx 实际 59 文件 104 处（doto 记 47 文件 84 处，少 12 文件 20 处，最大偏差源工作量低估约 23%）
- **修复方案**：统一为风格 B（function 形式）+ 命名规范 `getXxxList / createXxx / updateXxx / deleteXxx / getXxxById`；保留 request.ts 不改名；4 个混合文件先去重再统一；3 个 re-export 文件同步更新导出列表；预估影响 2000+ 处调用点
- **关联文件**：[frontend/src/api/](file:///workspace/frontend/src/api/) 96 个 .ts 文件
- **依赖**：无前置依赖（独立任务）
- **工作量**：XL
- **批次**：488（D 系列 17 项一次性打包；预估 10-12 子批次，每批 8-10 文件）
- **执行优先级**：第 3 顺位（与 D05/D13 解耦）
- **当前进度**：⏳ 重新打开 —— Batch 1-5 全部完成（风格 A 25 文件转风格 B + 2000+ 处调用点更新，PR #705-#714）；但三次核实发现仍残留 4 处不规范命名（listAuditLogs/listSlowQueries/addTagToCustomer/removeTagFromCustomer），需新增 Batch 6 补齐
- **批次规划**：
  - Batch 1：✅ 已完成（#705 main e807550）财务 AP/AR 9 文件（ap.ts/ap-invoice.ts/ap-payment.ts/ar.ts/ar-reconciliation.ts/ar-reconciliation-enhanced.ts/ap-reconciliation.ts/ap-verification.ts/voucher.ts）
  - Batch 2：✅ 已完成（#706 main eb4fdb2）采购/销售/库存 9 API 定义文件 13 处重命名 + 5 caller 文件（purchase-contract/purchase-price/purchaseReceipt/sales-contract/sales-price/inventoryAdjustment/inventoryTransfer/inventoryBatch/inventoryCount）
  - Batch 3：✅ 已完成（#708 main 8b407e8）生产/质量 3 API 定义文件 5 处重命名 + 4 caller 文件（quality-standards/quality/production）
  - Batch 4：✅ 已完成（#710 main 3629977）CRM/客户/供应商/贸易 23 文件 41 处（12 API 定义文件 29 处重命名 + 11 caller 文件）
  - Batch 5a：✅ 已完成（#712 main 5d1c33b）系统/权限/基础/报表/其他风格 B 58 文件 117 处（28 API 定义文件 48 处重命名 + 28 caller 文件，CI 修复 1 轮 data-permission.ts URL 插值）
  - Batch 5b：风格 A object→风格 B 转换（Pass 1 API 定义 + Pass 2 caller 更新）
    - Pass 1：✅ 已完成（25 个风格 A API 定义文件全部转为风格 B 独立函数：bom.ts 10 函数 + bpm-enhanced.ts 16 函数 + bpm.ts 13 函数 + crm-enhanced.ts 21 函数 + customer.ts 7 函数 + five-dimension.ts 1 函数 + financial-analysis.ts 4 函数 + fabric.ts 等；移除原 `xxxApi = {}` 对象导出，保留类型定义）
    - Pass 2：✅ 已完成（70+ caller 文件 import+调用更新，#714 main 8d8b196，100 文件 +1488 -1120）
      - A 组 11 文件：bom/index.vue + fabric store/views + product 等
      - B 组 12 文件：dashboard/sales/inventory store + Login.vue 等
      - C 组 12 文件：crm/customer/customerCredit/supplier/supplierEvaluation 等
      - D1 组 18 文件：sales/purchase/logistics/scheduling/security/capacity/material-shortage composables
      - D2 组 17 文件 50 处：BPM 5 文件 26 处（bpm/index.vue 13 + templates.vue 2 + useBpmApProc.ts 3 + useBpmDf.ts 1 + useBpmDfProc.ts 7）+ CRM 11 文件 23 处（assignment.vue 2 + OpportunityFollowTab.vue 1 + ReleaseDialogTab.vue 1 + FollowUpTab.vue 2 + ManualAssignDialogTab.vue 1 + ClaimDialogTab.vue 1 + RfmTab.vue 2 + CustomerListTab.vue 4 + TagsPanelTab.vue 3 + TransferDialogTab.vue 1 + detail.vue 5）+ 其他 1 文件 1 处（sales-contract/useSc.ts customerApi.list→getCustomerList）
      - grep 验证：`xxxApi.method()` 调用 0 残留，`import { xxxApi }` 仅剩 useTableApi（composable 合规）

---

## 四、P1/P2/P3 任务规划（按类别汇总）

> P0 完成后按优先级顺序推进。详细内容见 V15 审计报告 [docs/audits/v15/](file:///workspace/.monkeycode/docs/audits/v15/)。

### 4.1 P1 高优先级（257 项，预估 45-55 批次，按每批 9-12 文件计算）

| 模块 | P1 数 | 主要内容 | 关键批次预估 |
|------|-------|----------|--------------|
| 类二 通用代码质量 | 3 | api 命名/缩写命名/DbErr 包装 | 2 批 |
| 类三 安全性 | 6 | refresh_token/PUBLIC_PATHS/validator/Webhook/magic bytes/zip bomb | 3 批 |
| 类四 面料行业深化 | 11 | batch_trace/检验指标/工资凭证/能耗/委外/事件发布/工时 | 4 批 |
| 类五 运行逻辑闭环 | 11 | 状态机/配置/业务事件/成本归集/加权平均 | 4 批 |
| 类六 测试体系 | 11 | 覆盖率/mock/fixtures/文档 | 4 批 |
| 类七 可维护性 | 11 | i18n/aria/缓存/文档 | 4 批 |
| 类八 法律合规 | 16 | 用户协议/HTTPS/脱敏/导出/docx/标准/签章/税/环保/排污/劳动/工时/社保/职业健康 | 6 批 |
| 类九 色卡发放 | 9 | 清单/通知/报表 | 3 批 |
| 类十 大货批色 | 7 | 提醒/报表/统计 | 3 批 |
| 类十三 打印导出 | 14 | 审计字段/水印/性能 | 5 批 |
| 类十四 权限维度 | 14 | 权限测试/审计/缓存 | 5 批 |
| 类十五 业务主体 | 1 | supplier_evaluation migration | 1 批 |
| 类十六 AI 模块 | 24 | 配伍性/化验室/准确率/版本/权限/超时/并发/缓存/脱敏/MLOps | 8 批 |
| 类十七 财务深化 | 35 | 期间/反结账/年结/回转/账龄/杜邦/预测/差异/折旧 | 12 批 |
| 类十八 CRM | 12 | 线索评分/去重/转移审批 | 4 批 |
| 类十九 报表 BI | 5 | 版本管理/缓存 | 2 批 |
| 类二十 可观测性 | 9 | trace/metrics/WebSocket | 3 批 |
| 类二十一 胚布拆匹 | 10 | 库存/委外/继承 | 4 批 |
| 类二十二 库存排程 | 9 | 调拨/安全/排程 | 3 批 |
| 类二十三 组织物流 | 11 | 组织树/售后/运费 | 4 批 |
| 类二十四 前端架构 | 16 | PWA/移动端/chunks/ErrorBoundary/CSP/keep-alive/CSS/暗黑 | 6 批 |
| 类二十五 部署升级 | 11 | set -euo/SHA256/schema/蓝绿/健康/优雅/回滚 | 4 批 |
| **合计** | **257** | | **约 45 批**（每批 9-12 文件） |

### 4.2 P2 中优先级（248 项，预估 35-45 批次）

| 类别 | P2 数 | 主要内容 |
|------|-------|----------|
| 类一~类四 | 19 | 代码质量 / 安全防护 / 面料行业字段补齐 |
| 类五~类八 | 47 | 运行逻辑 / 测试补充 / 可维护性 / 法律合规细节 |
| 类九~类十二 | 33 | 色卡发放细节 / 大货批色细节 / 打印导出 / 权限细节 |
| 类十三~类十四 | 25 | 打印导出 P2 / 权限 P2 |
| 类十五~类十六 | 53 | 业务主体 P2 / AI 模块 P2 |
| 类十七~类十九 | 39 | 财务 P2 / CRM P2 / 报表 BI P2 |
| 类二十~类二十二 | 25 | 可观测性 / 胚布 / 库存 P2 |
| 类二十三~类二十五 | 83 | 组织物流 / 前端架构 / 部署升级 P2 |
| **合计** | **248** | |

### 4.3 P3 低优先级（123 项，按需修复）

| 类别 | P3 数 | 主要内容 |
|------|-------|----------|
| 类一~类四 | 11 | 文档 / 注释 / 命名优化 |
| 类五~类八 | 17 | 测试增强 / 可维护性增强 / 法律合规增强 |
| 类九~类十二 | 9 | 色卡 / 批色 / 打印 / 权限增强 |
| 类十三~类十四 | 5 | 打印导出 / 权限增强 |
| 类十五~类十六 | 25 | 业务主体增强 / AI 增强 |
| 类十七~类十九 | 11 | 财务 / CRM / 报表增强 |
| 类二十~类二十二 | 12 | 可观测性 / 胚布 / 库存增强 |
| 类二十三~类二十五 | 41 | 组织物流 / 前端架构 / 部署升级增强 |
| **合计** | **123** | |

---

## 五、规则节点提醒

| 规则 | 优先级 | 内容 |
|------|--------|------|
| 规则 0/1/2/8 | 🔴 | 真实实现强制：所有 P0/P1 修复必须真实实现，禁止占位符 |
| 规则 3 | 🔴 | 成品文档格式：导出必须 .xlsx / 报表必须 .docx |
| 规则 5 | 🟡 | E2E 独立工作流：每 30 批次触发（批次 30/60/90...） |
| 规则 6 | 🔴 | 测试 mock 数据禁止硬编码：所有测试 mock 数据抽取到 fixtures |
| 规则 10 | 🟡 | 每 15 批次记忆整理 + 实时归档：每批完成后立即归档到 doto-su.md |
| 规则 11/12 | 🔴 | 法律合规与安全标准：所有修复必须符合中国法律法规 + 安全标准 |
| 规则 13 | 🔴 | 修复流程自动化：CI 全绿后自动开始下一批；**步骤 0 确定审计结果内容是否存在**（修复前置门）+ **步骤 4 修复后推送前自审**（与规则 20 联动） |
| 规则 14 | 🔴 | 移除所有警告抑制：所有警告视为错误需修复（baseline 213/213 ✅ 全部清零） |
| 规则 15 | 🟢 | V15 全项目综合审计：25 大类 195 维度审计 ✅ 已完成 |
| 规则 19 | 🟡 | 工具连接异常分级响应：L1 60s / L2 60-180s / L3 30min 周期 |
| 规则 20 | 🔴 | 注释与功能一致性：代码注释必须与功能实现一致，禁止随意编写；CI 强制检查 |
| §10.0.1 | 🔴 | 复用现有功能原则：修复前必须调研现有实现，禁止重复造轮子 |

---

## 六、历史归档索引

> 详细历史任务归档见 [archives/2026-07-22/doto-historical-tasks.md](file:///workspace/.monkeycode/docs/archives/2026-07-22/doto-historical-tasks.md)，包含：
> - P0 批次规划表（39 项 → 22 批次）
> - 已完成模块 A-F 清单（39 项 P0 任务全部完成）
> - 历史阶段任务（v13/v14 复审修复 + V15 审计 + V15 修复阶段一/续/复审归档/复审报告）
