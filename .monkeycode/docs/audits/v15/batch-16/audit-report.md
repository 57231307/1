# V15 报表BI与通知协同审计报告（类十九·批次 16）

- **审计子代理**：V15 审计子代理（类十九报表BI与通知协同）
- **审计范围**：8 维度（报表定义/订阅推送/BI 分析/仪表板/通知中心/邮件服务/OA 公告与用户行为/五维度与页面浏览）
- **审计依据**：
  - `/workspace/.monkeycode/docs/audits/v15-review-plan-2026-07-15.md` 第 6273-6368 行（类十九）
  - `/workspace/backend/src/services/report_template_service.rs`
  - `/workspace/backend/src/services/report_subscription_service.rs`
  - `/workspace/backend/src/services/bi_analysis_service.rs`
  - `/workspace/backend/src/services/dashboard_service.rs`
  - `/workspace/backend/src/services/notification_service.rs`
  - `/workspace/backend/src/services/email_service.rs`
  - `/workspace/backend/src/services/email_log_service.rs`
  - `/workspace/backend/src/services/email_template_service.rs`
  - `/workspace/backend/src/services/event_notification_service.rs`
  - `/workspace/backend/src/services/five_dimension_service.rs`
  - `/workspace/backend/src/services/tracking_service.rs`
  - `/workspace/backend/src/services/report/mod.rs` / `report/job.rs` / `report/ds.rs`
  - `/workspace/backend/src/handlers/report_enhanced_handler.rs`
  - `/workspace/backend/src/handlers/bi_handler.rs`
  - `/workspace/backend/src/handlers/dashboard_handler.rs`
  - `/workspace/backend/src/handlers/email_handler.rs`
  - `/workspace/backend/src/handlers/tracking_handler.rs`
  - `/workspace/backend/src/handlers/five_dimension_handler.rs`
  - `/workspace/backend/src/models/report_template.rs` / `report_subscription.rs` / `report_definition.rs`
  - `/workspace/backend/src/models/notification.rs` / `notification_setting.rs` / `user_notification_setting.rs`
  - `/workspace/backend/src/models/email_log.rs` / `email_template.rs`
  - `/workspace/backend/src/models/oa_announcement.rs`
  - `/workspace/backend/src/models/page_view.rs` / `user_behavior.rs`
  - `/workspace/backend/src/routes/analytics.rs`
  - `/workspace/backend/src/main.rs`
- **审计方法**：Read 审计计划 + Grep 检索（report/bi/notification/email/announcement/page_view/five_dimension/permission/cache 等） + Read 关键文件 + 对照审计计划逐项核对
- **审计时间**：2026-07-16
- **审计原则**：只做审计不修改业务代码；所有审计内容使用中文；每个检查项附文件路径:行号证据

---

## 维度 1：报表定义与模板管理（19.1）

### 检查方法
- Read `/workspace/backend/src/services/report_template_service.rs`
- Read `/workspace/backend/src/models/report_template.rs`
- Read `/workspace/backend/src/models/report_definition.rs`
- Read `/workspace/backend/src/handlers/report_enhanced_handler.rs`
- Grep 检索 `report.*version` / `template.*version` / `报表.*权限` / `report.*permission`

### 发现

#### ✅ 已落实的项
1. **报表元数据基本完整**：`report_templates` 表包含 name/code/report_type/category/data_source/columns/filters/parameters/sort_by/sort_order/supported_formats/data_source_sql/description/is_public/status/created_by/created_at/updated_at 字段（`backend/src/models/report_template.rs:13-76`），覆盖名称/分类/数据源/参数/导出格式等核心元数据。
2. **SQL 注入风险已关闭**：`create` / `update` / `execute_custom_report` 三个入口均显式拒绝 `data_source_sql`，返回"出于安全考虑，自定义 SQL 报表功能已禁用"（`backend/src/services/report_template_service.rs:159-171, 255-260, 382-394`），P0-B 安全修复彻底关闭 SQL 注入攻击面。
3. **报表编码唯一性校验**：`create` 方法检查 `code` 是否已存在（`backend/src/services/report_template_service.rs:174-184`），避免重复创建。
4. **软删除语义**：`delete` 方法将 status 设为 INACTIVE（`backend/src/services/report_template_service.rs:303-321`），保留历史数据。
5. **访问权限基础校验**：`get_by_id` 校验 `is_public || created_by == user_id`（`backend/src/services/report_template_service.rs:226-231`），`update` / `delete` 仅创建者可操作（`:251-253, :310-312`）。
6. **分页参数防 DoS**：`list` 方法 `page_size.clamp(1, 100)` + `page.clamp(1, 1000)`（`backend/src/services/report_template_service.rs:330, 362`）。
7. **静态字段元数据**：`available_fields_for_type` 提供 sales/purchase/inventory/financial/custom 5 类报表的字段定义（`backend/src/services/report_template_service.rs:111-150`），替代原硬编码 JSON。

#### ❌ 缺陷项

**缺陷 1.1：报表模板无版本管理，无法回滚**
- **风险等级：P1**
- **证据**：
  - `/workspace/backend/src/models/report_template.rs:13-76`：`Model` 结构体无 `version` 字段，仅有 `updated_at`。
  - `/workspace/backend/src/models/report_definition.rs:13-52`：`report_definition` 表同样无 `version` 字段。
  - Grep 检索 `report.*version` / `template.*version` 全项目无匹配。
  - `update` 方法（`backend/src/services/report_template_service.rs:237-300`）直接覆盖原记录，未保留历史版本。
- **业务影响**：报表模板被修改后无法回滚到旧版本，业务报表配置变更失误时无法快速恢复，影响决策支持可用性。
- **修复建议**：新增 `report_template_versions` 表存储历史版本，每次 update 前将当前快照写入版本表，提供 `GET /reports/templates/:id/versions` 和 `POST /reports/templates/:id/rollback/:version` 接口。

**缺陷 1.2：报表权限未注册到权限系统，未按角色控制可见**
- **风险等级：P1**
- **证据**：
  - `/workspace/backend/src/services/report_template_service.rs:328-365`：`list` 方法仅按 `is_public || created_by == user_id` 过滤，未调用权限服务校验 `report:sales:view` 等权限码。
  - `/workspace/backend/src/handlers/report_enhanced_handler.rs:41-58, 61-74`：`create_report_template` / `list_report_templates` 仅依赖 `AuthContext`，无 `permission_check`。
  - Grep 检索 `report.*permission` / `report:sales:view` / `报表.*权限` 全项目无匹配。
- **业务影响**：报表包含销售/采购/库存/财务等敏感数据，任何登录用户都能创建/查看公开报表，财务报表可能被非财务角色查看，违反最小权限原则。
- **修复建议**：在 `permission` 表中注册 `report:sales:view` / `report:purchase:view` / `report:inventory:view` / `report:finance:view` 等权限码，`list` / `get_by_id` / `execute_custom_report` 调用 `PermissionService::check` 校验。

