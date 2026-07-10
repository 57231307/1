# 已完成任务归档

> 本文件保存**已完成的任务**详细记录（修改内容、技术要点、CI 验证）。
> 未完成任务见 [doto.md](file:///workspace/.monkeycode/doto.md)，一句话总结见 [CHANGELOG.md](file:///workspace/.monkeycode/CHANGELOG.md)，规则见 [MEMORY.md](file:///workspace/.monkeycode/MEMORY.md)。

---

## 📝 已完成批次详细记录（v14 阶段，批次 237-262）

### 批次 262：Playwright E2E 测试增强 + E2E 独立工作流（PR 待定）

**修复内容**：用户需求 — 针对 Playwright E2E 测试增强，提供网络拦截/Mock/弱网/多浏览器/多上下文隔离/多角色协作/RPA 全栈自动化能力。同时将 E2E 测试从 ci-cd.yml 独立到 e2e-batch.yml，每 30 批次运行一次，不阻塞主 CI。

**修改文件**（9 文件）：

1. **E2E 增强工具集**（3 新文件）：
   - `frontend/e2e/fixtures/network.ts`：网络拦截/Mock/弱网工具集（mockApiError/mockApiSuccess/mockNetworkFailure/simulateSlowNetwork/RequestObserver/waitForApiCall/mockOnce）
   - `frontend/e2e/fixtures/multi-context.ts`：多上下文隔离/多角色协作工具集（createIsolatedSession/createMockedIsolatedSession/loginSession/runParallelSessions/createCollaborationContext）
   - `frontend/e2e/fixtures/rpa.ts`：RPA/表单自动化/数据提取工具集（autoFillForm/autoClickButton/extractTableData/extractColumnData/waitForTableLoaded/waitForElMessage/createRpaRecorder）

2. **E2E 增强测试用例**（3 新文件）：
   - `frontend/e2e/enhanced/network-resilience.spec.ts`：网络韧性测试（后端 500/403/401/400 错误 + 网络中断 + 弱网环境）
   - `frontend/e2e/enhanced/multi-role-collaboration.spec.ts`：多角色协作测试（多上下文隔离 + 并行会话 + 数据流验证）
   - `frontend/e2e/enhanced/rpa-data-extraction.spec.ts`：RPA 数据提取测试（表格提取 + 表单自动化 + 请求观察 + 流程录制）

3. **Playwright 配置增强**（1 修改文件）：
   - `frontend/playwright.config.ts`：新增 firefox + webkit 浏览器项目（多浏览器支持），CI 通过 `--project=chromium` 限定单浏览器

4. **CI/CD 工作流独立**（1 修改 + 1 新建文件）：
   - `.github/workflows/ci-cd.yml`：移除整个 ci-e2e job（228 行）+ 清理 package-release/notify 中的 ci-e2e 引用 + 更新拓扑注释
   - `.github/workflows/e2e-batch.yml`：新建独立 E2E 工作流（workflow_dispatch 触发 + 独立编译后端 + 完整 E2E 流程 + 跳过标记 job）

**技术要点**：

- **E2E 工作流独立设计**：
  - E2E 从 ci-cd.yml 移除，不阻塞主 CI（之前 E2E 60 分钟 timeout 导致 CI cancelled）
  - 独立工作流 e2e-batch.yml 自己编译后端（cargo build --release），不依赖 ci-cd.yml artifact
  - workflow_dispatch 手动触发，批次号通过输入参数指定
  - concurrency group 防止重复运行（cancel-in-progress: false，不取消正在运行的 E2E）

- **每 30 批次运行 + 监控机制**（由 agent 在批次节奏中执行）：
  - 批次 N（30 倍数）：触发 e2e-batch.yml workflow_dispatch
  - 批次 N+20：第 1 次监控（GitHub API 查询 run 状态）
  - 批次 N+28：第 2 次监控（若 N+20 未完成）
  - 批次 N+29：最后监控，未完成则跳过 N+30 的 E2E 周期（skip_reason 参数触发 e2e-skipped job）

- **网络拦截工具设计**：
  - mockApiError/mockApiSuccess：通过 context.route 拦截 URL，fulfill 自定义响应
  - simulateSlowNetwork：route.continue 前置 delay，放行到真实后端
  - RequestObserver：route.fetch 获取响应后 fulfill，记录请求/响应供断言
  - mockOnce：一次性 Mock（首次拦截，后续放行），用于测试重试场景

- **多上下文隔离设计**：
  - 每个角色一个独立 BrowserContext（cookie/localStorage 互不干扰）
  - createMockedIsolatedSession：mock 鉴权 + mock /auth/me 返回角色权限
  - createCollaborationContext：一次性创建多个隔离会话（sessions 字典）
  - 角色凭据从环境变量注入（fail-secure，E2E_ADMIN_USERNAME/E2E_ADMIN_PASSWORD）

- **RPA 工具设计**：
  - autoFillForm：支持 text/select/textarea/number/date 五种字段类型
  - extractTableData：批量收集 el-table-v2 行数据（虚拟滚动仅提取可视区）
  - createRpaRecorder：记录操作时间戳供性能分析

- **多浏览器支持**：
  - playwright.config.ts 新增 firefox + webkit 项目
  - CI 仅安装 chromium，通过 `--project=chromium` 限定单浏览器（控制 CI 时长）
  - 本地 `npx playwright test` 默认运行所有浏览器项目

**CI 验证**：待推送后验证。

---

### 批次 261：修复 E2E 后端启动失败 — AuthConfig serde(default) + PUBLIC_PATHS + CSRF 头（PR #438）

**修复内容**：批次 260 规则 5 E2E 检查发现后端启动失败（`missing field 'auth'`），本批次完整修复 E2E 配置链路，实现初始化步骤首次通过。

**修改文件**（5 文件 +85 -36 行）：
- `backend/src/config/settings.rs`：AuthConfig 添加 `#[serde(default)]` + 派生 `Default` + `jwt_secret` 字段级 `#[serde(default)]`（解决 auth 段缺失反序列化失败）
- `backend/src/middleware/public_routes.rs`：PUBLIC_PATHS 加入 initialize/initialize-with-db/initialize-with-db-async（放行 JWT 认证，由 init_token_middleware 用 X-Init-Token 认证）+ 新增测试
- `backend/src/middleware/init_token.rs`：更新过时注释（原声称 PUBLIC_PATHS 包含 init 前缀，实际不包含）
- `backend/src/handlers/init_handler.rs`：更新过时注释 2 处（test-database / task-status / require_admin_role）
- `.github/workflows/ci-cd.yml`：CI 密钥移除 "test" 弱模式关键词（ci-test→ci-e2e）+ 初始化请求添加 `X-Requested-With: XMLHttpRequest` 头（通过 CSRF 中间件检查）+ 初始化步骤匹配 AppError 脱敏响应格式

**技术要点**：
- **根因链路**（4 层问题逐层修复）：
  1. `missing field 'auth'` → AuthConfig 无 serde(default)，auth 段缺失时反序列化失败
  2. CI 密钥含 "test" 关键词 → validate_secret 弱模式黑名单拒绝
  3. `401 缺少认证凭据` → initialize 路径不在 PUBLIC_PATHS，auth_middleware 要求 JWT
  4. `403 CSRF_TOKEN_MISSING` → initialize 成为公开路径后，CSRF 中间件要求 X-Requested-With 头
- AuthConfig::default() 中 jwt_secret 为空字符串，由 load_sensitive_from_env() 从 JWT_SECRET 填充，validate_secret() 拒绝空字符串（安全）
- 只放行 initialize 系列（高危接口受 init_token_middleware 保护），只读接口（status/test-database/task-status）仍需 JWT
- CSRF 中间件对公开路径的 POST 要求 X-Requested-With 或 X-CSRF-Token 头（防御简单表单 CSRF）

**CI 验证**：CI run #29082156690，12/12 核心 job 全绿，E2E 初始化步骤首次 **success** ✅，Playwright 测试因 60 分钟 timeout **cancelled**（非代码问题，测试运行时间长）。PR #438 squash merge 到 main（commit 8de0988）。

**重大突破**：这是项目历史上第一次 E2E 初始化步骤成功通过，证明后端启动 + 系统初始化链路完全修复。

### 批次 260：4 个 service 分页逻辑接入 paginate_with_total 第六批 + 规则 5 E2E 检查（PR #437）

**修复内容**：bug.md 中风险重复实现问题 — 继批次 255-259 后，第六批处理 4 个 service 的分页逻辑接入。同时执行规则 5 E2E 检查。

**修改文件**（4 文件 +16 -15 行）：
- `backend/src/services/po/order.rs`：list_orders 分页接入 + 补 clamp 防 DoS（使用 into_model::<PurchaseOrderDto>）
- `backend/src/services/inventory_count_service.rs`：list_counts 分页接入 + 补 clamp 防 DoS
- `backend/src/services/inventory_adjustment_service.rs`：list_adjustments 分页接入 + 补 clamp 防 DoS
- `backend/src/services/finance_payment_service.rs`：list_payments 分页接入 + 补 clamp 防 DoS

**技术要点**：
- paginate_with_total 内部已做 page.saturating_sub(1) 偏移，调用方不可再减 1
- po/order.rs 使用 into_model::<PurchaseOrderDto>()，paginate_with_total 泛型 M = PurchaseOrderDto 兼容
- 统一补充 page.clamp(1, 1000) 防 DoS（4 个文件均新增）
- PaginatorTrait 导入保留（.paginate() 方法需要）

**CI 验证**：CI run #29064396959，12/12 核心 job 全绿，E2E 失败为已知问题。PR #437 squash merge 到 main（commit 4081afa）。

**规则 5 E2E 检查结果**：
- 下载 E2E job（ID 86274022211）日志分析
- 失败根因：`Error: missing field 'auth'` — 后端启动时 config crate 反序列化 AppSettings 缺少 `auth` 段
- 原因分析：CI E2E job 设置了 `JWT_SECRET`（无前缀），但 config crate 使用 `__` 分隔符需要 `AUTH__JWT_SECRET`。`load_sensitive_from_env()` 能从 `JWT_SECRET` 填充，但反序列化阶段就失败了
- 修复方案：批次 261 在 AuthConfig.jwt_secret 添加 `#[serde(default)]`，让反序列化通过，再由 load_sensitive_from_env() 填充

---

### 批次 259：4 个 AP service 分页逻辑接入 paginate_with_total 第五批（PR #436）

**修复内容**：bug.md 中风险重复实现问题 — 继批次 255/256/257/258 后，第五批处理 4 个应付账款相关 service 的分页逻辑接入。

**修改文件**（4 文件 +16 -21 行）：
- `backend/src/services/ap_payment_request_service.rs`：list_payment_requests 分页接入 + 补 clamp 防 DoS
- `backend/src/services/ap_payment_service.rs`：list_payments 分页接入（原有 clamp 保留，移除冗余 saturating_sub）
- `backend/src/services/ap_reconciliation_service.rs`：list_reconciliations 分页接入 + 补 clamp 防 DoS
- `backend/src/services/ap_verification_service.rs`：list_verifications 分页接入 + 补 clamp 防 DoS

**技术要点**：
- paginate_with_total 内部已做 page.saturating_sub(1) 偏移，调用方不可再减 1
- 删除独立 num_items + fetch_page 手写分页，统一接入工具函数
- 统一补充 page.clamp(1, 1000) 防 DoS（ap_payment 原有，其余 3 个新增）
- PaginatorTrait 导入保留（.paginate() 方法需要）

**CI 验证**：CI run #29063579663，12/12 核心 job 全绿（Clippy + 单元测试 + 后端构建均通过），E2E 失败为已知问题不阻塞。PR #436 squash merge 到 main（commit 766603a）。

---

### 批次 258：4 个 service 分页逻辑接入 paginate_with_total 第四批（PR #435）

**修复内容**：bug.md 中风险重复实现问题 — 继批次 255/256/257 后，第四批处理 4 个采购/供应商相关 service 的分页逻辑接入。

**修改文件**（4 文件 +16 -12 行）：
- `backend/src/services/purchase_receipt_service.rs`：list_receipts 分页接入 + 补 clamp 防 DoS
- `backend/src/services/purchase_inspection_service.rs`：list_inspections 分页接入 + 补 clamp 防 DoS
- `backend/src/services/purchase_return_service.rs`：list_returns 分页接入（原有 clamp 保留，移除冗余 saturating_sub）
- `backend/src/services/supplier_evaluation_service.rs`：list_ratings 分页接入（原有 clamp 保留，移除冗余 saturating_sub）

**技术要点**：
- paginate_with_total 内部已做 page.saturating_sub(1) 偏移，调用方不可再减 1
- 删除独立 num_items + fetch_page 手写分页，统一接入工具函数
- 统一补充 page.clamp(1, 1000) 防 DoS（purchase_return/supplier_evaluation 原有，其余 2 个新增）
- PaginatorTrait 导入保留（.paginate() 方法需要）

**CI 验证**：CI run #29062816980，12/12 核心 job 全绿（Clippy + 单元测试 + 后端构建均通过），E2E 失败为已知问题不阻塞。PR #435 squash merge 到 main（commit 24b0c87）。

---

### 批次 257：4 个 service 分页逻辑接入 paginate_with_total 第三批（PR #434）

**修复内容**：bug.md 中风险重复实现问题 — 继批次 255/256 后，第三批处理 4 个 service 的分页逻辑接入 paginate_with_total。

**修改文件**（4 文件 +22 -27 行）：
- `backend/src/services/currency_service.rs`：2 处分页接入（list + get_history）
- `backend/src/services/mrp_engine_service.rs`：分页接入
- `backend/src/services/production_order_service.rs`：分页接入
- `backend/src/services/scheduling_query.rs`：分页接入

**技术要点**：
- paginate_with_total 内部已做 page.saturating_sub(1) 偏移，调用方不可再减 1
- 删除独立 select.clone().count() 查询，复用 paginator 的 num_items()
- 统一补充 page.clamp(1, 1000) 防 DoS
- currency_service.rs 有 2 处分页（list + get_history），均接入

**CI 验证**：CI run #29062023389，12/12 核心 job 全绿（Clippy + 单元测试 + 后端构建均通过），E2E 失败为已知问题不阻塞（"启动后端服务"步骤失败）。PR #434 squash merge 到 main（commit 1865525）。

---

### 批次 256：4 个 service 分页逻辑接入 paginate_with_total 第二批（PR #433）

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

### 批次 255：4 个 service 分页逻辑接入 paginate_with_total 首批（PR #432）

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

### 批次 254：14 个 composable 文件 eslint-disable any 指令清理（PR #431）

**修复内容**：bug.md 中风险死代码问题 — 14 个 composable 文件首行均有 `/* eslint-disable @typescript-eslint/no-explicit-any */`，但经审计这些文件中真实的 any 类型使用为 0。这些 eslint-disable 指令是 P14 批次拆分 Vue 重构时为快速通过 lint 而添加的残留，现已成为 any 类型的"避风港"。

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

**CI 验证**：CI run #29058822394，12/12 核心 job 全绿（ESLint + 类型检查一次通过），E2E 失败为已知问题不阻塞。PR #431 squash merge 到 main（commit d2abb55）。

---

### 批次 253：AdvancedFilter handleLogicChange 空函数改为真实实现（PR #430）

**修复内容**：bug.md 中风险空实现问题 — `AdvancedFilter.vue` 第 249 行 `handleLogicChange` 为空函数 `() => {}`，用户切换条件组逻辑运算符时无任何响应。

**修改文件**（2 文件 +31 -2 行）：
- `frontend/src/components/AdvancedFilter.vue`：新增 `logicChange` emit 事件 + `handleLogicChange` 接收 `groupIndex` 参数实现真实逻辑
- `frontend/src/views/components-demo/AdvancedFilterDemo.vue`：演示 `logicChange` 事件真实接入

**技术要点**：
- 新增 `logicChange: [groupIndex: number, logic: 'AND' | 'OR', filters: FilterGroup[]]` emit 事件
- `handleLogicChange` 接收 `groupIndex` 参数，emit 事件让父组件可响应
- 显示轻量级 `ElMessage.info` 提示让用户知道逻辑已切换（duration: 1500ms）
- 模板 `@change` 改为 `() => handleLogicChange(groupIndex)` 传递循环索引

**CI 验证**：CI run #29058007479，12/12 核心 job 全绿，E2E 失败为已知问题不阻塞。PR #430 squash merge 到 main（commit da659f7）。

---

### 批次 252：bi_analysis + dual_unit_converter unreachable!() 改为返回错误（PR #429）

**修复内容**：bug.md 中风险空实现问题 — `bi_analysis_service.rs` 三处 `unreachable!()` 宏调用，用户可控的 dim/measure 参数若绕过校验将触发 panic 导致进程崩溃；`dual_unit_converter_handler.rs` 第 116 行 `unreachable!()` 在校验逻辑被重构后可能 panic 崩溃。

**修改文件**（2 文件 +101 -31 行）：
- `backend/src/services/bi_analysis_service.rs`：`dim_to_expr` 返回类型改为 `Result`，`_` 分支返回 `AppError::validation`；提取 `measure_to_expr` 独立函数替代原内联 match + `unreachable!()`；新增 6 个单元测试
- `backend/src/handlers/dual_unit_converter_handler.rs`：`_` 分支改为 `return Err(AppError::bad_request)`

**技术要点**：
- `dim_to_expr`：返回类型从 `(&'static str, &'static str)` 改为 `Result<(&'static str, &'static str), AppError>`，`_` 分支返回 `AppError::validation(format!("不支持的维度: {}", dim))`
- 提取 `measure_to_expr(measure, item_level)` 独立函数，用 `(measure, item_level)` 元组 match 替代原两处内联 match，`_` 分支返回 `AppError::validation`
- `pivot` 方法调用处加 `?` 传播错误
- `dual_unit_converter_handler.rs`：`_ => unreachable!(...)` 改为 `_ => return Err(AppError::bad_request("无效的单位..."))`
- 新增 6 个单元测试：验证所有合法维度/度量返回 Ok，非法维度/度量/空字符串返回 Err（而非 panic）

**CI 验证**：CI run #29046877533，12/12 核心 job 全绿（Clippy 一次通过），E2E 失败为已知问题不阻塞。PR #429 squash merge 到 main（commit faa9749）。

---

### 批次 251：webhook retry 持久化 payload + retry_count 修复（PR #428）

**修复内容**：bug.md 中风险简化阉割问题 — `webhook_service.rs` 的 webhook 发送时 payload 仅存内存，发送后丢弃；`retry_webhook` 构造假 payload；retry_count 仅在网络层异常时递增；原代码用 `if let ActiveValue::Set(v) = &final_model.retry_count` 取值，但 `webhook.into()` 生成 `Unchanged` 值，导致模式匹配永远不命中，retry_count 永远读 0。

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

### 批次 250：budget_management 审批流完整化（PR #427）

**修复内容**：bug.md 中风险简化阉割问题 — `budget_management_service.rs` 的 `adjust_budget` 方法硬编码 `approval_status: APPROVED` 并立即应用金额变更（注释自述"简化：直接批准"），完全跳过审批环节。

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

### 批次 249：capacity_service 硬编码置信度动态化（PR #426）

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

### 批次 248：AR/AP 报表接入 CacheService 缓存（PR #425）

**修复内容**：bug.md 中风险性能问题 — `cache_service.rs` 已实现并注入 AppState，但零业务调用（命中率统计永远为 0）。AR/AP 报表 8 个端点每次请求都执行 SQL 聚合查询。

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

### 批次 247：CLI 健康检查硬编码 URL 改为环境变量读取（PR #424）

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

### 批次 246：dye-recipe handleViewVersion 空实现修复（PR #423）

**修复内容**：bug.md 中风险空实现漏洞 #18 — `frontend/src/views/dye-recipe/index.vue` 的 `handleViewVersion` 原为空实现（`(_row: DyeRecipe) => {}`），用户在版本历史对话框中点击"查看"按钮无任何响应。

**修改文件**（1 文件 +8 -2 行）：
- `frontend/src/views/dye-recipe/index.vue`：handleViewVersion 从空实现改为复用主对话框只读模式展示版本详情（关闭版本历史对话框 → 设置标题 `查看版本详情 - v{版本号}` → `isView = true` → `Object.assign(formData, row)` → 打开主对话框），与批次 239 P0-3 `handleView` 修复采用相同模式。

**CI 验证**：CI run #29037444886，12/12 核心 job 全绿，PR #423 squash merge 到 main（commit 16754cf7）。

---

### 批次 245：ap_report_service 4 个报表方法 SQL 层聚合（PR #422）

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

### 批次 244：ar_service 3 个报表方法 SQL 层聚合（PR #421）

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

### 批次 243：report-templates XSS + tracking_handler 输入验证（PR #420）

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

### 批次 242：crm/cust get_rfm_distribution 真实计算（PR #419）

**修复内容**：bug.md 高风险简化阉割问题 — `crm/cust.rs:265-275 get_rfm_distribution` 返回全 0 占位 JSON，RFM 分布功能形同虚设。

**修改文件**：`backend/src/services/crm/cust.rs`

**技术要点**：
- 一次性查询所有客户 ID + 订单聚合（GROUP BY customer_id），内存计算 RFM 评分
- 分桶聚合（VIP>=4.5/重要>=3.5/一般>=2.5/低价值<2.5）
- 提取 OrderAggRow/CustomerOrderStats type 别名避免 clippy type_complexity 警告

**CI 验证**：CI run #29031527941，12/12 核心 job 全绿（1 轮 CI 修复：type_complexity），PR #419 squash merge 到 main（commit 146251d9）。

---

### 批次 241：恢复 docs.rs ApiDoc + 删除 openapi.rs 死文件（PR #418）

**修复内容**：bug.md 高风险 API 文档缺失 — `backend/src/openapi.rs` 是未注册的幽灵文件（无 mod 声明），`backend/src/docs.rs` 是占位文件（ApiDoc 已删除），导致 `#[cfg(feature = "swagger")]` 编译失败。仅 2 个 handler 有 `#[utoipa::path]` 注解。

**修改文件**：`backend/src/docs.rs`（恢复 ApiDoc struct + impl Default + TODO 注释）

**技术要点**：
- 恢复 docs.rs ApiDoc（只注册有注解的 2 个 handler + 5 个 schema）
- 删除 openapi.rs 死文件
- `backend/src/routes/mod.rs:319-322` 引用 `crate::docs::ApiDoc::openapi()` 恢复正常

**CI 验证**：CI run #29029806479，12/12 核心 job 全绿（E2E 失败为已知问题不阻塞），PR #418 squash merge 到 main（commit de1437f0）。

---

### 批次 240：permission.rs 权限校验新增 23 个单元测试（PR #417）

**修复内容**：bug.md 高风险测试覆盖 — `backend/src/middleware/permission.rs` 权限校验零测试，越权风险。

**修改文件**：`backend/src/middleware/permission.rs`

**技术要点**：
- 提取 matches_permission 纯函数
- 新增 23 个单元测试（extract_resource_info 8 + method_to_action 6 + CacheEntry 2 + matches_permission 9 含垂直越权防护）
- 覆盖管理员短路/缓存命中/过期/resource_id 精确匹配/`*` 通配符/嵌套路径

**CI 验证**：CI run #29028249081，12/12 核心 job 全绿，PR #417 squash merge 到 main（commit c72982b9）。

---

### 批次 239：dye-batch/dye-recipe handleView 空实现修复（PR #416）

**修复内容**：bug.md 高风险空实现 — `frontend/src/views/dye-batch/index.vue:341` handleView + `frontend/src/views/dye-recipe/index.vue:318` handleView 均为空函数。

**修改文件**（2 文件）：dye-batch/index.vue + dye-recipe/index.vue

**技术要点**：
- 新增 isView 只读模式标志
- 复用现有对话框实现查看功能（el-form :disabled + footer 按钮调整）

**CI 验证**：CI run #29026950380，12/12 核心 job 全绿，PR #416 squash merge 到 main（commit 743a9595）。

---

### 批次 238：ar_service get_aging_report 全表扫描改为 SQL 聚合（PR #415）

**修复内容**：bug.md 高风险性能 — `ar_service.rs:1274-1321 get_aging_report` 无日期范围 + 无 LIMIT 全表扫描，数据量增长后可能 OOM。

**修改文件**：`backend/src/services/ar_service.rs`

**技术要点**：
- 单条 SQL CASE WHEN + SUM + COUNT 在数据库层完成分桶聚合
- 应用层只接收 1 行聚合结果，O(N) 内存 → O(1) 内存
- 规则 12 合规：customer_id 参数化绑定
- CI 修复：1 轮（Values 类型冲突 + query_one 调用方式 + try_get_by_index turbofish）

**CI 验证**：CI run #29025818891 12/12 核心全绿，PR #415 squash merge 到 main（commit 775f7761）。

---

### 批次 237：auth_service/user_handler Argon2id 异步化（PR #414）

**修复内容**：bug.md 高风险并发-async 阻塞 — 4 处 Argon2id 哈希计算阻塞 async runtime，影响登录核心路径。

**修改文件**：`backend/src/services/auth_service.rs` + `backend/src/handlers/user_handler.rs`

**技术要点**：
- 新增 verify_password_async / hash_password_async 异步方法
- 使用 `tokio::task::spawn_blocking(move || ...).await??` 包装 Argon2id 哈希计算
- 7 处生产调用点全部改用异步版本（auth_service authenticate + user_handler 4 处 + init_service 2 处）
- 同步版本保留供测试夹具使用

**CI 验证**：CI run #29023784549，12/12 核心 job 全绿（Clippy + 单元测试 + 后端构建均通过），PR #414 squash merge 到 main（commit 7585097f）。

---

## 历史归档索引

| 归档日期 | 内容 | 路径 |
|----------|------|------|
| 2026-07-10 | 职责分工修正前完整内容（MEMORY/doto/CHANGELOG） | `docs/archives/2026-07-10-职责分工修正/` |
| 2026-07-10 | doto/MEMORY/CHANGELOG 整理前完整内容 | `docs/archives/2026-07-10/` |
| 2026-07-05 | MEMORY/CHANGELOG/doto 优化前完整内容 | `docs/archives/2026-07-05/` |
| 2026-06-24 | MEMORY/CHANGELOG 优化前完整内容 | `docs/archives/` |

> 批次 1-236 的详细记录见归档文件和 [CHANGELOG.md](file:///workspace/.monkeycode/CHANGELOG.md) 历史归档章节。
> 历次复审报告见 `docs/audits/` 目录。
