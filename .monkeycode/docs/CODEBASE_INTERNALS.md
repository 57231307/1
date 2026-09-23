# 代码库地基事实（CODEBASE INTERNALS）

> 本文只记录**从代码里读出来、且带 `文件:行号` 证据**的事实，用于修缺陷时判断"前端调的/读的东西后端到底有没有"。
> 与 `INTERFACES.md` 的区别：`INTERFACES.md` 是早期设想稿，其中 gRPC、`@bingxi/sdk`、Python SDK、
> `backend/proto/`、`backend/docs/postman_collection.json`、`AUTH_001/BIZ_001` 错误码体系**在仓库中均不存在**
> （`backend/proto`、`backend/docs` 目录不存在；`backend/Cargo.toml` 无 tonic/grpc 依赖；全仓无 `AUTH_001` 命中）。
> 凡本文与 `INTERFACES.md` 冲突，以本文为准。

## 1. 一个 URL 是怎么拼出来的

- 前端 baseURL：`frontend/src/api/request.ts:118` → `VITE_API_BASE_URL || '/api/v1/erp'`。
  **调用方只写 baseURL 之后的部分**，例如 `request.get('/sales/sales-returns')`。
- 后端最终 Router：`backend/src/routes/mod.rs:454 create_router(state) -> Router<()>`，由
  `bootstrap/middleware_bootstrap.rs:84` 调用，`:536` 挂 state。
- 完整链路示例：
  `api/sales-return.ts` 的 `request.get('/sales/sales-returns')`
  → `GET /api/v1/erp/sales/sales-returns`
  → `routes/mod.rs:460` `.nest("/api/v1/erp/sales", sales::routes())`
  → `routes/sales.rs:210` `sales_returns()` merge
  → `handlers/sales_return_handler.rs:40 router()` → `.route("/sales-returns", get(list_sales_returns))`
  → handler `sales_return_handler.rs:71`。
- ⚠️ 同一业务可能有**两个不同前缀的挂载点**：`/api/v1/erp/sales/sales-returns`（正规，返回真实实体）
  与 `trading.ts:154` 走的 `/api/v1/erp/trading/sales-returns`（`handlers/advanced/reorder.rs`，
  其 `customer_name` 是 `format!("客户 #{}", id)` **拼出来的假数据**，见 `reorder.rs:98`）。
  改列表字段前必须先确定改的是哪一个，否则修了一个另一个仍在造假。
- 全量端点基线：`node frontend/scripts/route-snapshot.mjs --write` 生成
  `frontend/scripts/route-snapshot.txt`（`METHOD<TAB>path<TAB>handler<TAB>file`，8 层 / 1741 端点）。
  任何挂载重构都必须让 `--check` 逐字节不变。

## 2. 后端中间件洋葱（真实注册顺序 = 从内到外）

`backend/src/bootstrap/middleware_bootstrap.rs:73 apply_full_mode_layers`，声明顺序见 `:70-72` 注释。
由外到内实际执行：

```
normalize_empty_query_params → timeout → security_headers → rate_limiting
→ dynamic_router_middleware → circuit_breaker_middleware
→ [auth_chain] auth_middleware → omni_audit → csrf_middleware → permission_middleware → request_logging_middleware
→ cors → rls_context_middleware
→ http TraceLayer → metrics_middleware
→ trace_context_middleware → audit_context_middleware → DefaultBodyLimit(12MB) → handler
```

要点（都是踩过坑的位置）：
- **RLS 必须在 auth 内侧**：`middleware_bootstrap.rs:88-99` 记录了原缺陷——RLS 注册在 auth 链外侧时
  永远读不到 `AuthContext`，`SET LOCAL app.user_id` 静默跳过，PostgreSQL RLS 从未生效。
- `normalize_empty_query_params`（`utils/query_params.rs:55`）在**最外层**把空串/纯空白的 query 值剔除，
  所以 `?status=` 到 handler 是"键不存在"→ `None`。这是"空筛选条件恒 0 行"这一类缺陷的全局根治点，
  字段级替代方案是 `empty_str_as_none` / `empty_str_vec_as_none`（`query_params.rs:85,99`，
  目前仅 `purchase_order_handler.rs:515,522` 使用）。前端对应的一半是 `request.ts:95 serializeParams`。