**缺陷 1.3：报表元数据缺少刷新策略字段**
- **风险等级：P2**
- **证据**：
  - `/workspace/backend/src/models/report_template.rs:13-76`：`Model` 无 `refresh_strategy` / `refresh_interval` / `cache_ttl` 字段。
  - `/workspace/backend/src/services/report_template_service.rs` 全文未涉及刷新策略。
- **业务影响**：报表数据时效性无法配置，实时报表与日报表无法区分，可能因频繁查询压垮数据库或因缓存过长提供过期数据。
- **修复建议**：新增 `refresh_strategy`（REALTIME/HOURLY/DAILY）和 `cache_ttl_seconds` 字段，`execute_custom_report` 根据策略选择是否走缓存。

**缺陷 1.4：报表参数校验不足（parameters/filters 为裸 JSON）**
- **风险等级：P2**
- **证据**：
  - `/workspace/backend/src/services/report_template_service.rs:22-39`：`CreateReportTemplateRequest` 的 `columns: serde_json::Value` / `filters: Option<serde_json::Value>` / `parameters: Option<serde_json::Value>` 无 `#[derive(Validate)]`，无字段级校验。
  - 对比 `email_template_service.rs:23` 已派生 `Validate`，但 `CreateReportTemplateRequest` 未派生。
- **业务影响**：攻击者可提交任意 JSON 结构污染 columns/filters/parameters，虽然 SQL 入口已关闭，但仍可能影响后续报表渲染逻辑。
- **修复建议**：为 `CreateReportTemplateRequest` 派生 `Validate`，定义强类型 `ReportColumnDef` / `ReportFilterDef` / `ReportParameterDef` 子结构并约束字段。

---

## 维度 2：报表订阅与定时推送（19.2）

### 检查方法
- Read `/workspace/backend/src/services/report_subscription_service.rs`
- Read `/workspace/backend/src/models/report_subscription.rs`
- Read `/workspace/backend/src/services/report/job.rs`
- Read `/workspace/backend/src/handlers/report_enhanced_handler.rs:293-340, 464-497`
- Read `/workspace/backend/src/routes/analytics.rs:99-157`
- Grep 检索 `run_subscription` / `subscription.*runner` / `推送失败` / `tokio::spawn.*subscription`

### 发现

#### ✅ 已落实的项
1. **订阅元数据完整**：`report_subscriptions` 表包含 name/template_id/frequency/recipients/parameters/export_format/is_enabled/status/next_run_at/last_run_at/last_run_status/last_run_error/run_count/created_by 等字段（`backend/src/models/report_subscription.rs:42-97`）。
2. **订阅频率校验**：`create` 校验 frequency 必须为 DAILY/WEEKLY/MONTHLY（`backend/src/services/report_subscription_service.rs:73-78`），并计算 next_run_at。
3. **退订机制**：`delete` 软删除 + `is_enabled = false`（`backend/src/services/report_subscription_service.rs:156-170`），用户可随时退订；`toggle` 支持启用/禁用（`:173-207`）。
4. **手动触发接口**：`trigger` 方法将 next_run_at 设为现在（`:247-261`），`POST /subscriptions/:id/trigger` 和 `POST /subscriptions/:id/send` 端点已挂载（`backend/src/routes/analytics.rs:149-156`）。
5. **分页防 DoS**：`list` 方法 `page_size.clamp(1, 100)` + `page.clamp(1, 1000)`（`backend/src/services/report_subscription_service.rs:215, 241`）。
6. **cron 表达式解析**：`report/job.rs` 提供 `calculate_next_run` 方法支持 5 字段 cron 表达式（`backend/src/services/report/job.rs:20-103`）。

#### ❌ 缺陷项

**缺陷 2.1：定时推送无后台调度任务，next_run_at 永远不会自动触发**
- **风险等级：P0**
- **证据**：
  - `/workspace/backend/src/services/report_subscription_service.rs:247-261`：`trigger` 方法仅将 `next_run_at` 设为 `Utc::now()`，不实际执行报表生成或推送。
  - `/workspace/backend/src/handlers/report_enhanced_handler.rs:464-497`：`send_subscription_now` 仅调用 `service.trigger(id)`，返回 `"报表订阅已立即发送"` 但**实际未发送任何邮件/站内信**。
  - `/workspace/backend/src/main.rs:500-600`：后台任务仅有慢查询采集、admin 缓存清理、JTI 黑名单清理，**无报表订阅调度任务**。
  - Grep 检索 `run_subscription` / `subscription.*runner` / `tokio::spawn.*subscription` 全项目无匹配。
- **业务影响**：用户配置的日/周/月报表订阅永远不会自动执行，决策者收不到定期报表，订阅功能完全失效。"立即发送"接口也仅打日志不实际推送，存在功能欺骗。
- **修复建议**：在 `main.rs` 启动后台任务，每分钟扫描 `next_run_at <= now AND is_enabled = true AND status = ACTIVE` 的订阅，调用报表执行 + 邮件发送 + 更新 last_run_at/last_run_status；同时实现 `send_subscription_now` 真正发送邮件给 recipients。

**缺陷 2.2：订阅创建不校验订阅者对报表的查看权限**
- **风险等级：P1**
- **证据**：
  - `/workspace/backend/src/services/report_subscription_service.rs:65-106`：`create` 方法仅接收 `recipients: Vec<String>`（邮箱列表），**不校验 recipients 是否有该报表查看权限**，也不校验 `template_id` 是否存在或对订阅者可见。
  - `CreateSubscriptionRequest` 无 `template_id` 存在性校验、无 `recipients` 邮箱格式校验。
- **业务影响**：用户可将敏感财务报表订阅推送到任意邮箱（包括外部邮箱），导致数据泄露；订阅不存在的 template_id 也会成功创建，运行时才报错。
- **修复建议**：`create` 时校验 `template_id` 存在且 `is_public || created_by == user_id`；对每个 recipient 校验是否为系统内用户且有报表查看权限；使用 `validator` 校验邮箱格式。

**缺陷 2.3：推送失败无重试机制，无死信队列**
- **风险等级：P1**
- **证据**：
  - `/workspace/backend/src/services/report_subscription_service.rs`：无 retry_count / max_retries 字段，无重试逻辑。
  - `/workspace/backend/src/models/report_subscription.rs:42-97`：`Model` 仅有 `last_run_error` 单次错误信息，无 `retry_count` / `dead_letter` 标记。
  - 即使缺陷 2.1 的后台任务实现，也无重试 3 次指数退避机制。
- **业务影响**：推送因网络抖动失败后不会重试，订阅者漏收关键报表；失败信息仅覆盖最近一次，无法排查历史失败模式。
- **修复建议**：新增 `retry_count` / `max_retries` / `next_retry_at` 字段，失败后按指数退避（1min/5min/30min）重试 3 次，超过上限后转入死信队列（参考 `event_dead_letter` 表设计）。

---

## 维度 3：BI 分析与多维钻取（19.3）

### 检查方法
- Read `/workspace/backend/src/services/bi_analysis_service.rs`
- Read `/workspace/backend/src/handlers/bi_handler.rs`
- Grep 检索 `bi.*cache` / `bi.*data_scope` / `bi.*permission` / `bi.*user_id`

