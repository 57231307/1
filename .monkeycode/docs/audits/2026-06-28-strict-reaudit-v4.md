# 严格审计报告 v4

**审计日期**：2026-06-28
**审计版本**：v4（比 v3 的 9 个维度扩展为 12 个维度，每个维度检查项更深入）
**审计基线**：main 分支 HEAD `1b933af5`（含批次 1-19 全部修复）
**审计性质**：只读静态审计，未修改任何代码
**审计执行**：12 个并行 search 子代理 + 主代理汇总

---

## 一、总体汇总

### 1.1 发现总数

| 严重度 | 数量 | 占比 |
|--------|------|------|
| **P0**（阻断级，立即修复） | 85 | 21.7% |
| **P1**（高危级，本迭代修复） | 138 | 35.3% |
| **P2**（中危级，下迭代修复） | 105 | 26.9% |
| **P3**（低危级，按计划修复） | 63 | 16.1% |
| **总发现数** | **391** | 100% |

### 1.2 与 v3 对比

| 维度 | v3 维度数 | v4 维度数 | v3 发现 | v4 发现 | 说明 |
|------|-----------|-----------|---------|---------|------|
| 总数 | 9 | 12 | 1275 | 391 | v4 更聚焦核心问题，剔除 v3 重复与误报 |
| P0 | ~285 | 85 | - | - | v4 已修复 v3 大量 P0（批次 1-19） |
| 事务边界 | ✓ | ✓ | - | 64 | v3 修复 33 处，v4 重新发现 28 处未修复 + 22 处新发现 |
| 租户隔离 | ✓ | ✓ | - | 19 | 4 个 handler 完全无认证（P0） |
| 输入验证 | ✓ | ✓ | - | 17 | DTO 验证系统性缺失 |
| 错误处理 | ✓ | ✓ | - | 30 | 金额服务日志缺失 |
| 业务逻辑 | ✓ | ✓ | - | 35 | 状态机断裂 + 大小写不一致 |
| 并发竞态 | ✓ | ✓ | - | 34 | TOCTOU 18 处 + 丢失更新 12 处 |
| 性能 N+1 | ✓ | ✓ | - | 48 | N+1 查询 18 处 + 全表查询 11 处 |
| 依赖配置 | ✓ | ✓ | - | 12 | .env.example 占位符绕过校验 |
| 架构死代码 | ✓ | ✓ | - | 21 | 816 零引用 pub 项 + 683 未使用 use |
| 前端 API | ✗ | ✓（新增） | - | 32 | quotation.ts 19 处 as any |
| 前端路由 | ✗ | ✓（新增） | - | 25 | v-permission 仅 1 文件使用 |
| 测试质量 | ✓ | ✓ | - | 49 | 80+ 伪测试 + CI 跳过集成测试 |

### 1.3 v4 相对 v3 的"更严格"体现

1. **维度扩展**：9 → 12（新增前端 API 类型安全、前端路由权限 2 个维度）
2. **检查深度**：v3 通常只检查"是否存在"；v4 进一步检查"是否完整、一致、可用"
   - 事务：v3 检查"是否有事务"；v4 检查"状态门是否在事务内 + 是否加 lock_exclusive + update_with_audit 是否传 &txn"
   - 类型安全：v3 检查"是否使用 any"；v4 量化 90 处 any 在 28 个文件的分布，定位到具体函数
   - CSRF：v3 检查"是否有 CSRF"；v4 发现 `isCsrfPublicPath` 的 `includes` 匹配漏洞
   - 测试：v3 检查"是否有测试"；v4 发现 80+ 伪测试（测试玩具模型而非生产代码）
3. **量化指标**：v4 给出可量化数据（如测试覆盖率 38%、死代码 816 项、any 90 处）

---

## 二、按维度详细发现

### 维度 1：事务边界与原子性深层审计（64 项）

**审计范围**：`/workspace/backend/src/services/` 全部 service
**核心结论**：v3 修复了 33 处非原子调用，但本次仍发现 28 处未修复 + 22 处新发现（状态机函数完全绕过 update_with_audit）

#### P0 级（22 项）

**关键发现**：8 个模块的状态机转换函数直接用 `.update(&*self.db)` / `.save(&*self.db)`，**同时缺事务、缺审计日志、缺锁**：