- 权限是**按 URL 段推导**的，不是按 handler 声明：`middleware/permission.rs:85 permission_middleware`
  → `:259 extract_resource_info`（取 `/api/v1/erp/{seg3}/{seg4}` 作 resource_type，
  经 `utils/path_utils.rs:12 is_module_prefix` / `:102 resolve_module_prefixed_resource` 白名单）
  → `:72 extract_route_info` 决定 action（`?action=` 白名单 print/export/download → 路径末段关键词
  → `:317 method_to_action`：GET=read / POST=create / PUT,PATCH=update / DELETE=delete）
  → `:524 check_permission` 查 `role_permission` 表（模型 `models/role_permission.rs:8`，
  `resource_type`/`action` 支持 `"*"`，缓存 `:354 PERMISSION_CACHE`，管理员短路 `:524`）。
  **推论：路由挂载形状直接决定鉴权键。路径写法不统一 = 权限键不统一 = 可能静默放行或静默 403。**
  这就是"路由挂载唯一约定"（`.monkeycode/MEMORY.md:98`）的安全依据。
- 认证：httpOnly Cookie 方案，`AuthContext`（`middleware/auth_context.rs:47`）由 `FromRequestParts`（`:159`）
  注入，缺失即 `AuthRejection::unauthorized`（`:170`）。只记录"可能未登录"的端点用 `OptionalAuthContext`（`:176`）。
  数据权限：`auth.to_data_scope_context() -> DataScopeContext`（`:90`），配合 `utils/data_scope.rs:177 apply_data_scope`。

## 3. 响应信封（成功 2 套 + 失败 4 套，属待收敛项）

成功：
- `utils/response.rs:11 ApiResponse<T>` = `code:Option<u16>` + `data:Option<T>` + `message:Option<String>`
  + `total:Option<u64>`，后三者 `skip_serializing_if`；`Default` 的 code 是 `500`（`:22-31`）。
  构造：`success`(:82) / `success_paginated`(:91) / `success_with_message`(:111) / `error`(:120)
  / `error_with_status`(:129)。`IntoResponse` 用 `code.unwrap_or(200)` 反推 HTTP 状态（`:145-150`）。
- **分页有两套不同类型**：
  `utils/response.rs:34 PaginatedResponse<T>{items,total,page,page_size}`（无 total_pages）
  vs `models/dto/mod.rs:49 PageResponse<T>{total,page,page_size,total_pages,data}`（键是 **`data`** 不是 `items`）。
  前端读错键就是"列表恒空但不报错"。`PageResponse` 在 `backend/src` 有 18 处引用。
  `From<PaginatedResponse> for ApiResponse<Vec<T>>`（`response.rs:65-74`）会把 total 抬到顶层，
  这是"顶层也可能有 total"的来源（`frontend/src/types/api-response.ts:14` 的 `total?` 就来自这里）。

失败（同一语义 4 种形状）：
1. `utils/error.rs:143 AppError::into_response` → `{"code":"BUSINESS_ERROR"(字符串), "message":脱敏文案, "trace_id":uuid, "timestamp":i64}`。
2. handler 手写 `ApiResponse::error/error_with_status` → `{"code":数字, "data":null, "message":文案}`，无 trace_id。
3. `response.rs:152-168 unauthorized_response/forbidden_response` → `{"code":401|403,"message","data":null}`。
4. `auth_context.rs:35-42 AuthRejection::into_response` → `{"error":"Unauthorized","message":...}`。
- `AppError` 变体→HTTP（`error.rs:168-182`）：DatabaseError/InternalError→500，ValidationError/BusinessError/
  BusinessErrorDisplayable/BadRequest→400，NotFound→404，Unauthorized→401，PermissionDenied→403，
  NotImplemented→501，TooManyRequests→429（带 `Retry-After`，`:151-160`）。
- **安全边界**：`public_message()`（`error.rs:483-497`）默认返回脱敏常量，只有 `BusinessErrorDisplayable`
  才外显真实业务文案。前端 `request.ts:54 extractBackendMessage` 因此"优先展示后端 message"是对的设计。
- 失败信封上**没有 `errors` 数组**；`errors` 只存在于导入/批量类 DTO（`utils/import_export.rs:45` 等）。