### 发现

#### ✅ 已落实的项
1. **BI 多维分析完整**：`BiAnalysisService` 支持 5 维度聚合（time/customer/product/region/category）+ 4 钻取（year-to-month/month-to-day/customer-to-order/product-to-order）+ 4 切片上卷（slice/dice/rollup/pivot），共 16 个 HTTP 端点（`backend/src/services/bi_analysis_service.rs:297-1288`，`backend/src/handlers/bi_handler.rs:1-281`）。
2. **真实数据库查询**：所有方法使用 `Statement::from_sql_and_values` + `FromQueryResult` 真实查询 `sales_orders` / `sales_order_items` / `customers` / `products` / `product_categories` 表，排除 CANCELLED/DRAFT 状态（`backend/src/services/bi_analysis_service.rs:326-348` 等）。
3. **参数校验完整**：`sales_by_time` 校验日期范围（`:312-314`），`drilldown_year_to_month` 校验年份 1900-2999（`:780-782`），`drilldown_month_to_day` 校验月份 1-12（`:857-859`），`drilldown_customer_to_order` / `drilldown_product_to_order` 校验 ID > 0（`:942-944, 990-992`）。
4. **维度/度量白名单**：`dim_to_expr` / `measure_to_expr` 对非法维度/度量返回 `AppError::validation`（`backend/src/services/bi_analysis_service.rs:244-284`），批次 252 修复了原 `unreachable!()` panic。
5. **SQL 注入防护**：所有 SQL 使用参数化绑定（`$1` / `$2` ...），`granularity` 通过代码内 `match` 选择 `period_expr`，不接受用户输入拼接（`:317-324`）。
6. **limit 防 DoS**：`sales_by_customer` / `sales_by_product` 的 `limit.clamp(1, 100)`（`:386, 453`），`sales_trend` 的 `days.clamp(1, 365)`（`:590`）。
7. **钻取缺失数据补全**：`drilldown_year_to_month` 补全 12 个月（`:830-844`），`drilldown_month_to_day` 按月份天数补全（`:916-930`）。
8. **单元测试覆盖**：11 个测试覆盖参数校验和 `dim_to_expr` / `measure_to_expr` 边界（`:1305-1460`）。

#### ❌ 缺陷项

**缺陷 3.1：BI 查询无任何缓存，重复计算压力大**
- **风险等级：P1**
- **证据**：
  - `/workspace/backend/src/services/bi_analysis_service.rs` 全文无 `cache` / `Cache` 引用。
  - 对比 `dashboard_service.rs:139-145` 使用 `AppCache` + 5 分钟 TTL，BI 服务未接入任何缓存。
  - `/workspace/backend/src/handlers/bi_handler.rs:39-142`：8 个维度聚合端点每次都新建 `BiAnalysisService::new(state.db.clone())` 并执行 raw SQL。
- **业务影响**：BI 查询涉及多表 JOIN + SUM + GROUP BY，100 万行数据查询可能耗时数秒，多个用户并发查询相同维度会重复执行相同 SQL，压垮数据库。
- **修复建议**：`BiAnalysisService::new` 接收 `Arc<AppCache>`，对 `sales_by_time` / `sales_by_customer` / `kpi_summary` 等聚合结果按 (start_date, end_date, granularity) 生成缓存键，TTL 5 分钟。

**缺陷 3.2：BI 无数据权限过滤，销售员可看所有销售数据**
- **风险等级：P0**
- **证据**：
  - `/workspace/backend/src/services/bi_analysis_service.rs:341-345`：`sales_by_time` 的 SQL `WHERE s.order_date >= $1 AND s.order_date <= $2 AND s.status NOT IN ('CANCELLED', 'DRAFT')`，**无 `user_id` 过滤**。
  - `sales_by_customer` / `sales_by_product` / `sales_by_region` / `sales_by_category` / `profit_analysis` / `kpi_summary` 全部无 `user_id` / `created_by` 过滤。
  - `/workspace/backend/src/handlers/bi_handler.rs:39-49`：`sales_by_time` 接收 `_auth: AuthContext` 但**未使用** auth 上下文。
  - Grep 检索 `bi.*data_scope` / `bi.*user_id` / `bi.*permission` 全项目无匹配。
- **业务影响**：销售员可查看全公司销售数据（包括其他销售员的订单、客户、利润），违反数据范围控制（ALL/DEPT/DEPT_AND_BELOW/SELF/CUSTOM），泄露商业机密。
- **修复建议**：`BiAnalysisService` 方法接收 `user_id` + `data_scope`，根据 data_scope 拼接 `AND s.created_by = $user_id`（SELF）或 `AND s.created_by IN (SELECT user_id FROM users WHERE dept_id IN (...))`（DEPT_AND_BELOW）等条件。

**缺陷 3.3：BI 大数据性能无监控，无超时降级为异步导出**
- **风险等级：P2**
- **证据**：
  - `/workspace/backend/src/services/bi_analysis_service.rs`：无 `tokio::time::timeout` 包装，无 P95 响应时间监控，无异步导出降级逻辑。
  - `drilldown_customer_to_order` / `drilldown_product_to_order` 写死 `LIMIT 100`（`:957, 1006`），但 `sales_by_time` 等聚合查询无 LIMIT，全表扫描。
- **业务影响**：大数据量场景下 BI 查询可能超时阻塞请求线程，影响其他业务；用户无法获得查询进度反馈。
- **修复建议**：为 BI 查询包装 `tokio::time::timeout(Duration::from_secs(5))`，超时后返回异步任务 ID，后台执行并生成导出文件，通过通知中心告知用户。

---

## 维度 4：仪表板数据卡片实时刷新（19.4）

### 检查方法
- Read `/workspace/backend/src/services/dashboard_service.rs`
- Read `/workspace/backend/src/handlers/dashboard_handler.rs`
- Grep 检索 `dashboard.*card` / `dashboard.*layout` / `dashboard.*WebSocket` / `real_time.*push` / `仪表板.*配置`

### 发现

#### ✅ 已落实的项
1. **仪表板概览数据完整**：`DashboardOverview` 包含 total_products/total_warehouses/total_orders/total_sales/low_stock_count/pending_orders/monthly_sales 7 个核心指标（`backend/src/services/dashboard_service.rs:57-67`）。
2. **销售统计多维聚合**：`SalesStatistics` 提供 daily/weekly/monthly_sales + by_customer/by_product/by_salesperson（`:70-78`），批次 134 v9 P1 修复真实 raw SQL 聚合。
3. **库存统计真实聚合**：`InventoryStatistics` 包含 total_inventory/by_warehouse/by_category/turnover_rate/aging_analysis（`:94-101`），批次 135 v9 P1 修复 turnover_rate/by_category/aging_analysis 真实查询。
4. **5 分钟缓存**：`get_overview` / `get_sales_statistics` / `get_inventory_statistics` / `get_low_stock_alerts` 均使用 `AppCache` + `Duration::from_secs(300)`（`:165-169, 240-245, 365-372, 607-614, 841-848`）。
5. **并行查询优化**：`get_overview` 使用 `tokio::try_join!` 并行执行 7 个独立查询（`:207-223`），`get_inventory_statistics` 并行 4 个查询（`:532-537`）。
6. **低库存预警真实数据**：`get_low_stock_alerts` 关联 products + warehouses 表，并行查询（`:781-851`）。
7. **状态常量接入**：`master_data::ACTIVE` 替代硬编码 "active"（`:193, 495, 510, 517, 797`），批次 209 P2-5 修复。