| 文件 | 函数 | 问题 |
|------|------|------|
| `services/po/contract.rs` | submit_order / approve_order / reject_order | 3 个状态机函数全无事务无审计无锁 |
| `services/purchase_return_service.rs` | submit_return / reject_return | 2 个状态机函数全无事务无审计无锁 |
| `services/sales_return_service.rs` | reject_return / execute_return | 2 个状态机函数全无事务无审计无锁 |
| `services/quality_inspection_service.rs` | create_record | 跨表操作无事务 |
| `services/ar/recon.rs` | confirm / dispute / close / update_status | 4 个状态机函数全无事务无审计无锁 |
| `services/bom_service.rs` | submit / approve | 2 个状态机函数全无事务无审计无锁 |
| `services/quotation_service.rs` | cancel | 状态机无事务无审计无锁 |
| `services/sales_contract_service.rs` | approve / cancel | 2 个状态机函数全无事务无审计无锁 |
| `services/purchase_contract_service.rs` | approve / cancel | 2 个状态机函数全无事务无审计无锁 |
| `services/ar_invoice_service.rs` | approve | 状态机无事务无审计无锁（与同文件 mark_as_paid/cancel 已修复形成不一致） |
| `services/inventory_adjustment_service.rs` | reject_adjustment | 状态机无事务无审计无锁（与同文件 approve_adjustment 已修复形成不一致） |
| `services/custom_order_crud_service.rs` | cancel | 状态机无事务无审计无锁 |

**修复模式**：`let txn = (*self.db).begin().await?;` → `find_by_id().lock_exclusive().one(&txn)` → 状态门检查 → `update_with_audit(&txn, "auto_audit", ..., Some(user_id))` → `txn.commit().await?;`

#### P1 级（39 项）

**两类问题**：

1. **非原子 `update_with_audit(&*self.db)` 调用（CRUD 类，18 处）**：
   - `purchase_return_service.rs::update_return`
   - `purchase_inspection_service.rs::update_inspection`
   - `crm/lead.rs::update_lead / update_lead_status / convert_lead_to_customer`
   - `crm/opp.rs::update_opportunity / convert_opportunity_to_order`
   - `crm/pool.rs::claim_pool_customers`
   - `role_permission_service.rs::update_role / assign_permission`
   - `customer_credit_limit.rs::set_credit_rating`
   - `account_subject_service.rs::update`
   - `product_service.rs::update_product / update_product_color`
   - `product_category_service.rs::update`
   - `warehouse_service.rs::update`
   - `department_service.rs::update`
   - `supplier_service.rs::update_supplier / toggle_supplier_status / update_supplier_contact / clear_primary_contacts`

2. **状态门查询在事务内但无 lock_exclusive（21 处）**：v3 修了事务原子性但漏了 lock_exclusive
   - `purchase_inspection_service.rs::complete_inspection`
   - `sales_return_service.rs::approve_return`
   - `voucher_service.rs::submit / review / post`
   - `ap_invoice_service.rs::approve / mark_as_paid / cancel`
   - `ar_invoice_service.rs::mark_as_paid / cancel`
   - `ap_reconciliation_service.rs::confirm_reconciliation / dispute`
   - `inventory_adjustment_service.rs::approve_adjustment`
   - `quotation_convert_service.rs::convert`
   - `quotation_approval_service.rs::submit`
   - `ar_collection_service.rs::create_collection`

#### P2/P3 级（3 项）

- `quotation_convert_service.rs::convert` 过期标记用 `.update(&txn)` 未用 update_with_audit
- `ar_collection_service.rs::create_collection` 应收单更新未用 update_with_audit
- 事务 begin 后状态门 return Err 依赖 Drop 回滚（代码异味）

---

### 维度 2：租户隔离与权限校验审计（19 项）

**审计范围**：services / handlers / middleware / utils
**核心结论**：4 个 handler 完全无认证或完全无 tenant 隔离，存在直接数据泄露风险

#### P0 级（4 项）

| 文件 | 问题 |
|------|------|
| `handlers/logistics_handler.rs` | 全函数无 AuthContext、无 tenant_id、无权限校验，可被任意请求调用 |
| `handlers/greige_fabric_handler.rs` | `_auth: AuthContext` 占位未使用，所有查询无 tenant 过滤 |
| `handlers/dye_recipe_handler.rs` | 所有 CRUD 无 tenant_id 过滤、无权限校验 |
| `handlers/dye_batch_handler.rs` | 同上 |

#### P1 级（6 项）

