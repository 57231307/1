# 任务与历史

> 本文件记录**当前任务**与**历史任务索引**。
> 详细历史请查阅 [`.monkeycode/docs/archives/`](file:///workspace/.monkeycode/docs/archives/)。

---

## 🔄 当前任务：v14 深度调研报告修复（高风险 6 项全部完成，中风险 25 项进行中，已完成 11/25）

> **v14 深度调研报告已生成**（2026-07-09，[bug.md](file:///workspace/.monkeycode/bug.md)）：12 维度全量扫描，15 高/25 中/74 低风险，共 114 个问题。
> v13 后端 P0/P1 全部完成（批次 229-236），v13 剩余 P2 任务合并到 v14 队列。
> 用户指令启动 v14 修复流程，按优先级（高→中→低）+ 影响范围（核心路径→边缘功能）排序。
> **批次 237 已完成**：v14 P0-1 并发 async 阻塞修复（spawn_blocking 包装 Argon2id 哈希），PR #414 squash merge 到 main（commit 7585097f），分支已清理。CI 12/12 核心全绿。
> **批次 238 已完成**：v14 P0-2 性能-全表扫描修复（ar_service get_aging_report SQL 聚合），PR #415 squash merge 到 main（commit 775f7761），分支已清理。CI 12/12 核心全绿（1 轮 CI 修复：Values 类型冲突 + try_get_by_index turbofish）。
> **批次 239 已完成**：v14 P0-3 空实现-业务失效修复（dye-batch/dye-recipe handleView isView 只读模式），PR #416 squash merge 到 main（commit 743a9595），分支已清理。CI 12/12 核心全绿。
> **批次 240 已完成**：v14 P0-4 测试覆盖-安全核心修复（permission.rs 提取 matches_permission 纯函数 + 23 个单元测试），PR #417 squash merge 到 main（commit c72982b9），分支已清理。CI 12/12 核心全绿。
> **批次 241 已完成**：v14 P0-5 API 文档缺失修复（恢复 docs.rs ApiDoc + 删除 openapi.rs 死文件），PR #418 squash merge 到 main（commit de1437f0），分支已清理。CI 12/12 核心全绿（E2E 失败为已知问题不阻塞）。
> **批次 242 已完成**：v14 P0-6 RFM 分布简化阉割永久修复（真实批量计算所有客户 RFM 评分），PR #419 squash merge 到 main（commit 146251d9），分支已清理。CI 12/12 核心全绿（1 轮 CI 修复：type_complexity 警告提取 type 别名）。
> **v14 高风险 6 项全部完成**（P0-1 到 P0-6）。
> **批次 243 已完成**：v14 中风险安全漏洞修复（report-templates XSS + tracking_handler 输入验证），PR #420 squash merge 到 main（commit 0810fe3），分支已清理。CI 12/12 核心全绿（E2E 失败为已知问题不阻塞）。中风险 25 项已完成 1 项（安全漏洞 2 项）。
> **批次 244 已完成**：v14 中风险性能修复 — ar_service 3 个报表 SQL 聚合（get_statistics_report + get_daily_report + get_monthly_report），PR #421 squash merge 到 main（commit dcd8488d），分支已清理。CI 12/12 核心全绿（1 轮 CI 修复：clippy param_idx 未使用赋值警告）。中风险 25 项已完成 2 项（安全漏洞 2 项 + ar 报表性能 3 处）。
> **批次 245 已完成**：v14 中风险性能修复 — ap_report_service 4 方法 SQL 聚合（get_statistics_report + get_daily_report + get_monthly_report + get_aging_report），PR #422 squash merge 到 main（commit ae7d4619），分支已清理。CI 12/12 核心全绿（1 轮 CI 修复：clippy supplier_id.unwrap 警告 → 改用 supplier_id.map(|sid|) 模式）。中风险 25 项已完成 3 项（安全漏洞 2 项 + ar/ap 报表性能 7 处）。
> **批次 246 已完成**：v14 中风险空实现修复 — dye-recipe handleViewVersion（原空实现，复用主对话框只读模式展示版本详情），PR #423 squash merge 到 main（commit 16754cf7），分支已清理。CI 12/12 核心全绿。中风险 25 项已完成 4 项。
> **批次 247 已完成**：v14 中风险硬编码 URL 修复 — CLI 健康检查（cli/util/service.rs 从环境变量 SERVER__HOST/SERVER__PORT 读取，默认 127.0.0.1:8082），PR #424 squash merge 到 main（commit 47d86d86），分支已清理。CI 12/12 核心全绿。中风险 25 项已完成 5 项。
> **批次 248 已完成**：v14 中风险缓存未利用修复 — AR/AP 报表 8 端点接入 CacheService（TTL 60s，命中缓存跳过 SQL 查询），PR #425 squash merge 到 main（commit 53ce6b53），分支已清理。CI 12/12 核心全绿（1 轮 CI 修复：Option 类型用 {:?} 格式化）。中风险 25 项已完成 6 项。
> **批次 249 已完成**：v14 中风险简化阉割修复 — capacity_service forecast_capacity 硬编码 confidence: 0.8 改为动态计算（基于历史订单数量+当前负荷数据+预测期限三维），PR #426 squash merge 到 main（commit 82269a4），分支已清理。CI 12/12 核心全绿（1 轮 CI 修复：f64 类型标注消除 clamp 歧义）。中风险 25 项已完成 7 项。
> **批次 250 已完成**：v14 中风险简化阉割修复 — budget_management adjust_budget 硬编码 APPROVED 改为完整审批闭环（PENDING→APPROVED/REJECTED），新增 approve_adjustment/reject_adjustment/reject_plan 方法 + 3 条路由 + 前端 API，PR #427 squash merge 到 main（commit b2520cd），分支已清理。CI 12/12 核心全绿。中风险 25 项已完成 8 项。
> **批次 251 已完成**：v14 中风险简化阉割修复 — webhook retry 未持久化 payload（原 retry 构造假 payload + retry_count 值提取 bug 永远读 0），新增迁移 m0047（webhooks 表加 last_payload + last_event 列）+ trigger_webhook 发送前持久化原始 payload/event + retry_webhook 从持久化存储重投 + retry_count 对 HTTP 业务失败也计数/成功重置 0，PR #428 squash merge 到 main（commit 226af53），分支已清理。CI 12/12 核心全绿（Clippy 一次通过）。中风险 25 项已完成 9 项（简化阉割 3 项全部完成）。
> **批次 252 已完成**：v14 中风险空实现修复 — bi_analysis_service 3 处 unreachable!() + dual_unit_converter_handler 1 处 unreachable!() 改为返回 AppError 错误（dim_to_expr 返回 Result + 提取 measure_to_expr 独立函数 + handler 改 return Err），新增 6 个单元测试验证非法输入返回错误而非 panic，PR #429 squash merge 到 main（commit faa9749），分支已清理。CI 12/12 核心全绿（Clippy 一次通过）。中风险 25 项已完成 10 项。
> **批次 253 已完成**：v14 中风险空实现修复 — AdvancedFilter.vue handleLogicChange 空函数改为真实实现（新增 logicChange emit 事件 + 接收 groupIndex 参数 emit 事件让父组件可响应 + 显示轻量级提示 + Demo 页面演示真实接入），PR #430 squash merge 到 main（commit da659f7），分支已清理。CI 12/12 核心全绿。中风险 25 项已完成 11 项（空实现 4 项全部完成）。

> 用户最高优先级规则（2026-07-04/06/08 追加）已固化到 [MEMORY.md 一、规则 0-12](file:///workspace/.monkeycode/MEMORY.md)。
> 本文件仅记录任务进度，规则不在此重复。
> 规则 10 梳理时间：2026-07-09（批次 236 提前触发，用户明确要求"梳理项目的所有记忆"）

### v14 修复任务队列（按优先级排序）

#### 🔴 高风险修复队列（15 项，批次 237+ 优先处理）

| 批次 | 编号 | 问题 | 文件 | 修复方案 | 状态 |
|------|------|------|------|----------|------|
| 237 | P0-1 | 并发-async 阻塞（4 处高，最高优先级） | auth_service.rs:107/243/277 + user_handler.rs:196/538/563/578 | spawn_blocking 包装 Argon2id 哈希 | ✅ 已完成（PR #414, commit 7585097f, CI 12/12 核心全绿） |
| 238 | P0-2 | 性能-全表扫描（1 处高） | ar_service.rs:1274-1321 get_aging_report | SQL CASE WHEN + SUM + COUNT 聚合 | ✅ 已完成（PR #415, commit 775f7761, CI 12/12 核心全绿, 1 轮 CI 修复） |
| 239 | P0-3 | 空实现-业务失效（2 处高） | dye-batch/index.vue:341 + dye-recipe/index.vue:318 handleView | 新增 isView 只读模式，复用对话框 | ✅ 已完成（PR #416, commit 743a9595, CI 12/12 核心全绿） |
| 240 | P0-4 | 测试覆盖-安全核心（1 处高） | middleware/permission.rs 全文件零测试 | 提取 matches_permission + 23 个测试 | ✅ 已完成（PR #417, commit c72982b9, CI 12/12 核心全绿） |
| 241 | P0-5 | API 文档缺失（2 处高） | openapi.rs 死文件 + docs.rs 占位文件 | 恢复 docs.rs ApiDoc + 删除 openapi.rs 死文件 | ✅ 已完成（PR #418, commit de1437f0, CI 12/12 核心全绿） |
| 242 | P0-6 | 简化阉割-永久（1 处高） | crm/cust.rs:265-275 get_rfm_distribution | 真实计算 RFM 分布 | ✅ 已完成（PR #419, commit 146251d9, CI 12/12 核心全绿, 1 轮 CI 修复） |

#### 🟡 中风险修复队列（25 项，高风险完成后启动）

- **测试覆盖（7 项）**：handlers 100+ 文件覆盖率 10%、services 107 个无测试、frontend api 4.4%、ai 算法（pred/detect/rec）零测试、frontend store 多数无测试、middleware 多个零测试
- **空实现（4 项，全部完成 ✅）**：✅ dye-recipe handleViewVersion（批次 246 完成）、✅ bi_analysis unreachable! panic + dual_unit_converter unreachable（批次 252 完成，4 处 unreachable! 全部改为返回 AppError 错误）、✅ AdvancedFilter handleLogicChange（批次 253 完成，新增 logicChange emit 事件）
- **简化阉割（3 项，全部完成 ✅）**：✅ capacity_service 硬编码置信度 0.8（批次 249 完成）、✅ budget_management 跳过审批流（批次 250 完成）、✅ webhook retry 未持久化 payload（批次 251 完成）
- **死代码（1 项）**：14 个 composable 文件 eslint-disable any
- **重复实现（2 项）**：20 个 service 分页逻辑重复（应接入 paginate_with_total）、30+ view 表格逻辑重复（应接入 useTableApi）
- **项目规则符合性（1 项）**：cli/util/service.rs 硬编码健康检查 URL ✅ 批次 247 完成（PR #424, commit 47d86d86, CI 12/12 核心全绿）
- **性能问题（5 项）**：ar 报表 4 处未分页（3 处已修复）+ ap_report_service 4 方法未分页（已修复）+ 缓存未利用（已修复）
  - ✅ 批次 244：ar_service get_statistics_report + get_daily_report + get_monthly_report SQL 聚合（PR #421, commit dcd8488d, CI 12/12 核心全绿）
  - ✅ 批次 245：ap_report_service 4 方法 SQL 聚合（PR #422, commit ae7d4619, CI 12/12 核心全绿）
  - ✅ 批次 248：AR/AP 报表 8 端点接入 CacheService 缓存（PR #425, commit 53ce6b53, CI 12/12 核心全绿）
- **安全漏洞（2 项）**：report-templates XSS 潜在、tracking_handler 输入验证缺失 ✅ 批次 243 完成（PR #420, commit 0810fe3, CI 12/12 核心全绿）

#### 🟢 低风险修复队列（74 项，后续迭代）

- 占位符/Mock 存根（21 项，全部合理设计或测试夹具，多数无需修复）
- 项目规则符合性（11 项，多为配置层默认值或 best-effort 合理模式）
- 死代码（8 项，均合规标注）
- 其他（34 项）

#### 📋 合并到 v14 的历史遗留任务

- ⏳ v13 前端 P2：FE-P2-1（deepClone 接入）、FE-P2-2（删死代码）、FE-P2-3（非空断言）→ 合并到中风险
- ⏳ v13 后端 P2：P2-1/2/3 → 合并到中风险
- ⏳ FE-P2-3：i18n 覆盖率（200+ 视图硬编码中文，巨大工作量，后续迭代）
- ⏳ FE-P2-6：大列表虚拟化（966 处 el-table，巨大工作量，后续迭代）
- ⏳ P2-8 剩余 143 个无测试 service（非高优先级，后续迭代）
- ⏳ E2E 失败排查（连续多次"启动后端服务"失败，已知问题，待规则 5 节点）
- ⏳ 规则 10 触发：批次 240（=16×15）需梳理记忆文件

### v12 复审 P0/P1 修复进度（全部完成）

- ✅ P0-1：warehouse_handler update_location 欺骗性 stub（批次 197）
- ✅ P0-2：password_policy_service 安全功能未接入登录流程（批次 198）
- ✅ P1-1：4 个审批模型死代码清理（批次 201）
- ✅ P1-2：audit_alert_rule 模型死代码清理（批次 202）
- ✅ P1-3：dto/mod.rs 文件级 #![allow(dead_code)] 违规（批次 202）
- ✅ P1-4：BOM 创建 N+1 INSERT（批次 203）
- ✅ P1-5：报价单创建 N+1 INSERT（批次 203）
- ✅ P1-6：4 个 handler 丢弃请求体（批次 199）
- ✅ 前端 P1-1：CSV 导出违规（批次 204）
- ✅ 前端 P1-2：useApiLog any（批次 204）
- ✅ 前端 P1-3：Promise rejection（批次 204-ci）
- ✅ 前端 P1-4：WebSocket token 安全修复（批次 205）
- ✅ 前端 P1-5：搜索 Bug（批次 204）

### v12 复审 P2 修复进度（进行中）

- ✅ P2-6：supplier_service 事务保护缺失（批次 205 → fb05961，CI 类型错误 → 批次 206 修复）
- ✅ P2-2：report/ds.rs 桩代码补全（批次 206，aggregate_purchase_data / aggregate_finance_data / query_purchase_report）
- ✅ P2-4：unwrap/expect 加固（批次 207，5 处非测试代码：4 处加不变量注释 + 1 处改 fail-secure 模式 + Clippy unused imports 修复）
- ✅ P2-5：硬编码状态字符串替换为 status::* 常量（批次 208-212 全部完成，services 目录无残留硬编码 "active"/"inactive"）
  - 批次 208：4 个主数据 service 18 处 + master_data 子模块 ✅
  - 批次 209：3 个 service 15 处 + budget 子模块 ✅
  - 批次 210：4 个 service 16 处 + contract 子模块 ✅
  - 批次 210-ci：Clippy 修复 ✅
  - 批次 211：6 个 service 12 处 ✅ CI 12/12 核心全绿
  - 批次 212：11 个 service 11 处 ✅ CI Clippy 失败 → 212-ci 修复（recipe_opt.rs master_data 移入 #[cfg(test)]）✅
  - 批次 213：3 个 handler 10 处（product_handler + inventory_stock_handler + api_gateway_handler 8 处）✅ CI 12/12 核心全绿
  - **P2-5 后端全面完成**：services 22 个文件 + handlers 3 个文件，共 82 处替换 + 3 个新子模块（master_data/budget/contract）
- 🔄 P2-1：6 项级 #[allow(dead_code)] 状态常量接入业务（批次 214-216 全部完成）
  - 批次 208 已移除 SUBMITTED 的 allow 标注 ✅
  - 批次 214：删除 RECEIVED（冗余）+ FULFILLED→CONSUMED（值修正）+ 新增 LOCKED/RELEASED + 新增 purchase_receipt 子模块 + 10 处硬编码替换 ✅
  - 批次 215：实现采购订单 cancel_order 功能（service+handler+route）✅ 移除 purchase_order::CANCELLED 的 #[allow(dead_code)]
    - po/contract.rs cancel_order 方法：事务+lock_exclusive+状态校验+预算释放（事务内插入反向"调整"冲销记录）
    - 允许取消状态：DRAFT/PENDING_APPROVAL/APPROVED/PARTIAL_RECEIVED
    - purchase_order_handler.rs cancel_order handler + CancelOrderRequest DTO
    - routes/purchase.rs POST /orders/:id/cancel
  - 批次 216：实现销售发货 cancel_delivery 功能（service+handler+route）✅ 移除 sales_delivery::CANCELLED 的 #[allow(dead_code)]
    - so/delivery.rs cancel_delivery 方法：事务+lock_exclusive+状态校验+库存恢复+预留恢复+订单状态回退
    - restore_inventory 辅助方法：对称反向（quantity_available += qty，quantity_shipped -= qty）
    - 预留恢复：CONSUMED→PENDING，订单明细 shipped_quantity 回退
    - 订单状态回退：全部发货取消 SHIPPED→APPROVED，部分取消 SHIPPED→PARTIAL_SHIPPED
    - sales_order_handler.rs cancel_delivery handler + CancelDeliveryRequest DTO
    - routes/sales.rs POST /orders/:id/deliveries/:delivery_id/cancel
- ⏳ P2-3：N+1 写（与 P2-6 同处，已修复）
- ✅ P2-7：API 一致性 100% 完成（批次 217 改造 failover_handler.rs get_failover_status，CI 12/12 核心全绿）
  - 审计结果：~420 个 handler 中仅 1 个未用 ApiResponse，20 个合理例外（文件下载 13 + 健康检查 3 + Prometheus 1 + Auth Cookie 3 语义已包装）
- 🔄 P2-8：测试覆盖审计完成（181 个 service 文件，36 个有测试，143 个无测试，覆盖率 19.9%）
  - 高优先级 Top 15：po/order.rs、so/order.rs、inv/stock.rs、inv/adjust.rs、inventory_reservation_service.rs、mrp_engine_service.rs、voucher_service.rs、ar/recon.rs、ar/vfy.rs、ap_reconciliation_service.rs、accounting_period_service.rs、customer_credit_limit.rs、production_order_service.rs、bom_service.rs、so/order_workflow.rs
  - 5 个模块 0% 覆盖：po/、crm/、inv/、report/
  - 批次 221：customer_credit_limit.rs 19 个测试 ✅ CI 12/12 核心全绿
  - 批次 222：accounting_period_service.rs 14 个测试 ✅
  - 批次 223：po/order.rs 19 个 + inventory_reservation_service.rs 17 个 = 36 个测试 ✅ CI 12/12 核心全绿
  - 批次 224：inv/stock.rs 15 个 + so/order_workflow.rs 17 个 = 32 个测试 ✅ CI 12/12 核心全绿（E2E 失败已知不阻塞）
  - 批次 225：bom_service.rs 24 个 + ar/recon.rs 22 个 = 46 个测试（CI 被 226 取消，226 验证）
  - 批次 226：voucher_service.rs 29 个 + ap_reconciliation_service.rs 30 个 = 59 个测试（CI 被 227 取消，227 验证）
  - 批次 227：production_order_service.rs 55 个 + mrp_engine_service.rs 25 个 = 80 个测试（CI 被 228 取消，228 验证）
  - 批次 228：so/delivery.rs 25 个 + ar/vfy.rs 31 个 = 56 个测试 ✅ CI 12/12 核心全绿（验证批次 225-228 全部 342 个测试）
  - **14 个高优先级 service 共 342 个测试全部完成，CI 12/12 核心全绿**，inv/adjust.rs 为占位模块无代码无需测试
  - 重要发现：ar/vfy.rs 中 reconciliation_status 使用小写值但 status::ar 模块定义大写值，存在不一致（留待后续修复）
- 🔄 前端 P2 复审完成（6 个高优先级修复项）：
  - FE-P2-1：全局错误处理缺失（main.ts 未注册 errorHandler + unhandledrejection）
  - FE-P2-2：非空断言滥用（66 处 res.data! 分布在 32 文件）
  - FE-P2-3：i18n 覆盖率极低（200+ 视图硬编码中文，仅 12 文件接入 $t）
  - FE-P2-4：敏感信息存 localStorage（CompanyTab.vue 企业税号/银行账号）
  - FE-P2-5：深拷贝代码重复（9 文件 JSON.parse(JSON.stringify(...))）✅ 批次 218 修复
  - FE-P2-6：大列表未虚拟化（966 处 el-table 仅 1 处用 V2Table）

### 前端 P2 修复进度
- ✅ FE-P2-1：全局错误处理（批次 218 main.ts 注册 errorHandler + unhandledrejection）
- ✅ FE-P2-2：非空断言（批次 219-220，store 15 + composables/views 45 = 60 处全部修复）
- ✅ FE-P2-4：敏感信息 localStorage（批次 218 CompanyTab.vue 过滤敏感字段）
- ✅ FE-P2-5：深拷贝工具（批次 218 utils/index.ts deepClone + 12 文件 28 处替换）
- ⏳ FE-P2-3：i18n 覆盖率（200+ 视图硬编码中文，巨大工作量，后续迭代）
- ⏳ FE-P2-6：大列表虚拟化（966 处 el-table，巨大工作量，后续迭代）

### v13 复审修复进度（后端 P0/P1 全部完成 ✅）

- ✅ P0-1：warehouse_handler update_location 欺骗性 stub（批次 229）
- ✅ P1-1：硬编码状态字符串替换（批次 231-234，25 个业务域全覆盖）
  - 批次 231：5 个核心业务域（ar/ap/inventory/po/so）状态常量提取 + 替换
  - 批次 232：5 个业务域（sales/purchase/inventory_movement/production/warehouse）替换
  - 批次 233：8 个业务域（customer/supplier/product/bom/quality/contract/budget/cost）替换
  - 批次 234：7 个业务域（payment/ar_recon/accounting/fixed_asset/tax/payroll/crm）替换 + CI 修复
- ✅ P1-2：inventory_piece 状态字段约定定义 + RESERVED 常量补全（批次 235，含 barcode_scanner_handler Expr::cust 借用修复）
- ✅ P1-3：N+1 查询/写入重构（批次 236，4 处 INSERT 批量化）
  - `backend/src/services/ar_service.rs` auto_verify：明细 INSERT 批量化（N×M → 1）+ 发票 UPDATE 去重推迟（N×M → N×唯一发票数）
  - `backend/src/services/ar/vfy.rs` auto_match：8 个 INSERT 点批量化（N×M → 1）
  - `backend/src/services/so/delivery.rs` ship_order：发货明细 INSERT 批量化（N → 1）
  - `backend/src/services/so/delivery.rs` lock_inventory：预留记录 INSERT 批量化（N → 1）
  - 评估后保持现状：`po/receipt.rs` receive_order（乐观锁语义无法批量化）、`so/delivery.rs` cancel_delivery（每个明细 product_id 不同，需 CASE WHEN）
  - CI run #29019444093：12/12 核心 job 全绿（Clippy 通过是关键信号），E2E queued 不阻塞
  - PR #413 squash merge 到 main（commit eaa5c9b3），分支 fix/batch236-v13-p1-3-n1-refactor 已删除
- ⏳ P2-1/2/3：后端 P2 修复（待用户指令）
- ⏳ FE-P2-1/2/3：前端 P2 修复（待用户指令）
- ⏳ E2E 失败排查：连续多次"启动后端服务"失败（已知问题，非代码质量）

### v11 复审摘要（已全部完成 ✅，详细记录见 CHANGELOG.md）

- **复审报告**：`docs/audits/2026-07-06-reaudit-v11-backend.md`（后端 47 项）+ `docs/audits/2026-07-06-reaudit-v11-frontend.md`（前端 16 类）
- ✅ P0 三项修复（批次 143-145）
- ✅ P1 dead_code 全量真实接入（批次 158，58 处项级 allow 标注，4 轮 CI 修复 b7b2baa→f9796cb）
- ✅ 前端 P2-1 any 类型清理（批次 160-196，frontend/src 无真实 any 残留）
- ✅ 前端 P2-5 quality 分页接入（批次 161）
- ✅ 前端 P2-6 死代码清理 + P2-7 inventory any[] 类型化（批次 160）
- ⏳ v11 前端 P2-2：i18n 接入（仅 Login.vue，其余 ~150 个 .vue 文件硬编码中文）→ 合并到 v13 FE-P2-3

### 批次 190：E2E 加强测试（规则 5 首次执行，已完成）

> 规则 5 首次执行。CI run 28912297000 ci-e2e job 超时 cancelled，95 测试全失败。
> 修复 playwright.config.ts + ci-cd.yml ci-e2e job + 提取 mock 数据到 e2e/fixtures/auth.ts（规则 6）。
> 详细报告见 `e2e-reports/`。后续 E2E 失败排查为已知问题（连续多次"启动后端服务"失败，非代码质量）。

### 已完成批次（最近 20 个）

| 批次 | main commit | 内容 |
|------|-------------|------|
| 236 | `eaa5c9b` | v13 P1-3 N+1 查询/写入重构（4 处 INSERT 批量化）：ar_service.rs auto_verify 明细 INSERT 批量化+发票 UPDATE 去重推迟；ar/vfy.rs auto_match 8 个 INSERT 点批量化；so/delivery.rs ship_order 发货明细+lock_inventory 预留 INSERT 批量化；3 文件 +94 -57 行；CI run #29019444093 12/12 核心全绿（Clippy 通过），E2E queued 不阻塞；评估后保持现状：po/receipt.rs receive_order（乐观锁语义）+ so/delivery.rs cancel_delivery（每明细 product_id 不同需 CASE WHEN）|
| 235 | `00b38d8` | v13 P1-2 inventory_piece 状态字段约定定义 + RESERVED 常量补全 + barcode_scanner_handler Expr::cust 移除多余 & 借用修复；CI 12/12 核心全绿 |
| 234 | - | v13 P1-1 硬编码状态字符串替换（7 个业务域：payment/ar_recon/accounting/fixed_asset/tax/payroll/crm）+ CI 修复 |
| 233 | - | v13 P1-1 硬编码状态字符串替换（8 个业务域：customer/supplier/product/bom/quality/contract/budget/cost） |
| 232 | - | v13 P1-1 硬编码状态字符串替换（5 个业务域：sales/purchase/inventory_movement/production/warehouse） |
| 231 | - | v13 P1-1 硬编码状态字符串替换（5 个核心业务域：ar/ap/inventory/po/so 状态常量提取 + 替换） |
| 230 | - | v13 前端 FE-P1-1 修复 |
| 229 | - | v13 P0-1 warehouse_handler update_location 欺骗性 stub 修复 |
| 228 | - | v12 P2-8 测试覆盖补测：so/delivery 25 + ar/vfy 31 = 56 个测试（验证批次 225-228 全部 342 个测试），CI 12/12 核心全绿 |
| 227 | - | v12 P2-8 测试覆盖补测：production_order 55 + mrp_engine 25 = 80 个测试 |
| 226 | - | v12 P2-8 测试覆盖补测：voucher_service 29 + ap_reconciliation 30 = 59 个测试 |
| 225 | - | v12 P2-8 测试覆盖补测：bom_service 24 + ar/recon 22 = 46 个测试 |
| 224 | - | v12 P2-8 测试覆盖补测：inv/stock 15 + so/order_workflow 17 = 32 个测试，CI 12/12 核心全绿 |
| 223 | - | v12 P2-8 测试覆盖补测：po/order 19 + inventory_reservation 17 = 36 个测试，CI 12/12 核心全绿 |
| 222 | - | v12 P2-8 测试覆盖补测：accounting_period_service 14 个测试 |
| 221 | - | v12 P2-8 测试覆盖补测：customer_credit_limit 19 个测试，CI 12/12 核心全绿 |
| 217 | - | v12 P2-7 API 一致性 100%（改造 failover_handler.rs get_failover_status），CI 12/12 核心全绿 |
| 216 | `ecb841b` | v12 P2-1 销售发货 cancel_delivery 功能实现（service+handler+route），CI 12/12 核心全绿 |
| 215 | `e0f6590` | v12 P2-1 采购订单 cancel_order 功能实现（service+handler+route） |
| 214 | `7a8ae40` | v12 P2-1 死代码常量接入业务：status.rs 删除 RECEIVED + FULFILLED→CONSUMED + 新增 LOCKED/RELEASED + purchase_receipt 子模块 |

### 历史批次摘要（批次 110-213，详细记录见 CHANGELOG.md）

- **批次 197-213**：v12 P0/P1/P2 修复（warehouse_handler stub + password_policy + BOM/报价 N+1 + CSV 导出改 xlsx + WebSocket token 安全 + 状态字符串替换 82 处 + cancel_order/cancel_delivery 功能实现）
- **批次 158-196**：v11 P1 dead_code 全量真实接入（58 处项级 allow 标注）+ 前端 P2-1 any 类型清理（批次 160-196，frontend/src 无真实 any 残留）
- **批次 143-157**：v11 P0 三项修复 + 复审报告生成
- **批次 130-142**：v9 P0/P1 修复（bi_analysis 16 方法真实接入 + purchase_inspection 4 明细 CRUD + dashboard/statistics 占位补全）
- **批次 121-129**：v8 P1/P2 修复（event_kafka 删除 + crm 标签 + ElasticClient 真实实现 + SearchSyncer 接入 + print/import/report/financial 静态配置化）
- **批次 110-120**：v7 P0/P1/P2 修复（webhook + incoterms + api_keys + 占位符 + 通知日志化 + failover/cache 删除 + supplier 资质 + token_bucket 删除 + initialize_dimensions 接入）
- **批次 103-109**：v7 复审启动（search_api 真实接入 + cache_service 接入 AppState + messaging 删除 + webhook 真实接入）
- **批次 96-102**：v5/v6 复审 P0/P1/P2/P3 修复（ArService 真实实现 + 状态机 lock_exclusive + 分页 clamp + 金额精度）
- **批次 85-95**：v2/v3/v4 复审 P0-P3 修复（事务边界 + spawn panic 隔离 + FOR UPDATE）
- **批次 49-84**：v19 P0/P1/P2/P3 修复（早期审计修复）
- **批次 1-48**：早期修复（前端权限/路由/API 断链/安全漏洞）

### 复审维度（基于历次复审经验，v7+ 沿用）

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