前端侧对应类型：`frontend/src/types/api-response.ts`（经 `types/api.ts` 再导出）。
两份局部重复定义待清理：`api/audit.ts:63`、`api/slow-query.ts:60` 各自又声明了 `interface ApiResponse`。

## 4. 状态词表权威表（`backend/src/models/status/`，大小写/中文混用是**故意的**）

判据永远是"写入方写什么，前端比什么"，不许把中文词表英文化。

| 组 | 取值 | 文件:行 |
|---|---|---|
| `sales_order` / `sales_delivery` / `quotation` / `custom_order` / `sales_fabric_order` | 小写 `draft/pending/approved/...` | `status/sales.rs:14,46,75,89` |
| `sales_return` | **大写** `DRAFT/SUBMITTED/APPROVED/REJECTED/COMPLETED`（无 PENDING） | `status/sales.rs:56` |
| `purchase_order` / `purchase_receipt` / `inventory_piece` / `purchase_receipt_inspection` | 大写 | `status/purchase_inventory.rs:13,...,180` |
| `inventory_transfer` | 小写 `pending/approved/rejected/shipped/completed` | `purchase_inventory.rs:63` |
| `inventory_count` | 小写 `pending/in_review/completed`（**没有 in_progress**） | `purchase_inventory.rs:82` |
| `purchase_return` / `inventory_adjustment` / `inventory_reservation` / `shortage_alert_status` | 小写 | `purchase_inventory.rs:96,...` |
| `inventory_stock_quality_status` / `_grade` / `_status` | **中文** `合格/待检/不合格`、`一等品/二等品/等外品`、`正常/报废/已删除` | `purchase_inventory.rs:206,227,241` |
| `master_data` | 小写 `active/inactive/pending/approved/draft/retired/archived/rejected` | `status/general.rs:52` |
| `common` / `payment` / `login_log` / `email_log` / `active_status` / `batch_trace_operation_type` | 大写 | `general.rs:16,37,105,127,177` |
| `work_center` | 大写 `IDLE/OVERLOADED`（只有负载项两态） | `production.rs:64` |
| `mrp` / `scheduling` / 顶层 `PRODUCTION_*` | 大写 | `production.rs:16,26,...` |
| `flow_card` / `step_record` / `process_node` | 小写（flow_card 另有 `*_UPPER` 别名） | `production.rs:72,101` |
| `ap_invoice` / `ap_payment_request` / `accounting_period` / `ap_reconciliation` / `ap_verification` | 大写 `AUDITED/APPROVING/...` | `status/finance.rs` |
| `voucher` / `finance_invoice` / `finance_payment` / `ar_collection` / `ar_reconciliation` / `bad_debt_*` | 小写 | `status/finance.rs` |
| `contract` / `budget` / `bpm_task` / `crm_lead` / `logistics_event_type` | 小写（`contract` = `draft/active/cancelled`） | `status/bpm_crm_contract.rs:34` |
| `approval` / `logistics_waybill` / `crm_opportunity` | 大写 | `bpm_crm_contract.rs` |
| `quality_dyeing.rs` 全组（含 `dye_batch_lifecycle_status` 16 态、`dye_recipe`、`lab_dip_*`、`fabric_grade`） | 小写 | `status/quality_dyeing.rs` |
| `quality_inspection_result` | **中文** `待检/合格/不合格` | `quality_dyeing.rs:331` |
| `wage_*` / `energy_*` / `color_card` / `chemical_*` / `outsourcing_*` / `business_*` | 小写 | `status/wage_energy_chemical_business.rs` |

## 5. 列表要显示名称：唯一正确的做法（JOIN 富化）

照抄 `PurchaseOrderDto`，**不要**逐行 map 后再查（N+1），也**不要** `format!("客户 #{}", id)`（造假）：
- DTO：`services/po/order.rs:19`，展示名列全部 `Option<String>`（`supplier_name/warehouse_name/department_name`）。
- 填充：`services/po/order_ops/crud.rs:463 list_orders` →
  `Entity::find().column_as(supplier::Column::SupplierName, "supplier_name")...`
  + 三条 `JoinType::LeftJoin`（`:472-484`）→ `.into_model::<PurchaseOrderDto>()` → `paginate`（`:514-519`）。
  单次查询、无 N+1；详情同构（`:529-545`）。