#### ❌ 缺陷项

**缺陷 4.1：仪表板不支持自定义卡片，配置无法持久化**
- **风险等级：P1**
- **证据**：
  - `/workspace/backend/src/handlers/dashboard_handler.rs:31-83`：仅 4 个固定端点 `get_dashboard_overview` / `get_sales_statistics` / `get_inventory_statistics` / `get_low_stock_alerts`，无卡片配置 CRUD 接口。
  - Grep 检索 `dashboard.*card` / `dashboard.*layout` / `仪表板.*配置` / `卡片.*拖拽` 全项目无匹配。
  - 无 `dashboard_layout` / `dashboard_card` 表模型。
- **业务影响**：用户无法根据角色定制关注的卡片，所有用户看到相同布局，财务角色无法突出财务卡片，销售角色无法突出销售卡片。
- **修复建议**：新增 `dashboard_layouts` 表（user_id, card_config JSON, is_default），提供 `GET/PUT /dashboard/layout` 接口，前端按配置动态渲染卡片。

**缺陷 4.2：仪表板无 WebSocket 实时刷新，仅靠 5 分钟缓存**
- **风险等级：P1**
- **证据**：
  - `/workspace/backend/src/services/dashboard_service.rs:238-245`：缓存有效期 5 分钟，意味着数据最多延迟 5 分钟。
  - `/workspace/backend/src/websocket/notifications.rs`：WebSocket 仅用于通知推送，未用于仪表板数据推送。
  - Grep 检索 `dashboard.*WebSocket` / `real_time.*push` / `实时刷新` 全项目无匹配。
- **业务影响**：库存/订单/财务等关键指标延迟最多 5 分钟，库存预警、订单激增等场景无法实时响应，影响运营决策时效。
- **修复建议**：扩展 `websocket/notifications.rs` 增加 `broadcast_dashboard_update` 方法，关键业务事件（订单创建/库存调整）触发仪表板数据推送；前端订阅 `ws://dashboard/updates` 频道实时刷新卡片。

**缺陷 4.3：仪表板无角色控制可见卡片**
- **风险等级：P1**
- **证据**：
  - `/workspace/backend/src/handlers/dashboard_handler.rs:31-43`：`get_dashboard_overview` 接收 `_auth: AuthContext` 但未使用，所有登录用户看到相同的 total_sales/monthly_sales 等财务数据。
  - Grep 检索 `dashboard.*permission` / `dashboard.*data_scope` 仅 1 条匹配（logistics_handler 无关）。
- **业务影响**：销售员可查看全公司总销售额、月度销售额等财务敏感数据，违反最小权限原则。
- **修复建议**：`get_dashboard_overview` 根据 `auth.role_id` + `data_scope` 过滤 total_sales（SELF 范围仅统计自己的订单），财务卡片仅财务角色可见。

**缺陷 4.4：仪表板性能无 P95 监控，单卡片查询无 500ms 限制**
- **风险等级：P2**
- **证据**：
  - `/workspace/backend/src/services/dashboard_service.rs`：无 `tokio::time::timeout` 包装，无 P95 响应时间记录。
  - `get_overview` 并行 7 个查询，单个慢查询会拖累整体响应。
- **业务影响**：数据库压力高时仪表板首屏可能超过 2s，影响用户体验。
- **修复建议**：为每个查询包装 `tokio::time::timeout(Duration::from_millis(500))`，超时返回缓存数据或零值；接入 `tracing` 记录 P95 响应时间。

---

## 维度 5：通知中心多渠道去重（19.5）

### 检查方法
- Read `/workspace/backend/src/services/notification_service.rs`
- Read `/workspace/backend/src/models/notification.rs`
- Read `/workspace/backend/src/models/notification_setting.rs`
- Read `/workspace/backend/src/services/event_notification_service.rs`
- Grep 检索 `notification.*dedup` / `去重` / `notification.*throttle` / `notification.*webhook`

### 发现

#### ✅ 已落实的项
1. **通知类型枚举完整（部分）**：`NotificationType` 包含 Internal/Email/Sms/System 4 类（`backend/src/models/notification.rs:10-25`）。
2. **通知状态机完整**：`NotificationStatus` 包含 Unread/Read/Processed/Deleted 4 态（`:46-61`）。
3. **通知优先级分级**：`NotificationPriority` 包含 Low/Normal/High/Urgent 4 级（`:28-43`）。
4. **WebSocket 实时推送**：`create_notification` 创建后调用 `get_notification_broadcaster().broadcast_notification` 推送至在线客户端（`backend/src/services/notification_service.rs:94-99`），批次 24 v6 P0-2 修复。
5. **批量创建优化**：`batch_create_notifications` 支持批量插入并逐条推送（`:104-145`）。
6. **批量已读优化**：`batch_mark_as_read` 使用 `update_many` 批量更新（`:212-236`），批次 37 修复 N+1。
7. **未读数量查询**：`get_unread_count` 返回未读数量（`:181-189`）。
8. **权限校验**：`mark_as_read` / `delete_notification` / `get_notification` 校验 `notification.user_id == user_id`（`:198-200, 267-269, 290-292`）。
9. **通知设置**：`notification_settings` 表支持按 business_type 配置 enable_internal/enable_email/enable_sms（`backend/src/models/notification_setting.rs:11-30`）。
10. **业务事件集成**：`EventNotificationService` 提供订单提交/审批通过/发货/完成/库存预警/审批提醒等业务事件通知（`backend/src/services/event_notification_service.rs:135-294`）。
11. **用户通知偏好**：`UserNotificationSettingService` 支持 `should_send_email` / `should_send_internal` 校验（`:142-149, 188-191, 219-227, 261-268`）。

#### ❌ 缺陷项

**缺陷 5.1：通知渠道缺少 Webhook，与审计计划期望的 4 渠道不符**
- **风险等级：P1**
- **证据**：
  - `/workspace/backend/src/models/notification.rs:10-25`：`NotificationType` 仅 `Internal/Email/Sms/System` 4 类，**无 Webhook 变体**。
  - `/workspace/backend/src/models/notification_setting.rs:11-30`：`Model` 仅 `enable_internal/enable_email/enable_sms` 3 个布尔字段，**无 `enable_webhook`**。
  - `/workspace/backend/src/services/notification_service.rs:357-379`：`is_notification_enabled` 仅匹配 Internal/Email/Sms，无 Webhook 分支。
  - Grep 检索 `notification.*webhook` / `webhook.*notification` 全项目无匹配（webhook_integration_handler 是独立模块，未与通知中心集成）。
