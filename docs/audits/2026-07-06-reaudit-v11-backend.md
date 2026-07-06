# v11 第十一轮复审报告 — Rust 后端全项目复审

**审计基线**：main HEAD（v9/v10 复审修复完成后）
**审计时间**：2026-07-06
**审计方式**：6 维度并行扫描（占位符 / dead_code / 硬编码假数据 / 未接入功能 / unwrap&expect / 预留扩展空间）
**扫描范围**：
- `/workspace/backend/src/services/`（所有 .rs 文件）
- `/workspace/backend/src/handlers/`（所有 .rs 文件）
- `/workspace/backend/src/routes/`（所有 .rs 文件）
- `/workspace/backend/src/models/`（仅业务逻辑，SeaORM 自动生成模型适用例外）
- `/workspace/backend/src/utils/`（所有 .rs 文件）
- `/workspace/backend/src/middleware/`（所有 .rs 文件）
- 附加扫描：`/workspace/backend/src/config/`、`/workspace/backend/src/search/`、`/workspace/backend/src/websocket/`、`/workspace/backend/src/cli/`

**前序复审已修复**：
- v9：bi_analysis_service 16 方法、purchase_inspection 4 明细 CRUD、production_order_logs、ap_invoice get_statistics、dashboard_service 双方法
- v10：webhook platform 占位、report_engine filters 写死、financial_analysis dead_code、report_enhanced template_id、event_notification sender_id=0（16 处）、bpm_service sender_id=0、crm/assign 空占位模块（已完整实现为 CRM 分配服务）

**已删除的死代码模块**：token_bucket、failover（执行器部分）、cache/redis_client、EventBackend trait、tree_builder.rs、bpm_service_stub.rs（重命名）

**发现问题汇总**：约 47 项（P0×0 / P1×8 / P2×31 / P3×8）
- **本轮无 P0 阻塞项**：v9/v10 已修复所有阻塞生产的占位模块（ArService、bi_analysis、purchase_inspection、crm/assign 等）
- **P1 集中在"未接入功能"**：failover 配置全模块 dead_code、inventory_count 子模块未实现、tracking 多个统计路由未挂载、budget_management 多个扩展字段未接入模型
- **P2 以项级 `#[allow(dead_code)] + TODO(tech-debt)` 标注的预留 API/字段为主**，符合死代码处理规范，可在后续迭代逐项处理

---

## 一、关键发现概览

### P0 级（阻塞生产 — 紧急修复）

**本轮未发现 P0 阻塞项。**

v9/v10 复审已修复全部 P0 阻塞模块（ArService 真实实现、bi_analysis_service 16 方法真实查询、purchase_inspection 4 明细 CRUD 真实接入、dashboard_service 双方法真实查询、event_notification sender_id=0 修复 16 处、bpm_service sender_id=0 修复、crm/assign 占位模块真实实现为 CRM 分配服务）。

当前残留的 `#[allow(dead_code)] + TODO(tech-debt)` 项均不影响核心业务流程，仅作为预留 API/字段待后续迭代接入。

---

### P1 级（本迭代修复 — 未接入功能 / 占位实现）