- 反例（别抄）：`services/inv/inventory_move.rs:79-99` 逐字段 map、`items: vec![]`，
  子表靠 `HashMap` 批查（`inv/stock.rs:16-31 stock_map`）。
- 只有 `Option` 语义的字段才允许 `Option`；NOT NULL 列在前端接口里写 `?` 等于掩盖缺键。

## 6. 公共 helper 名录（写代码前先查这里，别再手搓一份）

| 能力 | 入口 | 备注 |
|---|---|---|
| 单据号生成 | `utils/number_generator.rs:13,29,78,94 DocumentNumberGenerator::generate_no{,_with_width,_with_txn,_with_width_txn}` | 格式 `{PREFIX}{YYYYMMDD}{seq}`，`pg_advisory_xact_lock` 防并发重号（`:50-57`）；服务侧用 `impl_generate_no!` 宏 |
| 单据号查重 | `GET /document-no/check` → `handlers/document_no_handler.rs:19`，返回 `ApiResponse<bool>`（true=已占用） | 7 类 doc_type 的 `exists` 匹配在 `:39-78` |
| 分页 | `utils/pagination.rs:10 paginate_with_total` | 与 `Query<T>` 的 `page.unwrap_or(1).clamp(1,1000)`、`page_size.unwrap_or(10).clamp(1,100)` 配合（例 `warehouse_handler.rs:145`） |
| CRUD 骨架 | `utils/crud_macro.rs:4 define_service!` / `:21 impl_generate_no!` / `:47 define_crud_handlers!` / `:182 define_tuple_crud_handlers!` | 宏**不注册路由**，路由仍在 `routes/*.rs`；`Service` 必须实现 `list/get/create/update/delete`（tuple 版是 `list→(Vec,u64)`、`get_by_id→Option`）；出参 `ApiResponse<serde_json::Value>` |
| 提示文案 | `utils/messages.rs:15 biz_msg::{CREATE_OK,UPDATE_OK,DELETE_OK}`、`:27 err_msg::*` | |
| 数据范围 | `utils/data_scope.rs:177 apply_data_scope`、`:83 build_data_scope_condition`、`:149 check_resource_owner` | 64 个 handler 引用 |
| 字段脱敏 | `utils/field_mask.rs` `mask_phone/mask_email/mask_id_card/mask_bank_card`、`:149 desensitize_json`、`:97 mask_contact_fields_for_role` | `utils/pii_mask.rs` 标注"待集成"，实际未挂载，别引用 |
| LIKE 转义 | `utils/sql_escape.rs:2 escape_like_pattern`、`:20 safe_like_pattern` | |
| 角色判定 | `utils/admin_checker.rs is_admin_role/get_role_code/is_auditor_role` | 被 permission 中间件用 |
| 导出 | `utils/xlsx_export.rs build_xlsx_response/xlsx_response`、`utils/docx_export.rs build_docx_response/docx_response` | |
| SSRF | `utils/ssrf_guard.rs:33 validate_url`（webhook 用） | |

## 7. 前端约定

- **api 模块**：两种声明风格并存，都合法——
  A（多数）`export const getX = () => request.get<ApiResponse<T>>(url)`；
  B `export function listX(): Promise<ApiResponse<{items,total}>>`。
  实体接口就写在本模块里（`api/supplier.ts:4`），`types/` 只放跨模块形状。
  命名：`getXxxList / getXxxById / createXxx / updateXxx / deleteXxx / exportXxx` + 域动词。
- **查询参数键**：规范是 `page` + **`page_size`**（`api/supplier.ts:52`，`composables/useTableApi.ts:64`）。
  已知偏离（属待收敛）：`pageSize`（`api/sales-return.ts:91`、`api/purchase-return.ts:49`）、
  `limit`（`api/ai-extend.ts:198,256`、`api/supplier.ts:222`）、`from/size`（`api/search.ts:37`）。
- **页面三层结构**：`index.vue` → `composables/useX.ts`(列表/表单/提交) + `useXProc.ts`(审批等业务动作)
  + `x Fmts.ts`(纯格式化) → `components/XTable.vue` / `XFilter.vue` / `XDialog.vue`。
  简单页可只有 `index.vue + components/`（`views/supplier/`）。
  复用件：`composables/useTableApi.ts:54`（含重试与 payload 键探测）、`useTableColumns.ts:7`、`useActionPrompts.ts`。
