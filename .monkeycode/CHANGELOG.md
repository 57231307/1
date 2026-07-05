# 任务精简总结

> 重要变更一句话摘要列表。详细历史请查阅 [`.monkeycode/docs/archives/`](file:///workspace/.monkeycode/docs/archives/)。
>
> 本文件保留批次 100+ 的详细记录，批次 1-99 的详细记录已归档到 `docs/archives/2026-07-05/CHANGELOG-2026-07-05-pre-optimization.md`。

---

## 2026-07-05 (批次 122 v8 复审 P1 crm 标签真实接入完成)

### 批次 122：v8 复审 P1 修复 — CRM 标签真实接入 crm_tag 表 + 路由路径修复

**PR #366，main commit `f181e1b`，8 文件 +161 -30 行**

| 修复项 | 内容 |
|--------|------|
| 新增 crm_tag 表 | migration m0040 + SQL up/down：id/name/color/category/created_by/created_at/updated_at + idx_crm_tag_category 索引 |
| 初始化预定义标签 | VIP/重点客户/潜在客户/新客户/流失客户 5 个标签 ON CONFLICT DO NOTHING 保证向后兼容 |
| 新增 crm_tag entity | backend/src/models/crm_tag.rs + models/mod.rs 注册 |
| list_tags 真实接入 | 原返回硬编码 5 个标签，改为查 crm_tag 表真实数据，返回 Vec<crm_tag::Model> |
| create_tag 真实接入 | 原用时间戳生成假 id 不持久化，改为 INSERT 到 crm_tag 表；CreateTagDto 增加 category 字段 |
| delete_tag 真实接入 | 原直接返回 {deleted: true} 空操作，改为 DELETE FROM crm_tag，rows_affected == 0 时返回 404 |
| 路由路径修复 | /crm-tags → /crm/tags 匹配前端 crm-enhanced.ts 调用（原前端 404 bug） |

**关键决策**：
- 选择专门标签表方案（方案 B）而非聚合去重（方案 A），因为前端 CustomerTag interface 期望 5 字段（id/name/color/category/created_at）且 addTagToCustomer/removeTagFromCustomer 使用 tagId 操作
- 保留 crm_lead.tags TEXT[] 数组字段向后兼容（add_tags handler 仍覆盖式更新该数组）
- 路由路径 /crm-tags 改为 /crm/tags 解决前后端路径不一致导致的 404

---

## 2026-07-05 (批次 121 v8 复审死代码清理首项完成)

### 批次 121：v8 复审死代码清理 — 删除 KafkaEventEnvelope（report/ds+job 误删已恢复）

**PR #365，main commit `71b9bfb`，1 文件 +5 -69 行**

| 修复项 | 内容 |
|--------|------|
| v8 P1 死代码清理 | 删除 event_kafka.rs 中 KafkaEventEnvelope struct + from_event + into_event（74 行，零业务调用方，KafkaBackend.publish/subscribe 使用 EventPayload 而非信封结构） |
| 保留项 | event_type_name 供测试断言使用，标记 #[cfg(test)] 避免非测试编译时 dead_code |

**关键决策与教训**：
- KafkaEventEnvelope 是早期设计遗留的信封结构，实际 publish/subscribe 使用 EventPayload，信封结构零业务调用方
- **CI 失败教训**：首次误删 report/ds.rs + report/job.rs（v8 子代理误报为死代码），CI 报 `no method named 'execute_report' found for struct 'ReportEngineService'`
- 根因：ds.rs 包含 `impl ReportEngineService { pub async fn execute_report ... }` 跨文件 impl 块，被 report_engine_handler 等调用
- 修复：从 HEAD~1 恢复 ds.rs + job.rs + mod.rs，仅保留 KafkaEventEnvelope 删除，force push 后 CI 12 项必检全绿
- **经验**：Rust 跨文件 `impl Struct` 块需谨慎评估，文件级 `#[allow(dead_code)]` 标记的是文件内部辅助方法，不代表整个文件是死代码

---

## 2026-07-05 (批次 120 v7 复审 P2 全部修复完成 - 13/13 项)

### 批次 120：v7 复审 P2 最后 2 项修复 — 辅助核算维度真实接入 + event_bus trait 死代码删除

**PR #364，main commit `4842e97`，5 文件 +43 -481 行**

| 修复项 | 内容 |
|--------|------|
| P2-7 真实接入（核心，违反规则 0） | assist_accounting_service.rs initialize_dimensions 移除 `#[allow(dead_code)]`，main.rs 启动时调用一次初始化 8 个辅助核算维度（幂等实现，tracing::warn! 降级不阻塞启动） |
| P2-10 删除 | event_bus.rs 删除 EventBackend trait + BroadcastBackend struct + impl + BridgeStream struct + impl + EventStream/SubscribeFuture 类型别名 + EventBusState.broadcast 字段 + backend_type() 方法 + EventBackendType 枚举；删除 tests/test_event_bus.rs（依赖被删除类型） |
| clippy 修复 | 模块文档注释行首 `+ ` 被误判为 Markdown 列表项标记，改为顿号分隔 |

**关键决策**：
- P2-7 违反规则 0（真实实现强制），initialize_dimensions 在 main.rs 启动时接入（init_event_bus_with_kafka_config 之后），初始化批次/色号/缸号/等级/车间/仓库/客户/供应商 8 个维度
- P2-10 KafkaBackend 已绕过 trait 抽象走独立路径，BroadcastBackend 从未被 EVENT_BUS.publish/subscribe 调用，trait + BroadcastBackend + BridgeStream + 类型别名全部为零业务调用方的死代码
- 旧 API（EVENT_BUS.publish/subscribe/start_event_listener）保持完全兼容
- v7 复审 P2 项至此全部修复完成（13/13 项）

**v7 复审 P2 修复总结 ✅**：P2-1 ~ P2-13 全部完成

---

## 2026-07-05 (批次 119 v7 复审 P2 继续修复完成 - 8/9 项)

### 批次 119：v7 复审 P2 修复 — 3 处死代码清理（token_bucket + data_permission + assist_accounting）

**PR #363，main commit `fd4faf7`，4 文件 -274 行**

| 修复项 | 内容 |
|--------|------|
| P2-2 删除 | utils/token_bucket.rs 整个文件删除（189 行）：TokenBucket + TokenBucketLimiter（生产限流已用 MemoryRateLimiter + Redis 双轨，零业务调用方） |
| P2-5 删除 | data_permission_service.rs check_data_permission 方法 + data_scope 模块 4 个未接入常量（DEPT/DEPT_AND_BELOW/SELF/CUSTOM），仅保留 ALL；移除 PaginatorTrait 导入 |
| P2-7 删除 | assist_accounting_service.rs create_assist_record 方法（58 行，零业务调用方）；移除 Decimal 导入；initialize_dimensions 保留待批次 120 接入 |

**关键决策**：
- P2-2/P2-5/P2-7（create_assist_record 部分）均为真死代码，按 grep 验证零业务调用 → 删除 → 清理导入 → 同步测试的成熟模式
- P2-7 initialize_dimensions 暂留，批次 120 在 main.rs 启动时接入（初始化 8 个辅助核算维度，幂等实现）
- v7 复审 P2 进度：9 项已完成 8 项，剩余 1 项复合项（P2-7 initialize_dimensions + P2-10 event_bus trait）

---

## 2026-07-05 (批次 118 v7 复审 P2 部分修复完成 - 5/9 项)

### 批次 118：v7 复审 P2 修复 — 供应商资质端点真实接入 + 4 处死代码清理

**PR #362，main commit `01c4475`，7 文件 -183 行**

| 修复项 | 内容 |
|--------|------|
| P2-9 真实接入（核心） | supplier_handler.rs list/create_supplier_qualifications 真实调用 service，移除 `#[allow(dead_code)]` |
| P2-6 删除 | cost_collection_service.rs 3 个 calculate 函数 + 10 个测试（业务已 inline） |
| P2-4 删除 | report/ds.rs cleanup_expired_cache（无调用方） |
| P2-8 删除 | fixed_asset_service.rs calculate_monthly_depreciation（depreciate 已用私有方法） |
| P2-13 删除 | websocket/notifications.rs connection_count + 相关测试 |

**关键决策**：
- P2-9 违反规则 0（真实实现强制），优先级最高：handler 返回硬编码空数组/假数据，改为真实调用 service 持久化
- P2-6/P2-4/P2-8/P2-13 均为真死代码，删除决策参考批次 115/116 模式（grep 验证零业务调用 → 删除 → 清理引用 → 同步测试）
- v7 复审 P2 进度：9 项已完成 5 项（P2-1/3/4/6/8/9/11/12/13），剩余 4 项（P2-2/5/7/10）

---

## 2026-07-05 (批次 117 v7 复审 P1-5 收尾完成 - P1 全部修复完成)

### 批次 117：v7 复审 P1 修复 — 剩余 4 处生产代码 .unwrap()/.expect() 安全化

**PR #361，main commit `dd19874`**

| 修复项 | 内容 |
|--------|------|
| P1-5 收尾 | 4 处生产代码 `.unwrap()/.expect()` 安全化：webhook_signature.rs（返回 Result）+ webhook_service.rs（warn 降级）+ date_utils.rs（expect + 不变量注释）+ timeout.rs（expect + 不变量注释） |

**关键决策**：
- `sign_webhook_payload` 改为返回 `Result<String, String>`，调用方 `match` + `tracing::warn!` 降级（与 `hash.rs::hmac_sha256_hex` 一致）
- `date_utils.rs` UTC+0/0,0,0 数学不变量改为 `expect` + 注释说明（比 `unwrap` 更明确）
- `timeout.rs` fallback 中 `Response::builder` 改为 `expect` + 不变量注释（INTERNAL_SERVER_ERROR 500 永远合法）
- v7 复审 P1 项至此全部修复完成（批次 114 修 3 处中风险 + 批次 117 修 4 处低风险）

**v7 复审 P1 修复总结 ✅**：P1-1 ~ P1-10 全部完成

---

## 2026-07-05 (批次 116 v7 复审 P1-4 修复完成)

### 批次 116：v7 复审 P1 修复 — 删除未接入业务的 Redis 缓存层模块

**PR #360，main commit `5e00b04`**

| 修复项 | 内容 |
|--------|------|
| P1-4 | 删除 `backend/src/cache/` 整个目录（2 文件 504 行）：mod.rs + redis_client.rs（CacheService Redis 后端 + CacheBackend trait + RedisBackend + CacheStats + NullBackend + 5 单元测试） |
| 代码清理 | 清理 `main.rs` / `lib.rs` 移除 cache 模块声明；清理 `user_service.rs` 移除 cache 字段 + with_cache() + cache_key() + 4 处 cache 调用；清理 `product_service.rs` 移除 cache 字段 + with_cache() + cache_key() + 3 处 cache 调用 |

**关键决策**：
- 决策依据：用户规则 0「真实实现强制」+「禁止遗留占位代码」+「不使用的文件必须删除」
- `crate::cache::CacheService`（Redis 后端）的 `with_cache()` 从未被任何 handler/service 调用
- `user_service` / `product_service` 的 cache 字段永远是 None，所有 cache 操作都不会执行
- 11 处辅助 API（from_env / is_enabled / stats / snapshot / new / disabled / connect / ping 等）全部 dead_code
- 保留：`utils/cache.rs::AppCache`（csrf/token_blacklist/dashboard 真实使用）+ `services/cache_service.rs::CacheService`（moka LRU，AppState 装配）

---

## 2026-07-05 (批次 115 v7 复审 P1-3 修复完成)

### 批次 115：v7 复审 P1 修复 — 删除未接入业务的 failover 抽象模块

**PR #359，main commit `e9f3996`**

| 修复项 | 内容 |
|--------|------|
| P1-3 | 删除 `backend/src/utils/failover/` 整个目录（4 文件 1015 行）：mod.rs（FailoverCall trait + FailoverError）/ database.rs（FailoverDatabase 4 处 dead_code）/ cache.rs（FailoverCache）/ circuit_breaker.rs（CircuitBreaker） |
| 测试清理 | 删除 2 个集成测试：`tests/failover_trait_test.rs` + `tests/failover_circuit_test.rs`（测试已删除的代码） |
| 模块清理 | `backend/src/utils/mod.rs` 移除 `pub mod failover;` |

**关键决策**：
- 决策依据：用户规则 0「真实实现强制」+「禁止遗留占位代码」+「不使用的文件必须删除」
- grep 验证：FailoverDatabase / FailoverCache / FailoverCall / CircuitBreaker 全部零业务调用
- 项目已有独立的 FailoverService（services/failover_service.rs）被 failover_handler 真实调用，不依赖被删模块
- 保留：failover_service.rs / failover_handler.rs / routes/failover.rs / config/failover.rs / models/failover_*

---

## 2026-07-05 (批次 114 v7 复审 P1-6/P1-5 修复完成 + .monkeycode 文件夹整理优化)

### 批次 114：v7 复审 P1 修复 — 通知路径 warn 日志化 + 启动期 expect 安全化 + 记忆文件整理

**PR #358，main commit `36a9730`**

| 修复项 | 内容 |
|--------|------|
| P1-6 | 10 处通知路径 `let _ =` 真实错误吞没 → `if let Err(e) = ... { tracing::warn!(...); }`：auth_handler(update_last_login) / purchase_return_handler(notify_approval_result reject) / inventory_adjustment_handler(notify_approval_result reject) / ap_payment_request_handler(notify_payment_request + notify_approval_result approve+reject) / purchase_receipt_handler(notify_purchase_arrived) / purchase_order_handler(notify_purchase_order_created + notify_approval_result reject) / crm_assignment_handler(history_service.create) |
| P1-5 | 3 处启动期 expect 安全化：main.rs:shutdown_signal ctrl_c + SIGTERM expect → if let Err + tracing::error! + exit(1)；cli/migrate.rs:get_db_connection DATABASE_URL expect → unwrap_or_else + eprintln + exit(1) |
| 记忆整理 | .monkeycode 文件夹整理优化：MEMORY.md 1791→395 行（规则 0 升级 + 用户习惯章节新增）；CHANGELOG.md 2039→302 行；doto.md 113→94 行；早期内容归档到 docs/archives/2026-07-05/ |

**关键决策**：
- 通知路径 warn 日志化修复模式：`let _ = svc.method().await;` → `if let Err(e) = svc.method().await { tracing::warn!(error=%e, context_id, "描述"); }`（错误可见可排查，不影响主业务流）
- 启动期 expect 安全化修复模式：`.expect("msg")` → `unwrap_or_else(|_| { eprintln!("友好提示"); std::process::exit(1); })` 或 `match` + `tracing::error!` + `std::process::exit(1)`（避免 panic 拖垮 runtime）
- Rust 2018+ 路径解析：`tracing::warn!` 宏可通过 crate 名路径直接调用，无需显式 `use tracing;`
- 用户习惯固化：批次修复工作流 / 沟通偏好 / 记忆管理偏好 / CI 验证偏好 / 分支策略偏好 全部写入 MEMORY.md 第十二章

---

## 2026-07-05 (批次 113 v7 复审 P1-1/P1-7/P1-8 修复完成)

### 批次 113：v7 复审 P1 修复 — webhook PUT 语义 + 占位符清理 + let _ = 检查存在性丢弃

**PR #357，main commit `9d65a72`**

| 修复项 | 内容 |
|--------|------|
| P1-1 | webhook_integration_handler PUT 语义修复：新增 `UpdateWebhookIntegrationRequest` DTO + `update_integration` handler；路由 `PUT /integration/:id` 调用 `test_integration` 改为 `PUT /:id` 调用 `update_integration`；保留 `POST /test-integration/:id` 作为唯一测试入口 |
| P1-7 | 占位符 2 处：`barcode_scanner_handler.rs` 移除 `scan_type` 占位符（表无此列）；`init_handler.rs` `port_num` 改为 `_port_num`（前缀 `_` 表示校验后不参与后续逻辑） |
| P1-8 | let _ = 检查存在性丢弃 5 处统一改为直接表达式语句：`webhook_service.rs:264` / `inventory_adjustment_service.rs:479` / `inv/batch.rs:522` / `quotation_handler.rs:321` / `budget_management_service.rs:541` |

**关键决策**：
- PUT 语义修复模式：新增 UpdateXxxRequest DTO + update_xxx handler，路由从 PUT /xxx/:id → 动作触发 改为 PUT /:id → 字段更新
- let _ = 检查存在性修复模式：去掉 `let _ =` 前缀，直接表达式语句 `xxx.await?;`（错误通过 `?` 传播，成功值作为副作用被丢弃）
- 占位符修复模式：`let _ = var;` → 变量名前缀 `_`（如 `_port_num`）或直接删除并加注释说明

---

## 2026-07-05 (批次 112 v7 复审 P1-9 修复完成)

### 批次 112：v7 复审 P1-9 修复 — api_keys 表 created_by 列持久化

**PR #356，main commit `6052810`**

| 修复项 | 内容 |
|--------|------|
| migration m0039 | api_keys 新增 `created_by INTEGER` 列 + `idx_api_keys_created_by` 索引（m0039_add_created_by_to_api_keys.rs + up.sql/down.sql） |
| model | `api_key::Model` 新增 `pub created_by: Option<i32>` 字段 |
| service | `ApiKeyService::create_api_key` 新增 `created_by: i32` 参数；`regenerate_api_key` 新增 `regenerated_by: i32` 参数（语义：新密钥的创建者） |
| handler | `key_to_json` 移除 created_by 参数，从 model.created_by.unwrap_or(0) 读取（NULL 历史数据兼容为 0）；create_api_key/regenerate_api_key 透传 auth.user_id |

**关键决策**：
- migration 模式：与 m0038 (ar_reconciliations.notes) 一致，使用 `ADD COLUMN IF NOT EXISTS` 幂等语句 + 索引
- 历史数据兼容：created_by NULL 时 `unwrap_or(0)` 返回 0 保持前端显示兼容（前端原接收 0 占位）
- regenerate 语义：重新生成视为新密钥的创建者变更，更新 created_by 为操作者（而非保留原 created_by）

---

## 2026-07-05 (批次 111 v7 复审 P1 修复完成)

### 批次 111：v7 复审 P1 修复 — incoterms 接入 + audit 日期过滤 + crm 公海池 keyword/source 接入

**PR #355（+ 621cb0a 直接提交），main commit `20a8ce7`**

| 修复项 | 内容 |
|--------|------|
| P1-2 | utils/incoterms.rs 8 处 dead_code 全部接入业务：quotation_service.validate_create/update 接入 Incoterms2020::from_code 校验 + 业务元数据日志记录 + all()/code() 派生合法代码列表 |
| P1-10(audit) | audit_enhanced_handler.rs start_date/end_date 接入 list_audit_logs 日期范围过滤（支持 RFC3339 和 YYYY-MM-DD 格式）；删除 OperationLogQuery（零业务引用真死代码） |
| P1-10(crm) | crm_pool_handler / crm_customer_handler dead_code 接入：LeadQuery 新增 source/keyword 字段，list_leads 接入 source 精确匹配 + keyword 模糊搜索（4 字段 OR），PoolQueryParams.industry 保留 dead_code（表无对应列） |

**关键决策**：
- incoterms 接入方式：通过 validate_price_terms 辅助方法封装 Incoterms2020::from_code 调用，create/update 均复用
- audit 日期过滤：支持 RFC3339 日期时间和 YYYY-MM-DD 日期两种格式，end_date 日期粒度视为当天 23:59:59
- crm keyword 模糊搜索：匹配 company_name / contact_name / mobile_phone / email 四字段（OR 关系），使用 LIKE %keyword%
- industry 字段：crm_lead 表无 industry 列，保留 dead_code 标注 + TODO 注释说明原因

---

## 2026-07-05 (批次 110 v7 复审 P0 修复完成)

### 批次 110：v7 复审 P0 修复 — webhook callback PUBLIC_PATHS + message_type/title/payload 接入业务

**PR #354，main commit `20a8c11`**

| 修复项 | 内容 |
|--------|------|
| P0-1 | `/api/v1/erp/webhooks/integrations/callback` 加入 PUBLIC_PATHS（HMAC-SHA256 签名验证替代 JWT 认证），测试用例同步更新 |
| P0-2 | `SendWebhookMessageRequest.message_type` / `title` 接入业务：send_wechat_message / send_dingtalk_message 根据 message_type 构建 text/markdown 不同 payload，钉钉 markdown 使用 title 字段 |
| P0-3 | `WebhookCallbackRequest.payload` 接入业务：handle_generic_callback 将完整 payload 写入结构化日志（tracing::info! event_type + payload），返回 payload_size/payload_keys 摘要给调用方核对 |

**关键决策**：
- PUBLIC_PATHS 安全等价：HMAC-SHA256 签名验证（webhook_secret + X-Webhook-Signature 头）提供与 JWT 等价的身份认证保证
- payload 持久化方案：当前先通过 tracing::info! 输出到日志聚合系统（项目无 webhook_logs 表，新增表需要 migration），后续接入 webhook_logs 表时可作为数据源迁移
- payload 摘要返回 payload_size + payload_keys（顶层字段名最多 10 个），便于调用方核对回执是否与发送内容一致

---

## 2026-07-04 (批次 109 v7 复审修复完成)

### 批次 109：v7 复审修复 — ar_reconciliation notes 持久化 + webhook 事件不匹配 4xx + 4 处 dead_code 接入

**PR #353，main commit `21776c5`**

| 修复项 | 内容 |
|--------|------|
| P1-1 | ar_reconciliation notes 字段持久化（migration m0038 + model + service create/update/generate/auto_match 接入） |
| P1-2 | retry_webhook 事件不匹配从 200+success=false 改为 400 BusinessError（trigger_webhook + handler 透传客户端错误） |
| P3-1 | ListResultsQuery.start_date/end_date 接入 ReconciliationQuery.list 日期过滤 |
| P3-2 | UpdateConfirmationStatusRequest.remark 接入 update_status 写入 notes 字段 |
| P3-3 | CreateDisputeApiRequest.customer_id 接入 create_dispute 校验客户一致性 |
| P3-4 | resolve_dispute 的 resolution 作为 remark 写入 notes 字段 |

**关键决策**：
- trigger_webhook 区分客户端错误（4xx Err）与服务端错误（200+success=false），仅 webhook 已禁用/事件不匹配返回 4xx
- update_status 新增 remark 参数而非新增方法，避免 API 分裂；现有调用方传 None 保持兼容
- customer_id 校验为可选（若提供则校验），保持 API 向后兼容

---

## 2026-07-04 (周期性安全审计 v7 完成)

### 安全审计 v7 完成：全代码库四维度高风险攻击面审计

**审计范围**：认证与访问控制、注入向量、外部交互、敏感数据处理

**审计结论**：未发现中等或更高严重度的已确认漏洞

| 维度 | 状态 | 关键安全措施 |
|------|------|-------------|
| 认证与访问控制 | ✅ 安全 | JWT 多层防护、RBAC 权限系统、CSRF 防护、速率限制、公开路径收敛 |
| 注入向量 | ✅ 安全 | SeaORM 参数化查询、路径遍历防护、命令注入防护、XSS/CSP 防护 |
| 外部交互 | ✅ 安全 | Webhook SSRF 防护（DNS 重绑定+TOCTOU 修复）、HMAC 签名、系统更新白名单 |
| 敏感数据处理 | ✅ 安全 | Argon2id 密码哈希、密钥独立管理、日志脱敏、httpOnly Cookie、API Key 哈希存储 |

**低危观察项**（4 项，均不构成可利用漏洞）：
- LOW-1：webhook_signature.rs 中已知安全的 expect
- LOW-2：数据权限服务预留 API（dead_code 标注）
- LOW-3：内存限流器锁中毒 fail-open（可用性优先设计）
- LOW-4：WebSocket token URL 参数传递（日志脱敏已覆盖）

---

## 2026-07-04 (批次 108 ar/recon 路由接入 + webhook 真实实现完成)

### 批次 108：ar/recon 路由接入 + webhook handler 真实实现 + 7 处 dead_code 标注移除

**PR #352，main commit `e73ddd7`**

| 修复项 | 内容 |
|--------|------|
| ar/recon 路由 | 接入 update/delete/send/close 4 端点 + 删除重复 confirm/dispute |
| webhook handler | 真实实现 test/retry/logs 3 端点（test_webhook 触发 test 事件验证配置；retry_webhook 重试失败调用；get_webhook_logs 返回执行状态） |
| dead_code | 移除 7 处 dead_code 标注（已接入业务） |

---

## 2026-07-04 (批次 107 cache_service 真实接入 AppState 完成)

### 批次 107 完成：cache_service L1 本地缓存真实接入 AppState（PR #351，main `c45f7e7`）

| # | 修复要点 | 影响文件 |
|---|---------|---------|
| P1-1 | utils/app_state.rs 新增 `cache_service: Arc<CacheService>` 字段，两个构造函数（with_secrets_and_cors 和 Default）均添加初始化 | utils/app_state.rs |
| P1-1 | services/cache_service.rs 移除 5 处 dead_code 标注（new / set_with_ttl / invalidate / default_ttl / impl Default） | services/cache_service.rs |
| 配套 | color_card 路由挂载状态确认：16 端点已完整实现，路由挂载在 `/api/v1/erp/color-cards`，无需修改 | routes/color_card.rs（无变更） |

**关键决策**：
- 两个同名 CacheService 区分：`services::cache_service::CacheService`（moka L1 本地缓存）vs `cache::redis_client::CacheService`（Redis L2 分布式缓存）
- cache_service 设计为 L1 进程内缓存（moka LRU + TTL），与 state.cache（AppCache/Redis L2）形成多级缓存架构
- L1 注入 AppState 而非全局单例，便于测试和未来按模块配置不同缓存策略

---

## 2026-07-04 (批次 106 performance_optimizer/operation_log_service 删除 + business_metrics 真实接入完成)

### 批次 106 完成：3 个预留模块按"真实接入或删除"原则处理（PR #350，main `7f2cc82`）

| # | 修复要点 | 影响文件 |
|---|---------|---------|
| P1-1 | 删除 performance_optimizer.rs（154 行 P4-1 样板代码，零业务引用，load_by_ids 占位实现） | services/performance_optimizer.rs（删除） |
| P1-1 | 同步删除 n_plus_one.rs（删除 performance_optimizer 后零业务引用） | utils/n_plus_one.rs（删除） |
| P1-3 | 删除 operation_log_service.rs（399 行，零业务引用，已被 omni_audit_service 完全替代） | services/operation_log_service.rs（删除） |
| P1-2 | MetricsService 新增 business_metrics 字段 + 注册到同一 Registry + /metrics 自动暴露 erp_* 指标 | services/metrics_service.rs |
| P1-2 | 移除 BusinessMetrics 的 4 处 dead_code 标注 + 删除 render_prometheus_metrics（重复）+ build_registry_and_metrics 改为 #[cfg(test)] | services/business_metrics.rs |
| 测试 | 新增 test_business_metrics_integrated_into_metrics_service 接入验证测试 | services/metrics_service.rs |

**关键决策**：
- business_metrics 与 metrics_service.rs 互补不重复（erp_* 业务指标 vs http_*/db_* 基础设施指标），接入方式是共享 Registry 而非新增端点
- performance_optimizer 是样板代码而非"未接入功能"，正确处理是删除而非真实接入
- operation_log_service 的 TODO 触发条件已满足但接入的是替代方案（omni_audit_service），保留前提已不成立

---

## 2026-07-04 (批次 105 messaging/ 死代码模块删除完成)

### 批次 105 完成：删除 messaging/ 死代码模块（PR #349，main `bc075ad`）

| # | 修复要点 | 影响文件 |
|---|---------|---------|
| P1 | 删除 messaging/kafka.rs（444 行 trait + mock 占位模块，仅在自身测试中被引用） | messaging/kafka.rs（删除） |
| P1 | 删除 messaging/bus.rs（111 行 mock 实现，无业务调用方） | messaging/bus.rs（删除） |
| P1 | 删除 messaging/mod.rs（8 行模块声明） | messaging/mod.rs（删除） |
| 配套 | lib.rs 移除 `pub mod messaging;` 模块声明 + 新增注释说明删除原因 | backend/src/lib.rs |

**关键决策**：messaging/ 是 P9-7 设计阶段的 trait + mock 占位模块，与 services/event_kafka.rs（P11-H2 rskafka 0.5 真实集成）形成重复实现。按用户新规则和 project_rules.md 第六节"死代码处理规范"删除而非真实接入；真实 Kafka 集成路径已存在于 services/event_kafka.rs。

---

## 2026-07-04 (批次 104 搜索 API 真实接入完成)

### 批次 104 完成：search_api.rs 3 个搜索端点真实接入 SearchClient（PR #348，main `e0a8672`）

| # | 修复要点 | 影响文件 |
|---|---------|---------|
| P0-1 | 3 个 handler 从 stub 真实接入 SearchClient（注入 State<AppState>，调用 search_client.search()，反序列化为 Doc 类型） | routes/search_api.rs |
| P0-1 | AppState 新增 search_client 字段 + init_search_client() 函数（根据 ELASTICSEARCH_URL 决定 mock/real 客户端） | utils/app_state.rs |
| P0-1 | 移除已接入项的 dead_code 标注（indices / SalesOrderItemDoc / SearchResult / SearchHit / SearchClient trait / SearchError / ElasticClient / real()） | search/elastic.rs |
| 配套 | mod.rs 仅 re-export 外部实际使用的项；.env.example 新增 ELASTICSEARCH_URL 配置示例 | search/mod.rs, .env.example |
| 测试 | 新增 test_search_sales_orders_with_mock_client 端到端测试 | routes/search_api.rs |

**设计决策**：采用可降级方案，CI 环境无 ES 时使用 mock 客户端，生产环境通过环境变量切换为真实客户端。

---

## 2026-07-04 (批次 103 预留 API/占位符功能实现完成)

### 批次 103 完成：用户新规则首批修复（PR #347，main `b788b11`）

| # | 修复要点 | 影响文件 |
|---|---------|---------|
| P0-3 | user_handler.rs 接入 PasswordPolicyService（is_common_password + contains_username_fragment + strength_feedback_zh） | 2 文件 |
| P0-4 | purchase_return_service.rs 删除 2 处过时 TODO 注释 | 1 文件 |
| P2-3 | role_handler.rs update_role/delete_role 添加 clear_admin_role_cache 调用 | 2 文件 |
| P1-7 | routes/analytics.rs 删除 api_keys() 旧死路由 + 移除 unused import | 1 文件 |
| CI 修复 | 删除 api_key_handler.rs 死代码模块 + 删除 ApiKeyService::list_api_keys 死方法 + 移除 unused get_password_feedback import | 4 文件（含 1 删除） |

---

## 2026-07-04 (批次 102 v6 P3 修复完成)

### 批次 102 完成：v6 P3 修复（7 项）+ 1 条 CI 修复（PR #346，main `ed27a6c`）

**v6 第六轮复审 P3 7 项全部修复完成**：
- P3-1/P3-2/P3-3/P3-4：状态字符串常量化扩展 66 处（4 service 文件）+ 错误分类修复 2 处
  - 新增 status.rs 4 模块：ar（6 常量）/ ap_invoice（1）/ ap_payment_request（2）/ voucher（4）
  - ar_service.rs（33 处）/ ap_invoice_service.rs（14 处）/ ap_payment_request_service.rs（10 处）/ voucher_service.rs（9 处）
  - voucher_service.rs 2 处科目不存在 bad_request → not_found
- P3-5：删除 stock_ledger.rs 占位模块（MovementType 枚举未被业务引用）
- P3-6：修正 inventory_stock_query.rs:270 注释（原注释"当前为 stub 实现"不准确）
- P3-7：删除 report/exp.rs:117 冗余 `let _ = new_layer;`
- CI 修复 1 条：COLLECTION_CANCELLED 加 dead_code allow（ar_service 未实现收款单取消操作）

---

## 2026-07-04 (批次 101 v6 P2 修复完成)

### 批次 101 完成：v6 P2 修复（7 项）（PR #345，main `835b990`）

**v6 第六轮复审 P2 7 项全部修复完成**：
- v6 复审维度 1-4 验证：v5 修复无回归，新发现 7 P2 + 10 P3
- P2-1/P2-2：customer_service update_customer + delete_customer 改为事务+锁+审计（begin txn + lock_exclusive + update_with_audit + commit），新增 user_id 参数；delete_customer 增加状态门（已 inactive 拒绝重复软删除）
- P2-3/P2-4/P2-5：purchase_return_service 3 处 `Some(0)` → `Some(user_id)`（update_item/delete/update_return_totals），5 个方法签名新增 user_id 参数
- P2-6：purchase_receipt_service calculate_receipt_total_txn 的 `Some(0)` → `Some(user_id)`，3 处内部调用方补传
- P2-7：finance_invoice_service approve_invoice 添加状态门（status != "pending" 拒绝重复审批，注意 finance_invoice 状态值是小写 "pending"）
- 配套：customer_handler.rs / purchase_return_handler.rs 调用方补传 auth.user_id

---

## 2026-07-04 (批次 100 P3-A 状态字符串常量化完成)

### 批次 100 完成：P3-A 状态字符串常量化（PR #344，main `61e2da2`）

**v5 复审 P3-A 修复完成**（状态字符串常量化，4 文件 70 处）：
- 新增 `models/status.rs` 3 模块 14 常量：
  - `common`: STATUS_DRAFT/PENDING/APPROVED/CANCELLED/COMPLETED/ACTIVE（通用状态）
  - `production`: PRODUCTION_SCHEDULED/IN_PROGRESS/PENDING_APPROVAL/REJECTED（生产订单专属）
  - `payment`: PAYMENT_REGISTERED/CONFIRMED/PAID/PARTIAL_PAID（付款专属）
- 4 个 service 文件 70 处硬编码状态字符串替换为常量引用：
  - production_order_service.rs（19 处）
  - ap_payment_service.rs（8 处）
  - ar_invoice_service.rs（15 处）
  - finance_report_service.rs（11 处）
- 保留 3 个历史模块（purchase_order/sales_order/approval）的 `#[allow(dead_code)] + TODO`

---

## 2026-07-04 (批次 99 P3 部分修复完成)

### 批次 99 完成：P3 部分修复（4 项）（PR #343，main `4761359`）

**v5 复审 P3 部分修复完成**（B 占位模块 + C dead_code 评估，4 项）：
- B 章节（占位模块删除，3 处）：删除 `services/po/purchase_return.rs`（纯注释占位）+ `services/ar/pay.rs`（纯注释占位）+ `services/stock_query.rs`（结构占位，StockFilter 未被业务引用）+ 同步删除 3 处 mod 声明
- C 章节（dead_code TODO 评估，8 文件 23 处 allow）：22 处保留（预留 API/半接线字段/模式样板），1 处删除（`auth_service.rs validate_token` 实例方法与 `validate_token_static` 重复实现）+ 同步删除 `decoding_key` 字段

**关键评估结论**：
- cache_service.rs / event_kafka.rs / performance_optimizer.rs / business_metrics.rs / operation_log_service.rs / ar/mod.rs / omni_audit_service.rs 的 22 处 `#[allow(dead_code)] + TODO` 均为预留 API，保留合理
- auth_service.rs 的 `validate_token` 实例方法与 `validate_token_static` 功能等价（唯一区别是用 `self.decoding_key` 还是局部构造 `DecodingKey`），从未被外部调用，属重复实现真死代码

---

## 历史归档（批次 1-98）

批次 1-98 的详细记录已归档到：
- [`.monkeycode/docs/archives/2026-07-05/CHANGELOG-2026-07-05-pre-optimization.md`](file:///workspace/.monkeycode/docs/archives/2026-07-05/CHANGELOG-2026-07-05-pre-optimization.md)

**早期批次摘要**：

| 批次范围 | 主要内容 | 状态 |
|---------|----------|------|
| 96-98 | v5 P0/P1/P2 修复（ArService 真实实现 + 状态机 lock_exclusive + 分页 clamp + 金额精度） | ✅ |
| 85-95 | v2/v3/v4 复审 P0-P3 修复（事务边界 + spawn panic 隔离 + FOR UPDATE） | ✅ |
| 49-84 | v19 P0/P1/P2/P3 修复（早期审计修复） | ✅ |
| 1-48 | 早期修复（前端权限/路由/API 断链/安全漏洞） | ✅ |
