# 项目代码深度调研评估报告（12 维度全量扫描）

## 基础信息
- 扫描时间：2026-07-09
- 项目主语言：Rust（backend/Cargo.toml）
- 次要语言：TypeScript/Vue（frontend/package.json + tsconfig.json）
- 扫描模式：全量扫描（纯静态 AST 文本分析，未编译，深度调研）
- 分析工具：ripgrep + 正则模式 + 语义级上下文分析
- 检测维度：12 类

## 调研基础数据
- backend 共 100 个文件含 `#[cfg(test)]`，全后端 749 个 `#[test]/#[tokio::test]` 用例，626 个 `pub fn/pub async fn`
- frontend 仅 12 个 .test.ts 文件
- backend services 共 159 个 .rs 文件，仅 52 个含 `#[cfg(test)]`（覆盖率 32.7%）
- backend handlers 共 100+ 个 .rs 文件，仅 12 个含 `#[cfg(test)]`（覆盖率约 10%）
- backend SeaORM entity 接入：100+ 个文件、729 处 `Entity::find/ActiveModel` 使用点
- 过滤目录：vendor、node_modules、dist、build、target、frontend/e2e、frontend/tests、backend/tests、.audit-reports、.github、.trae、docs、monitoring、deploy、scripts、database/migration、backend/migration、backend/migrations

---

## 一、测试覆盖不足点位列表

