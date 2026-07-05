# 任务与历史

> 本文件记录**当前任务**与**历史任务索引**。
> 详细历史请查阅 [`.monkeycode/docs/archives/`](file:///workspace/.monkeycode/docs/archives/)。

---

## 🔄 当前任务：v9 全项目复审 P0/P1 修复中（批次 131 完成，进行批次 132+）

> 用户最高优先级规则（2026-07-04 追加）已固化到 [MEMORY.md 一、规则 0](file:///workspace/.monkeycode/MEMORY.md)。
> 本文件仅记录任务进度，规则不在此重复。

实现规划：`docs/audits/2026-07-04-batch103-placeholder-impl-plan.md`

### v9 复审结果（2 个并行子代理扫描）

- **P0 阻塞（2 项）**：
  1. `bi_analysis_service.rs` 16 个方法全部返回硬编码 mock 数据 ← **批次 130 已修复 ✅**
  2. `purchase_inspection_handler.rs` 4 个明细 CRUD 端点全部占位 ← **批次 131 已修复 ✅**
- **P1 重要（4 项）**：
  1. `production_order_handler.rs:417-429` get_production_order_logs 固定空列表 ← 批次 132（进行中）
  2. `ap_invoice_handler.rs:301-314` get_statistics "统计报表功能开发中" 占位 ← 批次 133
  3. `dashboard_service.rs:267-271` get_sales_statistics 5 字段 vec![] 占位 ← 批次 134
  4. `dashboard_service.rs:377,379-380` get_inventory_statistics 3 字段硬编码占位 ← 批次 135

### 已完成批次（最近 15 个）

| 批次 | PR | main commit | 内容 |
|------|-----|-------------|------|
| 131 | #375 | `b141c66` | v9 P0 purchase_inspection 4 个明细 CRUD 真实接入（migration m0042 + entity + service 4 方法 + 2 DTO + handler 真实调用；11 文件 +376 -26 行，CI 一次通过）|
| 130 | #374 | `2a42d3d` | v9 P0 bi_analysis_service 16 个方法真实接入数据库查询（SeaORM raw SQL + FromQueryResult + 11 中间结构体；bi_handler 16 handler 改实例方法注入 state.db；5 轮 CI 修复：所有权移动/未使用导入/语法错误/不必要类型转换/不必要闭包；2 文件 +962 -272 行）|
| 129 | #373 | `8bd404b` | v8 P2 financial_analysis_handler execute_report 真实执行：调用 calculate_indicators 真实计算财务指标 + ExecuteReportParams 可选 period 参数 + 透明响应 completed/no_data（1 文件 +65 -17 行，CI 一次通过）|
| 128 | #372 | `09601cb` | v8 P2 report_enhanced_handler 字段定义静态配置化：ReportFieldDefinition struct + available_fields_for_type 静态方法替代硬编码 serde_json::json!（2 文件 +74 -37 行，无 CI 错误一次通过）|
| 127 | #371 | `66cbe81` | v8 P2 import_export_handler 接入 import_tasks 表：list_import_tasks 真实查询 + import_csv/import_excel 创建+更新任务记录（m0041 migration + import_task model + 3 个 service 方法 + 3 处 handler 修改，8 文件 +267 -14 行；CI 修复：list_import_tasks 签名全路径 + QuerySelect trait）|
| 126 | #370 | `2674df1` | v8 P2 print_handler 静态配置化（6 种内置打印模板）+ inventory_stock_query alert_type 派生计算（discrepancy/out_of_stock/low_stock/expiring/normal，3 文件 +181 -54 行）|
| 125 | #369 | `c4a269f` | v8 P1 SearchSyncer 接入 sales_order_service + product_service：PG→ES 写入同步（含 Decimal→f64 转换 + 硬删除 ES 文档删除 + start_event_listener 签名扩展，8 文件 +225 -45 行；CI 修复：补导出 SalesOrderItemDoc）|
| 124 | #368 | `bbdf267` | v8 P1 SearchSyncer 接入 customer_service：PG→ES 写入同步（create/update/delete 事务提交后调用 sync_customer，最终一致性策略，9 处 handler 调用点更新，5 文件 +82 -20 行）|
| 123 | #367 | `a819ab4` | v8 P1 ElasticClient::real() 真实实现：ClientInner enum 双模式 + reqwest 直连 ES REST API + ensure_indices 启动时索引初始化（5 文件 +466 -75 行）|
| 122 | #366 | `f181e1b` | v8 P1 crm 标签真实接入：新增 crm_tag 表（m0040）+ list_tags/create_tag/delete_tag 真实持久化 + 路由路径 /crm-tags → /crm/tags 修复前端 404（8 文件 +161 -30 行）|
| 121 | #365 | `71b9bfb` | v8 死代码清理：删除 event_kafka KafkaEventEnvelope struct + from_event + into_event（74 行）；误删 report/ds+job 已恢复（CI 教训：跨文件 impl 块需谨慎评估）|
| 120 | #364 | `4842e97` | v7 P2-7 initialize_dimensions 真实接入 main.rs 启动 + P2-10 删除 EventBackend trait + BroadcastBackend + BridgeStream + EventBackendType + backend_type（5 文件 +43 -481 行）|
| 119 | #363 | `fd4faf7` | v7 P2-2 删除 token_bucket.rs 整个文件 + P2-5 删除 data_permission check_data_permission + 4 scope 常量 + P2-7 删除 assist_accounting create_assist_record（4 文件 -274 行）|
| 118 | #362 | `01c4475` | v7 P2-9 supplier_handler 资质端点真实接入 + P2-6 cost_collection 3 函数删除 + P2-4 report/ds cleanup_expired_cache 删除 + P2-8 fixed_asset calculate_monthly_depreciation 删除 + P2-13 websocket connection_count 删除（7 文件 -183 行）|
| 117 | #361 | `dd19874` | v7 P1-5 收尾：4 处生产代码 .unwrap()/.expect() 安全化（webhook_signature 返回 Result + date_utils/timeout expect 加不变量注释） |
| 116 | #360 | `5e00b04` | v7 P1-4 删除未接入业务的 Redis 缓存层模块（2 文件 504 行 + 清理 user/product service cache 代码 105 行） |
| 115 | #359 | `e9f3996` | v7 P1-3 删除未接入业务的 failover 抽象模块（4 文件 1015 行 + 2 集成测试） |
| 114 | #358 | `36a9730` | v7 P1-6 通知路径 warn 日志化（10 处）+ P1-5 启动期 expect 安全化（3 处中风险）+ .monkeycode 文件夹整理优化 |
| 113 | #357 | `9d65a72` | v7 P1-1 webhook PUT 语义 + P1-7 占位符 2 处 + P1-8 let _ = 检查存在性 5 处 |
| 112 | #356 | `6052810` | v7 P1-9 api_keys 表 created_by 列持久化（migration m0039） |
| 111 | #355 + 621cb0a | `20a8ce7` | v7 P1-2 incoterms 接入 quotation_service + P1-10 audit/crm keyword/source |
| 110 | #354 | `20a8c11` | v7 P0 webhook callback PUBLIC_PATHS + message_type/title + payload 接入 |

### v7 复审 P1 修复总结 ✅

P1 项全部修复完成（P1-1 ~ P1-10），详见 [MEMORY.md 二、章节](file:///workspace/.monkeycode/MEMORY.md)。

### v7 复审 P2 修复总结 ✅

P2 项全部修复完成（P2-1 ~ P2-13，13/13 项），详见 [MEMORY.md 二、章节](file:///workspace/.monkeycode/MEMORY.md)。

### v8 复审 P1 修复总结 ✅

P1 项全部修复完成（批次 121-125）：
- 批次 121：删除 event_kafka KafkaEventEnvelope（死代码）
- 批次 122：crm 标签真实接入（list_tags/create_tag/delete_tag 持久化）
- 批次 123：ElasticClient::real() 真实实现（reqwest 直连 ES REST API）
- 批次 124：SearchSyncer 接入 customer_service（PG→ES 写入同步）
- 批次 125：SearchSyncer 接入 sales_order_service + product_service（PG→ES 写入同步）

### v8 复审 P2 修复总结 ✅（5/5 完成）

- ✅ 批次 126：print_handler 静态配置化 + inventory_stock_query alert_type 派生计算
- ✅ 批次 127：import_export_handler 接入 import_tasks 表（list_import_tasks 真实查询 + import_csv/import_excel 任务记录）
- ✅ 批次 128：report_enhanced_handler 字段定义静态配置化（ReportFieldDefinition struct + available_fields_for_type 静态方法）
- ✅ 批次 129：financial_analysis_handler execute_report 真实执行（calculate_indicators 真实计算 + 透明响应 completed/no_data）

### 下一步：启动 v9 全项目复审

v8 复审 P0/P1/P2 项全部修复完成（P1 5 项 + P2 5 项 = 10 项）。启动 v9 全项目复审，循环直到复审没有问题。

### 后续批次规划

- **批次 130+**：v9 全项目复审分批修复

### 复审维度（基于历次复审经验）

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

---

## 📜 历史任务索引

详细历史：见 [CHANGELOG.md](file:///workspace/.monkeycode/CHANGELOG.md) 与 [docs/archives/](file:///workspace/.monkeycode/docs/archives/)

| 批次范围 | 主要内容 | 状态 |
|---------|----------|------|
| 96-98 | v5 P0/P1/P2 修复（ArService 真实实现 + 状态机 lock_exclusive + 分页 clamp + 金额精度） | ✅ |
| 85-95 | v2/v3/v4 复审 P0-P3 修复（事务边界 + spawn panic 隔离 + FOR UPDATE） | ✅ |
| 49-84 | v19 P0/P1/P2/P3 修复（早期审计修复） | ✅ |
| 1-48 | 早期修复（前端权限/路由/API 断链/安全漏洞） | ✅ |