| # | 文件:行号 | 维度 | 问题描述 | 修复建议 |
|---|---------|------|---------|---------|
| P1-1 | `routes/inventory.rs:17` | 未接入功能 | `inventory_count` 子模块完全未实现：注释 `// TODO(tech-debt): inventory_count 子模块实现后恢复导入`，对应库存盘点功能（盘点单 CRUD、盘点差异生成、盘点审批）在 main 上未挂载任何路由 | 真实实现 inventory_count_handler 全套方法（盘点单 CRUD + 差异计算 + 审批流），或删除占位 TODO 注释及未使用 import |
| P1-2 | `routes/analytics.rs:567-575` | 未接入功能（死路由） | `tracking()` 路由仅挂载 `track_page_view` 一个端点；`get_page_view_stats` / `get_page_view_stats_by_day` / `get_popular_pages` / `record_behavior` / `get_funnel_analysis` / `get_user_path` 共 6 个统计接口在 `tracking_handler` 中存在但**未挂载到 main**（注释称"避免编译期 E0425"），形成死代码 | 真实实现 6 个统计接口的 DB 查询并挂载路由；或删除 handler 中的死方法 |
| P1-3 | `services/bi_analysis_service.rs:1086-1093` | 未接入功能 | `pivot` 方法注释 `透视矩阵需动态 SQL 构建，当前返回 row 维度聚合数据，col 维度分组待后续迭代实现`，并在响应 JSON 中暴露 `note` 字段告知前端"col 维度分组待后续迭代实现"，BI 透视分析功能不完整 | 实现动态 SQL 透视矩阵构建（row × col 交叉聚合），移除 note 占位字段 |
| P1-4 | `services/inventory_stock_query.rs:28-30` | 未接入功能 | `compute_alert_type` 注释 `OverStock（高于上限）和 SlowMoving（滞销）暂未实现，因为 inventory_stocks 表无 max_stock_point / last_movement_date 阈值字段`；当前库存预警仅 5 类（缺货/低于下限/即将过期/差异/正常），高库存与滞销预警缺失 | DB migration 添加 `inventory_stocks.max_stock_point` 与 `last_movement_date` 字段，扩展 compute_alert_type 实现 OverStock / SlowMoving 告警 |
| P1-5 | `services/stock_alert.rs:31,39` | dead_code（与 P1-4 联动） | `AlertType::OverStock` 和 `AlertType::SlowMoving` 两个变体标记 `#[allow(dead_code)]`，TODO 注释引用 P1-4 中的字段缺失问题；`desc()` 与 `code()` 已实现但 `compute_alert_type` 未派生 | 与 P1-4 同步修复，DB 字段补充后移除 dead_code 标注 |
| P1-6 | `config/failover.rs:17,26-27,49-50,68,72,85,90,97` | dead_code（整模块未接入业务） | 整个 failover 配置模块大面积标记 dead_code：`FailoverConfig.monitoring`、`DatabaseFailoverConfig`（文件级 `#[allow(dead_code)]`）、`CacheFailoverConfig`（文件级 `#[allow(dead_code)]`）、`MonitoringFailoverConfig.metrics_enabled/log_level`、`default_true`、`default_log_level`、`FailoverConfig::load_from_file`。注释明确：`数据库主备切换未接入`、`缓存主备切换未接入`、`failover 监控告警接入后移除`。**违反规则 0：保留的功能扩展空间视为未实现功能** | 二选一：(A) 真实实现数据库主备切换 + 缓存主备切换 + 监控告警接入；(B) 删除整个 failover 配置模块（failover 执行器已在 v8 删除，配置层保留无意义）。推荐 (B) |
| P1-7 | `services/auth_service.rs:619,631` | 占位实现 | `cleanup_revoked_users()` 仅返回日志 `"当前用户吊销表条目数：{}"`，无清理逻辑；`unrevoke_user(user_id)` 仅从内存 HashMap 移除，注释 `保留此函数以备后续引入"自动解除封禁"等业务策略`。两个函数均标记 `#[allow(dead_code)]`，属典型"占位 API" | 接入 cron 定时清理任务（如吊销记录 TTL 7 天后自动清理），或删除两个占位函数 |
| P1-8 | `services/budget_management_service.rs:38,51,71,77,85` | dead_code（扩展字段未接入模型） | 5 处 `#[allow(dead_code)]`：`BudgetSubject.budget_year/planned_amount/remark`、`BudgetSubjectItem` 整体、`BudgetExecution.actual_amount/expense_type/expense_date/remark`。注释统一为 `预算科目扩展字段接入模型后移除`。预算管理模块核心字段未持久化 | DB migration 添加对应字段到 `budget_subject` / `budget_subject_item` / `budget_execution` 表，移除 dead_code 标注；或删除未使用的扩展 DTO 字段 |

**P1 优先级判定理由**：
- P1-1/P1-2：影响用户可见功能（库存盘点 / 行为分析），路由文件级 TODO 是明确的"未实现"标记
- P1-3/P1-4/P1-5：BI 透视矩阵和库存预警功能不完整，影响业务决策
- P1-6：failover 配置整模块未接入业务，违反规则 0（保留扩展空间=未实现功能）；且执行器已删除，配置层保留无意义
- P1-7：auth_service 占位 API 长期未接入，违反规则 0
- P1-8：预算管理核心字段未持久化，影响业务数据完整性

---

### P2 级（下个迭代修复 — 预留 API / 标注失效 / 字段未接入）

#### P2-A：项级 `#[allow(dead_code)] + TODO(tech-debt)` 预留 API（21 项）

符合死代码处理规范的项级标注，建议按业务接入计划逐项处理。

