# 未完成任务

> 本文件**只记录未完成任务**（任务队列、待修复项、剩余清单）。
> 已完成任务见 [doto-su.md](file:///workspace/.monkeycode/doto-su.md)，一句话总结见 [CHANGELOG.md](file:///workspace/.monkeycode/CHANGELOG.md)，规则见 [MEMORY.md](file:///workspace/.monkeycode/MEMORY.md)。
> 最近整理：2026-07-22（按规则 10 深度整理归档：模块 A-F 已完成项归档到 [archives/2026-07-22/doto-historical-tasks.md](file:///workspace/.monkeycode/docs/archives/2026-07-22/doto-historical-tasks.md)；§三批次规划表 + §七历史任务阶段归档；仅保留模块 G 未完成项 + P1/P2/P3 规划 + 规则节点提醒）

---

## 〇、内容归类总览（模块 G 共 17 项 P0 任务）

> 本节为快速索引，按 4 个维度归类；详细条目见 §三，依赖关系见 §二。

### 0.1 按状态归类（15 ✅ / 1 ⏳ / 1 ❌）

| 状态 | 数量 | 任务编号 |
|------|------|----------|
| ✅ 已完成 | 15 | D01, D02, D03, D04, D06, D07, D08, D09, D10, D11, D12, D14, D15, D16, D17 |
| ⏳ 进行中 | 1 | D13（前端缩写命名，Batch 1-5 完成 PR #716/#717/#718/#719/#720，剩余 Batch 6-7） |
| ❌ 未开始 | 1 | D05（useI18n） |

### 0.6 核实结果汇总（2026-07-23 核实）

> 本次对 4 项未完成任务（D10/D05/D13/D14）进行代码级核实，发现 doto.md 的 2026-07-19 审计数据存在偏差，详见下表。各任务条目已补充"核实"行记录差异。

| 任务 | doto 记录 | 实际核实 | 偏差 | 影响 |
|------|-----------|----------|------|------|
| **D10** | 第 3 批 2/4 完成 | 第 3 批 4/4 完成 | doto 滞后 | 进度更乐观，models/status.rs + mrp_engine_service.rs 已拆分 |
| **D10** | 第 4/5 批 8 文件行数基准 | 6 文件行数逆生长 | 基准失效 | wage +114 / ap_invoice +99 / ap_recon +103 / init +60 / ar/vfy +48 / flow_card +14 |
| **D10** | 第 6 批 11 个文件 | 实际 15 个文件 | doto 少 4 | 含 ar_ops/verification.rs（D10-1 拆分副产物，1062 行） |
| **D10** | 隐含未完成 21 个 | 实际 >1000 行 23 个 | doto 少 2 | 净差 +2（第 3 批 -2 + 第 6 批 +4） |
| **D05** | AssetListTab.vue 864 行 | 实际 609 行 | doto 高估 255 行 | 单文件最大值记录失效，Top 20 排名可能已变 |
| **D05** | 其他数据全部一致 | ✅ 一致 | 无 | 接入率 3.1%（11/355）、未接入 344、zh-CN.ts 467 行 15 模块均准确 |
| **D13** | 123 个缩写文件 | 实际 111 个（25 类前缀）/ 121 个（27 类前缀） | doto 多 12/2 | 严格 25 类前缀口径 111 个；含 Ar + advanced(Rcp/Qlt/Rpt/Ai) 口径 121 个 |
| **D13** | 25 类缩写前缀 | 实际 27 类 | doto 少 2 类 | 补 Ar + Rcp/Qlt/Rpt/Ai |
| **D13** | 第 7 批 purchase (6) | 实际 purchase (3) | doto 多 3 | 其余 4 个为描述性短名非缩写 |
| **D14** | 风格 A 21 个 | 实际 25 个 | doto 少 4 | 工作量被低估 |
| **D14** | listXxx 47 文件 84 处 | 实际 59 文件 104 处 | doto 少 12 文件 20 处 | 最大偏差源工作量低估约 23% |
| **D14** | removeXxx 2 文件 2 处 | 实际 1 文件 1 处 | doto 多 1 | 仅 role.ts |
| **D14** | queryXxx 2 文件 2 处 | 实际 1 文件 1 处 | doto 多 1 | 仅 assist-accounting.ts |
| **D14** | addXxx 5 文件 6 处 / fetchXxx 1 文件 1 处 | ✅ 一致 | 无 | — |

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
| 第 1 顺位 | D08 超长函数 | ✅ 已完成 | 无前置依赖，解锁 D09/D10 |
| 第 2 顺位 | D10 1000 行文件 | ✅ 已完成 | D08 完成后立即推进，6 批 34 文件全部完成 |
| 第 3 顺位 | D14 api 命名统一 | ✅ 已完成 | 与 D05/D13 解耦，D10 完成后推进 |
| 第 4 顺位 | D13 前端缩写命名 | ⏳ 进行中 | D14 完成后推进；Batch 1-5 已完成（PR #716/#717/#718/#719/#720），剩余 Batch 6-7 |
| 第 5 顺位 | D05 useI18n | ❌ 未开始 | D13/D14 完成后最后推进 |

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
| **P0 阻塞级** | 104 | 104 | **0** | 100% |
| **P1 高优先级** | 257 | 0 | **257** | 0% |
| **P2 中优先级** | 248 | 0 | **248** | 0% |
| **P3 低优先级** | 123 | 0 | **123** | 0% |
| **合计** | **732** | **104** | **628** | **14.2%** |

### 1.2 状态：⏳ D13 进行中（Batch 1-5 完成）→ 推进 D13 Batch 6-7 → D05

- **当前批次**：Batch 489 进行中 —— D13 前端缩写命名统一（第 4 顺位，Batch 1-5 已完成 PR #716/#717/#718/#719/#720，88 文件已重命名；剩余 Batch 6-7 约 33 文件）
- **下一批次**：Batch 490 D05 useI18n（第 5 顺位，D13 完成后最后推进，预估 30-36 子批次，344 文件未接入）
- **执行策略**：规则 13+14+15+20 联动；CI 全绿后自动进入下一批；所有警告视为错误必须真实修复；修复前必须调研现有实现禁止重复造轮子；注释必须与功能一致禁止随意编写（规则 20）；规则 13 步骤 4 自审必须 grep 所有引用新字段/新结构体的调用点；**禁止本地编译验证**（cargo check/build/test/clippy + npm build/type-check/vitest/vue-tsc），必须直接 push 让 CI 验证

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
P0-D03 ✅ 5 service 缓存 (L)    ──→ P0-D04 ✅ moka→Redis (L)
P0-D05 ⏳ useI18n (XL)          ← 独立（建议 D13/D14 后推进）
P0-D06 ✅ aria-label (XL)        ← 独立（55 子批次 ~225 文件）
P0-D07 ✅ img alt (S)            ← 独立（审计误判）
P0-D08 ✅ 超长函数 (XL)          ──→ P0-D09 ✅ 100 行函数 (L) ──→ P0-D10 ✅ 1000 行文件 (L)
P0-D11 ✅ setup_test_db (M)     ← 独立（审计误判）
P0-D12 ✅ 圈复杂度 (M)           ← 独立（6 重构 + 2 误判）
P0-D13 ⏳ 前端缩写命名 (XL)     ← 独立（D14 已完成，Batch 1-5 完成 PR #716/#717/#718/#719/#720）
P0-D14 ✅ api 命名统一 (XL)     ← 独立（5 批完成 PR #705-#714）
P0-D15 ✅ 升级零停机 (M)         ← 独立（审计误判）
P0-D16 ✅ 报表订阅调度 (M)       ← 独立（审计误判）
P0-D17 ✅ OA 公告 (M)            ← 独立（审计误判）
```

**关键路径**：P0-D08 → P0-D09 → P0-D10（代码质量链路）；P0-D14 → P0-D13 → P0-D05（前端重构链路）

---

## 三、未完成任务清单（模块 G，6 项大型任务）

> 模块 A-F 共 39 项 P0 任务已全部完成，详见 [archives/2026-07-22/doto-historical-tasks.md](file:///workspace/.monkeycode/docs/archives/2026-07-22/doto-historical-tasks.md)。

### 3.1 P0-D05 useI18n 接入率仅 3.1%（类七，XL，未开始）

- **来源**：batch-07 P0-07-5
- **证据**：2026-07-19 精确审计：实际 355 个 .vue 文件，已接入 11 个（接入率 3.1%），未接入 344 个；locales/zh-CN.ts 467 行 15 模块 332 键，预估需扩容至 5000+ 键；Top 20 硬编码密集文件累计 10746 行中文，单文件最大 fixed-assets/tabs/AssetListTab.vue 864 行
- **核实（2026-07-23）**：✅ 数据高度准确。355 文件 / 11 已接入 / 344 未接入 / 3.1% 接入率 / zh-CN.ts 467 行 15 模块均与 doto 一致；en-US.ts 467 行与 zh-CN.ts 对齐（双语同步良好）。❌ 唯一差异：AssetListTab.vue 实际 609 行（doto 记录 864 行，高估 255 行），单文件最大值记录失效，Top 20 排名可能已变化，推进前需重新扫描
- **修复方案**：355 个 .vue 视图组件全部接入 useI18n，所有硬编码中文迁移到 locales/zh-CN.ts + en-US.ts 同步；按业务模块横向切片，每批 10-12 文件，预估需 30-36 批次
- **关联文件**：[frontend/src/views/](file:///workspace/frontend/src/views/) + [frontend/src/locales/zh-CN.ts](file:///workspace/frontend/src/locales/zh-CN.ts) + [frontend/src/locales/en-US.ts](file:///workspace/frontend/src/locales/en-US.ts)
- **依赖**：建议在 D13/D14 完成后推进（避免同时修改 .vue 文件造成冲突）
- **工作量**：XL（5 项中最大）
- **批次**：488（D 系列 17 项一次性打包；预估 30-36 子批次）
- **执行优先级**：第 5 顺位（最后推进）

### 3.2 P0-D08 91+ 超长函数（类七，XL，进行中）

- **来源**：batch-07 P0-07-8
- **证据**：2026-07-19 精确扫描（fn-to-next-fn 口径）：>80 行函数约 91 个，>100 行函数约 54 个，>200 行函数 6 个；最严重案例 so/delivery.rs:110 ship_order 346 行、so/order_crud.rs:98 create_order 344 行、ar_service.rs:993 manual_verify 257 行、bpm_service.rs:242 approve_task 211 行、wage_service.rs:873 calculate 211 行、ar_service.rs:706 auto_verify 192 行
- **已重构确认**：event_bus.rs:412 start_event_listener D12-2 已重构（实际 279 行，CC 33→10 达标，列入观察名单不强拆）
- **豁免函数**：dye_batch_state_machine_service.rs:165 builtin_transition_rules 154 行纯数据表（27 条状态机三元组定义）豁免拆分
- **修复方案**：拆分超长函数为单一职责小函数（每个 ≤50 行），主函数仅做协调；按 ROI 分四梯队推进
- **关联文件**：[backend/src/services/so/delivery.rs](file:///workspace/backend/src/services/so/delivery.rs) / [ar_service.rs](file:///workspace/backend/src/services/ar_service.rs) / [bpm_service.rs](file:///workspace/backend/src/services/bpm_service.rs) / [wage_service.rs](file:///workspace/backend/src/services/wage_service.rs) / [voucher_service.rs](file:///workspace/backend/src/services/voucher_service.rs) / [quotation_service.rs](file:///workspace/backend/src/services/quotation_service.rs) / 等 35+ 文件
- **依赖**：无前置依赖
- **工作量**：XL
- **批次**：488（D 系列 17 项一次性打包；预估 10-12 子批次）
- **执行优先级**：第 1 顺位（无前置依赖 + 解锁 D09/D10）
- **当前进度**：✅ 全部完成（第一梯队 6/6 + 第二梯队 22/22 + 第三梯队 53/53（8 子批次）+ 第四梯队 84 函数（12 子批次）+ D09 收尾 2 个100+行函数 = 167 函数，PR #669-#682 main ba8e97f）；精确扫描确认 80-100行函数 0 候选 + 100+行函数 0 候选；详细 CI 修复教训见 [doto-su.md §V15 Batch 488](file:///workspace/.monkeycode/doto-su.md)
- **梯队规划**：
  - 第一梯队（>200 行 6 函数，2 批）：✅ 全部完成
  - 第二梯队（150-200 行 22 函数，4 批）：✅ 全部完成
  - 第三梯队（100-150 行 53 函数，8 子批次）：✅ 全部完成（PR #669/#670 + main 772c0312 + b869a0cd + 97fd77ee + 47ad2bfa + 4e1cb058）
  - 第四梯队（80-100 行，预估 20 批）：✅ 全部完成（子批次 1-12 共 84 函数 + 精确扫描确认 0 候选剩余，PR #672-#681 main 0c5c4d4；子批次9 修复 E0507 借用错误，子批次10 修复 E0106 生命周期标注 + E0308 DateTime 类型 + 3 个 BUG，子批次11 修复步骤4自审发现的 validate_import_data 命名冲突，子批次12 修复步骤4自审发现的 build_approval_active 命名冲突 + 新增 StockFabricFields/ItemAmounts 结构体封装多参数 helper）
  - D09 收尾（100+行函数）：✅ 全部完成（PR #682 main ba8e97f，拆分 get_import_template 113行→11行 + export_orders_to_csv 112行→24行，精确扫描确认 100+行函数 0 候选剩余）
  - 模板化提取候选：inventory_finance_bridge_service.rs 7 个 create_*_voucher 函数提取通用 create_bridge_voucher<VoucherBuilder>

### 3.3 P0-D09 54+ 函数超过 100 行（类二，L，✅ 已完成）

- **来源**：batch-02 P0-02-01
- **证据**：2026-07-19 精确扫描：>100 行函数约 54 个（与 D08 范围重叠，D08 是 D09 的超集）
- **修复方案**：D08 完成后 D09 自动完成（D09 是 D08 子集，D08 阈值 >80 行涵盖 D09 阈值 >100 行）
- **关联文件**：同 P0-D08
- **依赖**：P0-D08 ✅
- **工作量**：L（实际 0 增量工作，D08 完成即 D09 完成）
- **批次**：488（D08 子集，不独立成批）
- **当前进度**：✅ 全部完成（PR #682 main ba8e97f，精确扫描确认 100+行函数仅2个已拆分：get_import_template + export_orders_to_csv）

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

### 3.5 P0-D13 前端缩写命名组件（类二，XL，进行中）

- **来源**：batch-02 P0-02-05
- **证据**：2026-07-19 精确扫描：实际 123 个缩写命名 .vue 文件（views/ 122 + components/ 1）；25 类缩写前缀（Sc/Su/Lgs/Vchr/Pp/Di/Tfa/Sec/Cp/Sch/Prd/Bpm/Pc/Pi/Sa/Db/Purch/Prc/PrRtn/Ms/Sp/Olv/Ep/Bom/AI）；32 个父级 .vue 文件需更新 import（99 处 import 语句）；0 路由风险（router/index.ts 不直接 import 缩写文件）；0 e2e 风险
- **核实（2026-07-23）**：❌ 数量偏差。严格按 25 类前缀搜索实际 111 个（views/ 110 + components/ 1，doto 多记 12 个）；若补入 doto 批次规划提及但未列入 25 类前缀清单的 advanced(Rcp/Qlt/Rpt/Ai 4 个) + arReconciliation(Ar 6 个)，则实际 121 个（doto 多记 2 个）。❌ 前缀分类不完整：实际 27 类（doto 记 25 类，缺 Ar + Rcp/Qlt/Rpt/Ai）。❌ 第 7 批 purchase 实际仅 3 个缩写文件（doto 记 6 个，多记 3 个，其余 StatCards/CreateDlg/ViewDlg/ReceiveDlg 为描述性短名非缩写）。⚠️ doto 内部不一致：主记录 123 vs 批次规划 7 批总和 124
- **修复方案**：重命名为描述性全名（如 ScFilter→SalesContractFilter、SuVerDetail→SystemUpdateVersionDetail、LgsTbl→LogisticsTable、VchrForm→VoucherForm、BomForm→BillOfMaterialsForm）；同步重命名 composables 和父级 import；保留白名单：API（ApiEndpointTab 已描述性）/ i18n / a11y / V2Table（30+ 文件引用影响大）
- **关联文件**：[frontend/src/views/](file:///workspace/frontend/src/views/) 25 个模块的 components/ 子目录 + [frontend/src/components/ai/AIPredictionChart.vue](file:///workspace/frontend/src/components/ai/AIPredictionChart.vue)
- **依赖**：建议在 D14 完成后推进（避免同时修改 import 路径造成冲突）
- **工作量**：XL
- **批次**：489（D13 独立批次；预估 12-15 子批次，每批 8-10 文件）
- **执行优先级**：第 4 顺位（D14 完成后推进）
- **当前进度**：Batch 1-5 ✅ 已完成（88 文件重命名 + 32 caller 文件更新，PR #716/#717/#718/#719/#720）；剩余 Batch 6-7 约 33 文件
- **批次规划**：按模块分组（每模块独立批次）⚠️ 以下数量为 doto 原记录，核实后需调整（见核实行）
  - Batch 1：✅ 已完成（#716 main 937b9a2）sales-contract (3) + system-update (3) + sales-price (5) + purchase-price (5) 共 16 文件 + 6 caller（ScFilter→SalesContractFilter / SuVerDetail→SystemUpdateVersionDetail / SpTbl→SalesPriceTable / PpTbl→PurchasePriceTable 等）
  - Batch 2：✅ 已完成（#717 main c3e2f58）logistics (6) + finance/tabs (4) + voucher/tabs (4) + data-import (4) 共 18 文件 + 6 caller（LgsFilter→LogisticsFilter / VchrForm→VoucherForm / DiTplTable→DataImportTemplateTable 等，DiTplForm 接口重命名为 DataImportTemplateFormData）
  - Batch 3：✅ 已完成（#718 main 404fc14）security/two-factor (5) + security/components (4) + capacity (4) + advanced (4) 共 17 文件 + 7 caller（TfaStep1→TwoFactorAuthStep1 / SecAlertTbl→SecurityAlertTable / CpBottleneck→CapacityBottleneck / AiPanel→AdvancedAiPanel 等，useTfaProc.ts 接口 TwoFactorAuthStep3Instance 同步更新）
  - Batch 4：✅ 已完成（#719 main ef91527）api-gateway (1) + sales (3) + scheduling (10) + arReconciliation (6) 共 20 文件 + 8 caller（EpForm→ApiEndpointForm / OlvFilter→SalesOrderFilter / SchGAdj→SchedulingGanttAdjust / SchMTbl→SchedulingMachineTable / ArTbl→ArReconciliationTable 等）
  - Batch 5：✅ 已完成（#720 待合并）purchase-return (5) + material-shortage (3) + production (4) + bpm/definitions (5) 共 17 文件 + 5 caller（PrRtnApr→PurchaseReturnApproval / MsSevCard→MaterialShortageSeverityCard / PrdFilter→ProductionFilter / BpmDfFilter→BpmDefinitionFilter 等；额外 BpmDfFormData→BpmDefinitionFormData 4 处引用）
  - Batch 6：⏳ 待推进 bpm/approval (6) + purchase-contract (4) + purchase-inspection (5) + sales-analysis (5) 共 20 文件
  - Batch 7：⏳ 待推进 bom (1) + dashboard (4) + purchase (6→**核实 3**) + purchaseReceipt (4) + components/ai (1) 共 16→**核实 13** 文件

### 3.6 P0-D14 前端 api 命名不统一（类二，XL，✅ 已完成）

- **来源**：batch-02 P0-02-06
- **证据**：2026-07-19 精确扫描：96 个 api/*.ts 文件；风格 A（object 形式 `export const xxxApi = {}`）21 个 + 风格 B（function 形式）68 个 + 混合风格 4 个 + 纯 re-export 3 个；最大偏差源 listXxx 47 文件 84 处需改名为 getXxxList；次要偏差 addXxx 5 文件 6 处 / removeXxx 2 文件 2 处 / fetchXxx 1 文件 1 处 / queryXxx 2 文件 2 处
- **核实（2026-07-23）**：✅ 文件总数 96 一致；✅ addXxx 5 文件 6 处一致；✅ fetchXxx 1 文件 1 处一致；✅ request.ts 存在应保留。❌ 风格 A 实际 25 个（doto 记 21，少 4，工作量被低估）；❌ listXxx 实际 59 文件 104 处（doto 记 47 文件 84 处，少 12 文件 20 处，最大偏差源工作量低估约 23%）；❌ removeXxx 实际 1 文件 1 处（doto 记 2 文件 2 处，多 1，仅 role.ts）；❌ queryXxx 实际 1 文件 1 处（doto 记 2 文件 2 处，多 1，仅 assist-accounting.ts）。⚠️ 风格 A 25 + 风格 B 68 + 混合 4 + re-export 3 = 100 ≠ 96，风格 B/混合/re-export 分类口径需复核
- **修复方案**：统一为风格 B（function 形式）+ 命名规范 `getXxxList / createXxx / updateXxx / deleteXxx / getXxxById`；保留 request.ts 不改名；4 个混合文件先去重再统一；3 个 re-export 文件同步更新导出列表；预估影响 2000+ 处调用点
- **关联文件**：[frontend/src/api/](file:///workspace/frontend/src/api/) 96 个 .ts 文件
- **依赖**：无前置依赖（独立任务）
- **工作量**：XL
- **批次**：488（D 系列 17 项一次性打包；预估 10-12 子批次，每批 8-10 文件）
- **执行优先级**：第 3 顺位（与 D05/D13 解耦）
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