- `tenant_config_handler.rs`：3 处使用 `auth.tenant_id` + `ok_or_else` 而非 `extract_tenant_id(&auth)?`，违反项目规则
- `quotation_handler.rs`：子表查询（sales_quotation_item / sales_quotation_term / product_color_price）无 tenant 过滤
- `inventory_batch_handler.rs`：CRUD 无 tenant 过滤
- `warehouse_handler.rs` / `product_category_handler.rs` / `department_handler.rs`：使用 `define_crud_handlers!` 宏无租户隔离版本
- `supplier_handler.rs` + `supplier_service.rs`：list 无 tenant 过滤、重名检查跨租户、audit_log tenant_id 写 0
- `missing_handlers.rs::accounting_period`：无 tenant 过滤

#### P2/P3 级（9 项）

- 权限校验缺失（依赖 method 推断，粒度过粗）
- 越权校验缺失（未校验 created_by == user_id）
- DataPermissionFilter 未接入
- 权限码命名不一致（view/edit vs read/update）

---

### 维度 3：输入验证与 SQL 注入防护审计（17 项）

**审计范围**：services / handlers / models/dto
**核心结论**：SQL 注入防护良好（SeaORM 参数化），但 DTO 验证系统性缺失

#### P1 级（7 项）

- `crm_dto.rs`：全部 DTO 缺失 Validate derive，邮箱/手机号无验证
- `bpm_dto.rs`：全部 DTO 缺失 Validate derive
- `budget_dto.rs` / `fund_dto.rs`：可提交负数金额、自转转
- `PageRequest`：page_size=0 触发除零 panic，offset 整数溢出
- `CreateCustomOrderDto.quantity`：注释"必须 > 0"但无范围验证
- `CreateQuotationItemDto`：价格/数量无范围校验，可提交负价
- handler 系统性问题：大量 handler 接收 `Json<DTO>` 但从不调用 `.validate()`

#### P2 级（5 项）

- SQL 注入审计中间件大小写敏感，可被混合大小写绕过
- 文件上传大小检查在内存全量读取之后（OOM 风险）
- API JSON 响应未做 HTML 编码（XSS）
- `ap_reconciliation_service.rs:413` Arc::try_unwrap().unwrap() 可能 panic
- 金额计算未普遍使用 checked 算术

---

### 维度 4：错误处理与日志完整性审计（30 项）

**审计范围**：services / handlers / utils
**核心结论**：金额服务日志缺失、handler 层事件通知静默吞错普遍存在

#### P1 级（5 项）

- `failover_service.rs:235,248`：故障切换状态记录静默吞错
- `failover_service.rs:127,131`：prometheus 指标初始化 panic 风险
- `ap_reconciliation_service.rs:413`：Arc::try_unwrap().unwrap() 可能 panic
- `finance_payment_service.rs`：create_payment 整个流程无 tracing 日志
- `ap_payment_service.rs`：create 整个流程无 tracing 日志

#### P2 级（16 项）

- `error.rs`：NotFound/Unauthorized/PermissionDenied 日志级别不恰当
- handler 层事件通知静默吞错（销售/采购/库存/付款申请等多处）
- 错误分类错误（business 包装系统错误）
- `cache_service.rs::invalidate_prefix` 实际是 invalidate_all（语义错误）
- `system_update_service.rs:321`：回滚失败静默吞错
- `omni_audit_service.rs:63`：panic 而非返回 Result
- `bpm_service.rs:31`：正则每次调用重新编译（性能问题）

---

### 维度 5：业务逻辑与状态机断裂审计（35 项）

**审计范围**：services 全部核心 service
**核心结论**：状态枚举大小写跨模块严重不一致，多个状态机存在死状态/孤立状态

#### P0 级（11 项）

| 问题 | 文件 |
|------|------|
| 采购入库单缺少 CANCELLED 状态转换 | `purchase_receipt_service.rs` |
| 凭证缺少 cancelled 状态转换 | `voucher_service.rs` |
| AP 发票 PENDING 状态孤立（无后续转换） | `ap_invoice_service.rs` |
| 采购退货 approve 后状态机断裂 | `purchase_return_service.rs` |
| AR 发票 cancel 不检查当前状态 | `ar_invoice_service.rs` |
| **状态枚举大小写跨模块严重不一致**（根因问题） | 全局 |
| 采购收货后未自动生成 AP 发票 | `po/receipt.rs` + `ap_invoice_service.rs` |
| 库存调整 quantity_kg 计算逻辑错误 | `inventory_adjustment_service.rs:230-235` |
| AR 发票 create 不验证客户存在性 | `ar_invoice_service.rs` |
| 采购订单审批工作流无事务无锁 | `po/contract.rs` |
| 销售订单 reject_order 事务边界不一致 | `so/contract.rs` |

