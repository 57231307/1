# 批次 103+ 预留 API/占位符功能实现规划

**生成时间**：2026-07-04
**触发规则**：用户追加规则"对所有预留的 api 及预留的功能/占位符功能/路由进行实现，对所有未真实接入的功能等需要真实接入"
**扫描范围**：`backend/src` 全目录
**扫描结果**：222 处 `#[allow(dead_code)]`（76 文件）+ 多处占位符/stub/未接入功能

## 优先级分类

### P0（需立即实现/接入，影响核心功能）

| # | 文件 | 问题 | 实现方案 | 批次 |
|---|------|------|---------|------|
| P0-1 | routes/search_api.rs | 3 个搜索端点返回空数据（stub） | 接入 Elasticsearch 后端（search/elastic.rs 已有 trait 定义） | 批次 104 |
| P0-2 | messaging/kafka.rs | KafkaProducer::publish 为 mock 实现 | 接入 rdkafka 真实客户端 | 批次 105 |
| P0-3 | services/password_policy_service.rs | 密码策略未接入 auth_service | 在 AuthService 中调用 validate_with_history | 批次 103 |
| P0-4 | services/purchase_return_service.rs | submit_return/reject_return 未透传 user_id（审计 Some(0)） | 从 AuthContext 透传 user_id | 批次 103 |

### P1（需接入或删除，清理死代码）

| # | 文件 | 问题 | 实现方案 | 批次 |
|---|------|------|---------|------|
| P1-1 | services/performance_optimizer.rs | 示例代码未接入业务 | 接入 inventory_stock_service 或删除 | 批次 106 |
| P1-2 | services/business_metrics.rs | 业务指标未在 metrics 端点暴露 | 在 main.rs 初始化 + /metrics 端点暴露 | 批次 106 |
| P1-3 | services/operation_log_service.rs | 已被 omni_audit_service 替代 | 评估删除或保留为独立操作日志 | 批次 106 |
| P1-4 | services/cache_service.rs | LRU 缓存未接入业务 | 接入 dashboard 热点数据或删除 | 批次 107 |
| P1-5 | services/color_card_scan_service.rs + color_card_borrow_service.rs | 色卡模块未挂载路由 | 实现路由挂载 | 批次 107 |
| P1-6 | services/ar/recon.rs | 对账模块路由未接入 | 实现 update/delete/confirm/dispute/close 路由 | 批次 108 |
| P1-7 | routes/analytics.rs api_keys() | 旧路由需删除 | 确认无外部调用后删除 | 批次 103 |
| P1-8 | routes/analytics.rs webhook 3 个 handler | retry/get_logs/test_webhook 未实现 | 实现 3 个 handler 并挂载路由 | 批次 108 |

### P2（功能扩展预留，待业务驱动）

| # | 文件 | 问题 | 实现方案 | 批次 |
|---|------|------|---------|------|
| P2-1 | utils/incoterms.rs | 销售报价单模块接入国际贸易术语 | quotation_pricing_service 接入 | 批次 109+ |
| P2-2 | utils/token_bucket.rs | 限流算法升级 | 切换为 TokenBucketLimiter | 批次 109+ |
| P2-3 | utils/admin_checker.rs | 角色缓存清理定时任务 | role_handler 修改角色后调用 clear_admin_role_cache | 批次 103 |
| P2-4 | report/ds.rs 11 个聚合方法 | 报表数据源接入 | handler 调用接入 | 批次 109+ |
| P2-5 | data_permission_service.rs | 数据权限校验未调用 | handler/校验接入 | 批次 109+ |
| P2-6 | cost_collection_service.rs 3 个 calculate 函数 | 成本采集模块未接入 | 业务接入 | 批次 109+ |
| P2-7 | assist_accounting_service.rs | 系统初始化脚本未调用 | 初始化脚本接入 | 批次 109+ |
| P2-8 | fixed_asset_service.rs calculate_monthly_depreciation | 折旧预览 API | handler 接入 | 批次 109+ |
| P2-9 | supplier_service.rs list_supplier_qualifications | 供应商资质模块 | handler 接入 | 批次 109+ |
| P2-10 | event_bus.rs EventBackend trait + BroadcastBackend | Kafka 真实集成完成后启用 | 依赖 P0-2 | 批次 110+ |
| P2-11 | cache/redis_client.rs | Redis 后端接入 | cache_service 接入 Redis | 批次 110+ |
| P2-12 | utils/failover/ | 主备隔离接入 | failover_service 接入 | 批次 110+ |
| P2-13 | websocket/notifications.rs connection_count | 监控端点暴露 | /health/ws 端点 | 批次 109+ |

### P3（已合理保留，不处理）

- models/ 目录 60+ 文件的文件级 `#![allow(dead_code)]`（SeaORM 自动生成模型例外）
- 测试用 dead_code
- fire-and-forget 场景的 `let _ =`
- 历史注释中提及的"占位符"（已修复，注释保留追溯历史）

## 批次执行计划

| 批次 | 主题 | 项数 | 优先级 |
|------|------|------|--------|
| 103 | 密码策略接入 + purchase_return user_id 透传 + admin_checker 缓存清理 + analytics.rs api_keys 删除 | 4 | P0+P1 |
| 104 | search_api.rs 接入 Elasticsearch | 3 | P0 |
| 105 | messaging/kafka.rs 接入 rdkafka | 1 | P0 |
| 106 | performance_optimizer 接入/删除 + business_metrics 端点暴露 + operation_log_service 评估 | 3 | P1 |
| 107 | cache_service 接入 + color_card 路由挂载 | 3 | P1 |
| 108 | ar/recon 路由接入 + webhook handler 实现 | 8 | P1 |
| 109+ | P2 项按业务驱动逐项接入 | - | P2 |