- **业务影响**：审计计划 19.5 期望"必须支持站内信/邮件/短信/Webhook 4 渠道"，当前缺少 Webhook 渠道，无法将通知推送到外部系统（如企业微信、钉钉、Slack）。
- **修复建议**：`NotificationType` 增加 `Webhook` 变体，`notification_settings` 增加 `enable_webhook` 字段，`create_notification` 触发时调用 `webhook_handler` 推送至用户配置的 webhook URL。

**缺陷 5.2：通知无去重机制，同一事件重复触发会产生通知轰炸**
- **风险等级：P1**
- **证据**：
  - `/workspace/backend/src/services/notification_service.rs:69-101`：`create_notification` 直接插入数据库，无 `(user_id, business_type, business_id)` 唯一约束检查，无 5 分钟窗口去重。
  - `/workspace/backend/src/services/event_notification_service.rs:135-175`：`notify_order_submitted` 每次调用都创建新通知，订单状态多次变更会触发多次通知。
  - Grep 检索 `notification.*dedup` / `notification.*throttle` / `5.*分钟.*notification` 全项目无匹配。
  - `/workspace/backend/src/models/notification.rs:64-100`：`notifications` 表无唯一索引，无 `dedup_key` 字段。
- **业务影响**：库存预警在阈值附近波动时可能每分钟触发一次通知，导致用户被通知轰炸；订单状态反复变更会产生重复通知。
- **修复建议**：`notifications` 表新增 `dedup_key` 字段（如 `inventory_alert:product_id:warehouse_id`），`create_notification` 前查询 5 分钟内相同 dedup_key 的通知，存在则跳过；或使用 Redis 分布式锁 + TTL 实现。

**缺陷 5.3：通知模板无动态管理，仅静态硬编码**
- **风险等级：P2**
- **证据**：
  - `/workspace/backend/src/services/email_service.rs:728-815`：`EmailTemplate` 是静态 struct，`notification_template` / `order_notification` / `approval_notification` / `inventory_alert` 方法硬编码 HTML 模板。
  - `/workspace/backend/src/models/notification.rs`：无 `notification_template` 表模型，无模板 CRUD 接口。
  - 对比 `email_template_service.rs` 有动态模板管理，但通知中心未接入。
- **业务影响**：通知文案修改需要发版上线，运营无法自助调整；不支持多语言（i18n），仅中文模板。
- **修复建议**：新增 `notification_templates` 表（code, title_template, content_template, language, variables），`create_notification` 时按 business_type + language 加载模板并替换变量。

**缺陷 5.4：通知模板不支持多语言**
- **风险等级：P2**
- **证据**：
  - `/workspace/backend/src/services/email_service.rs:730-772`：`notification_template` 硬编码中文"系统通知"、"此邮件由系统自动发送，请勿回复"、"面料管理系统"。
  - `/workspace/backend/src/services/event_notification_service.rs:152-165`：通知标题"订单已提交"、"订单审批通过"等硬编码中文。
  - Grep 检索 `i18n` / `language` 在通知模块无匹配。
- **业务影响**：外籍员工或海外客户无法收到英文通知，影响国际化业务。
- **修复建议**：接入 i18n 模块（如 `rust-i18n`），通知模板按 `Accept-Language` 头选择语言版本。

---

## 维度 6：邮件服务 SMTP 队列重试（19.6）

### 检查方法
- Read `/workspace/backend/src/services/email_service.rs`
- Read `/workspace/backend/src/services/email_log_service.rs`
- Read `/workspace/backend/src/services/email_template_service.rs`
- Read `/workspace/backend/src/handlers/email_handler.rs`
- Grep 检索 `SMTP` / `smtp_password` / `邮件.*队列` / `virus` / `clamav` / `25.*MB`

### 发现

#### ✅ 已落实的项
1. **多服务商支持**：`EmailService` 支持 sendgrid/aliyun/tencent 3 个 HTTP API 服务商（`backend/src/services/email_service.rs:177-187`），真实接入签名算法（SendGrid Bearer / 阿里云 RPC V1 HMAC-SHA1 / 腾讯云 TC3-HMAC-SHA256）。
2. **API URL 硬编码防注入**：`SENDGRID_API_URL` / `ALIYUN_DM_API_URL` / `TENCENT_SES_API_URL` 为 `&'static str` 常量（`:37-44`），H-2 修复删除 `api_url` 字段，禁止环境变量注入。
3. **HTML XSS 防护**：`escape_html` 函数转义 `& < > " ' /` 6 个字符（`:21-35`），`EmailTemplate::notification_template` 对所有用户输入做 escape（`:733-744`）。
4. **危险模式检测**：`send_html_email` 检测 `<script` / `javascript:` / `onerror=` 等 8 个危险模式并记录 warn 日志（`:223-242`）。
5. **邮件发送记录**：`email_logs` 表持久化 recipients/cc/bcc/subject/body/template_id/status/error_message/external_message_id/sent_at/retry_count（`backend/src/services/email_log_service.rs:53-79`）。
6. **发送配额限制**：`EMAIL_PER_USER_PER_HOUR = 50`，使用 `DashMap<(user_id, hour_bucket), AtomicU32>` 计数（`backend/src/handlers/email_handler.rs:31, 76-95`）。
7. **管理员权限校验**：`send_email` 仅 admin 角色可调用（`:65-73`）。
8. **模板参数渲染**：`render_template` 支持 `{{key}}` / `{{ key }}` 占位符替换（`:239-257`），批次 151 P2-A 修复。
9. **失败重试计数**：发送失败时调用 `increment_retry` 累加 retry_count 并重置为 PENDING（`:189-196`）。
10. **邮件统计**：`get_statistics` 返回 total/sent/failed/pending 计数（`backend/src/services/email_log_service.rs:167-193`）。
11. **HMAC-SHA256 防御性编程**：`hmac_sha256_bytes` 使用 match + error 日志兜底，不触发 panic（`:715-725`），L-12 修复。

#### ❌ 缺陷项

**缺陷 6.1：邮件发送同步阻塞，无异步队列**
- **风险等级：P1**
- **证据**：
  - `/workspace/backend/src/handlers/email_handler.rs:166-201`：`send_email` 直接 `await email_service.send_email(message)`，HTTP 请求同步阻塞当前请求线程。
  - 无 Redis 队列 / 后台 worker 消费邮件任务。
  - `batch_create_notifications`（`notification_service.rs:104-145`）也是循环逐条插入 + 推送。
- **业务影响**：SendGrid/阿里云/腾讯云 API 响应慢（1-5s）时，邮件发送会阻塞用户请求，批量邮件场景下严重拖累系统响应。
- **修复建议**：`send_email` 改为写入 `email_queue` 表 + 立即返回 PENDING，后台 worker 每秒扫描 PENDING 邮件并异步发送，发送后更新状态。