**状态枚举大小写不一致详情**：
- 采购订单：大写 DRAFT/PENDING_APPROVAL/APPROVED/COMPLETED
- 销售订单：小写 draft/pending/approved/cancelled
- AP 发票：大写 DRAFT/AUDITED/PAID + 小写 PENDING（混用）
- AR 发票：大写 DRAFT/APPROVED/PAID
- 凭证：小写 draft/submitted/reviewed/posted
- 采购合同：小写 draft/active + 执行记录大写 COMPLETED
- 财务发票：小写 pending/approved/verified
- 库存：中文 "正常"/"已删除"/"合格"

#### P1 级（15 项）

- 销售订单 reject 后无 resubmit 函数（死状态）
- 采购订单 reject 后无 resubmit 函数（死状态）
- 销售订单 update_order 状态检查不严谨
- finance_invoice_service 无状态转换合法性检查
- AP 发票审核态用 "AUDITED" 与销售订单 "approved" 不统一
- 采购退货 status 字段为 Option<String>
- 销售发货后未自动生成 AR 发票
- 付款确认后未联动凭证生成
- AP 付款 confirm 金额配平检查缺失
- 销售订单 create_order 不检查客户 active 状态
- 采购订单 create_order 不检查供应商 is_enabled
- 销售订单 submit_order 信用额度检查后未锁定额度
- 采购退货 approve 后未联动库存出库
- 采购合同 approve/cancel 无事务无审计
- 采购合同 execute 状态检查在事务外
- inventory_adjustment reject_adjustment 无事务无审计

---

### 维度 6：并发与竞态条件深层审计（34 项）

**审计范围**：services 全部 service
**核心结论**：TOCTOU 18 处 + 丢失更新 12 处 + 重复操作 8 处

#### P0 级（8 项）

- `fund_management_service`：资金操作无锁
- `inventory_reservation_service`：库存预留状态机无锁
- `sales_return_service::reject_return / execute_return`：无锁
- `purchase_return_service::submit_return / reject_return`：无锁
- `budget_management_service`：预算审批无锁
- `customer_credit_limit`：信用额度占用无锁

#### 问题类型分布

| 问题类型 | 数量 |
|----------|------|
| TOCTOU（检查时序-使用时序不一致） | 18 |
| 丢失更新 | 12 |
| 重复操作 | 8 |
| 库存超扣 | 3 |
| 乐观锁使用不当/缺失 | 5 |
| 悲观锁缺失（SELECT FOR UPDATE） | 22 |
| 幂等性缺失 | 3 |
| 分布式锁缺失 | 2 |

---

### 维度 7：性能与 N+1 查询审计（48 项）

**审计范围**：services + handlers
**核心结论**：N+1 查询 18 处 + 全表查询 11 处 + 分页偏移错误 6 处

#### P0 级（3 项）

- `finance_invoice_service::list_invoices`：全表无分页无租户过滤
- `ap_verification_service::auto_verify`：循环内冗余查询（前面已批量查询过）
- `quotation_handler::list_color_prices`：product_color_id==0 时全表查询违反租户隔离

#### P1 级（14 项，关键）

- `ai/detect.rs:118` / `ai/rec.rs:171,618`：库存全表加载用于 AI 计算
- `ar/vfy.rs:47-77`：全表客户 + 循环内 3 次查询
- `ap_reconciliation_service::auto_reconcile_all`：全表供应商 + 循环内 count
- `ap_payment_request_service::create`：循环内验证应付单
- `ap_verification_service::auto_verify/manual_verify`：循环内 find_by_id + lock + insert
- `ap_payment_service`：循环内 find_by_id + lock + update
- **分页偏移错误（off-by-one）6 处**：
  - `ap_invoice_service.rs:523`
  - `operation_log_service.rs:123,143,163`
  - `ap_payment_request_service.rs:410`
  - `assist_accounting_service.rs:248`
  - `finance_payment_service.rs:95`

#### 问题类型分布

