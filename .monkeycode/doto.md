# 未完成任务

> 本文件**只记录未完成任务**（任务队列、待修复项、剩余清单）。
> 已完成任务见 [doto-su.md](file:///workspace/.monkeycode/doto-su.md)，一句话总结见 [CHANGELOG.md](file:///workspace/.monkeycode/CHANGELOG.md)，规则见 [MEMORY.md](file:///workspace/.monkeycode/MEMORY.md)。
> 最近整理：2026-07-22（按规则 10 深度整理归档：模块 A-F 已完成项归档到 [archives/2026-07-22/doto-historical-tasks.md](file:///workspace/.monkeycode/docs/archives/2026-07-22/doto-historical-tasks.md)；§三批次规划表 + §七历史任务阶段归档；仅保留模块 G 未完成项 + P1/P2/P3 规划 + 规则节点提醒）

---

## 一、当前状态与总体进度

### 1.1 进度总览

| 优先级 | 总数 | 已完成 | 未完成 | 完成率 |
|--------|------|--------|--------|--------|
| **P0 阻塞级** | 104 | 103 | **1** | 99.0% |
| **P1 高优先级** | 257 | 0 | **257** | 0% |
| **P2 中优先级** | 248 | 0 | **248** | 0% |
| **P3 低优先级** | 123 | 0 | **123** | 0% |
| **合计** | **732** | **103** | **629** | **14.1%** |

### 1.2 状态：🔄 规则 13 连续执行中

- **当前批次**：Batch 488 进行中（12/17 阶段性完成）—— D 系列已完成 11 项（详见 [doto-su.md §V15 Batch 488](file:///workspace/.monkeycode/doto-su.md)）+ D08 第一/第二梯队 28 函数全部完成（CI 全绿）+ D08 第三梯队 53 函数全部完成（8 子批次）+ D08 第四梯队子批次 1-10 共 70 函数已完成（PR #672/#673/#674/#675/#676/#678 main c4561f48 + #679 main 7ef28645）；第四梯队 135 函数剩余 65 候选（约 9 子批次）；剩余 5 项大型任务 2026-07-19 精确审计完成（详见 §4 各项条目）
- **下一批次**：Batch 488 继续 —— 按依赖关系推荐顺序：① D08 超长函数（第 1 顺位，无前置 + 解锁 D09/D10，预估 10-12 子批次）→ ② D10 大文件拆分（第 2 顺位，D08 完成后立即推进，预估 5-6 子批次）→ ③ D14 api 命名（第 3 顺位，与 D05/D13 解耦，预估 10-12 子批次）→ ④ D13 缩写命名（第 4 顺位，D14 完成后推进，预估 12-15 子批次）→ ⑤ D05 useI18n（第 5 顺位，D13/D14 完成后最后推进，预估 30-36 子批次）；累计预估 67-81 子批次
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
P0-D08 ⏳ 超长函数 (XL)          ──→ P0-D09 ⏳ 100 行函数 (L) ──→ P0-D10 ⏳ 1000 行文件 (L)
P0-D11 ✅ setup_test_db (M)     ← 独立（审计误判）
P0-D12 ✅ 圈复杂度 (M)           ← 独立（6 重构 + 2 误判）
P0-D13 ⏳ 前端缩写命名 (XL)     ← 独立（建议 D14 后推进）
P0-D14 ⏳ api 命名统一 (XL)     ← 独立
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
- **当前进度**：第一梯队 6/6 + 第二梯队 22/22 + 第三梯队 53/53（8 子批次）+ 第四梯队 70/135（10 子批次，PR #672/#673/#674/#675/#676/#678 main c4561f48 + #679 main 7ef28645）；剩余 65 候选（约 9 子批次）；详细 CI 修复教训见 [doto-su.md §V15 Batch 488](file:///workspace/.monkeycode/doto-su.md)
- **梯队规划**：
  - 第一梯队（>200 行 6 函数，2 批）：✅ 全部完成
  - 第二梯队（150-200 行 22 函数，4 批）：✅ 全部完成
  - 第三梯队（100-150 行 53 函数，8 子批次）：✅ 全部完成（PR #669/#670 + main 772c0312 + b869a0cd + 97fd77ee + 47ad2bfa + 4e1cb058）
  - 第四梯队（80-100 行 135 函数，预估 20 批）：✅ 子批次 1-10 共 70 函数完成（PR #672/#673/#674/#675/#676/#678 main c4561f48 + #679 main 7ef28645，子批次9 修复 E0507 借用错误，子批次10 修复 E0106 生命周期标注 + E0308 DateTime 类型 + 3 个 BUG）；剩余 65 候选（约 9 子批次）
  - 模板化提取候选：inventory_finance_bridge_service.rs 7 个 create_*_voucher 函数提取通用 create_bridge_voucher<VoucherBuilder>

### 3.3 P0-D09 54+ 函数超过 100 行（类二，L，D08 完成后自动完成）

- **来源**：batch-02 P0-02-01
- **证据**：2026-07-19 精确扫描：>100 行函数约 54 个（与 D08 范围重叠，D08 是 D09 的超集）
- **修复方案**：D08 完成后 D09 自动完成（D09 是 D08 子集，D08 阈值 >80 行涵盖 D09 阈值 >100 行）
- **关联文件**：同 P0-D08
- **依赖**：P0-D08
- **工作量**：L（实际 0 增量工作，D08 完成即 D09 完成）
- **批次**：488（D08 子集，不独立成批）

### 3.4 P0-D10 30 个后端文件超过 1000 行（类二，L，未开始）

- **来源**：batch-02 P0-02-02
- **证据**：2026-07-19 精确扫描：实际 30 个 >1000 行文件，13 个 >1500 行，1 个 >2000 行（ar_service.rs 2067 行）；审计后新增越线 main.rs 1005 行 + init_service.rs 1287 行；28 个原审计文件全部仍 >1000 行无一下降；bi_analysis_service.rs 增长最快（+201 行 1461→1662）
- **修复方案**：按职责拆分为多个文件（如 ar_service.rs 拆分为 ar_service / ar_aging_service / ar_collection_service；models/status.rs 拆分为 status/sales / status/purchase / status/inventory；main.rs 拆为 main / routes_bootstrap / middleware_bootstrap）
- **关联文件**：[backend/src/services/ar_service.rs](file:///workspace/backend/src/services/ar_service.rs) (2067) / [production_order_service.rs](file:///workspace/backend/src/services/production_order_service.rs) (1998) / [so/delivery.rs](file:///workspace/backend/src/services/so/delivery.rs) (1930) / [voucher_service.rs](file:///workspace/backend/src/services/voucher_service.rs) (1841) / [energy_service.rs](file:///workspace/backend/src/services/energy_service.rs) (1800) / 等 30 文件
- **依赖**：P0-D08/D09（避免函数拆分和文件拆分同时进行造成冲突）
- **工作量**：L
- **批次**：488（D 系列 17 项一次性打包；预估 5-6 子批次，每批 5-6 文件）
- **执行优先级**：第 2 顺位（D08 完成后立即推进）
- **批次规划**：
  - 第 1 批：ar_service.rs (2067) + production_order_service.rs (1998) + so/delivery.rs (1930) 3 个 >1800 行文件
  - 第 2 批：voucher_service.rs (1841) + energy_service.rs (1800) + outsourcing_service.rs (1782) + business_mode_service.rs (1718) 4 个 >1700 行文件
  - 第 3 批：chemical_service.rs (1676) + bi_analysis_service.rs (1662) + models/status.rs (1577) + mrp_engine_service.rs (1556) 4 个 >1500 行文件
  - 第 4 批：dye_batch_state_machine_service.rs (1512) + wage_service.rs (1507) + ar/vfy.rs (1320) + ap_invoice_service.rs (1306) 4 个 >1300 行文件
  - 第 5 批：init_service.rs (1287) + flow_card_service.rs (1271) + ap_reconciliation_service.rs (1243) + search/elastic.rs (1230) 4 个 >1200 行文件
  - 第 6 批：剩余 11 个 1000-1200 行文件

### 3.5 P0-D13 前端 123 个组件缩写命名（类二，XL，未开始）

- **来源**：batch-02 P0-02-05
- **证据**：2026-07-19 精确扫描：实际 123 个缩写命名 .vue 文件（views/ 122 + components/ 1）；25 类缩写前缀（Sc/Su/Lgs/Vchr/Pp/Di/Tfa/Sec/Cp/Sch/Prd/Bpm/Pc/Pi/Sa/Db/Purch/Prc/PrRtn/Ms/Sp/Olv/Ep/Bom/AI）；32 个父级 .vue 文件需更新 import（99 处 import 语句）；0 路由风险（router/index.ts 不直接 import 缩写文件）；0 e2e 风险
- **修复方案**：重命名为描述性全名（如 ScFilter→SalesContractFilter、SuVerDetail→SystemUpdateVersionDetail、LgsTbl→LogisticsTable、VchrForm→VoucherForm、BomForm→BillOfMaterialsForm）；同步重命名 composables 和父级 import；保留白名单：API（ApiEndpointTab 已描述性）/ i18n / a11y / V2Table（30+ 文件引用影响大）
- **关联文件**：[frontend/src/views/](file:///workspace/frontend/src/views/) 25 个模块的 components/ 子目录 + [frontend/src/components/ai/AIPredictionChart.vue](file:///workspace/frontend/src/components/ai/AIPredictionChart.vue)
- **依赖**：建议在 D14 完成后推进（避免同时修改 import 路径造成冲突）
- **工作量**：XL
- **批次**：488（D 系列 17 项一次性打包；预估 12-15 子批次，每批 8-10 文件）
- **执行优先级**：第 4 顺位（D14 完成后推进）
- **批次规划**：按模块分组（每模块独立批次）
  - sales-contract (3) + system-update (3) + sales-price (5) + purchase-price (5) 第 1 批 16 文件
  - logistics (6) + finance/tabs (4) + voucher/tabs (4) + data-import (4) 第 2 批 18 文件
  - security/two-factor (5) + security/components (4) + capacity (4) + advanced (4) 第 3 批 17 文件
  - api-gateway (1) + sales (3) + scheduling (10) + arReconciliation (6) 第 4 批 20 文件
  - purchase-return (5) + material-shortage (3) + production (4) + bpm/definitions (5) 第 5 批 17 文件
  - bpm/approval (6) + purchase-contract (4) + purchase-inspection (5) + sales-analysis (5) 第 6 批 20 文件
  - bom (1) + dashboard (4) + purchase (6) + purchaseReceipt (4) + components/ai (1) 第 7 批 16 文件

### 3.6 P0-D14 前端 api 命名不统一（类二，XL，未开始）

- **来源**：batch-02 P0-02-06
- **证据**：2026-07-19 精确扫描：96 个 api/*.ts 文件；风格 A（object 形式 `export const xxxApi = {}`）21 个 + 风格 B（function 形式）68 个 + 混合风格 4 个 + 纯 re-export 3 个；最大偏差源 listXxx 47 文件 84 处需改名为 getXxxList；次要偏差 addXxx 5 文件 6 处 / removeXxx 2 文件 2 处 / fetchXxx 1 文件 1 处 / queryXxx 2 文件 2 处
- **修复方案**：统一为风格 B（function 形式）+ 命名规范 `getXxxList / createXxx / updateXxx / deleteXxx / getXxxById`；保留 request.ts 不改名；4 个混合文件先去重再统一；3 个 re-export 文件同步更新导出列表；预估影响 2000+ 处调用点
- **关联文件**：[frontend/src/api/](file:///workspace/frontend/src/api/) 96 个 .ts 文件
- **依赖**：无前置依赖（独立任务）
- **工作量**：XL
- **批次**：488（D 系列 17 项一次性打包；预估 10-12 子批次，每批 8-10 文件）
- **执行优先级**：第 3 顺位（与 D05/D13 解耦）
- **批次规划**：
  - Batch 1：财务 AP/AR 9 文件（ap.ts/ap-invoice.ts/ap-payment.ts/ar.ts/ar-reconciliation.ts/ar-reconciliation-enhanced.ts/ap-reconciliation.ts/ap-verification.ts/voucher.ts）
  - Batch 2：采购/销售/库存 18 文件
  - Batch 3：生产/质量/BOM/MRP 12 文件
  - Batch 4：CRM/客户/供应商/贸易 14 文件
  - Batch 5a/5b：系统/权限/基础/报表/其他 40 文件（拆 2 子批）

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
