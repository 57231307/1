# 全项目复审 v1 修复规划（批次 78-84）

**复审报告**：[2026-07-03-reaudit-v1.md](file:///workspace/docs/audits/2026-07-03-reaudit-v1.md)
**审计基线**：main HEAD `f0a495f1`
**问题总数**：61 项（P1×18 / P2×21 / P3×22）
**规划批次**：7 批（批次 78-84）

## 批次规划

### 批次 78：mark_as_paid user_id 透传 + ar_invoice delete 事务（P1，2 项）✅ 已完成

**主题**：审计日志 user_id 透传遗漏项 + delete 事务包裹
**级别**：P1
**项数**：2
**修复分支**：`fix/v19-batch78-mark-as-paid-p1`（已合并删除）
**合并 commit**：`f98f2717`（PR #321 squash merge，CI 12/13 全绿，E2E continue-on-error）
**影响范围**：6 文件 +43/-7

| # | 问题 | 文件 | 修复 |
|---|------|------|------|
| P1-1 | mark_as_paid 审计 user_id 仍为 Some(0) | ar_invoice_service.rs:416 | 函数签名增加 user_id + CollectionCompleted 事件透传 user_id + ar_collection_service.create_collection 发布时携带 user_id + 同步 event_kafka_payload/event_kafka/test_event_bus |
| P1-14 | ar_invoice_service.delete 未用事务包裹 | ar_invoice_service.rs:316-334 | 用 txn 包裹 delete_with_audit，状态检查改用 lock_exclusive 串行化 |

### 批次 79：CRUD + 状态检查 TOCTOU 修复（P1，8 项）✅ 已完成

**主题**：8 处辅助方法事务 + lock_exclusive 修复
**级别**：P1
**项数**：8
**修复分支**：`fix/v19-batch79-crud-toctou-p1`（已合并删除）
**合并 commit**：`e01b09dd`（PR #322 squash merge，CI 12/13 全绿，E2E continue-on-error）
**影响范围**：4 文件 +87/-29

| # | 问题 | 文件 | 修复 |
|---|------|------|------|
| P1-6 | add_return_item 事务外状态检查 | sales_return_service.rs:150-188 | find_by_id + 状态检查移入 txn + lock_exclusive |
| P1-7 | update_return 事务外状态检查 + 无事务更新 | sales_return_service.rs:191-246 | begin + lock_exclusive + update_with_audit(&txn) + commit |
| P1-8 | lock_reservation 事务外状态检查 + 无事务更新 | inventory_reservation_service.rs:47-71 | 同上模式 |
| P1-9 | release_reservation 事务外状态检查 + 无事务更新 | inventory_reservation_service.rs:74-99 | 同上模式 |
| P1-10 | delete_reservation 事务外状态检查 + 无事务删除 | inventory_reservation_service.rs:132-157 | 同上模式 |
| P1-11 | update_adjustment 事务外状态检查 + 无事务更新 | inventory_adjustment_service.rs:383-423 | 同上模式 |
| P1-12 | delete_adjustment 事务外状态检查 | inventory_adjustment_service.rs:426-449 | find_by_id + 状态检查移入 txn + lock_exclusive |
| P1-13 | reject_order 事务外状态检查 | so/contract.rs:16-60 | 同上模式 |

### 批次 80：错误处理修复（P1，4 项 + P1-15 合并）✅ 已完成

**主题**：expect panic 风险 + 静默吞错 + 事务包裹
**级别**：P1
**项数**：4 + P1-15 合并
**修复分支**：`fix/v19-batch80-error-handling-p1`（已合并删除）
**合并 commit**：`0bee1d78`（PR #323 squash merge，CI 12/13 全绿，E2E continue-on-error）
**影响范围**：5 文件 +83/-19

| # | 问题 | 文件 | 修复 |
|---|------|------|------|
| P1-3 | report 模块 4 处 .expect("合法时分秒") panic | report/ds.rs:260,264 + report/job.rs:70,113 | 改为 ok_or_else(\|\| AppError::internal("时分秒非法"))? |
| P1-4a | failover_service 2 处 let _ = 静默吞错 | failover_service.rs:233,246 | 合并 P1-15：用 txn 包裹 find + update/insert，失败 map_err()? 返回 Err |
| P1-4b | event_notification_service let _ = 静默吞错 | event_notification_service.rs:67 | 改为 if let Err(e) = ... { tracing::warn!(...); } |
| P1-4c | bpm_service let _ = 静默吞错 | bpm_service.rs:829 | 同上 |
| P1-15 | failover_service.update_circuit_status 未用事务 | failover_service.rs:233,246 | 合并 P1-4a：用 txn 包裹，失败返回 Err |

### 批次 81：Json<Value> 强类型 DTO 改造（P1，22 处）✅ 已完成

**主题**：21 个 handler Json<Value> → 强类型 DTO + validator + 金额字段 f64 → Decimal
**级别**：P1
**项数**：22
**修复分支**：`fix/v19-batch81-json-dto-p1`
**影响范围**：15 文件（15 个 handler）

| # | 问题 | 文件 | 修复 |
|---|------|------|------|
| P1-2a | budget_management_handler 3 处 Json<Value> | budget_management_handler.rs:461,522,602 | CreateBudgetDto + UpdateBudgetDto + validator |
| P1-2b | customer_credit_handler 2 处 Json<Value> | customer_credit_handler.rs:252,308 | CreateCreditDto + UpdateCreditDto + validator |
| P1-2c | financial_analysis_handler 2 处 Json<Value> | financial_analysis_handler.rs:84,171 | CreateIndicatorDto + UpdateIndicatorDto + validator |
| P1-2d | sales_order_handler create_delivery Json<Value> | sales_order_handler.rs:493 | CreateDeliveryDto + validator |
| P1-2e | login_security_handler unlock_account Json<Value> | login_security_handler.rs:210 | UnlockAccountDto + validator |
| P1-2f | production_order_handler update_status Json<Value> | production_order_handler.rs:403 | UpdateStatusDto + 状态白名单校验 |
| P1-2g | crm_handler Json<Value> | crm_handler.rs:137 | 强类型 DTO + validator |
| P1-2h | crm_customer_handler 2 处 Json<Value> | crm_customer_handler.rs:98,168 | 强类型 DTO + validator |
| P1-2i | purchase_inspection_handler 2 处 Json<Value> | purchase_inspection_handler.rs:154,175 | 强类型 DTO + validator |
| P1-2j | fixed_asset_handler Json<Value> | fixed_asset_handler.rs:204 | 强类型 DTO + validator |
| P1-2k | supplier_evaluation_handler Json<Value> | supplier_evaluation_handler.rs:261 | 强类型 DTO + validator |
| P1-2l | five_dimension_handler Json<Value> | five_dimension_handler.rs:155 | 强类型 DTO + validator |
| P1-2m | purchase_contract_handler Json<Value> | purchase_contract_handler.rs:191 | 强类型 DTO + validator |
| P1-2n | report_enhanced_handler Json<Value> | report_enhanced_handler.rs:291 | 强类型 DTO + validator |
| P1-2o | sales_contract_handler Json<Value> | sales_contract_handler.rs:189 | 强类型 DTO + validator |
| P1-5 | finance_invoice_handler 金额字段 f64 | finance_invoice_handler.rs:45,48,51 | f64 → Decimal + round_dp(2) 校验 |

### 批次 82：前端类型清理 + 按钮权限（P2，5 项）✅ 已完成

**主题**：前端 API any 类型清理 + v-permission 按钮权限补齐
**级别**：P2
**项数**：5
**修复分支**：`fix/v19-batch82-frontend-types-p2`
**影响范围**：约 20 文件（API 类型定义 + .vue 按钮权限）

| # | 问题 | 文件 | 修复 |
|---|------|------|------|
| P2-9a | custom-order.ts 11 处 data: any | custom-order.ts | 定义 11 个 DTO 接口（CustomOrderCreateDto 等）对齐后端 |
| P2-9b | inventory.ts:175 返回类型含 any | inventory.ts | 定义 InventoryReportSummary / Detail 接口 |
| P2-9c | 13+ API 文件 params?: any | 14 个 API 文件 | 定义 17 个 QueryParams 接口 + 3 处其他 any 清理 |
| P2-9d | types/api.ts:56 PageResult<T = any> | types/api.ts | 改为 PageResult<T = unknown> |
| P2-10 | v-permission 覆盖率极低 | 8 个 .vue 文件 | 顶部"新建"按钮补齐 v-permission（覆盖率 1→9 文件） |

### 批次 83：安全一致性 + 测试质量（P2，6 项）

**主题**：SQL 注入中间件决策 + IP 提取一致性 + 测试夹具修复
**级别**：P2
**项数**：6

| # | 问题 | 文件 | 修复 |
|---|------|------|------|
| P2-12a | sql_injection_audit 中间件未全局挂载 | main.rs / sql_injection_audit.rs | 全局挂载或删除并文档化 |
| P2-12b | rate_limit unknown_ip 聚合风险 | rate_limit.rs:274 | 缺失 IP 头时返回 400 |
| P2-12c | auth_handler client_ip 提取优先级不一致 | auth_handler.rs:228-233 | 抽取统一 extract_client_ip helper |
| P2-11a | inventory-store.test.ts 夹具不一致 | inventory-store.test.ts:117,129 | 改为合规 StockAdjustmentData 样本 |
| P2-11b | quotation_handler_test 名为集成但只测反序列化 | quotation_handler_test.rs | 重命名为 quotation_dto_serde_test.rs |
| P2-11c | custom_order_e2e_test 名为 E2E 但仅测状态机 | custom_order_e2e_test.rs | 重命名为 custom_order_state_machine_test.rs |

### 批次 84：P2/P3 杂项清理（P2/P3，14 项）

**主题**：分页默认值 + 返回值处理 + 金额精度 + 部署脚本 + TODO 标注
**级别**：P2/P3
**项数**：14

| # | 问题 | 文件 | 修复 |
|---|------|------|------|
| P2-1 | inventory_reservation 两次 update 事务合并 | inventory_reservation_service.rs:68,96 | 合并到单个 txn |
| P2-2 | 23 处 page.unwrap_or_default() | 多 handler 文件 | 改为 unwrap_or(1) + max(1) |
| P2-3 | 5 处 let _ = update_with_audit 返回值 | 多 service 文件 | 改为 let _result = 或明确类型 |
| P2-4 | 金额精度校验缺失 | 多 service 文件 | 统一加 round_dp(2) 校验 |
| P2-5 | deploy-latest.sh 迁移失败静默忽略 | deploy-latest.sh:233 | 移除 2>/dev/null \|\| true |
| P2-6 | custom_order_quality resolve_issue 事务外 | custom_order_quality_service.rs:101-125 | 事务 + lock_exclusive |
| P3-1 | 4 处 reason 属性缺 TODO 注释 | audit_log_service / slow_query_collector / audit_log | 补充 TODO 注释 |
| P3-2 | request.ts shouldRetry error: any | request.ts:218 | 改为显式类型 |
| P3-3 | CSRF_PUBLIC_PREFIXES 子串匹配 | request.ts:43-44 | 改为 startsWith |
| P3-4 | 多接口字段 any 类型 | 多 API 文件 | 逐项替换为显式接口 |
| P3-5 | 测试命名风格不统一 | backend/tests/ | 长期统一为 xxx_test.rs |
| P3-6 | playwright webServer 无 backend | playwright.config.ts | 文档化"前端独立冒烟测试" |
| P3-7 | sales_analysis_service 全表加载 | sales_analysis_service.rs:223 | 加 LIMIT 兜底或改数据库聚合 |
| P3-8 | ci-deps continue-on-error | ci-cd.yml:1354 | 补充 TODO 注释 |

## 进度跟踪

| 批次 | 主题 | 级别 | 项数 | 状态 |
|------|------|------|------|------|
| 78 | mark_as_paid user_id + delete 事务 | P1 | 2 | ✅ 已完成 |
| 79 | CRUD TOCTOU 修复 | P1 | 8 | ✅ 已完成 |
| 80 | 错误处理修复 | P1 | 4 | ✅ 已完成 |
| 81 | Json<Value> 强类型 DTO 改造 | P1 | 22 | ✅ 已完成 |
| 82 | 前端类型清理 + 按钮权限 | P2 | 5 | ✅ 已完成 |
| 83 | 安全一致性 + 测试质量 | P2 | 6 | ✅ 已完成 |
| 84 | P2/P3 杂项清理 | P2/P3 | 14 | 待启动 |

**P1 完成后**：第二轮复审
**P2 完成后**：第二轮复审
**P3 完成后**：第二轮复审，循环直到无问题