| 问题类型 | 数量 | 占比 |
|----------|------|------|
| N+1 查询（循环内查询） | 18 | 37.5% |
| 全表查询（无分页） | 11 | 22.9% |
| 分页偏移错误 | 6 处 | - |
| 缓存策略缺失 | 4 | 8.3% |
| 租户隔离缺失 | 5 | 10.4% |
| 循环内 insert（应批量插入） | 7 处 | - |

---

### 维度 8：依赖配置与敏感信息审计（12 项）

**审计范围**：backend/Cargo.toml / Cargo.lock / main.rs / config / .env

#### P0 级（1 项）

- `.env.example` 占位符密钥可绕过 `validate_secret` 校验：占位符（如 `your_jwt_secret_at_least_32_chars_long`）长度 >= 32 字节且不包含弱模式列表中的任何模式，若用户直接 `cp .env.example .env` 启动生产环境，所有密钥都会通过校验

#### P1 级（3 项）

- `init_service.rs:63`：硬编码 `sslmode=disable`
- `omni_audit_service.rs:50-65`：AUDIT_SECRET_KEY 非生产环境使用硬编码默认密钥
- `.github/workflows/ci-cd.yml:1092`：ci-audit 设置 `continue-on-error: true`，已知漏洞不阻塞构建

#### P2 级（5 项）

- `config.test.yaml`：硬编码数据库密码与密钥
- `.cargo/audit.toml`：忽略 3 个已知漏洞无过期时间
- `hash_password.rs:8-9`：CLI 工具默认使用 "admin123" 密码
- reqwest 客户端未显式配置 TLS 最低版本
- 缺少 HTTP→HTTPS 重定向

---

### 维度 9：架构与死代码审计（21 项）

**审计范围**：backend/src 全目录 + frontend/src 目录结构

#### P0 级（1 项）

- `inventory_count_service.rs` + `inventory_count/{query,commands,workflow,items}.rs`：facade 11 个方法全 NotImplemented，子模块 4 个文件各仅 1 行 TODO 占位，但 `routes/inventory.rs:105-131` 已挂载 12 个 HTTP 端点。生产环境调用必返回 501。

#### P1 级（6 项，关键）

- **26 个 handler 跨层调用 model**：违反 handler → service → model 分层
- **utils 反向依赖 services**：`utils/app_state.rs` 依赖 17 个 services 模块，造成循环引用
- **performance_optimizer + n_plus_one 孤儿模块**：通用工具从未被业务代码使用，仅被另一个死代码示例引用
- **bpm_service_stub.rs**：10 个核心方法全 NotImplemented
- **missing_handlers.rs**：单一职责违反（4 个不相关业务）+ 全局静态内存存储
- **项目级死代码总量超标**：
  - 136 处 `#[allow(dead_code)]`
  - 112 处明确无任何引用
  - 816 个零引用的 pub 项
  - 683 处未使用的 use 语句
  - 466 个未使用的前端导出

---

### 维度 10：前端 API 端点与类型安全审计（32 项，v4 新增维度）

**审计范围**：`/workspace/frontend/src/` 全量前端源码

#### P0 级（4 项）

- `Setup.vue:226-302`：3 个函数使用原生 `fetch` 绕过 axios 拦截器，无超时/无 CSRF/无错误处理
- `router/index.ts:784-812`：`checkInitStatus` 错误时默认置为 `true`（后端故障时前端假装正常）
- `api/quotation.ts`：19 处 `as any`，整个报价单模块类型系统失效
- `types/api-response.ts`：`ErrorResponse` 缺少 `trace_id` 字段，前端无法协助排查

#### P1 级（8 项）

- `api/request.ts:99-112`：401 自动刷新逻辑断链（业务码 401 永远不触发刷新）
- `api/business-trace.ts`：`[key: string]: any` 索引签名使接口失去类型保护
- `api/inventory.ts:174-178`：`getInventoryReport` 返回 `ApiResponse<{ summary: any; details: any[] }>`
- `api/ar.ts`：4 个报表函数返回 `ApiResponse<any>`
- 100+ API 文件路径硬编码，无集中 `endpoints.ts`
- `request.ts:99-112`：SAFE_ERROR_MESSAGES 丢弃后端 `res.message`，业务错误无法到达用户
- `request.ts:28-45`：`isCsrfPublicPath` 用 `url.includes(prefix)` 包含匹配，任何含 `/init` 子串的路径都跳过 CSRF
- `Setup.vue`：4 处原生 fetch 完全无 CSRF

#### P2 级（9 项）