| 文件路径 | 行号范围 | 函数/类名 | 风险等级 | 风险理由 | 业务影响 | 问题描述 | 修复建议 | 调研证据 |
|---|---|---|---|---|---|---|---|---|
| backend/src/middleware/permission.rs | 1-227 | permission_middleware / check_permission / extract_resource_info | 高 | 权限校验是核心安全模块，越权漏洞风险极高；当前 0 个测试用例 | 越权访问、垂直/水平权限漏洞 | permission_middleware 含缓存、资源 ID 解析、`*` 通配符匹配、admin 角色短路等分支，全部无单元测试覆盖 | 新增 `#[cfg(test)] mod tests`，覆盖：管理员短路、缓存命中/过期、resource_id 精确匹配、`*` 通配符、嵌套路径解析 | Grep `#[cfg\(test\)\]` 在 middleware/ 下命中 10 个文件，permission.rs 不在其中；该模块有 4 个 fn 定义 |
| backend/src/handlers/（100+ 个 handler 文件） | 全目录 | 100+ handler 模块 | 高 | 仅 12 个 handler 文件有测试（覆盖率约 10%） | 大量 API 端点无回归测试 | 关键 handler 如 auth_handler.rs、purchase_order_handler.rs、sales_order_handler.rs、inventory_stock_handler.rs 等核心模块无独立测试 | 优先为 auth/permission/inventory/sales/purchase 等 handler 补测试 | Grep `#[cfg\(test\)\]` 在 handlers/ 下命中 12 个文件 |
| backend/src/services/（159 个文件中 107 个无测试） | 全目录 | 107 个 service 模块 | 高 | 67.3% services 文件无 `#[cfg(test)]` | 大量业务逻辑无单测 | 关键 service 如 customer_service.rs、product_service.rs、supplier_service.rs、role_service.rs、email_service.rs、webhook_service.rs、currency_service.rs 等无测试模块 | 优先为核心 service（customer/product/auth/permission/email/webhook）补测试 | Grep 确认仅 52/159 services 文件含 `#[cfg(test)]` |
| frontend/src/api/（91 个 .ts 文件） | 全目录 | 91 个 API 模块导出函数 | 高 | 仅 4 个文件有测试（audit.ts/auth.ts/slow-query.ts/request.ts），覆盖率 4.4% | API 层变更无回归保障 | frontend/tests/unit/ 仅 12 个测试文件；91 个 api/*.ts 中绝大多数导出函数无对应测试 | 为关键 API 模块（user.ts/customer.ts/product.ts/inventory.ts/quotation.ts 等）补测试 | Glob `frontend/tests/unit/*.test.ts` 仅 12 个；Glob `frontend/src/api/*.ts` 91 个 |
| backend/src/services/ai/pred.rs | 1-300+ | AiAnalysisService::forecast_sales / fallback_forecast / build_seasonal_factors | 中 | 销售预测算法（WMA + 指数平滑 + 季节因子）核心数学逻辑无单测 | 预测结果错误影响补货决策 | 算法组合 60% 指数平滑 + 40% WMA、季节因子、置信度衰减等纯函数无单测 | 拆出纯函数（WMA/Holt/季节因子）单测，覆盖数据不足 7 天的 fallback 路径 | services/ai/ 下仅 quality_pred.rs/recipe_opt.rs 有测试，pred.rs 无 |
| backend/src/services/ai/detect.rs | 1-200+ | AiAnalysisService::detect_anomalies | 中 | Z-score/IQR 异常检测算法无单测 | 异常订单/库存未识别 | SPIKE/DROP/CRITICAL/WARNING 分级、ZERO_STOCK/LOW_STOCK/OVERSTOCK/SLOW_MOVING 分类逻辑无测试 | 单测 iqr_quartiles、Z-score 阈值分级、各异常类型识别 | 同上 |
| backend/src/services/ai/rec.rs | 1-680 | optimize_inventory / get_abc_classification / generate_recommendations / generate_association_recommendations | 中 | 库存优化、ABC 分类、关联规则（lift/support/confidence）算法无单测 | 推荐结果错误 | 关联规则 support>5%/confidence>30% 阈值、ABC 累计占比分类、安全库存 Z*σ*√LT 公式无测试 | 单测 compute_abc_classification、关联规则计算、安全库存计算 | 同上 |
| backend/src/middleware/timeout.rs | 1-34 | timeout_middleware | 中 | 超时控制影响所有请求，超时触发后 fallback Response 构造路径未测试 | 请求超时误判、fallback 路径 panic | 整个文件无 `#[cfg(test)]`，30s 超时分支与 INTERNAL_SERVER_ERROR fallback 未测试 | 用 tokio::time::pause 模拟超时，验证 408 响应；fallback 路径独立单测 | middleware/ 15 文件含 39 个 fn，timeout.rs 1 个 fn 无测试 |
| backend/src/middleware/omni_audit.rs | 1-200+ | omni_audit_middleware | 中 | 审计日志写入是合规关键，敏感路径脱敏分支（[REDACTED]）未测试 | 审计日志缺失/敏感信息泄露 | 含敏感路径脱敏、IP 提取（X-Real-IP/X-Forwarded-For/ConnectInfo 三级回退）、请求体 50KB 限制等分支，0 测试覆盖 | 新增测试模块覆盖敏感路径脱敏、IP 提取优先级、大请求体截断 | Grep 确认无 `#[cfg(test)]` |
| backend/src/middleware/request_validator.rs | 1-77 | request_validator_middleware / is_state_changing_method | 中 | 未认证状态变更请求的日志记录分支未测试 | 安全审计追溯能力 | JWT/Origin/access_token 三种认证识别逻辑无测试 | 新增测试覆盖：JWT 直通、Cookie 认证识别、未认证状态变更日志 | Grep 确认无 `#[cfg(test)]` |
| frontend/src/store/sales.ts | 1-99 | useSalesStore / fetchOrders / createOrder / submitOrder / approveOrder | 中 | 销售订单状态机操作无测试 | 订单状态变更错误 | 6 个 store 中仅 user/inventory 有测试，sales/dashboard/fabric/index 无 | 新增 sales-store.test.ts，覆盖 fetchOrders/createOrder/状态变更 | Glob `frontend/src/store/*.ts` 6 个；测试仅 user-store/inventory-store |
| backend/src/middleware/slow_query.rs | 1-91 | SlowQueryRecorder / slow_query_threshold / SlowQueryMetrics | 低 | 慢查询阈值通过环境变量覆盖逻辑未测试 | 慢查询采集配置失效 | BINGXI_SLOW_QUERY_MS 环境变量解析、阈值比较、finish() 上报逻辑无测试 | 单测 slow_query_threshold 解析边界、finish 阈值上下分支 | Grep 确认无 `#[cfg(test)]` |
| frontend/src/store/dashboard.ts | 1-70 | useDashboardStore / fetchStats / fetchSalesStats / fetchInventoryStats | 低 | 仪表盘数据加载无测试 | 仪表盘数据展示异常 | 无测试 | 新增 dashboard-store.test.ts | 同上 |

---

## 二、无真实空实现点位列表

| 文件路径 | 行号范围 | 函数/类名 | 风险等级 | 风险理由 | 业务影响 | 问题描述 | 修复建议 | 调研证据 |
|---|---|---|---|---|---|---|---|---|
| frontend/src/views/dye-batch/index.vue | 341 | handleView | 高 | "查看"按钮 handler 完全空，用户点击无反应 | 染缸查看功能完全失效 | `const handleView = (_row: DyeBatch) => {}` — 查看按钮回调为空函数 | 实现查看逻辑：打开详情对话框或路由跳转 | Read 文件 341 行确认；上下文 325-348 行有 handleCreate/handleEdit 实现完整 |
| frontend/src/views/dye-recipe/index.vue | 318 | handleView | 高 | "查看"按钮 handler 完全空 | 染色配方查看功能失效 | `const handleView = (_row: DyeRecipe) => {}` | 同上 | Read 文件 318 行确认 |
| backend/src/services/bi_analysis_service.rs | 254 | dimension_expr match 兜底 | 中 | dimension 参数来自用户请求，传入未预期值触发 panic | BI 多维分析传入非法 dimension 导致 500 panic | `_ => unreachable!()` 在 dimension_expr 函数中；前置 if 已限定合法值，但仍是 panic 风险 | 改为 `return Err(AppError::validation(...))` 或返回默认维度 | Read 行 240-256 确认；dimension 取自 query 参数 |
| backend/src/services/bi_analysis_service.rs | 1188/1203 | pivot_table measure match 兜底 | 中 | measure 参数来自用户请求，传入未预期值触发 panic | BI 透视表传入非法 measure 导致 500 panic | `_ => unreachable!()` 在 pivot_table 中 measure match；两处 | 改为 `return Err(AppError::validation(...))` | Read 行 1180-1206 确认；measure 取自 query 参数 |
| frontend/src/views/dye-recipe/index.vue | 363 | handleViewVersion | 中 | "查看版本"按钮 handler 空 | 版本详情查看失效 | `const handleViewVersion = (_row: DyeRecipe) => {}`；行 352 handleVersion 已实现（打开 versionList 对话框），两者易混淆 | 实现版本详情查看，或删除该函数避免误用 | Read 文件 363 行确认 |
| frontend/src/components/AdvancedFilter.vue | 249 | handleLogicChange | 中 | 逻辑切换（AND/OR）回调为空 | 高级筛选逻辑切换无响应，用户切换无效果 | `const handleLogicChange = () => {}`；模板中 @change 事件绑定该函数 | 实现逻辑切换：更新 conditions 间逻辑运算符 | Read 文件 249 行确认 |
| backend/src/handlers/dual_unit_converter_handler.rs | 116 | convert_dual_unit match 兜底 | 低 | 已通过参数校验，理论不可达；但仍存在 panic 风险 | 单位转换传入未预期值 panic | `_ => unreachable!("已通过单位参数校验，此处不可能到达")` | 改为 `return Err(...)` 防御性编程 | Read 行 100-120 确认；前面有参数校验 |
| frontend/src/components/BatchActions.vue | 124/134/141 | defaultActions 中 3 个 handler | 低 | 默认 actions 的 handler 空，但会被 props.actions 覆盖（if (props.actions.length > 0) return props.actions） | 仅在未传入 props.actions 时生效，影响有限 | `handler: async () => {}` 三处 | 加注释说明是默认占位，或抛错提示必须传入 actions | Read 文件 145-148 行确认 computedActions 优先用 props.actions |
| frontend/src/views/advanced/index.vue | 86/88/89 | tabLoaders 中 ai/recipe/quality 三个回调 | 低 | tab 懒加载映射，部分 tab 不需要主动加载（composable 内部 onMounted 处理） | 无业务影响 | `ai: () => {}, recipe: () => {}, quality: () => {}` | 加注释说明是无需主动加载的占位 | Read 文件 84-90 行确认 |
| backend/src/services/audit_log_service.rs | 445/450/523/528 | 测试代码 panic! | 低 | 在 `#[cfg(test)]` 模块内，是测试断言 | 无 | `panic!("severity 应为 Set")` 等测试断言 | 无需修复 | Read 行 435-460 确认在测试模块内 |
| backend/src/services/so/order_workflow.rs | 473/515/540/562/565/610/651 | 测试代码 panic! | 低 | 在 `#[cfg(test)]` 模块内 | 无 | `panic!("取消订单应返回 BusinessError")` 等测试断言 | 无需修复 | Grep 确认行号；该文件有 19 个测试用例 |

---

## 三、未真实外部接入点位列表

| 文件路径 | 行号范围 | 依赖模块 | 风险等级 | 风险理由 | 业务影响 | 问题描述 | 修复建议 | 调研证据 |
|---|---|---|---|---|---|---|---|---|
| backend/src/services/event_kafka.rs | 1-496 | Kafka（rskafka） | 低 | rskafka 真实连接 broker，真实 produce/fetch_records，无问题 | 无 | 真实实现：ClientBuilder::new(brokers).build()、partition_client.produce(vec![record])、fetch_records 拉取消费；含 5s 连接超时、3 次失败重试 | 无需修复 | Read 全文件确认；测试用例 1 个 |
| backend/src/search/elastic.rs | 1-1021 | Elasticsearch | 低 | reqwest 直连 ES REST API，PUT/POST/DELETE/_bulk/_count/_search 全实现，无问题 | 无 | 真实实现：http.put(&url).json(doc).send().await；NDJSON bulk 格式；同时支持 mock 内存模式用于 CI | 无需修复 | Read 全文件确认；16 个测试用例覆盖 mock 模式 |
| backend/src/services/webhook_service.rs | 1-312 | Webhook 外发 | 低 | reqwest 真实发送 HTTPS POST + HMAC-SHA256 签名 + SSRF 防护，无问题 | 无 | 真实实现：client.post(url).header("X-Webhook-Signature").body(body).send().await；含 resolve_to_addrs 防 DNS Rebinding、redirect::Policy::none() 防重定向 | 无需修复 | Read 全文件确认 |
| backend/src/services/system_update_service.rs | 600-758 | GitHub API + 更新包下载 | 低 | reqwest 真实调用 GitHub API + 下载更新包，无问题 | 无 | 真实实现：client.get(&url).send().await + response.json()；含 validate_download_url SSRF 校验、Policy::limited(3) 限制重定向 | 无需修复 | Read 行 600-758 确认 |
| backend/src/services/currency_service.rs | 290-350 | exchangerate-api.com | 低 | reqwest 真实调用 exchangerate-api.com，无问题 | 无 | 真实实现：client.get(&url).send().await + 解析 rates | 无需修复 | Read 行 290-350 确认 |
| backend/src/services/email_service.rs | 100-230 | sendgrid/aliyun/tencent | 低 | reqwest 真实发送邮件 API 请求，无问题 | 无 | 真实实现：支持 sendgrid/aliyun/tencent 三种 provider；含 HTML XSS 危险模式检测 | 无需修复 | Read 行 100-230 确认 |
| backend/src/config/settings.rs | 1-417 | 配置加载 | 低 | 真实从 config 文件 + 环境变量加载；含弱密钥黑名单校验，无问题 | 无 | 真实实现：config::Environment::default().separator("__") + std::env::var("DATABASE_PASSWORD")；JWT/COOKIE/WEBHOOK/AUDIT secret 全部走 validate_secret 校验 | 无需修复 | Read 全文件确认 |
| backend/src/handlers/webhook_handler.rs、webhook_integration_handler.rs | 全文件 | webhook handler 全部端点 | 低 | 全部调用 WebhookService 真实方法，无问题 | 无 | create_webhook/test_webhook/retry_webhook/get_webhook_logs/send_wechat_message/send_dingtalk_message/handle_generic_callback 全部真实调用 service | 无需修复 | Read 两个文件确认 |
| backend/src/models/fixed_asset_depreciation_record.rs | 1-84 | 注释遗留"占位符" | 低 | 注释说"批次 88 PH-2 占位符实现"，但代码是真实 Entity + service 真实调用 | 误导维护者认为未接入 | 文件级 #![allow(dead_code)] + 注释"占位符实现"，但 fixed_asset_service.rs:283-321 真实 insert 此 entity | 移除"占位符"注释，更新为"已接入 depreciate 业务" | Read 全文件 + fixed_asset_service.rs:283 确认真实 insert |
| backend/src/services/fixed_asset_service.rs | 283/368 | depreciate/dispose 注释遗留"占位符" | 低 | 注释说"占位符实现"，但代码真实写入数据库 | 误导维护者 | 行 283 注释"批次 88 PH-2 占位符实现：插入折旧期间记录"后真实 insert；行 368 注释"批次 88 PH-3 占位符实现"后真实 gain_loss 设置 | 移除"占位符"注释 | Read 行 270-330/360-390 确认真实 insert |
| backend/src/utils/app_state.rs | 70/215/307/318/324/329 | init_search_client 注释 | 低 | 注释解释 mock 模式是 CI 默认，real 模式需配置 ELASTICSEARCH_URL | 无 | 注释"当前为 mock 实现"；代码逻辑真实：if es_url.is_empty() { mock } else { real } | 无需修复 | Grep 多处注释确认 |
| backend/src/services/customer_service.rs | 78 | 注释"mock 模式（CI 环境）" | 低 | 说明 ES 同步在 mock/real 双模式行为 | 无 | 注释解释设计；代码真实调用 self.search_syncer.sync_customer(&doc).await | 无需修复 | Read 行 60-130 确认 |
| SeaORM entity 接入情况 | 全 backend | 100+ 个 entity | 低 | 729 处 Entity::find/ActiveModel 使用点，无问题 | 无 | Grep 确认 100+ 个文件使用 SeaORM entity，覆盖 services/handlers/middleware/utils | 无需修复 | Grep `::Entity::find|::Entity::find_by_id|::ActiveModel` 命中 100 文件 729 处 |

---

## 四、占位符/Mock 存根代码点位列表

| 文件路径 | 行号范围 | 标记关键词 | 风险等级 | 风险理由 | 业务影响 | 问题描述 | 修复建议 | 调研证据 |
|---|---|---|---|---|---|---|---|---|
| backend/src/search/elastic.rs | 264-268 | mock（设计模式） | 低 | mock 是开发/测试/CI 默认模式，real 模式真实接入 ES | 无 | `pub fn mock() -> Self { Self { inner: ClientInner::Mock(...) } }`；real() 真实 reqwest 直连 | 无需修复；建议加文档说明何时切换 real | Read 行 262-285 确认双模式 |
| backend/src/services/so/delivery.rs、order_workflow.rs | 1483-1503/704-771 | ElasticClient::mock()（测试夹具） | 低 | 测试代码用 mock 构造 SalesService 依赖 | 无 | `let search_client: Arc<dyn SearchClient> = Arc::new(ElasticClient::mock());` 在 `#[cfg(test)]` 模块内 | 无需修复 | Grep 多处确认在测试模块内 |
| backend/src/lib.rs | 12 | messaging/ mock 占位（历史注释） | 低 | messaging/ 模块已删除，仅保留注释说明 | 无 | 注释"批次 105 修复：messaging/ 模块已删除" | 无需修复 | Read lib.rs 确认 |
| backend/src/services/bi_analysis_service.rs | 9/262 | demo mock 数据（历史注释） | 低 | 注释说明已修复为真实查询数据库 | 无 | 注释"v9 批次 130 修复：原全部方法返回硬编码 mock 数据，现真实查询数据库"；代码真实 Statement::from_sql_and_values | 无需修复 | Read 行 255-350 确认真实 SQL 查询 |
| backend/src/services/bpm_process_definition_service.rs | 8 | stub（历史注释） | 低 | 文件已重命名消除 stub 误导 | 无 | 注释"批次 95 P3-15 修复：原文件名 bpm_service_stub.rs 含 'stub' 误导" | 无需修复 | Grep 确认 |
| backend/src/handlers/sales_price_handler.rs | 91 | stub（历史注释） | 低 | 已修复为真实接入请求体 | 无 | 注释"批次 199 P1-6：真实接入请求体，原 stub 丢弃 _req 导致 approved=false 仍执行批准" | 无需修复 | Grep 确认 |
| backend/src/handlers/purchase_price_handler.rs | 134 | stub（历史注释） | 低 | 同上 | 无 | 同上 | 无需修复 | Grep 确认 |
| backend/src/handlers/warehouse_handler.rs | 192 | stub（历史注释） | 低 | 已修复为真实字段更新 | 无 | 注释"批次 197 P0-1：真实接入字段更新逻辑，原 stub 仅返回原记录" | 无需修复 | Grep 确认 |
| backend/src/handlers/bpm_definition_handler.rs | 3 | stub dead_code（历史注释） | 低 | 已修复 | 无 | 注释"批次 67（P1 1-2 修复）：原所有 handler 标注 #[allow(dead_code)] 因 stub 未实现" | 无需修复 | Grep 确认 |
| backend/src/routes/system.rs | 163 | stub 占位（历史注释） | 低 | 已修复 | 无 | 注释"原 stub 占位未注册，现 service 层已实现真实逻辑" | 无需修复 | Grep 确认 |
| backend/src/routes/v1.rs | 7 | 占位 404 路由（历史注释） | 低 | 已修复 | 无 | 注释"批次 95 P3-16 修复：移除原占位 404 路由（v1_placeholder）" | 无需修复 | Grep 确认 |
| backend/src/services/purchase_return_service.rs | 6/120 | Some(0) 占位符（历史注释） | 低 | 已修复为真实操作人 user_id | 无 | 注释"P1 1-1 修复（批次 59b）：原 Some(0) 占位符改为真实操作人 user_id" | 无需修复 | Grep 确认 |
| backend/src/utils/cache.rs | 1/3/260 | TODO(tech-debt) | 低 | 标注后续接入时的处理方式 | 无 | `// TODO(tech-debt): 此文件已开启 dead_code 检查` 等技术债务标注 | 无需修复 | Grep 确认 |
| backend/src/services/ai/recipe_opt.rs | 184/196/215 | temp_sum（命名巧合） | 低 | temp_sum 是 temperature 简称，非占位符 temp_ | 无 | `let mut temp_sum = 0.0_f64;` 用于聚合染色温度；非 temp_ 占位符前缀 | 无需修复 | Read 行 175-220 确认是真实温度聚合变量 |
| backend/src/cli/util/backup.rs | 93-158 | temp_dir（临时目录变量） | 低 | 备份/恢复用的临时目录变量名 | 无 | `let temp_dir = "/tmp/bingxi_restore";` 用于 tar 解压临时目录 | 无需修复 | Read 行 90-160 确认真实备份逻辑 |
| backend/src/services/system_update_service.rs | 396 | temp_update 目录 | 低 | 更新包解压临时目录 | 无 | `let extract_dir = self.app_dir.join("temp_update");` | 无需修复 | Grep 确认 |
| backend/src/handlers/system_update_handler.rs | 199-237 | temp_dir 路径校验 | 低 | 上传文件保存到 temp_dir，含路径遍历防护 | 无 | `let temp_dir = std::env::temp_dir();` + canonical_save_path.starts_with(&canonical_temp_dir) 防路径遍历 | 无需修复 | Read 行 195-240 确认 |
| backend/src/services/ar/inv.rs | 234 | mock 辅助函数（注释） | 低 | 注释说明测试用辅助函数 | 无 | 注释"测试使用 mock 形式的辅助函数 compute_due_date / format_invoice_no" | 无需修复 | Grep 确认 |
| backend/src/handlers/import_export_handler.rs | 423 | mock State（注释） | 低 | 注释说明测试夹具需求 | 无 | 注释"handler 早期校验的测试需要 mock State/AppState/AuthContext" | 无需修复 | Grep 确认 |
| backend/src/config/settings.rs | 337/380/382/399 | placeholder 弱密钥黑名单 | 低 | 用于拦截弱密钥 | 无 | `"placeholder"` 在 weak_patterns 数组中，用于校验密钥强度 | 无需修复 | Read 行 380-410 确认 |
| frontend/src/locales/en-US.ts | 63 | placeholders i18n 注释 | 低 | i18n 键占位说明 | 无 | 注释"keys added when wiring Login.vue to i18n (with placeholders)" | 无需修复 | Grep 确认 |

---

## 五、功能简化阉割点位列表

| 文件路径 | 行号范围 | 简化逻辑说明 | 风险等级 | 风险理由 | 业务影响 | 问题描述 | 修复建议 | 调研证据 |
|---|---|---|---|---|---|---|---|---|
| backend/src/services/crm/cust.rs | 265-275 | get_rfm_distribution 返回全 0 占位 | 高 | 永久阉割，返回全 0 占位 JSON；注释"简化实现：返回示例分布"明确未实现 | CRM 客户分层看板永远显示 0 客户，RFM 分布统计功能形同虚设 | `Ok(serde_json::json!({ "VIP": 0, "重要": 0, ... }))` 直接返回零值占位 | 真实批量计算所有客户 RFM 评分并聚合分布；或删除该接口避免误导前端 | 调用链：路由 crm.rs 暴露 /crm/customers/rfm-distribution；同文件 compute_rfm_score 是真实实现，仅 distribution 是占位 |
| backend/src/services/capacity_service.rs | 460-474 | forecast_capacity 硬编码置信度 0.8 | 中 | 永久简化，无配置开关；置信度直接写死 0.8 不基于历史预测准确率计算 | 产能预测结果可信度评估失真，前端展示固定 80% 置信度误导决策 | `confidence: 0.8, // 简化的置信度` 直接返回常量，未做任何历史数据校准 | 基于过去 N 天预测值与实际值偏差计算 confidence，或加配置开关 + TODO 标注 | 调用链：handlers/capacity_handler 路由暴露；测试覆盖：仅 test_capacity.rs 基础用例 |
| backend/src/services/budget_management_service.rs | 560-601 | create_adjustment 跳过审批流 | 中 | 永久简化，无配置开关；注释"简化：直接批准，更新 plan 金额"明确跳过审批 | 预算调整单创建即生效，无审批环节，违反财务管控分离原则 | `approval_status: sea_orm::Set(approval::APPROVED.to_string())` 直接设置已批准，未走审批工作流 | 接入 BPM 审批流或加 approval_required 配置开关；当前应加 TODO 标注 | 调用链：handlers/budget_handler 暴露；测试覆盖：无 create_adjustment 专用单测 |
| backend/src/handlers/webhook_handler.rs | 137-189 | retry_webhook 未持久化历史 payload | 中 | 永久简化，注释明确"当前未持久化历史 payload"；用 retry 事件代替真实重试 | Webhook 重试实际是发送新 retry 事件而非重发原始失败 payload，接收方需自行识别重试场景 | 用 trigger_webhook(id, "retry", &retry_payload) 触发新事件，注释承认是简化方案 | 持久化 webhook delivery 历史 payload，重试时按 delivery_id 取原始 payload 重发 | 调用链：POST /webhooks/:id/retry；测试覆盖：无 retry_webhook 单测 |
| backend/src/services/customer_credit_evaluate.rs | 496-508 | 测试桩占位断言 | 低 | 测试代码而非生产代码；测试仅 assert_eq!(score, 70) 固定值断言，未真实调用 evaluate_cooperation_duration | 单元测试覆盖率虚高，cooperation_duration 与 credit_history 实际算法未被回归保护 | 测试函数体仅 `let score = 70; assert_eq!(score, 70);`，未调用被测函数 | 改为调用真实函数并校验范围；当前生产实现已真实（165-190 行） | 调用链：仅 #[cfg(test)] 内 |
| backend/src/services/ar/vfy.rs | 232-235, 333-352 | auto_verify 策略跳过 | 低 | 受 match_strategy 配置开关控制（run_exact / run_date_order 布尔变量），属临时简化带开关 | 当 match_strategy 关闭精确匹配/日期顺序匹配时，所有发票收款进入 UNMATCHED 列表 | `if run_exact { ... } else { unmatched_invoices = invoices.iter().collect(); }` 配置控制 | 已是配置开关模式，无需修复；建议补 TODO 标注后续默认开启 | 调用链：handlers/ar_reconciliation_handler；测试覆盖：ar_unit_tests.rs 部分覆盖 |
| backend/src/services/ar_service.rs | 175-225 | create_collection 顺序扣减策略 | 低 | 业务策略选择而非阉割；注释"简化策略：按发票顺序扣减"是规则明确 | 收款金额按发票列表顺序扣减，未做按比例/按到期日优先等智能分配 | `for inv_id in inv_ids { let allocate = remaining.min(invoice.unpaid_amount); ... }` | 已是明确业务规则；如需扩展可加 allocation_strategy 配置 | 调用链：handlers/ar_handler；测试覆盖：ar_unit_tests.rs 覆盖 |
| backend/src/services/scheduling_query.rs | 82-88 | get_gantt_data 闭包临时 fallback | 低 | 闭包内 fallback "未知"已被第 104-119 行的批量查询 work_centers + build_gantt_data 替换；注释"暂时 fallback"过期 | 实际无影响，scheduled_details 中的 wc_name 会在 build_gantt_data 中被 work_centers 替换 | 闭包内 `let wc_name = if o.work_center_id.is_none() { "未指定" } else { "未知" }` | 删除过期注释或修正为"占位值，将在下方批量查询替换" | 调用链：handlers/scheduling_handler；测试覆盖：test_scheduling.rs |
| backend/src/services/slow_query_collector.rs | 116, 166 | start_collection_loop 跳过首次/空查询 | 低 | 跳过首次立即触发（启动时手动执行）和跳过空查询，属业务合理 | 无负面影响，避免启动时重复采集 + 避免空字符串污染统计 | `if first_run { first_run = false; continue; }` | 无需修复，注释清晰 | 调用链：main.rs 启动 |
| backend/src/services/auth_service.rs | 671 | start_token_cleanup_loop 跳过首次 | 低 | 跳过首次立即触发，启动时无需清理；属业务合理 | 无负面影响 | `if first_run { first_run = false; continue; }` | 无需修复 | 调用链：main.rs 启动 |

---

## 六、死代码点位列表

| 文件路径 | 行号范围 | 死代码类型 | 风险等级 | 风险理由 | 业务影响 | 问题描述 | 修复建议 | 调研证据 |
|---|---|---|---|---|---|---|---|---|
| frontend/src/views/finance/tabs/composables/ 等 14 文件 | 1 | eslint-disable 类型逃逸 | 中 | 14 个 composable 文件首行 `/* eslint-disable @typescript-eslint/no-explicit-any */`，全文件禁用 any 检查 | 类型安全降级，潜在运行时错误风险 | 全文件级禁用 any 类型检查（useVchr / useVchrProc / usePc / useApiKey 等 14 个） | 将 any 替换为 unknown 或具体业务类型，移除 eslint-disable | 调用链：各业务 composable 已使用；测试覆盖：use-table-columns.test.ts 部分覆盖 |
| backend/src/models/*.rs | 1-2 | 文件级 #![allow(dead_code)] 抑制 | 低 | 项目规则明确允许 backend/src/models/ 例外（SeaORM 派生宏使用），且每文件均带 TODO(tech-debt) 注释 | 无业务影响，符合例外规则 | 文件首行 `#![allow(dead_code)]` + 次行 TODO 注释 | 无需修复，符合例外 | 100 个文件全部位于 models/，0 个位于 utils/handlers/services 等业务目录 |
| backend/src/utils/cache.rs | 1-3 | utils/ 模板合规 | 低 | 已移除文件级 #![allow(dead_code)]，仅保留注释说明；符合 utils/ 模板要求 | 无 | 注释说明"此文件已开启 dead_code 检查" | 无需修复 | utils/ 目录无任何 #![allow(dead_code)] 或项级 #[allow(dead_code)] |
| backend/src/models/status.rs | 238, 243 | 项级死代码-合规标注 | 低 | 项级 #[allow(dead_code)] + TODO(tech-debt) 完整标注，符合规范 | 库存预留 lock_reservation / release_reservation 方法尚未接入路由 | `#[allow(dead_code)] // TODO(tech-debt): lock_reservation 方法尚未接入路由，接入后移除` | 实现 lock_reservation / release_reservation 路由后移除标注 | 调用链：搜全项目无 lock_reservation 方法引用 |
| backend/src/models/dto/mod.rs | 32, 39, 45, 51 | 项级死代码-通用工具未接入 | 低 | 4 个分页工具方法均带 TODO 标注；但 utils/pagination.rs 已提供 paginate_with_total，应删除并统一接入 | PageRequest 工具类未实际使用，分页逻辑分散在 20+ service 中 | `#[allow(dead_code)] // TODO(tech-debt): 分页工具方法，待统一接入 paginate_with_total 后移除` | 全项目接入 paginate_with_total 后删除 PageRequest 4 个方法 | 调用链：搜全项目 PageRequest::new / page_clamped 无业务调用；paginate_with_total 仅 4 处使用 |
| backend/src/models/dto/bpm_dto.rs | 48 | 项级死代码-字段未启用 | 低 | 项级标注合规；模板 category 当前均为 __TEMPLATE__，子分类功能未实现 | BPM 模板分类二次筛选不可用 | `#[allow(dead_code)] // TODO(tech-debt): 模板子分类功能实现后移除` | 实现 BPM 模板子分类后移除 | 调用链：handler 未使用 category 字段 |
| backend/src/models/user_notification_setting.rs | 55 | 项级死代码-常量未启用 | 低 | 项级标注合规；NONE 通知类型未接入业务 | 用户无法配置"关闭所有通知" | `#[allow(dead_code)] // TODO(tech-debt): 通知类型 NONE 接入后移除` | 接入 NONE 通知类型后移除 | 调用链：搜全项目无 NotificationType::NONE 业务引用 |
| frontend/src/i18n/index.ts | 6-10 | 国际化渐进迁移 | 低 | TODO(tech-debt) 标注仅 Login.vue 接入；4506 行资源已就绪但 90% .vue 仍硬编码中文 | 国际化切换在大部分页面无效 | 注释明确"批次 23 v5 P0-1 仅完成 Login.vue 示范接入" | 逐页面替换硬编码文本为 $t() 调用 | 调用链：app.use(i18n) 已挂载；测试覆盖：无 i18n 单测 |
| frontend/src/store/user.ts | 7 | eslint-disable 单行 | 低 | 仅单行禁用 unused-vars，影响小 | 无 | `// eslint-disable-next-line @typescript-eslint/no-unused-vars` | 移除未使用变量或重命名 | 单点禁用，可控 |