- **权限**：`v-permission` / `v-permission-detail`（`directives/permission.ts:18,59`，无权限直接 `removeChild`），
  码表 `constants/permissions.ts:38 PERMISSIONS`（`kebab-case-复数:动作`，与后端 `init_service` 播种对齐）。
- **i18n**：只有 `locales/zh-CN.ts` 与 `en-US.ts` 两个文件，键名 `{module}.{view}.{field}`，
  状态标签 `{module}.statusLabels.<TOKEN>`。`.vue` 里用 `const { t } = useI18n({ useScope: 'global' })`；
  纯 `.ts`（utils/composables/formatters）里没有组件实例，用 `i18n.global.t`。
  `fallbackLocale: 'zh-CN'`（`i18n/index.ts:41-53`）。
  门禁 `scripts/check-i18n.mjs`：TS AST 解析两份语言包，校验 ①所有字面量 `t()/$t/msg.translate` 键在中英双侧都存在
  ②无重复键 ③每个值过 `@intlify/message-compiler` 编译；任一违规 `exit 1`（`:233`）。
- **状态标签的规范做法（唯一）**：`utils/sales-status.ts` / `utils/purchase-status.ts` 这种"词表模块"——
  常量数组 + `normalize*Status()`（未知 token **抛错**，不静默）+ `*LabelKey()` 出 i18n 键 + `*TagType()` 出 el-tag 类型。
  反例（待清理）：`views/sales-returns/composables/srFmts.ts:9,18,26,29` 手搓 map + 硬编码中文 label + `|| status` 兜底。
- **弹窗取消不是错误**：`ElMessageBox` 取消会 reject `'cancel'`/`'close'` 字符串。仓库既有写法是
  `catch (e) { if (e !== 'cancel') ... }`（108 个文件 197 处）；需要区分取消/关闭时用
  `distinguishCancelAndClose: true`（`useActionPrompts.ts:56,115`）；全局兜底在
  `utils/monitor.ts isDialogDismissal()`（已接入 `main.ts`）。
- `msg`（`utils/message.ts:43`，`message.` 命名空间 + CRUD 快捷方法）与 `logger`（`utils/logger.ts:8`，
  DEV 才输出 debug/info/warn，error 始终输出；`logAuxLoadFailure` 把 403 降级为 warn）是单例；
  `export.ts`/`print.ts`/`document-no.ts` 是无状态函数模块。金额/日期**没有**共享格式化模块（各处 `toFixed(2)`），
  XSS 出口统一走 `print.ts:320 escapeHtml`。

## 8. CI 与测试（唯一验证途径，禁止本地跑测试）

- 工作流：`.github/workflows/ci-cd.yml`（26 个 job）、`e2e-batch.yml`（仅手动）、`dead-code-audit.yml`（定时）。
- **必须通过的检查 = `ci-cleanup-retry`**（`ci-cd.yml:3652`，`if: always()`，`needs:` 列全 26 个 job
  `:3656-3682`，任一结果 ∉ {success,skipped} 即判负）。**新增 job 必须同时加进它的 `needs`**（`:22` 有明文要求）。
- Rust：`ci-test-rust`（`:1288`，PR-only，10 分片 nextest，PG16 service container，`--test-threads=1`，零容忍）。
  测试组织：绝大多数是 `backend/tests/*.rs`（约 230 个，命名 `handlers_*_test.rs`/`services_*_test.rs`/
  `<domain>_workflow_test.rs`），少量模块内联 `#[cfg(test)]`（17 处）。
  DB 夹具 `backend/tests/test_common/mod.rs:18 setup_test_db()`：`TEST_DATABASE_URL` 未设时**回退 sqlite::memory 并告警**
  （方言差距：JSONB/部分索引/DO 块/RLS），所以本地 sqlite 绿不等于 CI PG 绿；`#[ignore]` 大量用于"需真实已迁移 PG"，
  手动跑 `cargo test -- --ignored`。