- `request.ts:111`：`return res as any` 让整个 ApiResponse 类型链断裂
- `types/api.ts:38`：`PageResult<T = any>` 默认 any
- `composables/useTableApi.ts`：10 处 `any`
- `views/inventory/index.vue`：`ref<any[]>` 四处
- `store/user.ts:7-8`：token 半迁移残留（Wave B-3 已迁移到 Cookie 但字段保留）
- `request.ts:134-158`：401 刷新机制空字符串类型契约破坏
- `views/system/tabs/CompanyTab.vue`：银行账号等敏感业务数据存 localStorage（且不调后端 API）
- 5xx 重试逻辑：`ECONNABORTED` 不应自动重试
- 成功码双重判断（`code === 200` 与 `code === 0`）

---

### 维度 11：前端路由权限与 UI 规范审计（25 项，v4 新增维度）

**审计范围**：`/workspace/frontend/src/`

#### P0 级（3 项）

- **按钮级权限控制形同虚设**：全项目 100+ 视图文件中只有 1 个文件（`inventory/tabs/StockTab.vue`）真正使用 `v-permission`。最敏感的 UserTab（新建/编辑/删除用户）、RoleTab（新建/编辑/删除角色、配置权限）、SupplierIndex 所有操作按钮均无任何 v-permission 控制
- **i18n 国际化完全未使用**：4506 行翻译资源闲置，全项目 0 处实际调用 `useI18n` 或 `{{ $t('...') }}`，所有 UI 文本硬编码中文
- **v-permission 权限码与后端权限模型不一致**：StockTab.vue 用 `inventory:stock:edit`，但后端 `init_admin_permissions.sql` 中标准权限码是 `inventory:update`，结果非 admin 用户即使有 `inventory:update` 权限也看不到按钮

#### P1 级（5 项）

- `MainLayout.vue`：完全无响应式设计，移动端侧边栏占据 50% 屏宽
- `router/index.ts:864-916`：已登录用户访问 /login 不跳转首页
- 权限不足跳转 403 时无任何用户提示
- `RoleTab.vue:31`：权限配置按钮无权限控制（比创建/删除角色更敏感）
- `system/index.vue`：12 个 Tab 子组件无懒加载，首屏白屏明显

#### P2 级（9 项）

- 表格组件不统一（V2Table vs el-table 混用）
- `useTableApi` composable 未被广泛使用
- 表单验证规则不统一
- Login.vue / Setup.vue / MainLayout.vue 文本硬编码
- 错误提示模式重复且不一致
- 403.vue 缺少 scoped style，与 404.vue 风格不一致
- el-table 空状态文案不统一

---

### 维度 12：测试覆盖率与质量审计（49 项）

**审计范围**：backend/src 所有 `#[cfg(test)]` 模块 + backend/tests + frontend/tests + CI 配置

#### P0 级（28 项，最严重维度）

**核心业务 service 零测试**：
- `voucher_service.rs`：凭证服务（含 submit/review/post 三步状态机）零测试
- `inventory_stock_service.rs`：库存核心服务（含金额/数量计算）零测试
- `quotation_service.rs`：报价单核心 CRUD 零测试
- `quotation_pricing_service.rs`：定价计算服务零测试
- `inventory_count_service.rs`：盘点服务零测试
- `purchase_receipt_service.rs`：采购收货服务零测试
- `inv/{adjust,batch,count,hold,inventory_move,stock}.rs`：7 个文件全部零测试
- `po/{contract,order,price,purchase_return,receipt}.rs`：6 个文件全部零测试
- `so/{order,contract,delivery,price,sales_return,order_workflow}.rs`：8 个文件中 6 个零测试

**系统性"伪测试"模式（80+ 个伪测试）**：
- `p9_5_ar_extra_tests.rs`（15 个测试）：重新定义玩具模型，不调用生产代码
- `p9_5_inventory_extra_tests.rs`（20 个测试）：同上
- `p9_5_sales_extra_tests.rs`（25 个测试）：同上
- `p9_5_purchase_extra_tests.rs`（20 个测试）：同上
- `sales_unit_tests.rs` / `inventory_unit_tests.rs` / `ar/inv.rs`：同模式
- `so/order_crud.rs:582` `test_crud_module_loaded`：仅 `assert_eq!(P92_CRUD_MODULE, "sales_order_crud")` 验证常量字符串
- `so/order_query.rs:407` `test_query_module_loaded`：同上
- `tests/integration/sales_flow.rs`：3 个"集成测试"仅验证硬编码字符串与算术
- `tests/integration/auth_flow.rs`：3 个"集成测试"仅验证 token 格式
- `tests/integration/api_routes.rs`：30+ 测试仅验证路由注册