**缺陷 6.2：邮件失败重试机制不完整，无重试调度任务**
- **风险等级：P1**
- **证据**：
  - `/workspace/backend/src/services/email_log_service.rs:110-125`：`increment_retry` 仅累加 `retry_count` 并重置状态为 PENDING，**但无后台任务扫描 PENDING 邮件实际重试**。
  - `/workspace/backend/src/main.rs:500-600`：后台任务无邮件重试调度。
  - 无指数退避（1min/5min/30min）逻辑，无最大重试次数上限（retry_count 可无限增长）。
- **业务影响**：邮件发送失败后虽然标记为 PENDING + retry_count++，但永远不会被重试发送，关键业务通知（订单确认、库存预警）丢失。
- **修复建议**：`main.rs` 启动后台任务每分钟扫描 `status = PENDING AND retry_count < 3` 的邮件，按指数退避重试，超过 3 次转入 `FAILED` 死信。

**缺陷 6.3：邮件附件无病毒扫描，无大小限制**
- **风险等级：P1**
- **证据**：
  - `/workspace/backend/src/services/email_service.rs:113-115`：`EmailMessage.attachments: Option<HashMap<String, Vec<u8>>>` 接收附件，但 `send_via_sendgrid` / `send_via_aliyun` / `send_via_tencent` **三个发送方法均未处理 attachments 字段**，附件被静默丢弃。
  - `/workspace/backend/src/handlers/email_handler.rs:60-63`：`SendEmailRequest` 无 `attachments` 字段，无附件大小校验。
  - Grep 检索 `virus` / `clamav` / `病毒扫描` / `25.*MB` / `attachment_size` 全项目无匹配。
- **业务影响**：即使附件功能未实现，未来启用时若无病毒扫描和大小限制，可能被利用发送恶意附件（病毒/木马）或超大附件导致服务拒绝。
- **修复建议**：实现附件发送时集成 ClamAV 病毒扫描，限制单附件 ≤ 25MB，总附件 ≤ 50MB，扫描通过后才入队发送。

**缺陷 6.4：邮件模板无版本管理**
- **风险等级：P2**
- **证据**：
  - `/workspace/backend/src/models/email_template.rs`（通过 service 推断）：`EmailTemplateService` 的 `update` 方法直接覆盖原记录（`backend/src/services/email_template_service.rs:115-154`），无版本字段，无历史回滚。
- **业务影响**：邮件模板修改失误后无法回滚，影响邮件发送质量。
- **修复建议**：新增 `email_template_versions` 表，update 前写入历史版本。

---

## 维度 7：OA 公告与用户行为分析（19.7）

### 检查方法
- Read `/workspace/backend/src/models/oa_announcement.rs`
- Read `/workspace/backend/src/services/tracking_service.rs`
- Read `/workspace/backend/src/models/user_behavior.rs`
- Read `/workspace/backend/src/models/page_view.rs`
- Read `/workspace/backend/src/handlers/tracking_handler.rs`
- Grep 检索 `announcement.*route` / `oa_announcement` / `announcement.*approval` / `隐私政策` / `consent` / `behavior.*desensitize`

### 发现

#### ✅ 已落实的项
1. **OA 公告数据模型存在**：`oa_announcement` 表包含 title/content/announcement_type/publish_date/effective_date/expiry_date/publisher_id/status/is_top/attachments/remarks 等字段（`backend/src/models/oa_announcement.rs:13-58`）。
2. **用户行为追踪模型完整**：`user_behaviors` 表包含 session_id/user_id/event_type/event_target/event_data/path/ip_address/occurred_at（`backend/src/models/user_behavior.rs:10-40`）。
3. **页面访问模型完整**：`page_views` 表包含 session_id/user_id/path/referrer/user_agent/ip_address/viewed_at（`backend/src/models/page_view.rs:10-36`）。
4. **行为追踪服务真实实现**：`TrackingService` 提供 `record_page_view` / `record_behavior` / `get_page_view_stats` / `get_daily_stats` / `get_popular_pages` / `get_funnel_analysis` / `get_user_path` 7 个方法（`backend/src/services/tracking_service.rs:129-389`），批次 143 P1-2 修复。
5. **输入长度校验**：`PageViewRequest` / `BehaviorRequest` 使用 `validator::Validate` 约束 path ≤ 2048、session_id ≤ 128 等字段长度（`backend/src/handlers/tracking_handler.rs:31-71`），v14 中风险安全修复。
6. **SQL 注入防护**：所有统计查询使用 `Statement::from_sql_and_values` 参数化绑定（`backend/src/services/tracking_service.rs:172-275`）。
7. **漏斗分析真实实现**：`get_funnel_analysis` 按 session_id 逐步缩小集合统计转化率（`:282-311`）。
8. **单元测试覆盖**：`parse_date` 函数 4 个测试覆盖完整日期/日期字符串/None/无效格式（`:411-445`）。

#### ❌ 缺陷项

**缺陷 7.1：OA 公告无 service / handler / 路由，完全未实现业务功能**
- **风险等级：P0**
- **证据**：
  - `/workspace/backend/src/models/oa_announcement.rs` 仅有 Model 定义，文件级 `#![allow(dead_code)]` 标注为死代码（`:1-2`）。
  - Grep 检索 `announcement.*route` / `公告.*路由` / `oa.*route` 全项目无匹配。
  - `/workspace/backend/src/models/mod.rs:242` 仅 `pub mod oa_announcement;`，无对应 service / handler。
  - Grep 检索 `announcement.*approval` / `公告.*审批` / `announcement.*audit_log` 全项目无匹配。
  - `/workspace/backend/src/routes/analytics.rs` 全文无公告路由挂载。
- **业务影响**：OA 公告功能完全不可用，企业无法发布内部通知/公告/新闻，员工无法获取公司动态；已发布的公告无法撤回，无审计日志。
- **修复建议**：实现 `OaAnnouncementService`（create/audit/publish/revoke/archive/list）+ `OaAnnouncementHandler` + 路由挂载；公告发布需经审批流（接入 approval 模块），已发布仅可撤回并记录审计日志。

**缺陷 7.2：OA 公告无可见性控制，无部门/角色过滤**
- **风险等级：P1**
- **证据**：
  - `/workspace/backend/src/models/oa_announcement.rs:13-58`：`Model` 无 `visible_departments` / `visible_roles` / `visibility_scope` 字段。
  - 即使公告功能实现，当前模型也无法按部门/角色控制可见性。
- **业务影响**：敏感人事公告/财务公告可能被全员可见，违反数据权限。
- **修复建议**：新增 `visibility_scope`（ALL/DEPT/ROLE/CUSTOM）+ `visible_scope_config JSON` 字段，查询时按用户部门/角色过滤。

**缺陷 7.3：用户行为采集无隐私政策告知，无拒绝采集机制**
- **风险等级：P1**
- **证据**：
  - `/workspace/backend/src/handlers/tracking_handler.rs:82-102`：`track_page_view` 接收 `auth: AuthContext` 后直接持久化，无隐私政策弹窗确认，无 `consent` 字段。
  - `/workspace/backend/src/handlers/tracking_handler.rs:157-177`：`record_behavior` 同样直接持久化。
  - Grep 检索 `privacy.*policy` / `隐私政策` / `consent` / `数据采集同意` 全项目无匹配。