| # | 文件:行号 | 问题描述 | 修复建议 |
|---|---------|---------|---------|
| P2-1 | `services/voucher_service.rs:900` | `VoucherDetail` struct 标记 `预留 API，待凭证详情查询接入后移除` | 接入凭证详情查询路由（GET /vouchers/:id/details）或删除 |
| P2-2 | `services/audit_log_service.rs:60,84` | `AuditEvent::new` 和 `log_change` 预留 API | 接入审计日志同步写入路径或删除 |
| P2-3 | `services/email_log_service.rs:107` | `increment_retry` 预留 API（邮件重试调度任务） | 接入邮件重试 cron 任务或删除 |
| P2-4 | `services/email_service.rs:786` | `save_email_log` 预留 API | 接入邮件发送完成时统一回写日志或删除 |
| P2-5 | `services/purchase_receipt_service.rs:608` | `calculate_receipt_total` 预留 API（公共入口暂未被 handler 调用） | 接入采购收货 total 计算路由或删除 |
| P2-6 | `services/accounting_period_service.rs:176` | `check_date_locked_txn` 预留 API（ArCollectionService::create_collection 接入 handler 后移除） | 接入应收收款创建事务或删除 |
| P2-7 | `services/customer_service.rs:227` | `list_customers` 预留 API（CRM 客户模块统一迁移后接入，或删除） | 接入 CRM 客户列表路由或删除 |
| P2-8 | `services/report/mod.rs:96,121,159` | `CreateTemplateRequest`、`DataSource::as_str`、`AggregationType::as_str` 预留 API | 接入报表模板创建/调试输出或删除 |
| P2-9 | `services/omni_audit_service.rs:42` | `OmniAuditEngine.secret_key` 字段标记 dead_code，注释 `当前异步任务内联使用 secret_key 字符串，未通过 self 字段读取` | 重构异步任务通过 self.secret_key 引用，或删除字段 |
| P2-10 | `services/capacity_service.rs:106` | `LoadAnalysisQuery.work_center_id` 字段（报表模块接入后移除） | 接入按工作中心筛选产能负荷或删除字段 |
| P2-11 | `services/sales_analysis_service.rs:43,52,79` | `ProductRankingParams.dimension_type`、`CustomerRankingParams.dimension_type`、`ExportParams.format` 三个字段未接入 | 接入多维度排名筛选与导出格式或删除字段 |
| P2-12 | `services/assignment_history_service.rs:41,43` | `AssignmentHistoryQuery.date_from/date_to` 字段未接入 | 接入分配历史日期范围筛选或删除字段 |
| P2-13 | `services/five_dimension_service.rs:50,52` | `FiveDimensionQuery.page/page_size` 字段未接入 | 接入分页或删除字段 |
| P2-14 | `services/ar/mod.rs:78,154` | AR 模块匹配策略字段和备注字段未接入 | 接入应收匹配策略路由或删除字段 |
| P2-15 | `services/color_card_crud_service.rs:239` | `mark_lost` 方法（当前未接入路由，后续如需直接标记色卡遗失可接入 CRUD 路由） | 接入色卡遗失标记路由或删除方法 |
| P2-16 | `handlers/report_enhanced_handler.rs:348` | `ExportRequest` struct 标记 dead_code（报表导出 API 接入前端后移除） | 接入前端导出 API 调用或删除 struct |
| P2-17 | `handlers/barcode_scanner_handler.rs:44` | `ScanHistoryQuery.scan_type` 字段未接入 | 接入扫码历史按类型筛选或删除字段 |
| P2-18 | `handlers/email_handler.rs:43` | `SendEmailRequest.template_params` 字段未接入 | 接入模板参数解析或删除字段 |
| P2-19 | `handlers/crm_pool_handler.rs:26` | `PoolQueryParams.industry` 字段未接入 | 接入客户池按行业筛选或删除字段 |
| P2-20 | `services/report_template_service.rs:59` | `ReportTemplateQuery.status` 字段未接入 | 接入报表模板按状态筛选（ACTIVE/INACTIVE）或删除字段 |
| P2-21 | `services/scheduling_query.rs:27` | `scheduling_query` 模块整体标记 dead_code（排程诊断日志接入后移除） | 接入排程诊断日志端点或删除模块 |

#### P2-B：dead_code 标注失效（4 项 — 实际已被调用，应移除标注）