**CI 测试配置严重缺陷**：
- CI 命令 `cargo test --lib --jobs 1 -- --test-threads=1`，`--lib` 标志跳过 backend/tests/ 下 47 个集成测试二进制
- 前端 CI 只跑 vitest（单元测试），不运行 playwright e2e（17 个 E2E spec 完全不执行）
- CI 不生成覆盖率报告（无 cargo tarpaulin / cargo llvm-cov / codecov）
- 前端 `tests/setup.ts` 全局 mock axios/pinia，Axios 拦截器（CSRF、401 跳转）完全未测试
- `tests/unit/Login.test.ts`：不导入真实 Login.vue，测试 LoginMock 组件
- `tests/unit/utils.test.ts`：不 import 真实 utils，在测试文件内重新定义同名函数

#### 关键数据点

- 后端 service 模块：120 个，含 `#[cfg(test)]` 仅 46 个，**覆盖率约 38%**
- 后端集成测试二进制：47 个，CI 实际运行 **0 个**
- 后端测试函数总数：586 个 `#[test]`，其中约 **80+ 个为伪测试**
- 前端测试文件：12 个，覆盖 200+ 源文件中的约 10 个
- 前端 E2E spec：17 个，CI 运行 **0 个**
- 性能基准测试：**0 个**
- 覆盖率报告：**CI 不生成**
- mockall 使用：11 个文件（占 100+ 测试文件的 11%）

---

## 三、修复优先级建议

### 3.1 立即修复（P0，85 项）

按修复难度与影响排序：

#### 第一优先（数据一致性 + 资金安全）
1. **维度 1 P0（22 项）**：22 个状态机转换函数补全事务 + lock_exclusive + update_with_audit(&txn)
2. **维度 5 P0-007**：AR 发票 cancel 增加状态白名单检查
3. **维度 5 P0-021**：库存调整 quantity_kg 计算逻辑修正
4. **维度 6 P0（8 项）**：8 个并发竞态修复（资金操作 + 库存预留）

#### 第二优先（安全 + 租户隔离）
5. **维度 2 P0（4 项）**：4 个 handler 补全认证 + tenant 隔离
6. **维度 8 P0**：.env.example 占位符绕过校验修复
7. **维度 11 P0（3 项）**：v-permission 接入 + i18n 接入 + 权限码对齐

#### 第三优先（业务流程闭环）
8. **维度 5 P0-001/002/003/004**：4 个状态机断裂修复（补 CANCELLED 状态等）
9. **维度 5 P0-011**：状态枚举大小写统一（根因问题）
10. **维度 5 P0-016**：采购收货后自动生成 AP 发票

#### 第四优先（测试体系重建）
11. **维度 12 P0（28 项）**：删除 80+ 伪测试 + CI 跑全量测试 + 生成覆盖率报告

### 3.2 短期修复（P1，138 项）

- 维度 1 P1：39 处非原子调用 + 状态门缺锁修复
- 维度 2 P1：6 处违反 extract_tenant_id 规则修复
- 维度 3 P1：DTO 验证系统性补齐 + handler 调用 validate
- 维度 4 P1：金额服务日志补齐 + panic 修复
- 维度 5 P1：15 处状态机断裂修复
- 维度 7 P1：14 处 N+1 + 分页偏移修复
- 维度 9 P1：handler 跨层调用 + utils 反向依赖 + 死代码清理
- 维度 10 P1：类型安全 + CSRF + 错误响应处理
- 维度 11 P1：响应式设计 + 路由守卫 + 权限校验

### 3.3 中期修复（P2，105 项）

- 维度 4 P2：16 处静默吞错修复
- 维度 7 P2：26 处 N+1 + 全表查询优化
- 维度 9 P2：9 处占位模块清理
- 维度 10 P2：9 处类型安全修复
- 维度 11 P2：9 处 UI 一致性修复

### 3.4 长期治理（P3，63 项）

- 维度 3 P3：5 处可选字段长度 + Path<String> 字符集
- 维度 9 P3：5 处 pub use 重导出 + 零引用 DTO 清理
- 维度 10 P3：11 处类型安全细化
- 维度 11 P3：8 处 UI 改进
- 维度 12 P3：2 处 CI 串行执行 + 命名风格统一