- **业务影响**：违反《个人信息保护法》和 GDPR，未告知用户即采集行为数据，可能面临合规处罚；用户无法拒绝被追踪。
- **修复建议**：用户首次登录时弹出隐私政策确认，记录 `user_consent` 表；`track_page_view` / `record_behavior` 前校验用户是否同意采集；提供 `POST /privacy/opt-out` 接口允许用户退出追踪。

**缺陷 7.4：用户行为日志无脱敏，敏感操作明文记录**
- **风险等级：P1**
- **证据**：
  - `/workspace/backend/src/services/tracking_service.rs:150-164`：`record_behavior` 直接将 `event_data: Option<serde_json::Value>` 持久化到 `user_behaviors.event_data` JSONB 字段，无脱敏处理。
  - `/workspace/backend/src/handlers/tracking_handler.rs:56-71`：`BehaviorRequest.event_data` 仅校验长度，未脱敏手机号/身份证/邮箱等敏感字段。
  - Grep 检索 `behavior.*sensitive` / `behavior.*desensitize` / `behavior.*mask` / `user_behavior.*脱敏` 全项目无匹配。
- **业务影响**：用户查看客户手机号、身份证等敏感操作被明文记录到 event_data，日志泄露时暴露个人隐私。
- **修复建议**：`record_behavior` 前对 event_data 中的手机号/身份证/邮箱/银行卡等字段脱敏（如 `138****1234`），保留前 3 后 4 位。

---

## 维度 8：五维度分析与页面浏览统计（19.8）

### 检查方法
- Read `/workspace/backend/src/services/five_dimension_service.rs`
- Read `/workspace/backend/src/handlers/five_dimension_handler.rs`
- Read `/workspace/backend/src/services/tracking_service.rs`
- Grep 检索 `人.*机.*料.*法.*环` / `5M1E` / `quality.*dimension` / `page_view.*archive` / `page_view.*retention`

### 发现

#### ✅ 已落实的项
1. **面料五维编码业务对齐**：`FiveDimensionService` 实现面料行业的"产品五维编码"（产品ID + 批次号 + 色号 + 缸号 + 等级），与面料行业产品追踪业务对齐（`backend/src/services/five_dimension_service.rs:1-8`）。
2. **数据自动归集**：`get_stats` 从 `inventory_stocks` 表自动归集五维字段（product_id/batch_no/color_no/dye_lot_no/grade），无需手工录入（`:82-105`）。
3. **五维 ID 格式标准化**：`five_dimension_id = "P{}|B{}|C{}|D{}|G{}"`（`:137-144`），支持反向解析（`:209-231`）。
4. **仓库分布统计**：`WarehouseStock` 提供按仓库的 quantity_meters/quantity_kg 分布（`:33-39, 167-181`）。
5. **分页防 DoS**：`page_size.clamp(1, 100)`（`:194`）。
6. **搜索类型支持**：`search` 支持 product/batch/color/dye_lot/grade 5 种搜索类型（`:269-302`）。
7. **汇总统计**：`get_summary` 返回 total_products/total_batches/total_colors/total_meters/total_kg/total_stock_count/five_dimension_count（`:311-357`）。

#### ❌ 缺陷项

**缺陷 8.1：五维度分析概念错位，未实现"人/机/料/法/环"质量管理五维度**
- **风险等级：P1**
- **证据**：
  - `/workspace/backend/src/services/five_dimension_service.rs:1-8`：注释明确"五维编码：产品ID + 批次号 + 色号 + 缸号 + 等级"，这是**产品追踪五维**，非质量管理五维。
  - 审计计划 6360 行期望"五维度（如人/机/料/法/环）必须与面料行业业务对齐"。
  - Grep 检索 `人.*机.*料.*法.*环` / `5M1E` / `quality.*dimension` 在 backend/src 无匹配。
  - `/workspace/backend/src/utils/mod.rs:15`：注释"批次 348 v12 复审 P2-2：fabric_five_dimension 模块已删除（死代码）"，原 `utils/fabric_five_dimension.rs` 已删除。
- **业务影响**：审计计划期望的"人/机/料/法/环"质量管理五维度分析完全未实现，无法从生产/质量模块归集 5M1E 数据进行质量归因分析；当前实现的"产品五维编码"是库存追踪维度，与质量五维度是不同概念。
- **修复建议**：明确"五维度"业务定义——若指产品追踪五维，需在文档中澄清；若指质量管理 5M1E，需新增 `quality_five_dimension_service` 从生产工单（人：操作员/机：设备/料：原料批次/法：工艺参数/环：温湿度）归集数据。

**缺陷 8.2：页面浏览未按 SPA 路由切换统计，未区分有效浏览/跳出**
- **风险等级：P2**
- **证据**：
  - `/workspace/backend/src/handlers/tracking_handler.rs:82-102`：`track_page_view` 接收前端主动 POST 请求，依赖前端在路由切换时调用，**无 SPA 路由钩子保证**。
  - `/workspace/backend/src/services/tracking_service.rs:129-147`：`record_page_view` 仅记录单次访问，无 session 内多个 page_view 的关联分析，无跳出（bounce）判定逻辑。
  - 无 `is_bounce` / `duration` 字段，无法区分有效浏览与跳出。
- **业务影响**：页面浏览统计可能遗漏（前端未埋点）或重复（路由切换触发多次），跳出率无法计算，影响运营决策。
- **修复建议**：前端集成 Vue Router `afterEach` 钩子自动上报；后端 `page_views` 表增加 `duration` 字段，session 内仅有 1 条记录且 duration < 10s 标记为跳出。

**缺陷 8.3：页面浏览明细无 90 天保留策略，无自动归档汇总**
- **风险等级：P1**
- **证据**：
  - `/workspace/backend/src/models/page_view.rs:10-36`：`page_views` 表无 `archived` / `retention_until` 字段。
  - `/workspace/backend/src/main.rs:500-600`：后台任务无 page_view 归档调度。
  - Grep 检索 `page_view.*archive` / `page_view.*retention` / `page_views.*delete` / `tracking.*cleanup` 全项目无匹配。
  - 对比 `audit_cleanup_service` 有保留策略（main.rs:509-514），page_view 无类似机制。
- **业务影响**：page_views 明细数据无限增长，高流量场景下表膨胀导致查询性能下降，且违反数据最小化原则（保留过久的行为明细）。
- **修复建议**：新增后台任务每日扫描 `viewed_at < now - 90 days` 的 page_view 记录，聚合为 `page_view_daily_summary`（按 path + date 汇总 total_views/unique_sessions）后删除明细。

**缺陷 8.4：用户行为日志同样无保留策略**
- **风险等级：P1**
- **证据**：
  - `/workspace/backend/src/models/user_behavior.rs:10-40`：`user_behaviors` 表无保留期字段。
  - Grep 检索 `user_behavior.*expire` / `user_behavior.*retention` 全项目无匹配。