| # | 文件:行号 | 问题描述 | 修复建议 |
|---|---------|---------|---------|
| P2-22 | `services/color_card_scan_service.rs:17,31` | `ScanError` enum 和 `ColorCardScanService` impl 标记 `#[allow(dead_code)]`，但实际被 `handlers/color_card/scan_export.rs` 调用 | 移除 dead_code 标注 |
| P2-23 | `services/color_card_borrow_service.rs:46,79` | `BorrowStatus` impl 和 `ColorCardBorrowService` impl 标记 dead_code，但实际被 `handlers/color_card/borrow.rs` 调用 | 移除 dead_code 标注 |
| P2-24 | `services/color_price_batch_service.rs:41` | 整个 impl 块标记 dead_code，但实际被 `color_price_handler.rs:211,232` 调用 | 移除 dead_code 标注 |
| P2-25 | `services/slow_query_collector.rs:81,143` | `start_collect_task` / `collect_once` 标记 dead_code，但实际被 `main.rs:454` 和 `slow_query_handler.rs:224` 真实调用 | 移除 dead_code 标注 |

#### P2-C：硬编码响应数据（3 项）

| # | 文件:行号 | 问题描述 | 修复建议 |
|---|---------|---------|---------|
| P2-26 | `handlers/ai_extend_handler.rs:205-219` | `ai_health` 返回硬编码算法信息：`process_optimization.algorithm: "k-NN + 加权平均"`、`fallback: "典型参数表（80°C/45min/pH6.0/浴比1:8）"`、`quality_prediction.algorithm: "趋势分析 + 风险评分"`、`fallback: "保守默认（合格率 95% / 置信度 0.3）"`。注：健康检查端点返回固定元信息属可接受模式，但算法描述应与 service 实际实现保持一致 | 评估是否将算法元信息下沉到 `AiExtendService::algorithm_metadata()` 静态方法（与 ReportFieldDefinition 模式一致），避免 handler 与 service 算法描述脱钩 |
| P2-27 | `handlers/voucher_handler.rs:265-271` | `get_voucher_types` 返回硬编码 4 种凭证类型（记账/收款/付款/转账），使用 `serde_json::json!` 构造 | 改为静态配置化：定义 `VoucherTypeDefinition` struct + `VoucherService::available_voucher_types()` 静态方法（与 `ReportFieldDefinition` 批次 128 模式一致）；或下沉到 `models/voucher_type.rs` 作为枚举常量 |
| P2-28 | `services/bi_analysis_service.rs:1086-1093` | `pivot` 响应 JSON 中暴露 `note: "透视矩阵需动态 SQL 构建，当前返回 row 维度聚合数据，col 维度分组待后续迭代实现"`，向用户暴露内部实现细节 | 见 P1-3 修复 |

#### P2-D：搜索模块预留 API（1 项）

| # | 文件:行号 | 问题描述 | 修复建议 |
|---|---------|---------|---------|
| P2-29 | `search/elastic.rs:48,56,292` | `DocType` enum 两处变体 + 一个测试辅助方法标记 `#[allow(dead_code)]`，注释 `批次 104 已接入 search_api，DocType 保留为公共 API 预留` | 接入 SearchClient 公共 API 暴露 DocType 或删除 |

#### P2-D-2：utils 模块预留 API（4 项）

| # | 文件:行号 | 问题描述 | 修复建议 |
|---|---------|---------|---------|
| P2-30 | `utils/audit.rs:28` | `SecurityEvent::UserDeleted` 变体标记 dead_code，注释 `delete_user 改用 AuditLogService::record_async 落库，此变体当前无业务引用，保留以供未来 log_security_event 接入 DB 写入时使用` | 接入 `log_security_event` DB 写入或删除变体 |
| P2-31 | `utils/admin_checker.rs:55` | 整个 impl 块标记 dead_code，注释 `缓存清理定时任务接入后移除` | 接入缓存清理 cron 任务或删除 |
| P2-32 | `utils/import_export.rs:265` | `ExcelImporter` 字段标记 dead_code，注释 `Excel 导入功能接入后移除` | 接入 Excel 导入路由或删除 |
| P2-33 | `utils/color_space_converter.rs:168` | 函数标记 dead_code，注释 `仅测试调用；生产代码接入色卡匹配后移除` | 接入色卡匹配业务或删除 |

---

### P3 级（技术债务 — SeaORM 模型字段 / 标注规范 / 测试夹具）

