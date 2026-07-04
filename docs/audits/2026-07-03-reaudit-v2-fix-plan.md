# 全项目复审 v2 修复规划（批次 85-87）

**复审报告**：[2026-07-03-reaudit-v2.md](file:///workspace/docs/audits/2026-07-03-reaudit-v2.md)
**审计基线**：main HEAD `c40dd511`（批次 78-84 全部修复完成后）
**问题总数**：56 项（P1×9 / P2×19 / P3×28）
**规划批次**：3 批（批次 85-87）

## 修复策略

延续 v1 批次修复流程：
1. 按 P1 → P2 → P3 优先级分批，每批 CI 全绿后 squash merge 到 main
2. 复用 v1 已建立的标准修复模式（lock_exclusive 串行化 / round_dp(2) 精度校验 / startsWith 前缀匹配等）
3. P1 集中在维度 1（事务边界 TOCTOU）和维度 5（并发竞态），是本轮核心
4. P2 维度 1 的"有 txn 但无 lock_exclusive"9 项与 P1 同类问题，作为加固项合并到批次 85 后段或单独批次 86 处理
5. 全部完成后启动 v3 第三轮复审

## 扩展指令（2026-07-03 用户追加）

在批次修复过程中同步处理以下情况，并纳入复审计划：
1. **未完善功能**：评估并补全（如占位 TODO 注释、stub 返回）
2. **占位符功能**：全部真实实现，不留 stub
3. **未真实接入的功能/中间件**：进行真实接入（如定义但未挂载的中间件、未调用的 helper）
4. **未实现的功能/中间件**：补全真实实现

发现的所有扩展项汇总到本文件末尾"扩展完善清单"章节，随批次修复同步推进，并在 v3 复审中重点验证。

## 批次规划

### 批次 85：P1 修复 — 事务边界 TOCTOU + 并发竞态（9 项）

**主题**：8 处 update/approve/check+insert 事务边界 TOCTOU + 1 处 submit/self_approve 状态门+锁组合修复
**级别**：P1
**项数**：9
**修复分支**：`fix/v19-batch85-tx-boundary-p1`

| # | 文件 | 方法 | 问题 | 修复 |
|---|------|------|------|------|
| P1-1 | custom_order_crud_service.rs:168-213 | update | 状态门在 self.db，update 也在 self.db，无 txn 无 lock | begin txn + find_by_id(id).lock_exclusive().one(&txn) + 状态门 + update_with_audit(&txn) + commit |
| P1-2 | quotation_service.rs:211-363 | update | 状态门在 txn 外，line 360 update 在 txn commit 后 | 将状态门移入 txn + lock_exclusive，update 移到 commit 前 |
| P1-3 | cost_collection_service.rs:388-417 | approve | 全程无 txn 无 lock_exclusive | 用 txn 包裹 find_by_id + lock_exclusive + 状态门 + update + commit |
| P1-4 | fixed_asset_service.rs:188-242 | depreciate | 状态门在 txn 外 | 状态门移入 txn + lock_exclusive |
| P1-5 | fixed_asset_service.rs:245-304 | dispose | 状态门在 txn 外 | 状态门移入 txn + lock_exclusive |
| P1-6 | customer_service.rs:92-154 | create_customer | 检查编码存在 + insert 无 txn | begin txn + 检查存在（lock_exclusive 串行化） + insert + commit |
| P1-7 | field_permission_service.rs:117-164 | create_field_permission | 检查存在 + insert 无 txn | 同上模式 |
| P1-8 | data_permission_service.rs:124-165 | set_data_permission | find + update/insert 无 txn 无锁 | begin txn + lock_exclusive + upsert + commit |
| P1-9 | quotation_approval_service.rs:96-153 | submit + self_approve | submit 状态门在 self.db 无 lock；self_approve 有 lock 但无状态检查 | submit：状态门移入 txn + lock_exclusive；self_approve：在 lock 后加状态门检查 |

### 批次 86：P2 修复（19 项）

**主题**：9 处 update/delete 加 lock_exclusive + 金额精度校验补齐 + 全表加载 LIMIT + 前端 any 清理 + v-permission 编辑/删除按钮 + TraceLayer IP 修复
**级别**：P2
**项数**：19
**修复分支**：`fix/v19-batch86-p2-hardening`

| 维度 | # | 文件 | 修复 |
|------|---|------|------|
| 1 事务边界 | P2-1~P2-9 | role_service / ar_invoice_service / ap_invoice_service / ap_payment_request_service / customer_credit_limit / fixed_asset_service delete | 已有 txn 内 find_by_id 后追加 .lock_exclusive() 串行化 |
| 2 输入验证 | P2-10 | ar_invoice_service.rs:298-302 update | invoice_amount 加 round_dp(2) 精度校验（补 v1 批次 84 P2-4 遗漏） |
| 2 输入验证 | P2-11 | sales_fabric_order_handler.rs:175-184 | f64 → Decimal + 非负校验 + round_dp(2) |
| 6 N+1 | P2-12 | ai/rec.rs:170-171 | get_abc_classification 加 LIMIT 兜底 |
| 6 N+1 | P2-13 | ai/rec.rs:617-618 | generate_price_recommendations 加 LIMIT 兜底 |
| 9 前端类型 | P2-14 | bi.ts:160-193 | 5 处 BiResponseData<any> → 显式接口 |
| 9 前端类型 | P2-15 | ar.ts:175-187 | 4 处 ApiResponse<any> → 显式接口 |
| 9 前端类型 | P2-16 | report-enhanced/print-templates/api-gateway/product/sales-analysis | 6 处 ApiResponse<any> → 显式接口 |
| 10 路由权限 | P2-17 | 17+ .vue 文件 | 编辑/删除按钮批量补齐 v-permission |
| 11 测试质量 | P2-18 | inventory-store.test.ts | 6 处 `as any` → 显式类型断言 |
| 12 安全性 | P2-19 | main.rs:513-519 | TraceLayer IP 提取优先级对齐 audit_context（X-Real-IP → X-Forwarded-For → ConnectInfo） |

### 批次 87：P3 修复（28 项）

**主题**：低优先级清理 — 错误处理规范化 + 金额计算归一化 + LIMIT 兜底补齐 + TODO 注释 + 测试命名 + IP 提取统一
**级别**：P3
**项数**：28
**修复分支**：`fix/v19-batch87-p3-cleanup`

| 维度 | 项数 | 修复要点 |
|------|------|---------|
| 3 错误处理 | 5 | expect/unwrap 改为 ok_or_else / map_err / unwrap_or_default |
| 4 业务逻辑 | 7 | 金额计算统一 round_dp(2)，状态字符串提取常量 |
| 6 N+1 | 3 | 全表加载补 LIMIT 兜底 |
| 8 死代码 | 2 | reason 属性补 TODO 注释 |
| 9 前端 API | 2 | 索引签名 any → unknown / 显式接口 |
| 10 路由权限 | 2 | v-permission 遗漏补齐 |
| 11 测试质量 | 3 | 测试文件命名风格统一 |
| 12 安全性 | 2 | IP 提取 helper 统一复用 |

**延后项**（按 v1 经验，部分 P3 项可能延后到下一迭代，需在 PR 描述中明确标注）

### 批次 88：占位符功能实现（批次 85 评估发现，3 项）

**主题**：批次 85 P1 修复过程中发现的占位符/未完善功能实现
**级别**：功能完善（需 schema 变更）
**项数**：3
**修复分支**：`fix/v19-batch88-placeholder-impl`

| # | 文件 | 占位符 | 评估 | 实现方案 |
|---|------|--------|------|----------|
| PH-1 | custom_order_crud_service.rs:218-220 | `if let Some(v) = dto.notes { let _ = v; }` 注释"暂存到 yarn_spec 字段（无 notes 字段；如有需要扩展 schema）" | custom_order 模型无 notes 字段，DTO 有 notes 但被丢弃 | 新增 migration 添加 notes 列 + 修改 ActiveModel + update 方法接入 notes 字段 |
| PH-2 | fixed_asset_service.rs:191 depreciate | `period: &str` 参数只用于日志，未按期间记录折旧 | 无折旧期间记录表，无法跟踪每个期间的折旧 | 新增 fixed_asset_depreciation_records 表 + migration + 折旧时插入期间记录 |
| PH-3 | fixed_asset_service.rs:287 dispose | `let _disposal_gain_loss = req.disposal_value - net_book_value;` 计算后未使用 | fixed_asset_disposal 模型无 gain_loss 字段 | 新增 migration 添加 gain_loss 列 + 修改 ActiveModel + dispose 方法写入 gain_loss |

**说明**：这 3 项占位符实现都需要 schema 变更（migration），超出 P1 事务边界修复范围，安排到专门批次以降低 CI 失败风险。

## 进度跟踪

| 批次 | 主题 | 级别 | 项数 | 状态 |
|------|------|------|------|------|
| 85 | 事务边界 TOCTOU + 并发竞态 | P1 | 9 | ✅ 已完成（main `10f661d`） |
| 86 | P2 加固（lock_exclusive + 精度 + N+1 + 前端 + 安全） | P2 | 19 | ✅ 已完成（main `df8c424d`） |
| 87 | P3 清理（错误处理 + 金额 + LIMIT + 测试） | P3 | 28 | ✅ 已完成（main `cdec49e`，PR #330） |
| 88 | 占位符功能实现（schema 变更） | 功能完善 | 3 | ✅ 已完成（main `32302ca`，PR #331） |

**全部完成后**：v3 第三轮复审，循环直到无问题 ← 🔄 进行中

## v3 复审结果（2026-07-03）

**复审报告**：`docs/audits/2026-07-03-reaudit-v3.md`
**发现问题**：36 项（P0=1, P1=8, P2=12, P3=15）

| 批次 | 主题 | 级别 | 项数 | 状态 |
|------|------|------|------|------|
| 89 | P1 修复（id:Set(0) + 前端配套 + 查询 API + 死代码） | P1 | 8 | ✅ 已完成（main `ab55eeb`，PR #332） |
| 90 | P2 修复（约束冲突 + round_dp + 环境变量 + 中间件 + any 清理 + 测试） | P2 | 11 | ✅ 已完成（main `af0b224`，PR #333） |
| 90b | P2-12 联系人功能实现（需 schema 变更） | P2 | 1 | ✅ 已完成（main `5680ccb`，PR #334） |
| 91 | P0-1 api_gateway 11 端点全栈实现（占位符清理 + 表新建 + 真实接入） | P0 | 1 | ✅ 已完成（main `77b9375`，PR #335） |
| 92 | P3 修复（死代码/panic/吞错/占位user_id/折旧逻辑） | P3 | 15 | ✅ 已完成（main `e23104d`，PR #336） |
| 93 | P1 修复（id:Set(0) 推广 + delete TOCTOU） | P1 | 9 | ✅ 已完成（main `980dec0`，PR #337） |
| 94 | P2 修复（SQL注入+N+1+审计user_id+吞错+占位符） | P2 | 15 | ✅ 已完成（main `b71e7a2`，PR #338） |
| 95 | P3 修复（panic/unwrap/分页clamp/TOCTOU/CLI/前端占位）+ 5 条 CI clippy 修复 | P3 | 20 | ✅ 已完成（main `c9d03cb`，PR #339） |
| 96 | P0 修复（ArService 真实实现 + 前端 v-permission 补齐 40 处）+ 1 条 CI clippy 修复 | P0 | 17 | ✅ 已完成（main `acac30a`，PR #340） |
| 97 | P1 修复（v5 复审：并发主键/事件 user_id/金额精度 10 处/中间件真实接入）+ 2 条 CI 修复 | P1 | 16 | ✅ 已完成（main `f55e201`，PR #341） |
| 98 | P2 修复（v5 复审：分页 clamp 100+处/金额精度校验抽取/吞错改日志/前端 any 清理 113 处）+ 3 条 CI 修复 | P2 | 30+ | ✅ 已完成（main `e7fb8ee`，PR #342） |
| 99 | P3 修复（v5 复审：占位模块清理 3 处 + dead_code TODO 评估 8 文件 23 处，删除 1 处真死代码）| P3 | 4 | ✅ 已完成（main `4761359`，PR #343） |
| 100 | P3-A 修复（v5 复审：状态字符串常量化 4 文件 70 处，新增 status.rs 3 模块 14 常量）| P3 | 1 | ✅ 已完成（main `61e2da2`，PR #344） |

### 批次 99 详细修复项（v5 第五轮复审 P3 — 部分修复）

#### B 章节：占位模块删除（3 处）

| 文件 | 行数 | 占位类型 | 处理 |
|------|------|---------|------|
| services/po/purchase_return.rs | 16 | 纯注释占位（无代码，业务已由 purchase_return_service.rs 提供） | 删除文件 + po/mod.rs 删除 `pub mod purchase_return;` |
| services/ar/pay.rs | 13 | 纯注释占位（无代码，业务已由 ar_collection_service.rs 提供） | 删除文件 + ar/mod.rs 删除 `pub mod pay;` |
| services/stock_query.rs | 92 | 结构占位（StockFilter struct 未被业务引用，baseline dead_code） | 删除文件 + services/mod.rs 删除 `pub mod stock_query;` |

#### C 章节：dead_code TODO 评估（8 文件 23 处 allow）

| 文件 | allow 数 | 评估结论 |
|------|----------|---------|
| cache_service.rs | 5 | 保留（缓存核心 API：new/set_with_ttl/invalidate/default_ttl/Default，预留业务接入） |
| event_kafka.rs | 3 | 保留（Kafka 信封序列化预留，KafkaBackend 已激活但信封通道待接入） |
| performance_optimizer.rs | 6 | 保留（P4-1 性能优化模式样板，待真实 service 采纳） |
| business_metrics.rs | 4 | 保留（Prometheus 指标基础设施，待 /metrics 端点暴露） |
| operation_log_service.rs | 3 | 保留（审计日志写路径预留，读路径已由 audit_enhanced_handler 激活） |
| auth_service.rs | 3 | **删除 1 处**（validate_token 实例方法与 validate_token_static 重复实现）+ 同步删除 decoding_key 字段；保留 2 处（cleanup_revoked_users/unrevoke_user 预留策略入口） |
| ar/mod.rs | 3 | 保留（handler 已透传、service 未消费的半接线字段：UpdateReconciliationRequest/match_strategy/notes） |
| omni_audit_service.rs | 1 | 保留（已激活审计引擎的密钥注入扩展点） |

**评估汇总**：23 处 allow 中 22 处保留（预留 API/半接线字段/模式样板），1 处删除（重复实现真死代码）。

#### E/A/D/F 章节规划到批次 100

| 章节 | 项数 | 延后理由 |
|------|------|---------|
| E baseline 26 条残留 | 26 | CI baseline 机制（comm -23）不阻塞历史警告，技术债延后 |
| A 状态字符串常量化 | 53 | 需先建状态常量模块（如 models/status.rs），再分文件替换 |
| D 前端占位符 | 13 明确 + 25 隐式 | 需逐个评估业务合理性（真占位 vs 合理提示） |
| F CI 配置严格化 | 5 行（3 类）| 需配合 baseline 清理后才能移除 exit 0/--max-warnings 999999/continue-on-error |

### 批次 98 详细修复项（v5 第五轮复审 P2）

#### 子批 A：分页 clamp 系统性整改（约 100 处）

| 类别 | 文件数 | 处数 | 修复模式 |
|------|--------|------|---------|
| Handler 批量替换 | 35 | 56 | `.page.unwrap_or(1).max(1)` → `.page.unwrap_or(1).clamp(1, 1000)` |
| Service `fetch_page` u64 | 22 | 22 | `fetch_page(page.saturating_sub(1))` → `fetch_page(page.clamp(1, 1000).saturating_sub(1))` |
| Service `offset` i64 | 11 | 11 | `params.page.saturating_sub(1) * params.page_size` → `.max(1).min(1000).saturating_sub(1) * params.page_size` |
| 特殊位置 | 3 | 3 | financial_analysis_handler i64 / inventory_stock_handler_query 透传前 clamp / supplier_evaluation_service:288 paginator |
| DTO 层 | 1 | 已存在 | `PageRequest::page_clamped()` 已在批次 94 引入（main b71e7a2） |

防 DoS：page=u64::MAX 触发 offset 溢出导致 DB 全表扫描。

#### 子批 B：金额精度校验补齐（6 处）

| # | 文件 | 修复 |
|---|------|------|
| B-1 | 新增 `backend/src/utils/validator.rs` | 抽取统一 `validate_amount_range` + `validate_quantity_range`，追加 `round_dp(2)`（金额）/ `round_dp(4)`（数量）精度校验 |
| B-2 | `backend/src/utils/mod.rs` | 注册 `pub mod validator;` |
| B-3 | `handlers/ar_payment_handler.rs` | 删除本地 `validate_amount_range`，`#[validate(custom)]` 改为 `crate::utils::validator::validate_amount_range` |
| B-4 | `handlers/finance_payment_handler.rs` | 同上 |
| B-5 | `handlers/fund_management_handler.rs` | 同上 |
| B-6 | `handlers/customer_credit_handler.rs` | 同上 |

#### 子批 C：吞错修复（9 处）

| # | 文件 | 处数 | 修复 |
|---|------|------|------|
| C-1 | `services/system_update_service.rs` | 4 | `let _ = ...` 吞错改 `if let Err(e) = ... { tracing::warn!(...) }` 日志记录（rollback/权限设置/清理 .old/写日志） |
| C-2 | `services/crm/opp.rs` | 1 | 父级校验去掉冗余 `let _ =`，明确通过 `?` 传播 |
| C-3 | `services/department_service.rs` | 2 | 同上（create + update 父部门校验） |
| C-4 | `services/product_category_service.rs` | 2 | 同上（create + update 父类别校验） |

#### 子批 D：前端 any 清理（113 处）

| 类别 | 文件数 | 处数 | 修复 |
|------|--------|------|------|
| API 模块字段 any → 显式接口 | 7 | 9 | quotation TierPricingItem / report-enhanced 联合类型 / data-import 联合类型 / color-card BatchImportError / fund FundAccountStatusType / mrp MrpSupplyDetail / bpm-enhanced Record<string, unknown> |
| View `catch (error: any)` → `catch (error: unknown)` + 类型守卫 | 30 | 103 | `error instanceof Error ? error.message : String(error)`，覆盖 Login / user-profile / departments / system-update / print-templates / report-templates / bom / quality / inventory / bpm / mrp 等关键页面 |
| Login.vue catch | 1 | 1 | 单独修复 |

**总计**：112 文件，603 insertions(+)，367 deletions(-)

### 批次 97 详细修复项（v5 第五轮复审 P1）

| # | 文件 | 问题 | 修复方案 |
|---|------|------|---------|
| P1-1 | voucher_service.rs:387 | `id: Set(0)` 在并发 update 重写明细时可能触发主键约束异常 | 改为 `ActiveValue::NotSet` 让 DB 自增列生成 |
| P1-2 | event_bus.rs + ap_invoice_service.rs + ap_payment_service.rs + event_kafka_payload.rs + event_kafka.rs + test_event_bus.rs | PaymentCompleted 事件缺 user_id 字段，mark_as_paid 硬编码 `Some(0)` | 6 文件联动：枚举新增 user_id + mark_as_paid 签名扩展 + 事件发布透传 + 双向序列化 + 测试更新 |
| P1-3 | quotation_approval_service.rs:282 | `let _ = instance_id;` 占位抑制 + 后续 `if let Some(instance_id)` 变量未使用 | 改用 `is_some()` 判断（与 reject 方法一致） |
| P1-4 | purchase_return_service.rs:561,639 | 金额 5 项计算无 round_dp | 补 round_dp(2) |
| P1-5 | inventory_adjustment_service.rs:127,515,582 | amount 计算无 round_dp | 3 处补 round_dp(2) |
| P1-6 | so/order_crud.rs:220,447 | 销售订单金额 5 项计算无 round_dp | 2 处补 round_dp(2) |
| P1-7 | sales_return_service.rs:83 | total 累加无 round_dp | 补 round_dp(2) |
| P1-8 | bom_service.rs:546,551 | BOM 数量计算无 round_dp | 2 处补 round_dp(4) |
| P1-9 | material_shortage_service.rs:217-218 | qty_per_unit/total 计算无 round_dp | 2 处补 round_dp(4) |
| P1-10 | production_order_service.rs:592 | consumption_qty 计算无 round_dp | 补 round_dp(4) |
| P1-11 | mrp_engine_service.rs:325,328 | base_quantity/quantity_with_scrap 计算无 round_dp | 2 处补 round_dp(4) |
| P1-12 | inv/batch.rs:99,302,442,467,479 | quantity_kg 计算无 round_dp | 5 处补 round_dp(4) |
| P1-13 | sales_analysis_service.rs:405 | v5 复审误报，line 405 已有 round_dp_with_strategy | 无需修改 |
| P1-14 | middleware/csp.rs + main.rs:603-606 | csp_middleware 已定义但 main.rs 用 SetResponseHeaderLayer 注入，函数成为死代码 | 真实挂载 csp_middleware 到 main.rs production 路由，替代 SetResponseHeaderLayer(CONTENT_SECURITY_POLICY)；提供"仅在响应头未设置 CSP 时注入"语义，支持路由级精细化覆盖 |
| P1-15 | middleware/slow_query.rs:81-86 + inventory_stock_service.rs | SlowQueryRecorder/SlowQueryMetrics 全套定义但无业务调用链接入 + SlowQueryMetrics impl 为 no-op | inventory_stock_service::list_stock 接入 SlowQueryRecorder；SlowQueryMetrics impl 真实委托给 Metrics::record_slow_query（通过 self.metrics.auto-deref） |
| P1-16 | utils/app_state.rs:45,179,268 + middleware/api_gateway.rs | state.rate_limiter 字段构造初始化但全代码无读取引用，RateLimitStore 类型仅此一处使用 | 删除 rate_limiter 字段 + 2 处初始化 + 删除 middleware/api_gateway.rs 整个文件 + middleware/mod.rs 删除 pub mod 声明（实际限流由 MemoryRateLimiter/GLOBAL_LIMITER 提供） |

**CI 修复（2 条）**：
1. clippy: `unused variable: instance_id` — P1-3 修复删除 `let _ = instance_id;` 后变量未使用，改用 `is_some()` 判断
2. build: `error[E0599]` — SlowQueryMetrics impl 内 `self.record_slow_query(...)` 找不到方法（record_slow_query 是 Metrics 的方法不是 MetricsService），改为 `self.metrics.record_slow_query(...)` auto-deref Arc<Metrics>

## 扩展完善清单（2026-07-03 用户追加指令）

在批次修复过程中同步评估并完善占位符/未接入功能，以下为已发现项及处理状态：

### 批次 86 中发现并已处理（前端占位符）

| # | 文件 | 占位符 | 处理状态 |
|---|------|--------|---------|
| EX-1 | [sales-ext/tabs/PriceTab.vue](file:///workspace/frontend/src/views/sales-ext/tabs/PriceTab.vue) | `openPriceDialog` 仅显示 `ElMessage.info('请使用行内编辑')`，无编辑对话框 | ✅ 已补全：添加价格编辑对话框 + 表单校验 + create/update 提交逻辑 |
| EX-2 | [sales-ext/tabs/ReturnTab.vue](file:///workspace/frontend/src/views/sales-ext/tabs/ReturnTab.vue) | `openReturnDialog` 仅显示 `ElMessage.info('请使用行内编辑或参考 purchase-ext/tabs/ReturnTab.vue 实现')`，无编辑对话框 | ✅ 已补全：添加退货编辑对话框 + 明细表格 + 表单校验 + create/update 提交逻辑 |

### 批次 88 规划（后端占位符，需 schema 变更）

| # | 文件 | 占位符 | 评估 | 计划 |
|---|------|--------|------|------|
| PH-1 | [custom_order_crud_service.rs:218-220](file:///workspace/backend/src/services/custom_order_crud_service.rs) | `if let Some(v) = dto.notes { let _ = v; }` 注释"暂存到 yarn_spec 字段" | DTO 有 notes 字段但被丢弃 | 批次 88：新增 migration 添加 notes 列 |
| PH-2 | [fixed_asset_service.rs:191](file:///workspace/backend/src/services/fixed_asset_service.rs) | `period: &str` 参数只用于日志，未按期间记录折旧 | 无折旧期间记录表 | 批次 88：新增 fixed_asset_depreciation_records 表 |
| PH-3 | [fixed_asset_service.rs:287](file:///workspace/backend/src/services/fixed_asset_service.rs) | `let _disposal_gain_loss = req.disposal_value - net_book_value;` 计算后未使用 | fixed_asset_disposal 模型无 gain_loss 字段 | 批次 88：新增 migration 添加 gain_loss 列 |

### v3 复审重点验证项

1. 前端 EX-1/EX-2 占位符补全后的功能完整性（对话框交互、表单校验、API 调用）
2. 后端 PH-1/PH-2/PH-3 在批次 88 完成后的真实接入
3. 全项目扫描其他占位符/stub/未接入中间件（`TODO`、`FIXME`、`let _ =`、`stub`、`占位` 等模式）