- **业务影响**：用户行为明细无限增长，存储成本上升，且长期保留的行为数据增加隐私泄露风险。
- **修复建议**：与 page_view 同步实现 90 天归档策略，聚合为 `user_behavior_daily_summary` 后删除明细。

---

## 审计结果汇总

| 维度 | P0 | P1 | P2 | P3 | 已落实 | 总检查项 |
|------|----|----|----|----|--------|----------|
| 19.1 报表定义与模板管理 | 0 | 2 | 2 | 0 | 7 | 11 |
| 19.2 报表订阅与定时推送 | 1 | 2 | 0 | 0 | 6 | 9 |
| 19.3 BI 分析与多维钻取 | 1 | 1 | 1 | 0 | 8 | 11 |
| 19.4 仪表板数据卡片实时刷新 | 0 | 3 | 1 | 0 | 7 | 11 |
| 19.5 通知中心多渠道去重 | 0 | 2 | 2 | 0 | 11 | 15 |
| 19.6 邮件服务 SMTP 队列重试 | 0 | 3 | 1 | 0 | 11 | 15 |
| 19.7 OA 公告与用户行为分析 | 1 | 3 | 0 | 0 | 8 | 12 |
| 19.8 五维度分析与页面浏览统计 | 0 | 1 | 1 | 0 | 7 | 9 |
| **合计** | **3** | **17** | **8** | **0** | **65** | **93** |

---

## 修复优先级队列

### P0（阻塞级，3 项）

1. **缺陷 2.1**：定时推送无后台调度任务，next_run_at 永远不会自动触发
   - 文件：`backend/src/services/report_subscription_service.rs:247-261`、`backend/src/handlers/report_enhanced_handler.rs:464-497`、`backend/src/main.rs:500-600`
   - 影响：报表订阅功能完全失效，决策者收不到定期报表
   - 修复：main.rs 启动后台调度任务 + 实现 send_subscription_now 真实发送

2. **缺陷 3.2**：BI 无数据权限过滤，销售员可看所有销售数据
   - 文件：`backend/src/services/bi_analysis_service.rs:341-345` 等、`backend/src/handlers/bi_handler.rs:39-49`
   - 影响：销售员可查看全公司销售/利润数据，泄露商业机密
   - 修复：BiAnalysisService 方法接收 user_id + data_scope，按数据范围过滤

3. **缺陷 7.1**：OA 公告无 service / handler / 路由，完全未实现业务功能
   - 文件：`backend/src/models/oa_announcement.rs:1-2`、`backend/src/routes/analytics.rs`
   - 影响：OA 公告功能完全不可用
   - 修复：实现 OaAnnouncementService + Handler + 路由 + 审批流

### P1（高优先级，17 项）

4. **缺陷 1.1**：报表模板无版本管理，无法回滚
5. **缺陷 1.2**：报表权限未注册到权限系统，未按角色控制可见
6. **缺陷 2.2**：订阅创建不校验订阅者对报表的查看权限
7. **缺陷 2.3**：推送失败无重试机制，无死信队列
8. **缺陷 3.1**：BI 查询无任何缓存，重复计算压力大
9. **缺陷 4.1**：仪表板不支持自定义卡片，配置无法持久化
10. **缺陷 4.2**：仪表板无 WebSocket 实时刷新，仅靠 5 分钟缓存
11. **缺陷 4.3**：仪表板无角色控制可见卡片
12. **缺陷 5.1**：通知渠道缺少 Webhook，与审计计划期望的 4 渠道不符
13. **缺陷 5.2**：通知无去重机制，同一事件重复触发会产生通知轰炸
14. **缺陷 6.1**：邮件发送同步阻塞，无异步队列
15. **缺陷 6.2**：邮件失败重试机制不完整，无重试调度任务
16. **缺陷 6.3**：邮件附件无病毒扫描，无大小限制
17. **缺陷 7.2**：OA 公告无可见性控制，无部门/角色过滤
18. **缺陷 7.3**：用户行为采集无隐私政策告知，无拒绝采集机制
19. **缺陷 7.4**：用户行为日志无脱敏，敏感操作明文记录
20. **缺陷 8.3**：页面浏览明细无 90 天保留策略，无自动归档汇总
21. **缺陷 8.4**：用户行为日志同样无保留策略

### P2（中优先级，8 项）

22. **缺陷 1.3**：报表元数据缺少刷新策略字段
23. **缺陷 1.4**：报表参数校验不足（parameters/filters 为裸 JSON）
24. **缺陷 3.3**：BI 大数据性能无监控，无超时降级为异步导出
25. **缺陷 4.4**：仪表板性能无 P95 监控，单卡片查询无 500ms 限制
26. **缺陷 5.3**：通知模板无动态管理，仅静态硬编码
27. **缺陷 5.4**：通知模板不支持多语言
28. **缺陷 6.4**：邮件模板无版本管理
29. **缺陷 8.1**：五维度分析概念错位，未实现"人/机/料/法/环"质量管理五维度
30. **缺陷 8.2**：页面浏览未按 SPA 路由切换统计，未区分有效浏览/跳出

### P3（低优先级，0 项）

无 P3 级别缺陷。

---

## 审计结论

类十九报表 BI 与通知协同审计专项共检查 93 项，已落实 65 项（70%），发现缺陷 28 项（P0:3 / P1:17 / P2:8 / P3:0）。

**核心风险**：
1. **报表订阅功能完全失效**（缺陷 2.1）：next_run_at 字段存在但无后台任务触发，订阅永远不会自动执行，"立即发送"接口也仅打日志不实际推送。
2. **BI 数据权限缺失**（缺陷 3.2）：销售员可查看全公司销售/利润数据，违反数据范围控制。
3. **OA 公告完全未实现**（缺陷 7.1）：仅有数据模型，无 service/handler/路由，功能不可用。
4. **邮件重试机制不完整**（缺陷 6.2）：increment_retry 标记 PENDING 但无后台任务实际重试。
5. **通知无去重**（缺陷 5.2）：同一事件重复触发会产生通知轰炸。
6. **用户行为采集合规缺失**（缺陷 7.3、7.4）：无隐私政策告知、无脱敏机制，违反《个人信息保护法》。
7. **五维度概念错位**（缺陷 8.1）：实现的是产品追踪五维编码，非质量管理 5M1E 五维度。

**亮点**：
1. SQL 注入防护到位：报表自定义 SQL 入口已彻底关闭（P0-B 修复）。
2. 邮件 HTML XSS 防护完整：escape_html + 危险模式检测。
3. BI 多维分析真实实现：16 个端点真实查询数据库，非 mock 数据。
4. 仪表板 5 分钟缓存 + 并行查询优化。
5. 通知 WebSocket 实时推送（批次 24 v6 P0-2 修复）。
6. 邮件发送配额限制（每用户每小时 50 封）。
7. 用户行为追踪输入长度校验（v14 中风险安全修复）。

**建议**：P0 缺陷应立即修复（报表订阅调度、BI 数据权限、OA 公告实现），P1 缺陷应在下一迭代修复（通知去重、邮件异步队列、隐私合规等）。