#### P3-A：SeaORM 自动生成模型文件级 `#![allow(dead_code)]`（适用例外，无需修复）

依据项目规则第六章第 1 节例外：`/workspace/backend/src/models/` 下 SeaORM 自动生成模型可保留文件级 `#![allow(dead_code)]`，原因是模型字段由 SeaORM 派生宏使用，不能手工逐字段标注。

以下 27 个模型文件均带有 `// TODO(tech-debt): 业务接入或重评估后逐项移除` 注释，属合规例外：

`models/sales_quotation_term.rs`、`models/supplier_evaluation.rs`、`models/api_key.rs`、`models/inventory_transfer_item.rs`、`models/work_center.rs`、`models/sales_delivery_item.rs`、`models/purchase_return_item.rs`、`models/ap_payment.rs`、`models/warehouse.rs`、`models/supplier_product.rs`、`models/fixed_asset_depreciation_record.rs`、`models/finance_payment.rs`、`models/customer_followup.rs`、`models/ap_reconciliation.rs`、`models/supplier_category.rs`、`models/fixed_asset.rs`、`models/log_api_access.rs`、`models/unqualified_product.rs`、`models/ar_aging_analysis.rs`、`models/product_supplier_mapping.rs`、`models/sales_return_item.rs`、`models/department.rs`、`models/ar_reconciliation.rs`、`models/process_log.rs`、`models/supplier_contact.rs`、`models/purchase_return.rs`、`models/dye_batch.rs`、`models/user.rs`、`models/failover_status.rs`、`models/email_log.rs`、`models/fund_transfer_record.rs`、`models/dye_recipe.rs`、`models/budget_execution.rs`、`models/inventory_adjustment_item.rs`、`models/currency.rs`、`models/customer_color_price.rs`、`models/quality_inspection.rs`、`models/sales_delivery.rs`、`models/color_card_borrow_record.rs`、`models/dto/mod.rs`、`models/quality_inspection_record.rs`、`models/inventory_transaction.rs`

**处理建议**：保持现状，但应在业务接入后逐文件评估是否仍有未使用字段。

#### P3-B：状态常量 / 业务字段 dead_code（2 项）

| # | 文件:行号 | 问题描述 | 修复建议 |
|---|---------|---------|---------|
| P3-1 | `models/audit_log.rs:75` | 预留 API 标记 | 评估是否接入或删除 |
| P3-2 | `models/status.rs:83,149,177,196` | 状态常量标记 dead_code（4 处） | 评估是否接入业务流程状态机或删除 |

#### P3-C：测试夹具 / 集成测试辅助（2 项）

| # | 文件:行号 | 问题描述 | 修复建议 |
|---|---------|---------|---------|
| P3-3 | `websocket/notifications.rs:204` | `集成测试直接调用，后续重构为 auth_service 公共方法后移除` | 重构为 auth_service 公共方法后移除标注 |
| P3-4 | `services/crm/mod.rs:68` | `公共 API 重导出，业务接入后评估是否保留`（`#[allow(unused_imports)]`） | 评估保留或移除重导出 |

#### P3-D：生产代码 unwrap/expect 残留（5 项 — 全部为不变量保护）

以下 `unwrap()/expect()` 调用均附带"不变量"注释，属可接受模式（依据 P9-1 安全加固规范）：

| # | 文件:行号 | 问题描述 | 风险评估 |
|---|---------|---------|---------|
| P3-5 | `utils/date_utils.rs:9,26` | `FixedOffset::east_opt(0).expect("不变量：east_opt(0) 永远合法")` 和 `and_hms_opt(0,0,0).expect("不变量：永远合法")` | 安全：参数为编译期常量，数学上必合法 |
| P3-6 | `services/email_service.rs:691` | `HmacSha256::new_from_slice(key).expect("HMAC-SHA256 key 长度错误")` | 安全：HMAC-SHA256 接受任意长度 key，永不返回 Err |
| P3-7 | `middleware/timeout.rs:30` | `Response::builder with valid status 500 永远成功` | 安全：HTTP 500 是合法 status，Builder 必成功 |
| P3-8 | `utils/unwrap_safe.rs:70,81` | 业务化宏 `expect_required` / `unwrap_or_log`，已封装 panic 风险 | 设计如此：业务必填项为空时 panic 是预期行为 |
| P3-9 | `utils/password_validator.rs:72` | `Regex::new(pattern).expect("密码校验正则编译失败")` | 安全：正则为编译期常量字符串，CI 编译时已验证 |