- E2E：`frontend/playwright.config.ts` `workers:1 / retries:0 / fullyParallel:false / timeout 420s`，
  `globalSetup: e2e/global-setup.ts`（分片账号 `e2e_admin_sN`，`loginWithRetry` 8 次退避，写
  `e2e/.auth/storage-state.json`；角色凭据 `ensureRoleUsers()` → `role-credentials.json`），
  `baseURL: http://localhost:3000`，`testIgnore: setup-wizard/`。
  分片：36 片——flow 0-19、smoke 20-24、traversal 25-29、extras 30-35。
  共享件：`e2e/flow/helpers.ts`（`loginViaUI:1174`、`ensureTestEntities:953`、`getCtx:160` 提供
  `ctx.warehouseIds/productIds/dyeBatchId/supplierId`、`apiCall:997`（带 CSRF 与 `x-new-csrf-token` 恢复）、
  `apiCallExpectFail`、`verifyStatusTransition/verifyIllegalTransition/verifyPermissionDenied:1535-1584`、
  库存四维 `ensureStockInWarehouse/seedFourDimStockIn/verifyStockFourDim:1593-1727`、
  `trackPageHealth/assertPageHealthy:2116-2222`、`withEntity:2394`、`generateTotp:2274`）；
  `e2e/flow/ui-helpers.ts`（`safeGoto`、`pickListArray`、`create*UI` 系列、`readEntityIds`、`uiDeleteRow` 等）；
  `e2e/diagnose-fixture.ts` 是所有 flow spec 的 `test/expect` 来源。禁止 mock，真后端 + 真 PG。
- 本地允许的检查（仅此清单）：`prettier --check`、`eslint`、`vue-tsc -b --force`、`cargo fmt`/`rustfmt --check`、
  `playwright test --list`、`bash -n`、以及下面的门禁脚本。
- **门禁脚本接入状态（"缺登记的补登记"就是这条）**：
  | 脚本 | 比什么 | CI 是否跑 |
  |---|---|---|
  | `frontend/scripts/check-contract.mjs` | TS 接口字段 ↔ Rust DTO 字段，幽灵字段判负 | ✅ `ci-cd.yml:414` |
  | `scripts/check-i18n.mjs`(frontend) | i18n 键双语齐全 + 无重复 + 可编译 | ✅ `ci-cd.yml:418` |
  | `check-api-paths.mjs` | 前端调用 URL ↔ 后端真实路由 | ❌ 未接入 |
  | `check-api-envelope.mjs` | 前端信封键 ↔ handler 出参形状 | ❌ 未接入 |
  | `check-api-request.mjs` | 前端实参键集 ↔ 后端 `Json<T>/Query<T>` 字段集 | ❌ 未接入 |
  | `check-route-mount.mjs` | 路由挂载结构约定 R1/R2/R3 | ❌ 未接入 |
  | `route-snapshot.mjs --check` | 全量端点基线漂移 | ❌ 未接入 |
  根目录 `scripts/`（`api-crud-test.sh` 等）不被任何 workflow 引用，是手动脚本。

## 9. 已知假绿 / 盲区（改 CI 时优先处理）

1. `ci-test-fe`：`ci-cd.yml:1603-1605` 用 `npx vitest ... | tee` 后 `EXIT_CODE=$?`，取到的是 **tee** 的状态，
   vitest 失败被吞（无 `pipefail`）。
2. `ci-e2e` smoke 分片：`:2512-2513` 同样是 `timeout ... | tee` + `$?`。flow/traversal/extras 分片已用
   `run_shard` 回显真实码并把超时杀进程判为 124（`:2568-2581`）。
3. `ci-lint-fe`：`:1074` `jq ... // echo 0`，eslint JSON 不可解析时算 0 错误 → 绿。
4. `ci-fmt-rust`：格式不符会自动 `cargo fmt --all` 并提交 `[skip ci]` 后 `exit 0`（`:296-328`，设计上的自愈；
   推送失败才判负）。
5. `ci-lint-rust` clippy 只对**新增**告警文本判负，基线 `backend/.clippy-baseline.txt`，且该步 `continue-on-error`。
6. 类型盲区已补一半：`tsconfig.e2e.json` 覆盖 e2e，但缺 `@types/node` 使 93 处 Node API 表达式仍为 any。
7. 响应**键名**级一致性（后端 `to_value(Model)` 出参键 vs 前端接口键）目前无门禁，靠人工按实体核对——
   这正是本文第 1/3/5 节存在的原因。