---

## 七、重复实现点位列表

| 文件路径 | 行号范围 | 重复类型 | 重复相似度 | 风险等级 | 风险理由 | 业务影响 | 问题描述 | 修复建议（抽取建议） | 调研证据 |
|---|---|---|---|---|---|---|---|---|---|
| backend/src/services/ar_invoice_service.rs 等 20 文件 | 各 service get_list 方法 | 跨模块重复-分页逻辑 | 90%+ | 中 | 完全重复 90%+；每处均 let total = query.clone().count + offset/limit.all | 维护成本高，分页 off-by-one 修复需逐文件改；utils/pagination.rs 已提供 paginate_with_total 但仅 4 处使用 | `let total = query.clone().count(&*self.db).await?; let items = query.offset(...).limit(...).all(&*self.db).await?;` 在 20 个 service 中重复 | 统一接入 paginate_with_total | 20 处命中 vs 4 处使用 paginate_with_total；命中文件：ar_invoice_service.rs:238, sales_contract_service.rs:109, fixed_asset_service.rs:137, email_template_service.rs:200, fund_management_service.rs:69, quality_inspection_service.rs:90/161/282, report_subscription_service.rs:233, purchase_contract_service.rs:110, inventory_stock_query.rs:286, purchase_price_service.rs:62, production_order_service.rs:321, quality_standard_service.rs:78, currency_service.rs:116/194, assignment_history_service.rs:171, cost_collection_service.rs:149, product_category_service.rs:42, scheduling_query.rs:149 |
| frontend/src/views/ai-extend/index.vue 等 30+ 文件 | 各 view setup 顶部 | 跨模块重复-前端表格逻辑 | 70-90% | 中 | 高度相似 70-90%；每处均 const loading = ref(false); const total = ref(0); const list = ref([]) + 自实现 fetch | 表格分页/重试/loading 逻辑分散，useTableApi 已提供但仅 3 处使用 | 直接 ref + watch + fetch 模式散落在 30+ view 中 | 统一接入 useTableApi composable | useTableApi 仅 3 处使用（quality/RecordTab, production/usePrd, sales/useOlv）；useTableColumns 仅 1 处使用；命中：ai-extend/process-detail.vue:17, ai-extend/index.vue:18, ai-extend/process-optimization.vue:22-24, ai-extend/quality-prediction.vue:25-27, greige-fabrics/index.vue:136, custom-orders/list.vue:148, custom-orders/tracking.vue:92, accountSubject/tabs/SubjectListTab.vue:145, inventoryBatch/tabs/BatchListTab.vue:156, crm/pool.vue:205-207, crm/leads/index.vue:261-263, dye-recipe/index.vue:241-243, crm/opportunities/index.vue:284-286, fiveDimension/index.vue:30-31, crm/detail.vue:267, customer/index.vue:146 等 |
| backend/src/utils/sql_escape.rs | 4-25 | 工具函数-非重复 | - | 低 | 功能不重复：escape_like_pattern 仅转义特殊字符，safe_like_pattern 在转义前后加 % 通配符，符合单一职责 | 无 | 两个函数职责清晰分层 | 无需修复 | safe_like_pattern 内部调用 escape_like_pattern |
| backend/src/services/*_service.rs | 各 get_list/get_by_id | CRUD 命名不一致 | - | 低 | 部分 service 用 list_*，命名风格不完全统一 | 无功能影响，仅风格不一致 | sales_contract_service.rs:82 用 get_list，ar_invoice_service.rs:219 用 get_list，但 capacity_service.rs:121 用 list_work_centers | 统一为 list_* 命名风格 | 影响范围小 |

---

## 八、项目规则符合性问题列表

| 文件路径 | 行号范围 | 违反规则 | 风险等级 | 风险理由 | 业务影响 | 问题描述 | 修复建议 | 调研证据 |
|---|---|---|---|---|---|---|---|---|
| backend/src/cli/util/service.rs | 191 | 硬编码-健康检查 URL | 中 | 违反"项目禁止硬编码"规则；端点写死 127.0.0.1:8082 无法通过配置覆盖 | 部署到非 8082 端口的环境健康检查失效 | `"http://127.0.0.1:8082/health"` 直接硬编码 | 改为从环境变量或配置文件读取 backend_url | 调用链：CLI 健康检查命令；测试覆盖：无 |
| backend/src/config/settings.rs | 178-179, 290-291 | 硬编码-默认 CORS | 低 | 默认值可被配置覆盖，但仍是硬编码 localhost | 开发环境默认值，生产环境通过配置覆盖 | `"http://localhost:3000".to_string()` 等默认 CORS 来源 | 改为从 .env.example 引导，默认值留空 | 配置层，影响有限 |
| backend/src/observability/config.rs | 58, 76 | 硬编码-默认 OTLP | 低 | 默认 OTLP endpoint，可被环境变量覆盖 | 生产环境通过 OTEL_EXPORTER_OTLP_ENDPOINT 覆盖 | `otlp_endpoint: "http://localhost:4317".to_string()` | 默认值改为空，强制配置 | 配置层，影响有限 |
| backend/src/handlers/omni_audit_handler.rs | 254-319 | SQL 拼接-参数化安全（合规） | 低 | 使用 $N 占位符 + 参数绑定；LIKE 用 safe_like_pattern 转义；select_fields 仅常量拼接 | 无 SQL 注入风险，合规 | `format!("SELECT {} FROM omni_audit_logs{} ORDER BY id DESC LIMIT ${} OFFSET ${}", ...)` 但所有变量均参数化 | 无需修复，已合规 | safe_like_pattern 转义 % _ \；filter.include_sensitive 仅常量选择 |
| backend/src/services/init_service.rs | 679 | 硬编码-测试密码 | 低 | 测试夹具中硬编码密码 "p@ss word"，非生产代码 | 无生产影响 | `password: "p@ss word".to_string()` | 测试夹具可接受 | 仅 #[cfg(test)] 内 |
| backend/src/utils/app_state.rs | 286 | 硬编码-测试密钥 | 低 | 测试夹具 webhook_secret，非生产代码 | 无生产影响 | `webhook_secret: "test_webhook_secret_for_unit_tests_only_min_32_bytes".to_string()` | 测试夹具可接受 | 仅测试函数内 |
| backend/src/services/batch_service.rs | 153, 311, 356 | 错误吞没-best-effort | 低 | 事务已失败，回滚失败不应阻塞错误返回路径；属合理 best-effort 模式 | 无 | `txn.rollback().await.ok();` 忽略回滚错误 | 无需修复，注释明确"P2 2-9 修复：事务内任一失败则整体回滚" | 调用链：handlers/batch_handler |
| backend/src/handlers/inventory_stock_handler.rs | 266, 393 | 错误吞没-best-effort | 低 | 通知设置查询失败不应阻塞库存预警主业务；注释明确"避免循环内 N+1 查询" | 无 | `let setting = event_service.get_setting_for_user(0).await.ok();` 忽略通知设置错误 | 无需修复，best-effort 合理 | 调用链：库存预警路由 |
| backend/src/utils/audit.rs | 44 | 错误吞没-best-effort 合规 | 低 | 注释明确"best-effort 写入，调用方以 .await.ok() 形式忽略错误" | 审计日志失败不阻塞主业务，符合规范 | 文档注释明确 best-effort 模式 | 无需修复 | 设计层合规 |
| backend/src/services/customer_credit_evaluate.rs | 33-39 | 错误吞没-defensive | 低 | .await.ok().flatten().map(...).unwrap_or_else(...) 防御性降级 | 无业务影响，仅日志展示降级 | `let customer_name = ... .await.ok().flatten().map(...).unwrap_or_else(...)` | 无需修复，防御性编程合理 | 调用链：信用评估 API |
| backend/src/utils/（全部 8 个核心文件） | - | 死代码规范-合规 | 低 | 项目规则要求 utils/ 全部移除文件级 #![allow(dead_code)]；扫描确认全部合规 | 无 | utils/ 下 0 个文件级抑制，0 个项级 #[allow(dead_code)]（仅 cache.rs 注释提及） | 无需修复，符合 utils/ 模板 | 8 个核心文件全部合规 |
| backend/src/main.rs | 514 | 死代码规范-已接入 | 低 | 项级 #[allow(dead_code)] 标注的方法已真实接入 main.rs 启动流程 | 无 | `// 批次 120 P2-7 修复：启动时初始化 8 个辅助核算维度（幂等实现）` | 无需修复，已接入 | 调用链：main.rs:512-516 真实调用 |

---

## 九、性能问题点位列表

| 文件路径 | 行号范围 | 性能问题类型 | 风险等级 | 风险理由 | 业务影响 | 问题描述 | 修复建议 | 调研证据 |
|---|---|---|---|---|---|---|---|---|
| backend/src/services/ar_service.rs | 1274-1321 | get_aging_report 未分页/全表扫描 | 高 | 无 date 范围限制 + 无 LIMIT，customer_id 缺省时全表扫描所有未取消且未付清的发票 | 账龄报表在数据量增长后单次查询可能返回数万行，导致内存峰值与响应延迟，可能触发 OOM | `let invoices = q.all(&*self.db).await?;` 仅 filter Status != CANCELLED 和 UnpaidAmount > 0，无日期上限/分页 | 改用 SQL 层聚合（CASE WHEN + SUM GROUP BY bucket）或加默认近 1 年日期范围 + LIMIT 上限 | 调用方：ar_report_handler.rs；测试覆盖：仅 ar_unit_tests.rs 测试业务逻辑，无该报表性能测试 |
| backend/src/services/ar_service.rs | 1108-1160 | get_statistics_report 未分页查询 | 中 | 有日期范围 filter 但无 LIMIT，宽日期范围（如 5 年）仍可能返回大量行 | 大日期范围报表延迟高，影响前端响应 | `let invoices = q.all(&*self.db).await?;` filter by InvoiceDate 范围 | 改为 SQL 聚合查询 SUM/COUNT 直接在 DB 层完成 | 同上 |
| backend/src/services/ar_service.rs | 1164-1216 | get_daily_report 未分页查询 | 中 | 同上，宽日期范围下全量加载到内存做 HashMap 聚合 | 内存占用与日期跨度成正比 | `let invoices = q.all(&*self.db).await?;` 后内存聚合 | 改为 GROUP BY invoice_date SQL 聚合 | 同上 |
| backend/src/services/ar_service.rs | 1218-1270 | get_monthly_report 未分页查询 | 中 | 同上 | 同上 | `let invoices = q.all(&*self.db).await?;` 后内存聚合 | 改为 GROUP BY date_trunc('month', invoice_date) SQL 聚合 | 同上 |
| backend/src/services/ap_report_service.rs | 25-216 | 4 个报表方法未分页查询 | 中 | 7 处 .all(&*self.db) 全量加载发票到内存做聚合 | 同 AR 报表，宽日期范围下内存压力 | 第 42/135/146/163/216/230/262 行 .all(&*self.db) | 同上，改为 SQL 聚合 | 无对应单元测试覆盖性能 |
| backend/src/services/cache_service.rs + 各 service | 全文件 | 缓存未利用 | 中 | L1 缓存（moka LRU+TTL）已在 AppState 注入但仅 dashboard_service.rs 使用；customer/supplier/product/inventory 等热点查询未走缓存 | 高频列表查询、产品/客户下拉框等热点数据未命中缓存，DB 压力大 | grep 显示仅 dashboard_service.rs:165/240/278/367/484/609/786/843 使用 cache.get_*，其它 service 文件无 state.cache_service 引用 | 在 customer_service、product_service、supplier_service 的列表/详情查询接入 L1 缓存，配合写操作 invalidate_prefix 失效 | cache_service.rs 单测覆盖 8 个用例，但业务接入率低 |
| backend/src/services/inventory_stock_service.rs | 170-238 | check_low_stock 未分页查询 | 低 | .all(&txn) 全量加载低库存记录到内存，并循环构造事件 | 单仓库低库存记录通常有界，但跨仓库扫描时无 LIMIT 兜底 | `let low_stock_items = query.all(&txn).await?;` | 加 LIMIT 10000 兜底防极端场景 | 有 inventory_unit_tests.rs 但未覆盖大数据量场景 |
| backend/src/services/ai/rec.rs | 36-60 | optimize_inventory 未分页查询 | 低 | 全量加载 inventory_stock + 90 天 transactions 到内存做计算 | 数据量增长后内存压力 | `let stocks = select.all(&*self.db).await?;` + `let transactions = ...all(&*self.db).await?;` | 改为流式迭代或分页处理；transactions 改用 SQL 聚合 | 无单元测试覆盖 |

**前端性能未检测到问题**：el-table 在 100+ Vue 文件中使用，但 73 处均搭配 el-pagination；V2Table 组件已封装 el-table-v2 虚拟滚动；路由 100% 使用 `() => import()` 懒加载；未发现 watch 误用。

**未检测到问题**：N+1 查询（ap_verification_service.rs:96、ar_service.rs:178/568、production_order_service.rs:642 已通过批量查询 + HashMap O(1) 查找规避）；过度克隆（仅 Arc::new(... db.clone()) 模式，Arc clone 廉价）；内存泄漏（无 Box::leak / static mut）。

---

## 十、安全漏洞点位列表

| 文件路径 | 行号范围 | 漏洞类型 | 风险等级 | 风险理由 | 业务影响 | 问题描述 | 修复建议 | 调研证据 |
|---|---|---|---|---|---|---|---|---|
| frontend/src/views/report-templates/index.vue | 171 + 344-350 | sanitizedPreview + handlePreview XSS（潜在） | 中 | 行 348 将行数据 r[f] 直接拼接到 HTML 字符串再交 DOMPurify 净化，DOMPurify 默认允许 <img> <a> 等标签，攻击者可注入 <img src=x> 但 onerror 已被 FORBID_ATTR 拦截 | 报表预览页若数据来自外部导入，可能展示误导性图片/链接（脚本执行已被 DOMPurify+FORBID_TAGS 拦截） | `bodyHtml = res.data.rows.map(r => ... '<td>${String(r[f] ?? "")}</td>' ...)` 字符串拼接后整体 sanitize | 对单元格值先 HTML escape 再拼接，DOMPurify 作为第二层防御 | 调用方：previewReportTemplate API；print-templates/index.vue 同模式但 previewData.value 来自后端，风险较低 |
| backend/src/handlers/tracking_handler.rs | 28-53 | PageViewRequest / BehaviorRequest 输入验证缺失 | 中 | 7 个请求体结构体（含 path/event_type/event_data: serde_json::Value）未派生 Validate，未限制字段长度，event_data 接受任意 JSON | 攻击者可发送超大 path/超深嵌套 event_data 触发 DB 写入放大或 JSON 解析 CPU 耗尽 | `#[derive(Debug, Deserialize)] pub struct PageViewRequest { pub path: String, ... }` 无 #[validate(length(max=...))] | 给所有字符串字段加 #[validate(length(max=N))]，event_data 限制最大深度/字节数 | tracking_handler 7 个端点全部要求 AuthContext（已认证），降低了风险 |
| backend/src/openapi.rs | 76-95 | components.schemas 敏感信息泄露（潜在） | 低 | 直接暴露 Model 完整 schema（purchase_contract::Model 等），若模型后续新增 password_hash/api_key 等字段会自动同步到 Swagger UI 公开文档 | Swagger UI 公开环境下泄露内部字段结构，便于攻击者构造 payload | `crate::models::purchase_contract::Model` 等直接作为 schema 注册 | 改为定义专属 ResponseDto 并 #[serde(skip_serializing)] 敏感字段，或在 schema 上白名单字段 | 项目当前未启用 swagger feature 时无暴露，但 Cargo.toml 默认开启 |

**未检测到问题**：SQL 注入（omni_audit_handler.rs:237/308 动态拼接 WHERE 但用 $N 参数占位符 + safe_like_pattern 转义；未发现 format!() 直接拼接用户输入到 SQL）；CSRF（middleware/csrf.rs 覆盖所有 POST/PUT/PATCH/DELETE，公开路径要求自定义头）；认证授权（PUBLIC_PATHS 仅 6 条，全部合理；admin 检查统一走 admin_checker::is_admin_role 带缓存；permission_middleware 全局挂载）；JWT 密钥（settings.rs:318 从 JWT_SECRET 环境变量读取，强制 32 字节强度校验）；密码时序攻击（Argon2 verify_password 内部常数时间比较，init_token.rs:64 和 webhook_signature.rs:60 用 subtle::ConstantTimeEq）；SSRF（ssrf_guard.rs 被所有 webhook 调用使用，validate_url_and_resolve 防御 RFC1918/loopback/metadata）；越权访问（m0029 已 drop tenant columns，单租户系统）；日志泄露（auth.rs:28-58 对 Authorization 头和 username 做脱敏）。

---

## 十一、并发与线程安全问题点位列表

| 文件路径 | 行号范围 | 并发问题类型 | 风险等级 | 风险理由 | 业务影响 | 问题描述 | 修复建议 | 调研证据 |
|---|---|---|---|---|---|---|---|---|
| backend/src/services/auth_service.rs | 107 | AuthService::authenticate async 误用/阻塞操作 | 高 | verify_password 是同步函数，内部 Argon2id（m=65536KB=64MB, t=3, p=4）单次哈希耗时 50-100ms，直接在 async fn 内调用会阻塞当前 worker 线程 | 登录高峰期（如早 9 点集中登录）会让 tokio worker 阻塞，导致其它请求排队，整个服务吞吐下降 | `pub async fn authenticate(...) { ... Self::verify_password(password, &user.password_hash)?; ... }` 同步调用 | 用 tokio::task::spawn_blocking(move || Self::verify_password(...)).await?? 包装 | 全项目 grep spawn_blocking 零结果；调用方：auth_handler.rs:309 login 端点 |
| backend/src/services/auth_service.rs | 243-258 | AuthService::verify_password async 误用 | 高 | 同上，被 authenticate 和 user_handler.rs:538/563 直接调用 | 修改密码接口（change_password）会两次调用 verify_password + 一次 hash_password，累计阻塞 150-300ms | 同步函数签名 `pub fn verify_password(password: &str, hash: &str) -> Result<bool, AuthError>` | 同上，或在调用方 spawn_blocking | 测试 test_password_hash_and_verify 验证逻辑但不验证并发安全 |
| backend/src/services/auth_service.rs | 277-291 | AuthService::hash_password async 误用 | 高 | 同上，被 user_handler.rs:196/578 和 init_service.rs:170/574 直接调用 | 创建用户 / 修改密码接口阻塞 worker | 同步函数签名 `pub fn hash_password(password: &str) -> Result<String, AuthError>` | 同上 | 同上 |
| backend/src/handlers/user_handler.rs | 196, 538, 563, 578 | create_user / change_password async 误用 | 高 | 4 处在 async handler 内直接调用同步 hash_password / verify_password | 同上 | `let password_hash = AuthService::hash_password(&payload.password)?;` 在 pub async fn create_user 内 | 在 handler 内 tokio::task::spawn_blocking 包装哈希计算 | 测试 test_password_hash_and_verify 覆盖正确性 |
| backend/src/middleware/rate_limit.rs | 30, 74-109 | MemoryRateLimiter::check 锁粒度（潜在） | 低 | std::sync::Mutex 在 async 中间件内 try_lock，持锁期间做 HashMap 查找/插入（O(1)），但全局单一 Mutex 在高并发下可能成为瓶颈 | 高 QPS 下限流计数可能成为串行点；当前 180 req/min/user 阈值下影响可忽略 | `storage: Arc<std::sync::Mutex<HashMap<String, RateLimitInfo>>>` 单锁 | 可改用 DashMap 分片锁（项目已有 DashMap 依赖），或保留 try_lock fail-open 策略 | 已有 3 个单元测试覆盖 |
| backend/src/services/event_bus.rs | 167-178 | EVENT_BUS_STATE / lock_event_bus_state 锁粒度（潜在） | 低 | 全局 std::sync::Mutex<EventBusState> 在事件发布路径同步持锁，订阅者注册/查找均走该锁 | 事件高频发布时（如批量库存扣减）可能阻塞；当前 publish 已改为 tokio::spawn 异步消费，影响降低 | `static EVENT_BUS_STATE: LazyLock<std::sync::Mutex<EventBusState>>` | 评估改用 RwLock 或 DashMap 分片；当前实现注释说明设计权衡 | 有 4 个 spawn 调用点包装消费逻辑 |

**未检测到问题**：数据竞争（AppState 所有共享字段均 Arc<T> 包装，DashMap 用于并发 HashMap，OnceLock/LazyLock 用于全局初始化）；死锁风险（grep 跨行嵌套锁模式零结果，未发现 lock().await 内再 lock().await）；async spawn 任务（tokio::spawn 19 处均有 catch_unwind 或 best-effort 注释，无未取消的 dangling task）；资源泄漏（数据库连接走 Arc<DatabaseConnection> 连接池）。

---

## 十二、API 契约与文档完整性问题点位列表

| 文件路径 | 行号范围 | 问题类型 | 风险等级 | 风险理由 | 业务影响 | 问题描述 | 修复建议 | 调研证据 |
|---|---|---|---|---|---|---|---|---|
| backend/src/openapi.rs | 10-74 | ApiDoc::paths 文档完整性/契约一致性 | 高 | 仅注册 8 个 handler 的接口路径，而 handlers/ 目录共 115 个 handler 文件，约 95% 接口无 OpenAPI 文档 | 前端联调只能靠抓包，外部集成方无法通过 Swagger 了解接口契约；接口变更无文档同步保障 | `#[openapi(paths(auth_handler::login, ..., fund_management_handler::get_transfer_record))]` 仅覆盖 auth/user/purchase_contract/sales_contract/fixed_asset/budget/quality_standard/fund 8 个域 | 按业务域分批补全 paths 注册，可拆分为多个 OpenApi 子文档；优先补 quotations/custom_orders/color_card/color_price/crm 等核心业务 | frontend/src/api/ 共 95 个 API 文件，大量调用未在 OpenAPI 注册的端点 |
| backend/src/openapi.rs | 76-95 | components.schemas 文档完整性 | 高 | 仅注册 5 个 schema（LoginRequest/LoginResponse/ApiResponse<String>/4 个 Model），绝大多数 DTO 未注册 | Swagger UI 响应示例缺失，前后端字段命名一致性无机器校验 | `schemas(LoginRequest, LoginResponse, ApiResponse<String>, purchase_contract::Model, sales_contract::Model, fixed_asset::Model, budget_management::Model)` | 给所有 #[derive(ToSchema)] 的 DTO 补充 schema 注册；可建立 schema 自动收集脚本 | models/ 下有 30+ DTO 文件未在 OpenAPI 出现 |
| backend/src/handlers/health_handler.rs | 126 | health_check 错误响应统一 | 低 | 健康检查端点直接返回 (status_code, Json(health)) 而非 ApiResponse::success(health) | 健康检查通常是基础设施探针，非业务接口，独立格式可接受；但与全局 ApiResponse 契约不一致 | `(status_code, Json(health))` 直接返回 | 评估是否需要统一为 ApiResponse（K8s probe 通常只看 status code，可不改） | K8s probe 仅检查 HTTP 状态码，不影响业务 |
| backend/src/utils/response.rs | 11-23 | ApiResponse<T> 错误响应统一（潜在） | 低 | code 字段为 Option<u16> 直接用 HTTP 状态码（200/500 等），未定义业务错误码命名空间 | 业务错误（如 TooManyRequests）与 HTTP 状态码混用，前端难以区分业务错误类型 | `pub code: Option<u16>` + error_code() 在 error.rs:415 单独实现 | 评估引入独立的 error_code: Option<String> 字段（如 USER_NOT_FOUND），与 HTTP code 分离 | 已有 CSRF_TOKEN_MISSING/CSRF_TOKEN_INVALID 等业务码，但未在 ApiResponse 顶层暴露 |

**未检测到问题**：RESTful 规范（路由全部用名词复数 + HTTP 方法语义，/users GET/POST/PUT/DELETE 等）；版本化（所有业务路由 /api/v1/erp/* 前缀）；参数验证（#[derive(Validate)] 在 30+ DTO 文件中使用，覆盖 length/range/email/custom，page_size 普遍 clamp(1, 100) 防 DoS）；契约一致性（前端 frontend/src/api/ 与后端 routes/ 路径一致，前后端均用 snake_case 字段，无驼峰/下划线混用问题）；v1.rs 为空 Router 占位符合规范。

---

## 十三、模块级整体评估

| 模块路径 | 健康度评分 | 12类问题分布 | 测试覆盖 | 接入完整性 | 规则符合度 | 性能/安全/并发评分 | 优先级建议 |
|---|---|---|---|---|---|---|---|
| backend/src/middleware/ | 60 | 测试覆盖缺(5)、并发锁粒度(1) | 极低：15 文件仅部分有测试，permission/timeout/omni_audit/request_validator/slow_query 零测试 | 高：CSRF/auth/permission/rate_limit 全部接入路由 | 高：合规 best-effort、参数化查询 | 安全 85/并发 70/性能 80 | 高：补 permission/omni_audit/timeout 单测；评估 rate_limit 锁粒度 |
| backend/src/handlers/ | 65 | 测试覆盖缺(1)、空实现 unreachable(2)、输入验证缺(1) | 极低：覆盖率约 10%（12/100+） | 高：webhook 全部真实接入 | 中：tracking_handler 输入验证缺；bi unreachable panic | 安全 75/并发 70/性能 75 | 高：补核心 handler 单测；修复 bi unreachable；补 tracking 验证 |
| backend/src/services/ | 70 | 测试覆盖缺(4)、简化阉割(4)、性能未分页(6)、async 阻塞(3)、缓存未用(1) | 低：52/159 含测试（32.7%）；ai 子模块算法零测试 | 高：Kafka/ES/Webhook/Email/GitHub/汇率全部真实接入 | 中：硬编码 URL、跳过审批、硬编码置信度 | 性能 60/安全 75/并发 65 | 高：auth spawn_blocking；ar/ap 报表 SQL 聚合；接入缓存；补 ai 算法单测 |
| backend/src/utils/ | 90 | 无新增问题 | 高：8 个核心文件全部合规，多数有测试 | 高：response/cache/pagination/ssrf_guard/sql_escape 全部被业务调用 | 高：utils/ 模板全部合规 | 性能 85/安全 85/并发 85 | 低：维持现状 |
| backend/src/models/ | 85 | 项级死代码(4，均合规标注) | 不适用（SeaORM 自动生成） | 高：729 处真实使用 | 高：符合例外规则 | 不适用 | 低：按 TODO 逐步接入业务 |
| backend/src/routes/ | 80 | 文档完整性(2) | 不适用（路由注册层） | 高：全部 RESTful + /api/v1 前缀 | 高：合规 | 不适用 | 中：补 openapi paths/schemas 注册 |
| backend/src/search/ | 85 | mock 设计模式(1，合理) | 高：16 个测试用例覆盖 mock 模式 | 高：reqwest 真实直连 ES REST | 高：合规 | 性能 80 | 低：维持现状 |
| backend/src/config/ | 80 | 硬编码默认值(2) | 中：settings 有 validate_secret 测试 | 高：真实从 env + config 加载 | 中：localhost 默认值 | 安全 80 | 低：默认值改空强制配置 |
| frontend/src/api/ | 50 | 测试覆盖严重不足(1) | 极低：4/91 文件有测试（4.4%） | 高：与后端路由一致 | 中：缺验证 | 性能 75/安全 75 | 高：补关键 API 模块单测 |
| frontend/src/components/ | 70 | 空实现(2) | 低：仅 v2-table 有测试 | 中：AdvancedFilter handleLogicChange 空 | 中 | 性能 80 | 中：实现 AdvancedFilter 逻辑切换；补单测 |
| frontend/src/store/ | 55 | 测试覆盖缺(2)、eslint-disable(1) | 低：仅 user/inventory 有测试 | 高 | 中 | 性能 80 | 高：补 sales/dashboard store 单测 |
| frontend/src/views/ | 60 | 空实现(2)、重复表格逻辑(1)、XSS 潜在(1) | 极低 | 高 | 中：report-templates XSS 潜在 | 性能 85/安全 70 | 高：实现 dye-batch/dye-recipe handleView；统一 useTableApi；修复 report-templates XSS |
| frontend/src/composables/ | 65 | eslint-disable any(1，14 文件) | 低：use-table-columns 有测试 | 高：useTableApi 已提供但接入率低 | 中：any 类型逃逸 | 性能 80 | 中：移除 eslint-disable，替换 any；推广 useTableApi |
| frontend/src/utils/ | 85 | 无问题 | 中 | 高 | 高 | 性能 85 | 低：维持现状 |
| backend/src/services/ai/ | 55 | 测试覆盖缺(3)、性能未分页(1) | 极低：仅 quality_pred/recipe_opt 有测试 | 高：真实算法实现 | 中 | 性能 65/安全 75 | 高：补 pred/detect/rec 算法单测；rec 优化库存改流式 |
| backend/src/services/crm/ | 65 | 简化阉割(1，高风险) | 低 | 高：compute_rfm_score 真实 | 中：get_rfm_distribution 占位 | 性能 75 | 高：实现 rfm_distribution 真实计算 |
| backend/src/services/ar/ | 60 | 性能未分页(4，1 高风险) | 中：ar_unit_tests 部分覆盖 | 高 | 高 | 性能 55/安全 80 | 高：ar 报表改 SQL 聚合 + 加 LIMIT |
| backend/src/services/ap/ | 65 | 性能未分页(1) | 低 | 高 | 高 | 性能 60 | 中：ap 报表改 SQL 聚合 |

---

## 汇总统计
- 高风险问题总数：15
- 中风险问题总数：25
- 低风险问题总数：74
- 各维度问题分布：
  - 维度一（测试覆盖不足）：13（高 4 / 中 7 / 低 2）
  - 维度二（无真实空实现）：11（高 3 / 中 4 / 低 4）
  - 维度三（未真实外部接入）：13（高 0 / 中 0 / 低 13，全部真实接入或历史注释遗留）
  - 维度四（占位符/Mock 存根）：21（高 0 / 中 0 / 低 21，全部合理设计或测试夹具）
  - 维度五（功能简化阉割）：10（高 1 / 中 3 / 低 6）
  - 维度六（死代码）：9（高 0 / 中 1 / 低 8）
  - 维度七（重复实现）：4（高 0 / 中 2 / 低 2）
  - 维度八（项目规则符合性）：12（高 0 / 中 1 / 低 11）
  - 维度九（性能问题）：8（高 1 / 中 5 / 低 2）
  - 维度十（安全漏洞）：3（高 0 / 中 2 / 低 1）
  - 维度十一（并发与线程安全）：6（高 4 / 中 0 / 低 2）
  - 维度十二（API 契约与文档完整性）：4（高 2 / 中 0 / 低 2）
- 重点待完善模块：
  - backend/src/middleware/（健康度 60，安全核心模块零测试）
  - backend/src/handlers/（健康度 65，测试覆盖率约 10%）
  - backend/src/services/（健康度 70，含 async 阻塞、性能未分页、简化阉割多项高风险）
  - backend/src/services/ai/（健康度 55，核心算法零单测）
  - backend/src/services/crm/（健康度 65，RFM 分布永久阉割）
  - backend/src/services/ar/（健康度 60，账龄报表全表扫描高风险）
  - frontend/src/api/（健康度 50，测试覆盖率 4.4%）
  - frontend/src/store/（健康度 55，多数 store 无测试）
  - frontend/src/views/（健康度 60，含空实现与 XSS 潜在）
  - frontend/src/composables/（健康度 65，14 文件 any 类型逃逸）
- 落地优先级：优先完善高风险无实现、未接入核心业务代码、安全漏洞、并发问题；其次补充缺失测试覆盖、处理死代码与重复实现、性能问题；最后清理生产环境占位 Mock、规则符合性、API 契约问题

## Top 高风险问题清单（按优先级）
1. **并发-async 阻塞**（backend/src/services/auth_service.rs:107/243/277 + user_handler.rs:196/538/563/578）：Argon2id 密码哈希在 async 上下文直接调用未用 spawn_blocking 包装，登录/创建用户/修改密码阻塞 tokio worker 50-100ms
2. **性能-全表扫描**（backend/src/services/ar_service.rs:1274-1321 get_aging_report）：无日期范围 + 无 LIMIT 全表扫描，数据量增长后可能 OOM
3. **空实现-业务失效**（frontend/src/views/dye-batch/index.vue:341 + dye-recipe/index.vue:318 handleView）：查看按钮 handler 完全空，功能失效
4. **测试覆盖-安全核心**（backend/src/middleware/permission.rs 全文件）：权限校验零测试，越权风险
5. **API 文档缺失**（backend/src/openapi.rs:10-95）：仅覆盖 8/115 handlers（7%），95% 接口无 OpenAPI 文档
6. **简化阉割-永久**（backend/src/services/crm/cust.rs:265-275 get_rfm_distribution）：返回全 0 占位，RFM 分布功能形同虚设

## 调研方法说明
本报告采用深度调研方法：对每个命中点位读取完整函数上下文（前后 50 行）、追溯调用链与依赖链、对比测试覆盖、进行语义级判定。12 类维度独立扫描，结果去重合并。报告中的风险理由、业务影响、调研证据均基于实际代码分析。

## 建议 CI/CD 深度分析
本报告为纯静态文本扫描 + 深度调研结果。如需基于类型信息/调用图/控制流图的深度分析，建议在 CI/CD 中手动触发：
- .github/workflows/dead-code-audit.yml（死代码审计，每周一自动跑，也可 Actions 页面 "Run workflow" 手动触发）
- 或在 PR 中触发 .github/workflows/ci-cd.yml 获取 clippy/test/构建详细结果