**测试代码中的 unwrap/expect（约 155 处）**：分布在 `#[cfg(test)]` 块和 `*_unit_tests.rs` / `*_handler.rs` 测试模块中，属合法测试夹具，无需修复。

#### P3-E：占位模块 / 扩展空间注释（保留合规）

以下文件以注释形式保留"扩展空间"说明，但模块本身有真实实现或合理空实现，不构成违规：

| 文件:行号 | 状态 |
|---------|------|
| `services/so/price.rs:6` | `保留扩展空间` 注释，模块本身有真实价格计算逻辑 |
| `services/inv/adjust.rs:5`、`hold.rs`、`count.rs` | 占位模块（v8 教训：SeaORM 跨文件 impl 块需谨慎评估，保留以维持模块结构） |
| `services/so/sales_return.rs:5` | 占位模块，sales_return 业务在 `services/sales_return_service.rs` 实现 |
| `services/crm/assign.rs:3` | v10 批次 140 已真实实现为 CRM 分配服务（注释说明原占位已修复） |
| `routes/v1.rs:7` | `批次 95 P3-16 修复：移除原占位 404 路由（v1_placeholder）`，占位已清理 |
| `routes/system.rs:156` | `原 stub 占位未注册，现 service 层已实现真实逻辑`，占位已清理 |

---

## 二、修复规划

### 批次 141：P1 修复（8 项）— `fix/v30-batch141-p1-unimplemented-features`

**子批 A**：failover 配置模块清理（1 项）
- P1-6: 删除整个 `config/failover.rs` 模块（执行器已删，配置层保留无意义），同步清理 `routes/failover.rs` 路由文件、`models/failover_status.rs` 模型、`config/` 中相关引用

**子批 B**：未接入功能真实实现或路由清理（3 项）
- P1-1: 实现 `inventory_count_handler` 全套方法（盘点单 CRUD + 差异计算 + 审批流），或删除 `routes/inventory.rs:17` 的 TODO 注释及未使用 import
- P1-2: 实现 `tracking_handler` 6 个统计接口的 DB 查询并挂载路由，或删除死方法
- P1-3/P2-28: 实现 `bi_analysis_service.pivot` 动态 SQL 透视矩阵构建，移除 `note` 占位字段

**子批 C**：库存预警扩展（2 项，依赖 DB migration）
- P1-4/P1-5: DB migration 添加 `inventory_stocks.max_stock_point` 与 `last_movement_date` 字段，扩展 `compute_alert_type` 实现 OverStock / SlowMoving 告警，移除 `stock_alert.rs:31,39` dead_code 标注

**子批 D**：占位 API 接入或删除（2 项）
- P1-7: 接入 `cleanup_revoked_users` 定时清理 cron 或删除；接入 `unrevoke_user` 用户重新激活路由或删除
- P1-8: DB migration 添加 `budget_management` 5 个扩展字段，或删除 DTO 中未使用字段

### 批次 142：P2-A 修复（21 项 — 预留 API 评估）— `fix/v30-batch142-p2-dead-code-cleanup`

按业务接入计划逐项处理：每项二选一（接入业务 / 删除）。建议按服务模块分组：
- 子批 A：voucher / audit_log / email 双模块（P2-1 ~ P2-4）
- 子批 B：purchase_receipt / accounting_period / customer（P2-5 ~ P2-7）
- 子批 C：report 模块（P2-8）
- 子批 D：字段级 dead_code（P2-9 ~ P2-14）
- 子批 E：handler 字段（P2-16 ~ P2-20）
- 子批 F：color_card / scheduling 单项（P2-15、P2-21）

### 批次 143：P2-B 修复（4 项 — 标注失效清理）— `fix/v30-batch143-p2-stale-annotations`

仅需移除失效的 `#[allow(dead_code)]` 标注（4 个文件 7 处），CI 验证不会失败。
- P2-22: `services/color_card_scan_service.rs:17,31`
- P2-23: `services/color_card_borrow_service.rs:46,79`
- P2-24: `services/color_price_batch_service.rs:41`
- P2-25: `services/slow_query_collector.rs:81,143`

### 批次 144：P2-C 修复（3 项 — 硬编码响应下沉）— `fix/v30-batch144-p2-static-config`

