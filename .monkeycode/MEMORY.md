# 项目规则记忆

> 本文件是项目的**规则记忆**，记录必须遵守的规则、指令、偏好和工作流规范。
> 历史归档与详细内容请查阅 [`.monkeycode/docs/archives/`](file:///workspace/.monkeycode/docs/archives/)。

---

## 一、关键项目规则（必读，按优先级排序）

### 🔴 规则 0（最高优先级，2026-07-04 追加）：真实实现强制

> 对所有预留的 api 及预留的功能/占位符功能/路由进行实现，
> 对所有未真实接入的功能等需要真实接入，
> 对所有遇到的错误均进行统一修复，
> 对所有的功能均需要真实接入。

**强制执行要求**：
- 所有 `#[allow(dead_code)] + TODO(tech-debt)` 标记的预留 API 逐个评估并真实接入业务或删除
- 所有占位符功能（stub / placeholder / TODO 注释 / `let _ =` 占位）必须真实实现
- 所有未真实接入的功能 / 中间件 / 路由进行真实接入（不允许遗留死路由）
- 所有遇到的编译错误 / CI 错误 / 运行时错误必须统一修复，禁止 `|| true` / `unwrap_or_default()` 掩盖
- 所有功能必须真实接入业务链路：路由 → handler → service → model → DB 全链路打通

**修复模式参考**：
- `let _ = svc.method().await;`（吞错）→ `if let Err(e) = svc.method().await { tracing::warn!(error=%e, context, "描述"); }`
- `let _ = exists_check;`（检查存在性丢弃结果）→ 直接表达式语句 `exists_check.await?;`
- `.expect("msg")`（启动期 panic）→ `unwrap_or_else(|_| { eprintln!("友好提示"); std::process::exit(1); })`
- 占位符 `let _ = var;` → 变量名前缀 `_`（如 `_port_num`）或直接删除并加注释

### 规则 1-9（常规规则）

1. **CI/CD Only 验证**：禁止本地编译/构建。所有验证必须通过 CI/CD pipeline。
2. **每项修复 1 commit**：bug 修复按"每项 1 commit"原则，便于回滚和审计。
3. **多语言禁止**：项目所有文本必须使用中文（注释、用户界面、文档）。
4. **任务管理**：使用 TodoWrite 跟踪进度，状态实时更新。
5. **memory 优先**：每次操作前查看 MEMORY.md / doto.md / bug.md。
6. **关键变更必记录**：CHANGELOG.md 记录所有重要变更。
7. **公开端点收敛**：当前仅登录/刷新/健康检查可匿名访问（2026-06-25 优化）。
8. **~~租户隔离~~**（2026-06-28 已删除）：租户功能已完整删除，`extract_tenant_id` 函数、`AuthContext.tenant_id`、`AppClaims.tenant_id`、所有 tenant_id 列/字段/过滤/索引/管理表均已移除。项目不再支持多租户。
9. **批次迭代工作流**（2026-06-27 确认）：每次修复批次完成后必须推送到 main 触发 CI 验证，CI 全绿后才继续下一批。流程：修复 → commit → push → 监控 CI → 全绿后继续。禁止积累多批未验证的修改。

---

## 二、当前任务状态（2026-07-05 批次 129 完成 - v8 复审 P2 全部完成 5/5，启动 v9 全项目复审）

> 用户最高优先级规则已在「一、规则 0」固化，本节仅记录修复进度。

### v7 复审 P1 修复进度（批次 110-117 已完成，P1 全部修复 ✅）

| 批次 | PR | main commit | 修复项 | 状态 |
|------|-----|-------------|--------|------|
| 110 | #354 | `20a8c11` | P0-1/P0-2/P0-3 webhook callback PUBLIC_PATHS + message_type/title + payload 接入业务 | ✅ |
| 111 | #355 + 621cb0a | `20a8ce7` | P1-2 incoterms 接入 quotation_service + P1-10 audit 日期过滤 + crm keyword/source | ✅ |
| 112 | #356 | `6052810` | P1-9 api_keys 表 created_by 列持久化（migration m0039 + model + service + handler 透传） | ✅ |
| 113 | #357 | `9d65a72` | P1-1 webhook PUT 语义修复 + P1-7 占位符 2 处 + P1-8 let _ = 检查存在性 5 处 | ✅ |
| 114 | #358 | `36a9730` | P1-6 通知路径 warn 日志化（10 处）+ P1-5 启动期 expect 安全化（3 处中风险） | ✅ |
| 115 | #359 | `e9f3996` | P1-3 删除未接入业务的 failover 抽象模块（4 文件 1015 行 + 2 集成测试） | ✅ |
| 116 | #360 | `5e00b04` | P1-4 删除未接入业务的 Redis 缓存层模块（2 文件 504 行 + 清理 user/product service cache 代码 105 行） | ✅ |
| 117 | #361 | `dd19874` | P1-5 剩余 4 处生产代码 .unwrap()/.expect() 安全化（webhook_signature 返回 Result + date_utils/timeout expect 加不变量注释） | ✅ |

### v7 复审 P1 修复总结

P1 项全部修复完成（P1-1 ~ P1-10）：
- P1-1 webhook PUT 语义 ✅（批次 113）
- P1-2 incoterms 接入 ✅（批次 111）
- P1-3 failover 模块删除 ✅（批次 115）
- P1-4 cache 模块删除 ✅（批次 116）
- P1-5 .unwrap()/.expect() 安全化 ✅（批次 114 中风险 + 批次 117 低风险）
- P1-6 通知路径 warn 日志化 ✅（批次 114）
- P1-7 占位符 ✅（批次 113）
- P1-8 let _ = 检查存在性 ✅（批次 113）
- P1-9 api_keys created_by 持久化 ✅（批次 112）
- P1-10 audit 日期过滤 ✅（批次 111）

### v7 复审 P2 修复进度（批次 118-120 全部完成 ✅，13/13 项）

| 批次 | PR | main commit | 修复项 | 状态 |
|------|-----|-------------|--------|------|
| 118 | #362 | `01c4475` | P2-9 supplier_handler 资质端点真实接入 + P2-6 cost_collection 3 函数删除 + P2-4 report/ds cleanup_expired_cache 删除 + P2-8 fixed_asset calculate_monthly_depreciation 删除 + P2-13 websocket connection_count 删除 | ✅ |
| 119 | #363 | `fd4faf7` | P2-2 删除 token_bucket.rs 整个文件 + P2-5 删除 data_permission check_data_permission + 4 scope 常量 + P2-7 删除 assist_accounting create_assist_record | ✅ |
| 120 | #364 | `4842e97` | P2-7 initialize_dimensions 真实接入 main.rs 启动 + P2-10 删除 EventBackend trait + BroadcastBackend + BridgeStream + EventBackendType + backend_type | ✅ |

**批次 120 修复明细**：
- P2-7 真实接入：assist_accounting_service.rs initialize_dimensions 移除 `#[allow(dead_code)]`，main.rs 启动时调用一次（init_event_bus_with_kafka_config 之后），初始化 8 个辅助核算维度（批次/色号/缸号/等级/车间/仓库/客户/供应商），幂等实现，tracing::warn! 降级不阻塞启动
- P2-10 删除：event_bus.rs 删除 EventBackend trait + BroadcastBackend struct + impl + BridgeStream struct + impl + EventStream/SubscribeFuture 类型别名 + EventBusState.broadcast 字段 + backend_type() 方法 + EventBackendType 枚举；删除 tests/test_event_bus.rs（依赖被删除类型）
- clippy 修复：模块文档注释行首 `+ ` 被误判为 Markdown 列表项标记（doc_lazy_continuation lint），改为顿号分隔

**P2 项状态总览**（v7 复审报告 P2 + batch103-placeholder-impl-plan.md P2 合并，全部完成 ✅）：
- P2-1 incoterms 接入 ✅（批次 111，归入 P1-2）
- P2-2 token_bucket 限流算法 ✅（批次 119 删除）
- P2-3 admin_checker 缓存清理 ✅（批次 103）
- P2-4 report/ds cleanup_expired_cache ✅（批次 118）
- P2-5 data_permission check_data_permission + 4 scope 常量 ✅（批次 119 删除）
- P2-6 cost_collection 3 函数 ✅（批次 118）
- P2-7 assist_accounting create_assist_record ✅（批次 119 删除）；initialize_dimensions ✅（批次 120 真实接入 main.rs 启动）
- P2-8 fixed_asset calculate_monthly_depreciation ✅（批次 118）
- P2-9 supplier 资质端点真实接入 ✅（批次 118）
- P2-10 event_bus EventBackend trait ✅（批次 120 删除）
- P2-11 cache/redis_client ✅（批次 116 删除）
- P2-12 failover ✅（批次 115 删除）
- P2-13 websocket connection_count ✅（批次 118）

### v8 全项目复审修复进度（批次 121+ 进行中）

v7 复审 P0/P1/P2 项全部修复完成（P0 4 项 + P1 10 项 + P2 13 项 = 27 项）后，启动 v8 全项目复审扫描。

**v8 复审结果**（Task 子代理扫描 + 人工验证）：
- P0 项：color_card_scan_service（**误报**，scan_export.rs 调用）
- P1 真实项：crm_customer_handler list_tags 硬编码标签、search/elastic.rs stub 实现、event_kafka KafkaEventEnvelope（已处理）
- P1 误报：stock_alert（sensitive_action_alert 使用）、config/failover（failover_service 使用）、sales_analysis_service（handler 使用）
- P2 项：print_handler 空列表占位、import_export_handler 空列表占位、report_enhanced_handler 硬编码字段定义、financial_analysis_handler 假执行状态、inventory_stock_query alert_type 硬编码

| 批次 | PR | main commit | 修复项 | 状态 |
|------|-----|-------------|--------|------|
| 121 | #365 | `71b9bfb` | v8 死代码清理：删除 event_kafka KafkaEventEnvelope struct + from_event + into_event（零业务调用方），保留 event_type_name 标记 #[cfg(test)] | ✅ |
| 122 | #366 | `f181e1b` | v8 P1 crm 标签真实接入：新增 crm_tag 表 + list_tags/create_tag/delete_tag 真实持久化 + 路由路径 /crm-tags → /crm/tags 修复前端 404 | ✅ |
| 123 | #367 | `a819ab4` | v8 P1 ElasticClient::real() 真实实现：ClientInner enum 双模式（Mock/Real）+ reqwest 直连 ES REST API + ensure_indices 启动时索引初始化 | ✅ |
| 124 | #368 | `bbdf267` | v8 P1 SearchSyncer 接入 customer_service：PG→ES 写入同步（create/update/delete 事务提交后调用 sync_customer，最终一致性策略） | ✅ |
| 125 | #369 | `c4a269f` | v8 P1 SearchSyncer 接入 sales_order_service + product_service：PG→ES 写入同步（含 Decimal→f64 转换 + 硬删除 ES 文档删除 + start_event_listener 签名扩展） | ✅ |
| 126 | #370 | `2674df1` | v8 P2 print_handler 静态配置化（6 种内置打印模板）+ inventory_stock_query alert_type 派生计算（discrepancy/out_of_stock/low_stock/expiring/normal） | ✅ |
| 127 | #371 | `66cbe81` | v8 P2 import_export_handler 接入 import_tasks 表：list_import_tasks 真实查询 + import_csv/import_excel 创建+更新任务记录 | ✅ |
| 128 | #372 | `09601cb` | v8 P2 report_enhanced_handler 字段定义静态配置化：ReportFieldDefinition struct + available_fields_for_type 静态方法替代硬编码 serde_json::json! | ✅ |
| 129 | #373 | `8bd404b` | v8 P2 financial_analysis_handler execute_report 真实执行：调用 calculate_indicators 真实计算财务指标 + ExecuteReportParams 可选 period 参数 + 透明响应 completed/no_data | ✅ |

**批次 121 修复明细**：
- 删除 event_kafka.rs 中 KafkaEventEnvelope struct + from_event + into_event（74 行，零业务调用方）
- 保留 event_type_name 供测试断言使用，标记 #[cfg(test)] 避免非测试编译时 dead_code
- **CI 失败教训**：首次误删 report/ds.rs + report/job.rs（v8 子代理误报为死代码），CI 报 `no method named 'execute_report' found for struct 'ReportEngineService'`。根因：ds.rs 包含 `impl ReportEngineService { pub async fn execute_report ... }` 跨文件 impl 块，被 report_engine_handler 等调用。修复：从 HEAD~1 恢复 ds.rs + job.rs + mod.rs，仅保留 KafkaEventEnvelope 删除，force push 后 CI 全绿

**批次 122 修复明细**：
- 新增 migration m0040 + SQL：创建 crm_tag 表（id/name/color/category/created_by/created_at/updated_at），初始化 5 个预定义标签（VIP/重点客户/潜在客户/新客户/流失客户）保证向后兼容
- 新增 crm_tag entity 模型并在 models/mod.rs 注册
- list_tags：原返回硬编码 5 个标签，改为查 crm_tag 表真实数据
- create_tag：原用时间戳生成假 id 不持久化，改为 INSERT 到 crm_tag 表；CreateTagDto 增加 category 字段
- delete_tag：原直接返回 {deleted: true} 空操作，改为 DELETE FROM crm_tag，不存在时返回 404
- **路由路径修复**：/crm-tags 改为 /crm/tags 匹配前端 crm-enhanced.ts 调用（原前端 404 bug）
- 返回结构含 id/name/color/category/created_at 完整字段，匹配前端 CustomerTag interface

**批次 123 修复明细**：
- `elastic.rs`：新增 `ClientInner` enum 双模式（Mock 内存 HashMap / Real reqwest::Client），`ElasticClient` 改持有 `inner: ClientInner`
- `ElasticClient::real(url)` 从 stub（返回 mock storage）改为真实创建 reqwest::Client（30s timeout），消除"日志显示真实但实际 mock"的误导
- 4 个 SearchClient trait 方法全部支持 Real 模式：
  - `index_doc`: PUT /{index}/_doc/{id}
  - `search`: POST /{index}/_search，构建 ES Query DSL（multi_match + term filter + highlight）
  - `delete_doc`: DELETE /{index}/_doc/{id}（404 视为幂等成功）
  - `bulk_index`: POST /_bulk NDJSON 格式（action_header\n + source\n）
- 新增 `ensure_indices(base_url)` async 函数 + 3 个索引 mapping 定义（sales_orders/customers/products）
  - 索引幂等创建：PUT /{index} 返回 200（创建成功）或 400（已存在），均视为成功
- `main.rs`：在 `initialize_dimensions` 之后调用 `ensure_indices()`（仅当 ELASTICSEARCH_URL 配置时，错误降级 tracing::warn! 不阻塞启动）
- `search/mod.rs`：导出 `ensure_indices` 供 main.rs 调用
- `app_state.rs`：更新 `init_search_client()` 注释说明 real() 已真实实现
- **CI clippy 修复**（首次推送 3 项新警告）：bulk_index Mock 分支 storage 改为 _、search Real 分支 filter_map 改为 map、barcode_scanner_handler ScanHistoryQuery.scan_type 加 #[allow(dead_code)] + TODO
- **架构说明**：本批次仅完成 ES 客户端基础设施 + 索引初始化，SearchSyncer 接入 PG→ES 写入同步留待后续批次

**批次 124 修复明细**：
- `customer_service.rs`：CustomerService 注入 `search_syncer: Arc<SearchSyncer>` 字段，构造函数签名改为 `new(db, search_client: Arc<dyn SearchClient>)`
- 新增 `build_customer_doc(model: &customer::Model) -> CustomerDoc` 字段映射工具函数
  - tier 映射 customer_type（retail/wholesale/vip，更接近"等级"语义）
  - 其余字段直接从 Model 取值（id/code/name/contact_person/phone/email/address）
- 新增 `sync_customer_to_es(model, operation)` 私有方法（最终一致性策略，ES 失败仅 tracing::warn! 不回滚 PG）
- `create_customer` / `update_customer` / `delete_customer` 事务提交后调用 `sync_customer_to_es`
  - 软删除不删除 ES 文档，保留便于搜索历史客户（status 字段已同步）
- `elastic.rs`：移除 `SearchSyncer` struct 级别 `#[allow(dead_code)]`（已接入 customer_service）
- `elastic.rs`：`sync_sales_order` / `sync_product` 保留方法级 `#[allow(dead_code)]`（批次 125 接入）
- `search/mod.rs`：新增导出 `SearchSyncer` 供 customer_service 注入
- `customer_handler.rs`（5 处）+ `crm_customer_handler.rs`（4 处）：调用点改为 `CustomerService::new(state.db.clone(), state.search_client.clone())`
- **设计决策**：
  - ES 同步失败仅记录日志，不回滚 PG 事务（最终一致性，PG 是主数据源，ES 是搜索副本）
  - 同步时机：事务提交后同步（避免 PG 回滚后 ES 残留脏数据）
  - 软删除保留 ES 文档（status=inactive 同步，便于搜索历史客户）
  - mock 模式（CI 环境）下同步到内存 HashMap，real 模式同步到真实 ES

**批次 125 修复明细**：
- `services/so/order.rs`：SalesService struct 注入 `search_syncer: Arc<SearchSyncer>` 字段，`new()` 签名改为 `new(db, search_client: Arc<dyn SearchClient>)`
- `services/so/order_crud.rs`：新增 `decimal_to_f64` 工具函数（Decimal→f64，使用 `to_string().parse()` 避免精度损失），新增 `build_sales_order_doc` 静态方法（SalesOrderDetail→SalesOrderDoc，items.color_no 空串→None），新增 `sync_sales_order_to_es` 私有方法
- `create_order` / `update_order`：事务提交后调用 `get_order_detail` 取最新数据 → `sync_sales_order_to_es`（最终一致性策略）
- `delete_order`：删除前保存 `order_no_for_es`，事务提交后调用 `search_syncer.delete_sales_order(order_no)`（硬删除 ES 文档）
- `services/product_service.rs`：ProductService struct 注入 `search_syncer: Arc<SearchSyncer>`，`new()` 签名改为 `new(db, search_client: Arc<dyn SearchClient>)`，新增 `build_product_doc`（product::Model→ProductDoc，category/color_no/pantone_code 暂设 None，后续迭代 join 关联表）+ `sync_product_to_es`
- `create_product`：insert 后 `sync_product_to_es("create")`
- `update_product`：update_with_audit 后 `sync_product_to_es("update")`
- `delete_product`：delete_with_audit 后 `search_syncer.delete_product(id)`（硬删除 ES 文档）
- `search/elastic.rs`：移除 `sync_sales_order` / `sync_product` 的 `#[allow(dead_code)]`（已真实接入），新增 `delete_sales_order(order_no)` 和 `delete_product(product_id)` 方法（调用 `client.delete_doc`）
- `search/mod.rs`：新增导出 `SalesOrderItemDoc` 供 `order_crud.build_sales_order_doc` 使用
- `handlers/sales_order_handler.rs`（16 处）+ `handlers/product_handler.rs`（12 处）：调用点改为 `XxxService::new(state.db.clone(), state.search_client.clone())`
- `services/event_bus.rs`：`start_event_listener` 签名扩展为 `(db, search_client: Arc<dyn SearchClient>)`，2 处闭包内 `SalesService::new(db.clone())` 改为 `SalesService::new(db.clone(), search_client.clone())`
- `main.rs`：`start_event_listener` 调用点更新传入 `app_state.search_client.clone()`
- **CI 修复**：首次推送 Rust 后端构建失败 `error[E0432]: unresolved import crate::search::SalesOrderItemDoc`，根因 `search/mod.rs` 的 `pub use` 列表遗漏 `SalesOrderItemDoc` 导出，补导出后 CI 全绿
- **设计决策**：
  - 硬删除 vs 软删除：销售订单/产品硬删除需调用 ES delete_doc；客户软删除保留 ES 文档（批次 124 实现）
  - Decimal→f64 转换：使用 `to_string().parse()` 避免 `Decimal::to_f64` 边界值精度损失
  - 字段映射：SalesOrderDoc.items.color_no 空字符串→None；ProductDoc.category/color_no 暂设 None（后续迭代 join product_category/product_color 表）

**批次 126 修复明细**：
- `handlers/print_handler.rs`：新增 `builtin_print_templates()` 静态函数返回 6 种内置打印模板
  - 对应 PrintService 支持的 6 种单据类型：sales_order/sales_contract/purchase_order/purchase_receipt/inventory_transfer/voucher
  - `list_print_templates`: 从原 `vec![]` 占位改为返回 `builtin_print_templates()`
  - `get_print_template`: 从原硬编码 `Err(not_found)` 改为从内置列表按 id 查找，找不到时返回 404
- `services/stock_alert.rs`：重写 AlertType 枚举
  - 删除死代码 AlertLevel 枚举（sensitive_action_alert.rs 有独立 AlertLevel，本模块零业务调用方）
  - AlertType 接入 inventory_stock_query.compute_alert_type 业务，移除 dead_code 标注
  - 新增 OutOfStock 变体（缺货）+ code() 方法返回前端约定的稳定字符串
  - 新增 ALERT_TYPE_NORMAL 常量 + EXPIRING_THRESHOLD_DAYS 常量（默认 30 天）
- `services/inventory_stock_query.rs`：
  - 新增 `compute_alert_type(s: &inventory_stock::Model) -> &'static str` 私有函数
    判定优先级：discrepancy（状态异常）> out_of_stock（缺货）> low_stock（低于下限）> expiring（即将过期）> normal
  - `get_stock_alerts`: alert_type 字段从硬编码 "normal" 改为 `compute_alert_type(&s)` 派生计算
  - 返回字段新增 reorder_point / expiry_date / stock_status，便于前端展示阈值上下文
- **设计说明**：
  - 打印模板为系统内置，不需要动态 CRUD 管理（实际渲染逻辑在 PrintService.generate_pdf）
  - alert_type 派生计算基于库存数量/补货点/过期日期/库存状态
  - TODO(tech-debt): OverStock（高于上限）和 SlowMoving（滞销）暂未实现，因 inventory_stocks 表无 max_stock_point / last_movement_date 阈值字段，后续迭代补充字段后再接入

**批次 127 修复明细**：
- 新增 migration `m0041_create_import_tasks` + SQL：创建 import_tasks 表（id/import_type/status/total_rows/imported_rows/failed_rows/user_id/created_at/updated_at）+ 2 个索引（created_at DESC / user_id）
- 新增 `models/import_task.rs` SeaORM entity model，在 `models/mod.rs` 注册
- `services/import_export_service.rs`：ImportExportService 新增 3 个 task 管理方法
  - `create_import_task(import_type, total_rows, user_id) -> i32`：导入前创建任务记录（status=running）
  - `update_import_task(task_id, &ImportResult)`：导入完成更新 imported_rows/failed_rows/status（success/failed/partial）
  - `list_import_tasks() -> Vec<import_task::Model>`：按 created_at DESC 倒序返回最近 100 条
- `handlers/import_export_handler.rs`（3 处修改）：
  - `import_csv`：解析 CSV 后 create_import_task，验证失败/导入完成两条路径均 update_import_task（tracing::warn! 降级不阻断）
  - `import_excel`：同 import_csv 模式
  - `list_import_tasks`：从 `vec![]` 占位改为调用 service.list_import_tasks() 真实查询 + Model→ImportTaskItem DTO 映射（i64→u64 安全转换，DateTime→RFC3339）
- **状态判定规则**：failed==0→success；imported==0→failed；其他→partial
- **CI 修复**：首次推送 3 个错误
  - E0433: list_import_tasks 签名使用 import_task::Model 但 use 在函数体内 → 改用全路径 `crate::models::import_task::Model`
  - E0282: handler 中 tasks 类型推导失败（级联错误）→ 修复 E0433 后自动解决
  - E0599: `.limit(100)` 方法未找到 → 函数体内 `use sea_orm::{QueryOrder, QuerySelect}`
- **设计决策**：
  - task 创建时机：解析 CSV/Excel 后、验证前（确保验证失败也落库一条记录）
  - task 更新失败不阻断主流程（仅 tracing::warn!），保证用户得到原始导入响应
  - total_rows = 解析后的实际行数（不含表头），imported_rows/failed_rows 由 ImportResult 提供
  - 限制 100 条记录避免列表过大（按 created_at DESC 倒序）

**批次 128 修复明细**：
- `services/report_template_service.rs`：新增 `ReportFieldDefinition` struct（field/title/data_type，&'static str）+ `available_fields_for_type(template_type: &str) -> Vec<ReportFieldDefinition>` 静态方法
  - 集中管理 5 种模板类型 + 通配符的字段定义：sales/purchase/inventory/financial/custom
  - 类型化 struct 替代 `serde_json::json!` 宏，编译时检查
  - `&'static str` 避免运行时 String 分配，零成本抽象
- `handlers/report_enhanced_handler.rs`：`get_available_fields` 从 38 行硬编码 match + `serde_json::json!` 块改为调用 `ReportTemplateService::available_fields_for_type(&template_type)`
  - 返回 JSON 结构完全向后兼容（field/title/data_type 三字段不变）
- **设计决策**：
  - 字段元数据绑定 DB schema（sales_orders 表有 order_no 列等），不宜放数据库动态管理
  - 采用静态配置化模式，与 print_handler 批次 126 一致
  - 已存在 `report_definition` 表但零业务引用（死表），本批次未复活该表（字段定义与报表定义是不同概念，前者是 schema 元数据，后者是用户自定义报表模板）

**批次 129 修复明细**：
- `handlers/financial_analysis_handler.rs`：重写 `execute_report` handler，从假执行状态改为调用 `calculate_indicators` 真实计算
  - 新增 `ExecuteReportParams` struct（可选 `period` 参数，格式 YYYY-MM，缺失时默认当前年月）
  - 验证指标存在（`financial_analysis::Entity::find_by_id`）
  - 调用 `service.calculate_indicators(&period, auth.user_id)` 真实计算所有预定义财务指标
    - 读取指定期间的科目余额（account_balance）
    - 按科目代码前缀分类汇总（1xxx 资产 / 2xxx 负债 / 6xxx 损益）
    - 计算流动比率/速动比率/资产负债率/应收账款周转率/应付账款周转率
    - 落库到 financial_analysis_results 表
  - 从计算结果中筛选当前指标 `all_results.iter().find(|r| r.indicator_id == id)`
  - 透明响应：有结果返回 `completed` + 计算值；无结果返回 `no_data` + 说明（可能缺少科目余额或指标未在预定义列表中）
  - 返回字段新增 `period` 和 `total_indicators_computed`，便于前端展示执行上下文
- 移除未使用的 `use sea_orm::QueryOrder` 和 `financial_analysis_result` 模型导入
- **设计决策**：
  - 财务指标相互关联（流动比率需要流动资产 + 流动负债），无法孤立计算单个指标，因此 `calculate_indicators` 会计算所有预定义指标，本接口从中筛选当前指标返回
  - `calculate_indicators` 是幂等的：每次执行会重新读取科目余额并落库新结果（不删除旧记录，保留历史趋势数据）
  - 透明响应 `no_data` 状态：当指标不在预定义列表（CURRENT_RATIO/QUICK_RATIO/DEBT_ASSET_RATIO/AR_TURNOVER/AP_TURNOVER）中时，例如自定义指标，会返回 `no_data` 而非 `completed`，避免误导用户

### v8 复审 P2 修复总结

P2 项全部修复完成（5/5）：
- ✅ P2：print_handler 空列表占位（批次 126，静态配置化 6 种内置打印模板）
- ✅ P2：inventory_stock_query alert_type 硬编码（批次 126，派生计算 discrepancy/out_of_stock/low_stock/expiring/normal）
- ✅ P2：import_export_handler list_import_tasks 空列表占位（批次 127，新建 import_tasks 表 + handler 真实接入）
- ✅ P2：report_enhanced_handler 硬编码字段定义（批次 128，静态配置化 ReportFieldDefinition struct）
- ✅ P2：financial_analysis_handler 假执行状态（批次 129，调用 calculate_indicators 真实计算）

### 下一步：启动 v9 全项目复审

v8 复审 P0/P1/P2 项全部修复完成（P1 5 项 + P2 5 项 = 10 项）。启动 v9 全项目复审，循环直到复审没有问题。

v9 复审维度（沿用 v8）：
1. 死代码扫描（`#[allow(dead_code)]` + TODO 标记的预留 API/功能）
2. 占位符 TODO 扫描（stub / placeholder / `let _ =` 占位）
3. 未接入功能扫描（路由 → handler → service → model → DB 全链路检查）
4. 硬编码假数据扫描（`vec![]` / `serde_json::json!` / 硬编码字符串状态）
5. unwrap/expect 残留扫描（生产代码中的 panic 风险）

### 历史批次索引

| 批次范围 | 主要内容 | 状态 |
|---------|----------|------|
| 103-109 | v7 复审启动：search_api 真实接入/cache_service 接入 AppState/messaging 删除/webhook 真实接入 | ✅ |
| 96-102 | v5/v6 复审 P0/P1/P2/P3 修复（ArService 真实实现 + 状态机 lock_exclusive + 状态字符串常量化） | ✅ |
| 85-95 | v2/v3/v4 复审 P0-P3 修复（事务边界 + spawn panic 隔离 + FOR UPDATE） | ✅ |
| 49-84 | v19 P0/P1/P2/P3 修复（早期审计修复） | ✅ |
| 1-48 | 早期修复（前端权限/路由/API 断链/安全漏洞） | ✅ |

详细历史：见 [CHANGELOG.md](file:///workspace/.monkeycode/CHANGELOG.md) 与 [docs/archives/](file:///workspace/.monkeycode/docs/archives/)

---

## 三、文件定义

| 文件 | 用途 | 说明 |
|------|------|------|
| `MEMORY.md` | 项目规则记忆 | 规则、规范、关键经验（必须遵守） |
| `doto.md` | 任务与历史 | 当前任务 + 历史归档索引（实时更新） |
| `CHANGELOG.md` | 任务精简总结 | 任务一句话摘要列表（PR 完成后更新） |
| `bug.md` | 漏洞登记 | 实时检测与修复（修复后删除条目） |
| `docs/archives/` | 历史归档 | 已优化前的完整内容（按日期保留） |

---

## 四、基础规范

### 沟通语言
- 使用中文进行回复和沟通

### 编码规范
- 禁止硬编码，所有文本需使用中文
- 代码注释必须使用中文

### 项目标识
- 项目名称统一（以 main 仓库 README 为准），所有文档/界面/输出信息一致

### 开发辅助
- 每次新增或修改功能时，必须调用合适的技能或 MCP 工具
- 严格按照技能规范进行开发

### 任务管理
- 使用中文建立待办任务（doto.md）
- 每完成一个待办任务，立即标记为"已完成"

### 记忆管理
- 实时查看和更新 `MEMORY.md` 规则记忆文档
- 关键内容存储在 `MEMORY.md`，变更记录到 `CHANGELOG.md`
- **路径策略（2026-06-19 确认）**：test 分支合并入 main 时 `-X theirs` 会覆盖 `.monkeycode/`，必须以 main 版本为准

### 死代码与未使用文件处理
- **不使用的文件/代码/文件夹必须删除**（删除前评估影响范围，删除后更新受影响文件）
- 修改文件后保存前**必须交叉自审**（检查引用、配置、文档是否同步）
- **功能必须接入项目**（尽可能减少 TODO，禁止遗留占位代码）

### Bug.md 实时漏洞管理
- **实时检测** `.monkeycode/bug.md` 漏洞文件
- 发现漏洞 → 立即启动修复（按 P0/P1/P2 优先级）
- **修复一个漏洞后立即从 bug.md 删除对应条目**
- 所有漏洞修复完成后保留 `bug.md` **空文件**（不删除，作为漏洞登记占位）

### 任务规划管理
- 所有任务规划文件保存在 `.monkeycode/docs/` 下

### 数据库配置
- 数据库类型：PostgreSQL
- 连接方式：远程数据库连接模式

### 功能实现依据
- 新增功能接口、数据库操作需遵循现有规范

### 打包与发布要求
- 打包时必须进行全面测试：功能测试、兼容性测试、稳定性测试

---

## 五、安全规范

### 敏感信息保护
- 禁止硬编码敏感信息（密码、密钥、令牌等）
- 使用环境变量或配置管理工具

### 输入验证
- 所有用户输入必须验证和清理
- 使用参数化查询防止 SQL 注入
- 对输出进行编码防止 XSS 攻击

---

## 六、CI/CD 强制

### 本地编译禁止
- **禁止**本地编译验证（`cargo build` / `cargo check` / `cargo test` / `cargo fmt -- --check` / `cargo clippy` / `npm run build` / `vue-tsc` / `pnpm typecheck` 等）
- **禁止**本地启动服务做端到端验证
- 所有验证走 GitHub Actions CI：修改代码 → commit → push → 监控 run → 失败拉 logs → 修复 → 重 push
- **唯一允许的本地操作**：文件 diff、语法、文本类（git status、cat、grep、sed、Edit、Write）

### CI 监控 API
- `/repos/{owner}/{repo}/commits/{sha}/check-runs` —— 查询 check run 状态
- `/repos/{owner}/{repo}/actions/runs/{id}/logs` —— 下载 logs zip
- `/repos/{owner}/{repo}/check-runs/{id}/annotations` —— 错误标注
- `/repos/{owner}/{repo}/actions/runs/{id}/jobs` —— 查询 job 列表

### 服务器环境
- 服务名称：bingxi-backend（systemd），安装目录：`/opt/bingxi-erp`
- 后端端口：8082，日志目录：`/opt/bingxi-erp/backend/logs`，备份目录：`/opt/bingxi-erp/backups`
- 环境配置：`/etc/bingxi-erp/.env`
- 部署命令：`bingxi update`（CLI 工具）
- 部署方式：CICD 构建 → GitHub Release → 手动部署到生产服务器
- **禁止** Docker 容器部署（不得创建 Dockerfile、docker-compose.yml）

### 部署限制
- 不安装 PostgreSQL 客户端（用远程数据库 39.99.34.194:5432）
- 不安装 Redis（用远程 Redis 服务器）
- 只需安装 Nginx、curl

---

## 七、核心经验（关键排错与开发经验）

### 集成测试跨 crate 调用私有函数
- `tests/` 目录下的集成测试编译为**独立二进制 crate**，`fn foo()` 对集成测试 crate 不可见
- 修复：`fn foo()` → `pub fn foo()`（或使用 `pub(crate)` 限制可见性）
- 错误模式：`error[E0624]: associated function compose_color_no is private`

### 沙箱网络限制
- **限制**：沙箱环境出站 22 端口（github.com SSH）被防火墙阻断
- **可用**：443 端口（github.com HTTPS）正常，包括 `git push` HTTPS 远程
- **应对策略**：沙箱内可通过 HTTPS 完成 commit → push → CI 全流程

### .monkeycode 目录 gitignore 规则
- `.gitignore` 默认忽略 `.monkeycode/`，仅白名单：`MEMORY.md` / `doto.md` / `bug.md` / `CHANGELOG.md`
- `.monkeycode/docs/` 子目录不在白名单
- **添加新归档文件**必须用 `git add -f` 强制添加

### 集成测试 `crate` 语义
- `tests/` 目录下的集成测试编译为独立二进制，`crate` 关键字指向**测试二进制本身**
- 引用 lib.rs 暴露的模块必须用 `Cargo.toml` 中的 `name` 字段（连字符 `-` 转下划线 `_`），即 `bingxi_backend`
- 单元测试（`src/` 内的 `#[cfg(test)]`）中 `crate` 指向 lib，两者语义不同

### Clippy Baseline 脆弱性
- `backend/.clippy-baseline.txt` 用 `comm -23` 精确行比较检测"新警告"
- 修改单行代码会导致 baseline 中后续行号全偏移，触发大量"假新警告"
- **修复**：删除 `backend/.clippy-baseline.txt`，让 CI 在 bootstrap 模式下重建
- **快速诊断**：CI 误报"大量新警告"时，先检查 baseline 首行内容

### Cache::get 返回值语义
- `backend/src/utils/cache.rs` 的 `Cache` trait 定义 `fn get(&self, key: &K) -> Option<V>`，返回值已 **Clone**（不是 `Option<&V>`）
- 不能在结果上调用 `.copied()`（仅 `Option<&T>` 或迭代器支持）

### JTI 黑名单→Redis 迁移设计
- **现状**：`auth_service.rs` 用 `static JTI_BLACKLIST: LazyLock<RwLock<HashMap<String, i64>>>`，多实例不共享
- **迁移方案**：优先用 Redis SETEX（`SET key value EX <ttl>`），TTL 到期自动清理
- **失败回退**：Redis 不可用时降级到原 HashMap（避免阻塞业务）

### SSRF 防护双重校验必要性
- **单次校验的弱点**：create 时校验 `url` 指向公网，但攻击者可注册合法公网域名后修改 DNS 记录为内网 IP（DNS Rebinding）
- **必须双重校验**：`create_webhook` 时校验 + `trigger_webhook` 发送前**再次**校验
- **校验内容**：协议白名单 + 主机名黑名单 + IP 黑名单（RFC1918/loopback/link-local 含云元数据 169.254.169.254）

### DashMap vs std::sync::Mutex 选型
- 高频/性能关键：DashMap
- 安全关键 + 锁中毒需防御：std::sync::Mutex + try_lock
- **关键模式**：`let Ok(mut g) = self.storage.try_lock() else { return; };`（Rust 1.65+ let-else）

### 日志脱敏按字符而非字节
- **风险**：截断 UTF-8 字符串用字节切片 `&s[..n]` 可能切到字符中间，panic
- **正确做法**：用 `chars().take(n)` 按 Unicode 字符截断

### totp-rs 5.5 熵源确认
- `Secret::generate_secret()` 内部用 `rand::thread_rng()` → `OsRng` → 操作系统 CSPRNG
- **安全等级**：密码学安全（160 bits 熵，符合 RFC 4226 推荐）

### GitHub Token 安全存储
- **绝不写入任何 git 跟踪文件**（.git/config / MEMORY.md / doto.md / CHANGELOG.md / commit message）
- **存储位置**：沙箱本地 `~/.git-credentials`（600 权限，git credential helper = store 自动读取）
- **沙箱网络限制**：SSH 22 端口被防火墙阻断，必须用 HTTPS push

### GitHub Actions Log 100KB 截断与详细日志获取
- **限制**：GitHub Web UI 的 CI run log 最多显示尾部 100KB
- **解决方案**：用 `https://api.github.com/repos/{owner}/{repo}/actions/jobs/{job_id}/logs` 获取**单 job 完整 log**

### u16 永真比较与 Clippy 极端比较警告
- **触发模式**：`x >= 0xff00 && x <= 0xffff`（u16 类型，`<= 0xffff` 永远为真）
- **Clippy lint**：`absurd_extreme_comparisons`
- **通用规则**：写数值比较前先想"类型边界"

### 分布式限流回退必须真实回退
- 错误设计：`check_redis_rate_limit` 返回 `Ok(true)`（未配置 Redis），`check_rate_limit` 直接放行
- 正确设计：返回 `Result<Option<bool>>`：`Ok(Some(allowed))` / `Ok(None)`（应回退）/ `Err(_)`（应回退）

### Cargo build --release vs cargo test 编译差异
- 某些编译错误在 `cargo test`（dev build）中不会触发，但在 `cargo build --release`（`opt-level=2`）会触发
- **CI 防护**：依赖 `🏗️ Rust 后端构建` job 跑 `cargo build --release` 早期发现问题

### `|| true` 反模式
- `assert!(some_expr.is_ok() || true)` 是恒真式断言，无测试价值却能**掩盖编译错误**
- CI 中应使用 `cargo check --tests` 或 `cargo test --no-run` 提前发现编译错误

### SeaORM Trait 必导
- `Entity::find()` → 需 `use sea_orm::EntityTrait;`
- `.filter()` → 需 `use sea_orm::QueryFilter;`
- `.gte()/.lt()/.gt()/.lte()/.eq()` → 需 `use sea_orm::ColumnTrait;`
- `.count()/.all()/.paginate()` → 需 `use sea_orm::PaginatorTrait;`
- 清理 sea_orm trait 导入时**不能批量删**，必须**逐个静态验证**

### Clippy Lint 名规范
- rustc builtin lint：`unused_variables` / `unused_imports` / `dead_code`（不带 `clippy::` 前缀）
- clippy 内置 lint：`clippy::redundant_clone` / `clippy::too_many_arguments` 等
- `clippy::unused_variables` 是**无效 lint 名**，触发 `unknown_lints` 警告

### Validator 限制
- `#[validate(length(max = X))]` 只支持**整数字面量**
- 必须用：`length(max = 10_485_760)` ✅ 而非 `length(max = 10 * 1024 * 1024)` ❌

### 子代理协作模式
- 大批量相似任务（如 40 个文件清理）使用 8 轮 × 5 个子代理的并行结构
- 子代理仅**编辑文件**，不直接推 PR；主代理汇总后开 1 个 PR
- 子代理不得操作 `.monkeycode/` 目录或 `CHANGELOG.md`（避免污染记忆）

### 子代理 sea_orm 清理警示
- 子代理清理 sea_orm trait 导入时**必须**先 grep 使用点，再决定是否删除

---

## 八、工作流协作

### 工作角色定位
- 主代理角色：总控（项目经理/架构师）
- 子代理（Task 工具）= 员工，负责具体执行
- 主代理职责：分析任务 → 拆解 → 分配 → 总结成果 → 推 PR

### GitHub 分支策略
- `main` 为主分支（正式版），不允许删除
- `test` 为测试分支，不允许删除
- 所有修复/功能变更在修复分支进行
- 验证后自动合并入 main
- 修复分支合并后自动删除

### 提交信息规范
- 使用中文编写提交信息
- 描述"做了什么"和"为什么"

### 代码审查
- 所有代码变更需经过审查
- 审查重点：代码质量、安全性、性能、测试覆盖

### 日志诊断技能自动触发
- 技能名：`/log-diagnosis` 日志诊断技能（自动触发）
- 触发关键词：日志、错误日志、异常日志、崩溃日志、服务器日志、traceId、错误码、异常堆栈
- 报告保存：`.diagnosis/reports/{YYYY-MM-DD}_{问题描述}.md`

---

## 九、代码规范

### 命名约定
- 使用有意义、描述性的名称
- 遵循项目或语言的命名规范
- 避免缩写和单字母变量（除约定俗成的，如循环中的 `i`）

### 代码组织
- 相关代码放在一起
- 保持适当的抽象层次
- 函数只做一件事，保持单一职责原则

### 注释与文档
- 注释解释"为什么"而不是"做什么"
- 为公共 API 提供清晰的文档
- 保持文档与代码同步更新

### 死代码处理规范
- **禁止**文件级 `#![allow(dead_code)]` 全局抑制（CI 会失败）
- **禁止**crate 级 `#![allow(unused_imports)]` / `#![allow(unused_variables)]`
- 真正未使用项**显式删除**（git 保留历史）；保留项加 `pub` 修饰或 `#[allow(dead_code)]` + TODO
- **例外**：`backend/src/models/` 下的 SeaORM 自动生成模型可保留文件级 `#![allow(dead_code)]`

### CI 死代码强制
- 配置：`backend/.clippy.toml` `warn` 段开启 `dead_code`/`unused_imports`/`unused_variables`
- 工作流：`.github/workflows/ci-cd.yml` `cargo clippy --all-targets -- -D warnings`
- 任何死代码警告都会让 CI 失败

---

## 十、性能与错误处理

### 数据库查询
- 优化查询，避免 N+1
- 使用适当索引
- 大数据量查询分页处理

### 缓存策略
- 合理使用缓存，明确失效策略
- 避免缓存过期数据

### 资源管理
- 及时释放不再使用的资源
- 避免内存泄漏
- 合理控制并发数量

### 错误处理
- 业务错误：返回友好提示
- 系统错误：记录详细日志，返回通用错误
- 验证错误：明确指出失败原因
- 尽可能实现优雅降级，提供重试机制

---

## 十一、文档与持续改进

### API 文档
- 所有 API 接口必须有文档：接口路径、请求参数、响应格式、示例

### 代码文档
- 复杂逻辑必须有注释说明
- 公共函数必须有文档注释
- 保持文档与代码同步更新

### 持续改进
- 定期审查代码质量，及时重构
- 记录技术债务，制定偿还计划
- 关注新技术发展，定期团队分享

---

## 十二、用户习惯与协作偏好（2026-07-05 整理）

> 本章固化用户在历次会话中明确表达的工作习惯与协作偏好，作为代理行为的强制约束。

### 批次修复工作流（用户 2026-06-27 确认）

> 开始进行修复，按修复一个批次，推送 ci，ci 全绿,合并到 main，删除修复分支，
> 修复下一个分支，创建修复分支，修复下一个分支依次类推，直到所有任务全部修复完成。
> 修复完成后进行全项目复审，对复审出来的问题按这个流程进行继续评估修复。直到复审没有问题。

**强制流程**：
1. 创建修复分支 `fix/batchN-xxx`
2. 修改代码（严格遵守 CI/CD Only，禁止本地编译）
3. `commit`（中文 commit message，描述"做了什么"和"为什么"）
4. `git push -u origin fix/batchN-xxx`
5. 创建 PR（中文标题 + 中文 body，列出修复项）
6. 监控 CI（用 GitHub API 轮询 check-runs，12 项必检全绿即可合并，E2E 非阻塞）
7. squash merge 到 main（commit_title 带 `(#PR号)` 后缀）
8. 删除本地 + 远程修复分支
9. `git checkout main && git pull origin main` 同步
10. 更新 MEMORY.md / doto.md / CHANGELOG.md 记录完成
11. 开始下一批次

**禁止**：
- 禁止积累多批未验证的修改
- 禁止本地 `cargo build` / `cargo test` / `npm run build` 等任何构建命令
- 禁止 `git push --force` 到 main / test 分支
- 禁止跳过 CI 直接合并（必须等 12 项必检全绿）

### 沟通偏好

- **回复语言**：中文（代码注释也用中文）
- **简洁高效**：直接给方案和结果，避免冗长解释
- **进度可见**：使用 TodoWrite 跟踪，每完成一项立即标记
- **错误透明**：遇到失败立即报告，不掩盖

### 记忆管理偏好

- `.monkeycode/` 文件夹定期整理优化（用户 2026-07-05 明确要求）
- 主文件（MEMORY/doto/CHANGELOG）保持精简，早期内容归档到 `docs/archives/YYYY-MM-DD/`
- 用户习惯和新规则必须写入项目规则文件（MEMORY.md 一、章节），不能只留在对话上下文
- 关键变更必须实时记录到 CHANGELOG.md

### CI 验证偏好

- **12 项必检全绿即可合并**：环境信息 / 依赖图 / Rust 构建 / Rust Clippy / Rust 格式 / Rust 单元测试 / 前端构建 / 前端格式 / 前端 ESLint / 前端类型检查 / 前端测试 / 依赖审计
- **E2E 非阻塞**：前端 E2E 测试 `continue-on-error`，不阻塞合并
- CI 失败时：用 `/actions/runs/{id}/jobs` 查 job 列表 → `/actions/jobs/{job_id}/logs` 拉单 job 完整 log（Web UI 限 100KB）

### 分支策略偏好

- `main`：主分支（正式版），不允许删除，不允许 force push
- `test`：测试分支，不允许删除
- 修复分支：`fix/batchN-简短描述`，合并后立即删除
- 修复分支可 force push（仅 amend commit message 时，无协作影响）

---

## 十三、归档索引

完整历史内容（优化前的详细记录）：

- 完整 MEMORY（2026-07-05 优化前）：`.monkeycode/docs/archives/2026-07-05/MEMORY-2026-07-05-pre-optimization.md`
- 完整 CHANGELOG（2026-07-05 优化前）：`.monkeycode/docs/archives/2026-07-05/CHANGELOG-2026-07-05-pre-optimization.md`
- 完整 doto（2026-07-05 优化前）：`.monkeycode/docs/archives/2026-07-05/doto-2026-07-05-pre-optimization.md`
- 完整 MEMORY（2026-06-24 优化前）：`.monkeycode/docs/archives/MEMORY-2026-06-24-pre-optimization.md`
- 完整 CHANGELOG（2026-06-24 优化前）：`.monkeycode/docs/archives/CHANGELOG-2026-06-24-pre-optimization.md`

历史审计报告：
- `.monkeycode/docs/audits/2026-06-29-strict-reaudit-v7.md` —— v7 第七轮全项目复审
- `.monkeycode/docs/audits/2026-06-28-strict-reaudit-v5.md` —— v5 第五轮复审
- `.monkeycode/docs/audits/2026-06-28-strict-reaudit-v6.md` —— v6 第六轮复审
