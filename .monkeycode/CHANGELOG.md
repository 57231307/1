# 任务精简总结

> 重要变更一句话摘要列表。详细历史请查阅 [`.monkeycode/docs/archives/`](file:///workspace/.monkeycode/docs/archives/)。
>
> 本文件保留批次 243+ 的详细记录（v14 修复阶段），批次 1-242 的详细记录已归档到 `docs/archives/2026-07-10/CHANGELOG-2026-07-10-pre-cleanup.md`。

---

## 2026-07-10 (批次 256 v14 中风险重复实现修复 — service 分页逻辑接入 paginate_with_total 第二批，CI 12/12 核心全绿)

### 批次 256：v14 中风险重复实现修复 — 4 个 service 分页逻辑接入 paginate_with_total 第二批

**修复内容**：bug.md 中风险重复实现问题 — 继批次 255 首批 4 文件后，第二批处理 4 个 service 的 list 方法手写 num_items + fetch_page 分页逻辑，与已封装的 paginate_with_total 工具函数重复，违反 DRY 原则。

**修改文件**（4 文件 +26 -25 行）：
- `backend/src/services/email_log_service.rs`：list 标准替换 + 补 clamp 防 DoS
- `backend/src/services/email_template_service.rs`：list 标准替换（原有 clamp 语义保留）
- `backend/src/services/report_subscription_service.rs`：list 标准替换 + 补 clamp 防 DoS
- `backend/src/services/report_template_service.rs`：list 标准替换 + 补 clamp 防 DoS

**技术要点**：
- paginate_with_total 内部已做 page.saturating_sub(1) 偏移，调用方不可再减 1
- 删除独立 select.clone().count() 查询，复用 paginator 的 num_items()
- 统一补充 page.clamp(1, 1000) 防 DoS
- PaginatorTrait 导入保留（.paginate() 方法需要）

**CI 验证**：CI run #29060776609，12/12 核心 job 全绿（Clippy 一次通过），E2E 失败为已知问题不阻塞。PR #433 squash merge 到 main（commit 4f83af05）。

---

## 2026-07-10 (批次 255 v14 中风险重复实现修复 — service 分页逻辑接入 paginate_with_total 首批，CI 12/12 核心全绿)

### 批次 255：v14 中风险重复实现修复 — 4 个 service 分页逻辑接入 paginate_with_total

**修复内容**：bug.md 中风险重复实现问题 — 35 个 service 文件手写 `num_items + fetch_page` 分页逻辑，与已封装的 `paginate_with_total` 工具函数重复，违反 DRY 原则。首批处理 4 个文件。

**修改文件**（4 文件 +15 -10 行）：
- `backend/src/services/sales_price_service.rs`：`list_strategies` 标准替换 + 补 clamp 防 DoS
- `backend/src/services/ap_invoice_service.rs`：`get_list` 标准替换 + 补 clamp 防 DoS
- `backend/src/services/role_service.rs`：`list_roles` 修复 fetch_page(page) 未做 saturating_sub(1) 偏移的 bug + 补 clamp
- `backend/src/services/supplier_service.rs`：`list_suppliers` 保留原有 clamp，移除冗余 saturating_sub

**技术要点**：
- `paginate_with_total` 内部已做 `page.saturating_sub(1)` 偏移，调用方不可再减 1
- `role_service.rs` 修复现存 bug：原 `fetch_page(page)` 直接传 1-indexed 页码，未做偏移，导致第一页数据跳到第二页
- 统一补充 `page.clamp(1, 1000)` 防 DoS（supplier_service 原有，其余 3 个新增）
- `PaginatorTrait` 导入保留（`.paginate()` 方法需要）

**CI 验证**：CI run #29059632346，12/12 核心 job 全绿（Clippy 一次通过），E2E 失败为已知问题不阻塞。PR #432 squash merge 到 main（commit 026fcc3）。

---

## 2026-07-10 (批次 254 v14 中风险死代码修复 — composable eslint-disable any 清理，CI 12/12 核心全绿)

### 批次 254：v14 中风险死代码修复 — 14 个 composable 文件 eslint-disable any 指令清理

**修复内容**：bug.md 中风险死代码问题 — 14 个 composable 文件首行均有 `/* eslint-disable @typescript-eslint/no-explicit-any */`，但经审计这些文件中真实的 any 类型使用为 0（已在早期批次中替换为 `unknown` + 类型守卫或具体业务类型）。这些 eslint-disable 指令是 P14 批次拆分 Vue 重构时为快速通过 lint 而添加的残留，现已成为 any 类型的"避风港"，与 v11 前端 P2-1 any 类型清理成果相矛盾。

**修改文件**（14 文件 +0 -14 行）：
- `frontend/src/views/voucher/tabs/composables/useVchrLst.ts` + `useVchrLstProc.ts`
- `frontend/src/views/system-update/composables/useSysUpd.ts` + `useSysUpdProc.ts`
- `frontend/src/views/sales-price/composables/useSp.ts`
- `frontend/src/views/sales-contract/composables/useSc.ts`
- `frontend/src/views/purchase-price/composables/usePp.ts` + `usePpProc.ts`
- `frontend/src/views/purchase-contract/composables/usePc.ts` + `usePcProc.ts`
- `frontend/src/views/finance/tabs/composables/useVchr.ts` + `useVchrProc.ts`
- `frontend/src/views/arReconciliation/composables/useArDisp.ts`
- `frontend/src/views/api-gateway/composables/useApiKey.ts`

**技术要点**：
- 审计结果：14 个文件共 2836 行，any 匹配行 31 行（全部为指令 + 注释），真实 any 类型使用 0 处
- 所有文件的 catch 块已使用 `catch (error: unknown)` + `error instanceof Error` 类型守卫
- ref/参数/返回值均使用具体业务实体类型（VoucherEntity/SalesPrice/PurchaseContract 等）
- 删除 eslint-disable 指令后 ESLint + 类型检查一次通过，确认无遗漏的 any 使用

**CI 验证**：CI run #29058822394，12/12 核心 job 全绿（ESLint + 类型检查一次通过），E2E 失败为已知问题不阻塞。PR #431 squash merge 到 main（commit d2abb55）。

---

## 2026-07-10 (批次 253 v14 中风险空实现修复 — AdvancedFilter handleLogicChange 真实实现，CI 12/12 核心全绿)

### 批次 253：v14 中风险空实现修复 — AdvancedFilter handleLogicChange 空函数改为真实实现

**修复内容**：bug.md 中风险空实现问题 — `AdvancedFilter.vue` 第 249 行 `handleLogicChange` 为空函数 `() => {}`，绑定在第 36 行的条件组 AND/OR 逻辑切换下拉框 `@change`。用户切换条件组逻辑运算符时无任何响应，多条件组合查询能力失效。

**修改文件**（2 文件 +31 -2 行）：
- `frontend/src/components/AdvancedFilter.vue`：新增 `logicChange` emit 事件 + `handleLogicChange` 接收 `groupIndex` 参数实现真实逻辑
- `frontend/src/views/components-demo/AdvancedFilterDemo.vue`：演示 `logicChange` 事件真实接入（自动更新筛选结果）

**技术要点**：
- 新增 `logicChange: [groupIndex: number, logic: 'AND' | 'OR', filters: FilterGroup[]]` emit 事件
- `handleLogicChange` 接收 `groupIndex` 参数，emit 事件让父组件可响应（如自动重新查询或更新预览）
- 显示轻量级 `ElMessage.info` 提示让用户知道逻辑已切换（duration: 1500ms）
- 模板 `@change` 改为 `() => handleLogicChange(groupIndex)` 传递循环索引
- Demo 页面 `handleLogicChange` 演示：自动更新 `filterResult` 以反映新的逻辑关系

**CI 验证**：CI run #29058007479，12/12 核心 job 全绿，E2E 失败为已知问题不阻塞。PR #430 squash merge 到 main（commit da659f7）。

---

## 2026-07-10 (批次 252 v14 中风险空实现修复 — unreachable! panic 改为返回错误，CI 12/12 核心全绿)

### 批次 252：v14 中风险空实现修复 — bi_analysis + dual_unit_converter unreachable! 改为防御性错误处理

**修复内容**：bug.md 中风险空实现问题 — `bi_analysis_service.rs` 三处 `unreachable!()` 宏调用（`dim_to_expr` 函数第 254 行 dimension 维度匹配 `_` 分支 + 第 1188 行 item 级 measure 匹配 `_` 分支 + 第 1203 行 order 级 measure 匹配 `_` 分支），用户可控的 dim/measure 参数若绕过校验将触发 panic 导致进程崩溃；`dual_unit_converter_handler.rs` 第 116 行 `unreachable!()` 在校验逻辑被重构后可能 panic 崩溃。

**修改文件**（2 文件 +101 -31 行）：
- `backend/src/services/bi_analysis_service.rs`：`dim_to_expr` 返回类型改为 `Result`，`_` 分支返回 `AppError::validation`；提取 `measure_to_expr` 独立函数替代原内联 match + `unreachable!()`；新增 6 个单元测试
- `backend/src/handlers/dual_unit_converter_handler.rs`：`_` 分支改为 `return Err(AppError::bad_request)`

**技术要点**：
- `dim_to_expr`：返回类型从 `(&'static str, &'static str)` 改为 `Result<(&'static str, &'static str), AppError>`，`_` 分支返回 `AppError::validation(format!("不支持的维度: {}", dim))`
- 提取 `measure_to_expr(measure, item_level)` 独立函数，用 `(measure, item_level)` 元组 match 替代原两处内联 match，`_` 分支返回 `AppError::validation`
- `pivot` 方法调用处加 `?` 传播错误
- `dual_unit_converter_handler.rs`：`_ => unreachable!(...)` 改为 `_ => return Err(AppError::bad_request("无效的单位..."))`，防御性返回错误
- 新增 6 个单元测试：验证所有合法维度/度量返回 Ok，非法维度/度量/空字符串返回 Err（而非 panic）

**CI 验证**：CI run #29046877533，12/12 核心 job 全绿（Clippy 一次通过），E2E 失败为已知问题不阻塞。PR #429 squash merge 到 main（commit faa9749）。

---

## 2026-07-10 (批次 251 v14 中风险简化阉割修复 — webhook retry payload 持久化，CI 12/12 核心全绿)

### 批次 251：v14 中风险简化阉割修复 — webhook retry 从假 payload 改为重投持久化原始数据

**修复内容**：bug.md 中风险简化阉割问题 — `webhook_service.rs` 的 webhook 发送时 payload 仅存内存，发送后丢弃；`retry_webhook` 构造假 payload `{"webhook_id","retry":true}`，接收方拿不到原始业务上下文；retry_count 仅在网络层异常时递增（HTTP 业务失败 4xx/5xx 不计数）；原代码用 `if let ActiveValue::Set(v) = &final_model.retry_count` 取值，但 `webhook.into()` 生成 `Unchanged` 值，导致模式匹配永远不命中，retry_count 永远读 0。

**修改文件**（7 文件 +95 -33 行）：
- `backend/migration/src/m0047_add_last_payload_to_webhooks.rs`：新增迁移模块
- `backend/migrations/20260710000001_add_last_payload_to_webhooks/up.sql` + `down.sql`：webhooks 表添加 last_payload + last_event 列
- `backend/migration/src/lib.rs`：注册 m0047 迁移
- `backend/src/models/webhook.rs`：新增 last_payload + last_event 字段
- `backend/src/services/webhook_service.rs`：trigger_webhook 发送前持久化 payload + event；retry_count 修复（HTTP 业务失败也递增，成功重置 0，修复 ActiveValue 值提取 bug）
- `backend/src/handlers/webhook_handler.rs`：retry_webhook 从持久化存储读取原始 payload + event 重投

**技术要点**：
- 新增迁移 m0047：webhooks 表添加 `last_payload TEXT` + `last_event VARCHAR(100)` 列
- `trigger_webhook`：发送前将 `last_payload = Set(Some(payload.to_string()))` + `last_event = Set(Some(event.to_string()))` 持久化
- retry_count 修复：在 `webhook.into()` 之前从 Model 直接读取 `let current_retry_count = webhook.retry_count;`（非 ActiveValue），HTTP 业务失败（Ok(delivery) 但 delivery.success=false）也递增计数，成功时重置为 0
- `retry_webhook` handler：从 `webhook.last_payload` + `webhook.last_event` 读取持久化数据，调用 `trigger_webhook` 重投原始业务数据
- 修复 retry_count 值提取 bug：原 `if let ActiveValue::Set(v) = &final_model.retry_count` 永远不匹配（`webhook.into()` 生成 Unchanged 而非 Set）

**CI 验证**：CI run #29045660807，12/12 核心 job 全绿（Clippy 一次通过），E2E 失败为已知问题不阻塞。PR #428 squash merge 到 main（commit 226af53）。

---

## 2026-07-10 (批次 250 v14 中风险简化阉割修复 — budget_management 审批流完整化，CI 12/12 核心全绿)

### 批次 250：v14 中风险简化阉割修复 — budget_management adjust_budget 审批流跳过改为完整审批闭环

**修复内容**：bug.md 中风险简化阉割问题 — `budget_management_service.rs` 的 `adjust_budget` 方法硬编码 `approval_status: APPROVED` 并立即应用金额变更（注释自述"简化：直接批准"），完全跳过审批环节。预算金额调整属高风险财务操作，应经审批人审核。

**修改文件**（4 文件 +207 -9 行）：
- `backend/src/services/budget_management_service.rs`：修改 `adjust_budget` + 新增 `approve_adjustment`/`reject_adjustment`/`reject_plan` 方法
- `backend/src/handlers/budget_management_handler.rs`：新增 3 个 handler 函数
- `backend/src/routes/finance.rs`：新增 3 条路由
- `frontend/src/api/asset.ts`：新增 3 个前端 API 函数

**技术要点**：
- `adjust_budget`：创建调整单改为 PENDING 状态（原 APPROVED），不再立即应用金额变更
- `approve_adjustment`：PENDING → APPROVED，事务内对调整单和预算方案双重 `lock_exclusive`，审批通过后实际应用金额变更
- `reject_adjustment`：PENDING → REJECTED，不应用金额变更
- `reject_plan`：DRAFT → REJECTED，补全预算方案审批闭环
- 新增路由：`POST /budgets/adjust/:id/approve`、`POST /budgets/adjust/:id/reject`、`POST /budgets/plans/:id/reject`
- 审批状态机：DRAFT → PENDING → APPROVED（应用金额变更）/ REJECTED（不应用）

**CI 验证**：CI run #29044585502，12/12 核心 job 全绿，PR #427 squash merge 到 main（commit b2520cd）。

---

## 2026-07-10 (批次 249 v14 中风险简化阉割修复 — capacity_service 硬编码置信度动态化，CI 12/12 核心全绿)

### 批次 249：v14 中风险简化阉割修复 — capacity_service forecast_capacity 硬编码置信度 0.8 改为动态计算

**修复内容**：bug.md 中风险简化阉割问题 — `capacity_service.rs` 的 `forecast_capacity` 方法硬编码 `confidence: 0.8`，无法反映历史数据量和预测期限对预测可信度的影响。

**修改文件**（1 文件 +109 -2 行）：
- `backend/src/services/capacity_service.rs`：`forecast_capacity` 方法 + 新增 `calculate_forecast_confidence` 辅助方法 + 5 个单元测试

**技术要点**：
- 查询工作中心已完成历史订单数量（`ProductionOrderEntity::find().filter(Status.eq("COMPLETED")).count()`）
- 置信度三维动态计算：
  1. 基础置信度（历史订单数量）：0→0.30, 1-5→0.50, 6-20→0.70, 21-50→0.80, 50+→0.85
  2. 当前负荷加成：有排产数据 +0.05，无排产数据 -0.10
  3. 预测期限衰减因子：7天内×1.0, 30天内×0.92, 90天内×0.78, 180天内×0.62, 更长×0.45
- 最终置信度限制在 [0.10, 0.95] 区间，避免极端值
- 新增 `PaginatorTrait` 导入用于 `count()` 方法
- CI 修复：1 轮（`f64` 类型标注消除 `clamp` 方法歧义 `error[E0689]: can't call method clamp on ambiguous numeric type {float}`）

**CI 验证**：CI run #29043478176，12/12 核心 job 全绿（Clippy + 单元测试 + 后端构建均通过），PR #426 squash merge 到 main（commit 82269a4）。

---

## 2026-07-10 (批次 248 v14 中风险缓存未利用修复 — AR/AP 报表接入 CacheService，CI 12/12 核心全绿)

### 批次 248：v14 中风险缓存未利用修复 — AR/AP 报表 8 端点接入 CacheService

**修复内容**：bug.md 中风险性能问题 — `cache_service.rs` 已实现并注入 AppState，但零业务调用（命中率统计永远为 0）。AR/AP 报表 8 个端点每次请求都执行 SQL 聚合查询，重复查询浪费数据库资源。

**修改文件**（2 文件 +158 -8 行）：
- `backend/src/handlers/ar_report_handler.rs`：4 个端点（statistics/daily/monthly/aging）接入 CacheService
- `backend/src/handlers/ap_report_handler.rs`：4 个端点（statistics/daily/monthly/aging）接入 CacheService

**技术要点**：
- 缓存 key 命名遵循 `module:` 前缀规范（`ar:report:xxx` / `ap:report:xxx`）
- TTL 60 秒，平衡新鲜度与数据库负载
- 缓存仅作加速层，`CACHE_ENABLED=false` 时自动短路返回 None
- 命中缓存时直接反序列化返回，跳过 service 调用
- 未命中时执行查询并写入缓存
- CI 修复：1 轮（`Option<i32>`/`Option<NaiveDate>` 未实现 Display，缓存 key 拼接改用 `{:?}`）

**CI 验证**：CI run #29041889011，12/12 核心 job 全绿，PR #425 squash merge 到 main（commit 53ce6b53）。

---

## 2026-07-10 (批次 247 v14 中风险硬编码 URL 修复 — CLI 健康检查，CI 12/12 核心全绿)

### 批次 247：v14 中风险硬编码 URL 修复 — CLI 健康检查

**修复内容**：bug.md 中风险漏洞 #17 — `backend/src/cli/util/service.rs:191` 硬编码 `http://127.0.0.1:8082/health`，部署到非 8082 端口环境时健康检查失效。

**修改文件**（1 文件 +25 -6 行）：
- `backend/src/cli/util/service.rs`：
  1. 新增 `backend_host()` / `backend_port()` / `backend_health_url()` 辅助函数，从环境变量 `SERVER__HOST` / `SERVER__PORT` 读取（默认 `127.0.0.1` / `8082`）
  2. `cmd_health`：健康检查 URL 改为 `backend_health_url()` 动态拼接
  3. `cmd_status`：端口监听检查也改为从 `backend_port()` 读取端口

**技术要点**：
- 与 config crate 的 `SERVER__HOST` / `SERVER__PORT` 环境变量约定一致
- 使用 `std::env::var` + `unwrap_or_else` 提供合理默认值（非 `require_env` 退出模式）

**CI 验证**：CI run #29038390548，12/12 核心 job 全绿，PR #424 squash merge 到 main（commit 47d86d86）。

---

## 2026-07-10 (批次 246 v14 中风险空实现修复 — dye-recipe handleViewVersion，CI 12/12 核心全绿)

### 批次 246：v14 中风险空实现修复 — dye-recipe handleViewVersion

**修复内容**：bug.md 中风险空实现漏洞 #18 — `frontend/src/views/dye-recipe/index.vue` 的 `handleViewVersion` 原为空实现（`(_row: DyeRecipe) => {}`），用户在版本历史对话框中点击"查看"按钮无任何响应。

**修改文件**（1 文件 +8 -2 行）：
- `frontend/src/views/dye-recipe/index.vue`：handleViewVersion 从空实现改为复用主对话框只读模式展示版本详情（关闭版本历史对话框 → 设置标题 `查看版本详情 - v{版本号}` → `isView = true` → `Object.assign(formData, row)` → 打开主对话框），与批次 239 P0-3 `handleView` 修复采用相同模式。

**CI 验证**：CI run #29037444886，12/12 核心 job 全绿，PR #423 squash merge 到 main（commit 16754cf7）。

---

## 2026-07-09 (批次 245 v14 中风险性能修复 — ap_report_service 报表 SQL 聚合，CI 12/12 核心全绿)

### 批次 245：v14 中风险性能修复 — ap_report_service 4 个报表方法 SQL 层聚合

**修复内容**：bug.md 中风险性能问题 — ap_report_service.rs 4 个报表方法全量加载发票到内存做聚合，宽日期范围查询可能导致 OOM。

**修改文件**（1 文件 +424 -219 行）：
- `backend/src/services/ap_report_service.rs`：
  1. `get_statistics_report`：原 `.all()` 加载全部发票后内存 COUNT/SUM/过滤逾期 → 主聚合 SQL（COUNT/SUM/CASE WHEN overdue）+ by_status GROUP BY + by_type GROUP BY
  2. `get_daily_report`：原 3 次 `.all()` 全量加载 → 3 个 `query_one` 聚合查询（新增/到期/付款）
  3. `get_monthly_report`：原 2 次 `.all()` 全量加载做余额计算 → 2 个 `query_one` 聚合查询（月初/月末余额）
  4. `get_aging_report`：原全量加载未付清发票内存分桶 → SQL CASE WHEN + SUM + COUNT 分桶聚合 + 未到期单独查询

**技术要点**：
- 规则 12 合规：全部参数（start_date/end_date/status/supplier_id/today）使用 `$N` 参数化绑定
- CI 修复：1 轮（clippy `supplier_id.unwrap()` after `is_some()` 警告 → 改用 `supplier_id.map(|sid|)` 模式，i32 为 Copy 可直接多次 map；消除 `supplier_param_idx` 中间变量，每个子查询独立计算参数索引）
- 性能收益：O(N) 内存 → O(1) 内存（统计/日/月报表）/ O(分组数) 内存（by_status/by_type）

**CI 验证**：CI run #29036375275，12/12 核心 job 全绿，PR #422 squash merge 到 main（commit ae7d4619）。

---

## 2026-07-09 (批次 244 v14 中风险性能修复 — ar_service 报表 SQL 聚合，CI 12/12 核心全绿)

### 批次 244：v14 中风险性能修复 — ar_service 3 个报表方法 SQL 层聚合

**修复内容**：bug.md 中风险性能问题 — ar_service.rs 3 个报表方法全量加载发票到内存做聚合，宽日期范围查询可能导致 OOM。

**修改文件**（1 文件 +148 -87 行）：
- `backend/src/services/ar_service.rs`：
  1. `get_statistics_report`：原 `.all()` 加载全部发票后内存 COUNT/SUM/过滤逾期 → SQL `COUNT(*) + COALESCE(SUM) + COUNT(CASE WHEN overdue)` 单行聚合
  2. `get_daily_report`：原 `.all()` 加载后 HashMap 按日聚合 + 内存排序 → SQL `GROUP BY invoice_date + ORDER BY`
  3. `get_monthly_report`：原 `.all()` 加载后 HashMap 按月份聚合 + 内存排序 → SQL `GROUP BY to_char(invoice_date, 'YYYY-MM') + ORDER BY`
  4. 删除 `DailyAgg` / `MonthlyAgg` 死代码 struct（原内存聚合辅助结构）

**技术要点**：
- 规则 12 合规：全部参数（status/customer_id/start_date/end_date/today）使用 `$N` 参数化绑定
- CI 修复：1 轮（clippy `param_idx` 未使用赋值警告 → 改用 `params.len() + 1` 模式消除手动递增变量）
- 性能收益：O(N) 内存 → O(1) 内存（统计报表）/ O(分组数) 内存（日/月报表）

**CI 验证**：CI run #29034578201，12/12 核心 job 全绿，PR #421 squash merge 到 main（commit dcd8488d）。

---

## 2026-07-09 (批次 243 v14 中风险安全漏洞修复，CI 12/12 核心全绿，中风险 1/25 完成)

### 批次 243：v14 中风险安全漏洞修复（XSS 防护 + 输入验证）

**修复内容**：bug.md 深度调研报告中风险安全漏洞 — 2 个问题：
1. report-templates/index.vue XSS 潜在风险：报表预览单元格值直接拼接 HTML，DOMPurify 默认允许 `<img>`/`<a>` 标签
2. tracking_handler.rs 输入验证缺失：path/event_type/event_data 等字段无长度约束，超大字段可触发 DoS

**修改文件**（2 文件 +33 -4 行）：
- `frontend/src/views/report-templates/index.vue`：引入 escapeHtml（@/utils/print），报表预览表头字段名与单元格值均经 HTML 转义后再拼接，形成双层防护（escapeHtml 转义 + DOMPurify 净化）
- `backend/src/handlers/tracking_handler.rs`：PageViewRequest + BehaviorRequest 添加 `#[derive(Validate)]` + 各字段 `#[validate(length(max=N))]` 约束，handler 中调用 `req.validate()` 校验

**技术要点**：
- 复用项目已有的 escapeHtml 工具函数（@/utils/print），避免重复实现
- validator crate 的 Validate derive 实现 Rust 输入校验，与 serde Deserialize 协同工作
- 安全收益：消除 XSS 潜在风险（防止后端数据含恶意 `<img onerror>` 误导用户）+ 防止超大字段 DoS

**CI 验证**：CI run #29032882693，12/12 核心 job 全绿（Rust Clippy + 单元测试 + 后端构建、前端 ESLint/类型检查/构建/测试均通过），E2E 失败为已知问题不阻塞。PR #420 squash merge 到 main（commit 0810fe3）。

---

## 历史归档

> 批次 1-242 的详细记录已归档到 [docs/archives/2026-07-10/CHANGELOG-2026-07-10-pre-cleanup.md](file:///workspace/.monkeycode/docs/archives/2026-07-10/CHANGELOG-2026-07-10-pre-cleanup.md)。
> 批次 1-99 的更早记录见 [docs/archives/2026-07-05/](file:///workspace/.monkeycode/docs/archives/2026-07-05/)。