- P2-26: `ai_extend_handler.ai_health` 算法元信息下沉到 `AiExtendService::algorithm_metadata()` 静态方法
- P2-27: `voucher_handler.get_voucher_types` 改为静态配置化（`VoucherTypeDefinition` struct + `VoucherService::available_voucher_types()`）
- P2-28: 与 P1-3 一并修复

### 后续迭代：P3 项

- P3-A：保持现状，业务接入后逐文件评估
- P3-B/P3-C/P3-D：低优先级技术债务，按业务节奏处理
- P3-E：合规保留，无需修复

---

## 三、维度扫描统计

| 维度 | 扫描范围 | 命中项数 | 备注 |
|------|---------|---------|------|
| 1. 占位符/stub（todo!()/unimplemented!()// TODO// FIXME/let _ =/功能开发中/暂未实现） | 全 src 目录 | 0 个 `todo!()`/`unimplemented!()`，约 80 处 `// TODO(tech-debt)` 注释（绝大多数符合规范） | v8/v9/v10 已清理所有阻塞生产占位符；当前残留 TODO 均为项级标注，符合死代码处理规范 |
| 2. dead_code（`#[allow(dead_code)]`） | 全 src 目录 | services/ 24 项 + handlers/ 4 项 + utils/ 4 项 + middleware/ 0 项 + config/ 8 项 + search/ 3 项 + websocket/ 1 项 + models/ 27 项（例外） | 4 项标注失效（P2-B），其余均为合规项级标注 |
| 3. 硬编码假数据（serde_json::json! 硬编码响应） | 全 src 目录 | 3 项（P2-26/27/28） | 仅 3 处违反规则 0；其余 serde_json::json! 用法均为响应包装（合法） |
| 4. 未接入功能（路由→handler→service→model→DB 全链路） | routes/ + handlers/ | 3 项（P1-1 inventory_count、P1-2 tracking 6 路由、P1-3 bi_analysis pivot） | v9/v10 已修复 4 处未接入功能；当前残留 3 处 |
| 5. unwrap/expect 残留（生产代码 panic 风险） | 全 src 目录 | 5 项生产代码（全部为不变量保护，P3-D）+ 约 155 项测试夹具 | 无违规 |
| 6. 预留功能扩展空间（预留/扩展空间/未来支持/后续实现） | 全 src 目录 | 8 项需评估（P1-6 failover、P1-7 auth_service、P1-8 budget_management、P2-1~P2-21 部分项）+ 6 项合规保留（P3-E） | failover 整模块为最大违规 |

---

## 四、规则符合性评估

| 规则 | 符合情况 | 说明 |
|------|---------|------|
| 规则 0：真实实现强制 | **部分违反** | P1-1/P1-2/P1-3/P1-6/P1-7/P1-8 共 6 项违反规则 0；P2-A 21 项为合规项级标注（已加 TODO + tech-debt） |
| 死代码处理规范 — 禁止文件级 `#![allow(dead_code)]` | **符合** | 仅 `models/` 下 27 个 SeaORM 自动生成模型保留文件级标注（适用例外）；`utils/` 8 个核心文件全部开启死代码检查（v8 已建立模板） |
| 死代码处理规范 — 项级 `#[allow(dead_code)] + TODO(tech-debt)` | **符合** | 所有项级标注均附 TODO 注释，符合模板 |
| CI 强制（`cargo clippy --all-targets -- -D warnings`） | **符合** | dead_code 配置在 `backend/.clippy.toml` warn 段开启 |
| 禁止本地编译验证 | **符合** | 本轮复审为只读扫描，未执行任何 `cargo build/check/test/clippy` 命令 |

---

## 五、关键文件清单

### P1 修复涉及文件（8 项）

```
/workspace/backend/src/routes/inventory.rs                          # P1-1
/workspace/backend/src/routes/analytics.rs                          # P1-2
/workspace/backend/src/services/bi_analysis_service.rs              # P1-3, P2-28
/workspace/backend/src/services/inventory_stock_query.rs            # P1-4
/workspace/backend/src/services/stock_alert.rs                      # P1-5
/workspace/backend/src/config/failover.rs                           # P1-6
/workspace/backend/src/services/auth_service.rs                     # P1-7
/workspace/backend/src/services/budget_management_service.rs        # P1-8
```

### P2 修复涉及文件（29 项）