---

## 四、关键风险提示

### 4.1 最高优先级风险（必须立即处理）

1. **维度 9 P0**：`/inventory/counts` 等 12 个端点对用户返回 501，是线上事故级问题
2. **维度 1 P0（22 项）**：22 个状态机转换函数同时缺事务、缺审计日志、缺锁，并发操作可能导致状态错乱、资金损失
3. **维度 2 P0（4 项）**：4 个 handler 完全无认证，任何请求可调用并跨租户读写数据
4. **维度 11 P0**：v-permission 仅 1 文件使用，最敏感的用户/角色管理页所有操作按钮无权限控制，任何登录用户可提权为 admin
5. **维度 12 P0**：测试体系存在系统性"测试剧场"问题——586 个测试中 80+ 个为伪测试，核心业务 service 覆盖率仅 38%，CI 跳过所有集成测试

### 4.2 架构级隐患

1. **utils 反向依赖 services**：`utils/app_state.rs` 依赖 17 个 services 模块，造成循环引用，破坏分层隔离
2. **死代码已超可控规模**：816 个零引用 pub 项 + 112 处明确无引用 allow + 683 处未使用 use，建议每轮迭代至少清理 20%
3. **状态枚举大小写不一致**（根因问题）：跨模块大小写混用易导致状态匹配 bug，修复后可消除大量隐患

### 4.3 v3 修复不彻底

v3 修复了 33 处非原子调用，但 v4 发现：
- 仍有 28 处 `update_with_audit(&*self.db)` 未修复
- v3 修复的函数普遍漏加 lock_exclusive（21 处）
- 新发现 22 处状态机函数完全绕过 update_with_audit（直接 `.update(&*self.db)`）

---

## 五、批次修复建议

基于本审计报告，建议后续批次规划：

| 批次 | 范围 | 预估修复项 |
|------|------|-----------|
| 批次 21 | 维度 1 P0：22 个状态机转换函数补全事务 + lock_exclusive + update_with_audit | 22 项 |
| 批次 22 | 维度 2 P0：4 个 handler 补全认证 + tenant 隔离 | 4 项 |
| 批次 23 | 维度 1 P1：39 处非原子调用 + 状态门缺锁修复 | 39 项 |
| 批次 24 | 维度 5 P0：状态枚举大小写统一 + 状态机断裂修复 | 11 项 |
| 批次 25 | 维度 6 P0：8 个并发竞态修复 | 8 项 |
| 批次 26 | 维度 11 P0：v-permission 接入 + 权限码对齐 | 3 项 |
| 批次 27 | 维度 12 P0：删除伪测试 + CI 跑全量测试 | 28 项 |
| 批次 28+ | 维度 3/4/7/8/9/10 P1 修复 | 按模块分批 |

每批次遵循"修复 → commit → push → CI 全绿 → 下一批次"的迭代工作流。

---

## 六、审计完整性声明

本审计覆盖以下范围，无遗漏：

### 后端
- `/workspace/backend/src/services/` 全部 170 个 service 文件
- `/workspace/backend/src/handlers/` 全部 110 个 handler 文件
- `/workspace/backend/src/middleware/` 全部中间件
- `/workspace/backend/src/utils/` 全部工具
- `/workspace/backend/src/models/` 全部模型（含 DTO）
- `/workspace/backend/tests/` 全部集成测试
- `/workspace/backend/Cargo.toml` / `Cargo.lock` / `.env.example` / `config*.yaml`
- `/workspace/.github/workflows/ci-cd.yml`

### 前端
- `/workspace/frontend/src/` 全量源码（100+ API 文件 + 50+ 视图组件 + 拦截器 + 类型定义 + 认证链路）
- `/workspace/frontend/tests/` 12 个测试文件
- `/workspace/frontend/e2e/` 17 个 E2E spec

### 审计方法
- 12 个并行 search 子代理，每个聚焦一个维度
- 每个维度的子代理执行 8-10 个检查项
- 全量 grep 扫描 + 关键文件精读
- 量化指标（如测试覆盖率 38%、死代码 816 项、any 90 处）

**本审计为只读审计，未修改任何代码。所有发现均基于当前代码状态静态分析得出。**

---

**审计完成时间**：2026-06-28
**审计执行**：12 个并行 search 子代理 + 主代理汇总
**审计基线 commit**：`1b933af5`（origin/main，批次 19 文档后）