```
/workspace/backend/src/services/voucher_service.rs                  # P2-1
/workspace/backend/src/services/audit_log_service.rs                # P2-2
/workspace/backend/src/services/email_log_service.rs                # P2-3
/workspace/backend/src/services/email_service.rs                    # P2-4
/workspace/backend/src/services/purchase_receipt_service.rs         # P2-5
/workspace/backend/src/services/accounting_period_service.rs        # P2-6
/workspace/backend/src/services/customer_service.rs                 # P2-7
/workspace/backend/src/services/report/mod.rs                       # P2-8
/workspace/backend/src/services/omni_audit_service.rs               # P2-9
/workspace/backend/src/services/capacity_service.rs                 # P2-10
/workspace/backend/src/services/sales_analysis_service.rs           # P2-11
/workspace/backend/src/services/assignment_history_service.rs       # P2-12
/workspace/backend/src/services/five_dimension_service.rs           # P2-13
/workspace/backend/src/services/ar/mod.rs                           # P2-14
/workspace/backend/src/services/color_card_crud_service.rs          # P2-15
/workspace/backend/src/handlers/report_enhanced_handler.rs          # P2-16
/workspace/backend/src/handlers/barcode_scanner_handler.rs          # P2-17
/workspace/backend/src/handlers/email_handler.rs                    # P2-18
/workspace/backend/src/handlers/crm_pool_handler.rs                 # P2-19
/workspace/backend/src/services/report_template_service.rs          # P2-20
/workspace/backend/src/services/scheduling_query.rs                 # P2-21
/workspace/backend/src/services/color_card_scan_service.rs          # P2-22
/workspace/backend/src/services/color_card_borrow_service.rs        # P2-23
/workspace/backend/src/services/color_price_batch_service.rs        # P2-24
/workspace/backend/src/services/slow_query_collector.rs             # P2-25
/workspace/backend/src/handlers/ai_extend_handler.rs                # P2-26
/workspace/backend/src/handlers/voucher_handler.rs                  # P2-27
/workspace/backend/src/search/elastic.rs                            # P2-29
/workspace/backend/src/utils/audit.rs                               # P2-30
/workspace/backend/src/utils/admin_checker.rs                       # P2-31
/workspace/backend/src/utils/import_export.rs                       # P2-32
/workspace/backend/src/utils/color_space_converter.rs               # P2-33
```

---

## 六、复审结论

### 整体评估

**v11 复审结论**：项目代码质量较 v9/v10 显著提升，无 P0 阻塞项。残留问题以"未接入功能"和"预留 API/字段未接入业务"为主，符合规则 0 的项级标注模式已大规模落地。

### 主要风险

1. **failover 配置模块（P1-6）**：整模块 dead_code，违反规则 0，建议直接删除（执行器已删，配置层保留无意义）
2. **inventory_count 子模块（P1-1）**：库存盘点功能完全未实现，影响库存管理完整性
3. **tracking 6 个统计路由（P1-2）**：handler 中存在但未挂载，形成死代码
4. **bi_analysis pivot 矩阵（P1-3）**：BI 透视分析功能不完整，向用户暴露内部实现细节
5. **budget_management 5 个扩展字段（P1-8）**：预算管理核心字段未持久化

### 建议优先级

1. **本迭代（批次 141）**：P1-6 failover 删除（最快收益） + P1-1/P1-2/P1-3 未接入功能评估（真实实现或路由清理）
2. **下个迭代（批次 142-144）**：P2 项逐批处理
3. **后续迭代**：P3 项按业务节奏处理

### 复审状态

- ✅ 占位符/stub：v9/v10 已清理所有阻塞生产占位符，本轮无新增
- ✅ dead_code 标注规范：项级 `#[allow(dead_code)] + TODO(tech-debt)` 模板已大规模落地
- ⚠️ 未接入功能：3 项残留（P1-1/P1-2/P1-3）
- ⚠️ 预留扩展空间：failover 整模块违规（P1-6）
- ✅ unwrap/expect：无违规（5 项生产代码均为不变量保护）
- ✅ 硬编码假数据：3 项（P2-26/27/28），影响范围有限

---

**报告生成时间**：2026-07-06
**审计员**：Trae AI（GLM-5.2 模型）
**审计范围**：v11 全项目复审（6 维度并行扫描）
**下一步**：按批次 141-144 修复规划推进，所有修复必须经 CI/CD 验证（GitHub Actions API 监控 run 状态）
