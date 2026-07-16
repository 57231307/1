# V15 打印导出审计报告（类十三·批次 11）

- **审计子代理**：V15 审计子代理（类十三打印导出审计与权限控制）
- **审计范围**：10 维度（13.1-13.10）
- **审计依据**：
  - `/workspace/.monkeycode/docs/audits/v15-review-plan-2026-07-15.md` 第 4017-4758 行
- **审计方法**：Read 审计计划 + Grep 检索（13 个后端 print/export 端点 + 25+ 前端本地导出按钮）+ Read 关键文件 + 对照审计计划逐项核对
- **审计时间**：2026-07-16
- **审计原则**：只做审计不修改业务代码

---

## 审计关键发现（总览）

本次审计覆盖 13 个后端 print/export 端点 + 25+ 前端本地导出按钮。审计发现：
1. **业务级审计接入率仅 23%（3/13）**：仅 `export_csv`、`export_excel_type`、`audit_log_handler::export_audit_logs` 接入 `OperationType::Export` 审计；其余 10 个端点完全无业务级审计，仅靠 omni 中间件记录通用 `API_CALL`。
2. **2 个敏感数据导出端点（染色配方、缸号）handler 缺 AuthContext**：无法关联调用者身份，是 V15 重大权限缺口。
3. **前端 25+ 本地导出按钮完全无审计**：审计日志、AP/AR 发票、凭证、客户列表等敏感页面通过 `exportToExcel`/`printData` 纯前端生成文件，绕过后端审计与权限校验。
4. **审计日志页面前端导出"假按钮"陷阱**：后端 `audit_log_handler::export_audit_logs`（含 admin 限制 + 审计落库）已实现，但前端 `system/audit-log/index.vue` 实际用的是本地 `exportToExcel`，完全绕过后端校验与审计。
5. **权限模型缺 print/export action**：`method_to_action` 仅按 HTTP 方法映射 CRUD，所有 print/export 请求被识别为 `read`，无差异化权限。
6. **染色配方导出无二级审批**：染色配方为企业核心技术机密，当前任何有 `read` 权限的角色都可批量导出全部配方。
7. **导出文件无水印**：xlsx_export.rs 不调用 `set_header`/`set_footer`，导出文件无任何用户/IP/时间标识，二次泄露无法追溯。
8. **打印 HTML 是占位假数据**：`PrintService::get_print_data` 返回硬编码 stub 数据（如 `"客户名称"`、`format!("SO-{:06}", id)`），未查询真实业务库。

---

## 维度 1：13.1 打印导出端点合理性审计

### 检查方法
- Grep 检索 `backend/src/routes/` 下的所有 print/export/pdf 路由注册
- Grep 检索 `frontend/src/` 下的 `exportToExcel`、`printData`、`window.print` 调用点
- Grep 检索 `OperationType::Export` 在 backend/handlers 中的接入点
- Read 关键 handler 文件以验证 handler 签名与审计接入

### 发现

#### ✅ 已落实的项

1. **后端 5 个打印端点全部保留且路由注册完整**（`backend/src/routes/sales.rs:67-68`、`routes/purchase.rs:69-79`、`routes/inventory.rs:103-104`、`routes/sales.rs:143-144`）：
   - `GET /sales/orders/:id/print` → `print_handler::sales_order_print_html`
   - `GET /sales/sales-contracts/:id/print` → `print_handler::sales_contract_print_html`
   - `GET /purchase/orders/:id/print` → `print_handler::purchase_order_print_html`
   - `GET /purchase/receipts/:id/print` → `print_handler::purchase_receipt_print_html`
   - `GET /inventory/transfers/:id/print` → `print_handler::inventory_transfer_print_html`

2. **后端 13 个导出端点全部保留且路由注册完整**（`backend/src/routes/*.rs`）：
   - `GET /sales/orders/export` → `sales_order_handler::export_orders`
   - `GET /purchase/orders/export` → `purchase_order_handler::export_orders`
   - `GET /production/dye-batches/export` → `dye_batch_handler::export_dye_batches`
   - `GET /production/dye-recipes/export` → `dye_recipe_handler::export_dye_recipes`
   - `GET /production/mrp-history/:id/export` → `mrp_handler::export_calculation`
   - `GET /color-cards/export/:id` → `color_card::scan_export::export_color_card`
   - `GET /ar-reconciliations-enhanced/:id/pdf` → `ar_reconciliation_enhanced_handler::export_reconciliation_pdf`
   - `GET /export/csv/:export_type` → `import_export_handler::export_csv`（已审计）
   - `GET /export/excel/:export_type` → `import_export_handler::export_excel_type`（已审计）
   - `GET /audit-logs/export` → `audit_log_handler::export_audit_logs`（已审计）
   - `GET /audit/logs/export` → `audit_enhanced_handler::export_audit_logs`
   - `GET /login-logs/export` → `login_security_handler::export_login_logs`
   - `GET /products/export` → `product_handler::export_products`
   - `GET /crm/leads/export` → `crm_handler::export_leads`
   - `GET /crm/opportunities/export` → `crm_handler::export_opportunities`
   - `POST /reports-enhanced/export/pdf` → `report_enhanced_handler::export_pdf`
   - `POST /reports-enhanced/export/excel` → `report_enhanced_handler::export_excel`

3. **规则 3 .xlsx 格式合规**：所有导出 handler 通过 `build_xlsx_response`（`backend/src/utils/xlsx_export.rs:120`）或 `generate_xlsx`（`backend/src/handlers/import_export_handler.rs:225、249、306`）统一返回 `application/vnd.openxmlformats-officedocument.spreadsheetml.sheet`。

4. **审计日志导出 admin 限制已落实**：`backend/src/handlers/audit_log_handler.rs:257` 调用 `require_admin_role(&state, &auth).await?`，非 admin 角色返回 403。

5. **前端导出工具 `escapeHtml` 已落实 XSS 防护**：`frontend/src/utils/print.ts:249-270` 对用户可控字符串做 5 个特殊字符转义。

#### ❌ 缺陷项

##### 缺陷 1-1：打印 HTML 返回硬编码占位数据（P0）
**风险等级：P0**
**证据**：
- `backend/src/services/print_service.rs:57-142` 各 `get_*_print_data` 方法返回硬编码 stub：
```rust
async fn get_sales_order_print_data(&self, id: i32) -> Result<PrintData, AppError> {
    let mut data = HashMap::new();
    data.insert("order_no".to_string(), serde_json::json!(format!("SO-{:06}", id)));
    data.insert("customer_name".to_string(), serde_json::json!("客户名称"));  // ← 占位假数据
    Ok(PrintData { template: "sales_order".to_string(), data, items: Vec::new() })
}
```
- `print_handler.rs:12-23` 调用 `render_print_html` → `PrintService::get_print_data` → `generate_pdf`，全程不查 DB
- `print_handler.rs:19-23` 5 个 handler 函数签名无 `AuthContext`、无审计调用：
```rust
pub async fn sales_order_print_html(
    Path(doc_id): Path<i32>,
    State(_): State<AppState>,
) -> Result<Html<String>, AppError> {
    render_print_html("sales_order", doc_id).await
}
```

**业务影响**：
1. 用户实际点击"打印销售订单"按钮，得到的是 `客户名称: "客户名称"` 假数据，无法用作客户签约/发货单据
2. 5 个打印 handler 全部缺 `AuthContext`，omni 中间件只能记录匿名 `API_CALL`，无法关联调用者
3. 任何登录用户都可调用，无任何角色权限校验

**修复建议**：
1. `PrintService::get_*_print_data` 接入真实 DB 查询（按 `id` 查 `sales_order`、`purchase_order` 等表）
2. 5 个 print handler 补 `auth: AuthContext` 参数
3. 5 个 print handler 补 `OperationType::Print` 审计落库（需先扩展枚举）
4. 补 `sales.order.print`/`purchase.order.print` 等 print 专属权限码校验

---

##### 缺陷 1-2：染色配方导出无任何权限/审计/审批控制（P0）
**风险等级：P0**
**证据**：
- `backend/src/handlers/dye_recipe_handler.rs:199-267` `export_dye_recipes` 函数签名缺 `AuthContext`：
```rust
pub async fn export_dye_recipes(
    State(state): State<AppState>,
    Query(query): Query<DyeRecipeListQuery>,  // ← 无 AuthContext
) -> Result<axum::response::Response, AppError> {
    let mut q = dye_recipe::Entity::find().filter(dye_recipe::Column::IsDeleted.eq(false));
    // ...
    let recipes = q.all(&*state.db).await?;  // ← 全量查询无 limit
    // ...直接 build_xlsx_response 返回
}
```
- `backend/src/routes/production.rs:124-125` 路由无任何 middleware 限制
- `backend/database/init_admin_permissions.sql:1-66` 权限码表无 `dye_recipe.export` 权限码
- 无 `export_approval_request` 表/migration（`Grep "export_approval_request"` 全仓 0 命中）
- 无 `approval_token` 字段（`Grep "approval_token"` 全仓 0 命中）

**业务影响**：
- 染色配方是印染企业核心技术机密，泄露直接丧失竞争力（V15 审计计划明确标 🔴 禁止）
- 任何拥有 `read` 权限的销售员/采购员/仓库员都可调用 `/production/dye-recipes/export` 一次性下载全部配方
- 缺 AuthContext → omni 审计日志记录匿名 API_CALL，无法追溯
- 全量 `q.all()` 无 limit → 大数据量可能 OOM 或拖垮 DB

**修复建议**：
1. handler 补 `auth: AuthContext` 参数
2. 默认禁止所有角色导出染色配方
3. 新增 `dye_recipe_master` 角色和 `dye.recipe.export` 权限码
4. 引入 `export_approval_request` 表 + 二级审批 token 流程
5. 限制单次导出条数（强制单条 + 二级审批）
6. 导出文件加水印（用户名+IP+时间）

---

##### 缺陷 1-3：缸号导出缺 AuthContext/审计/审批（P0）
**风险等级：P0**
**证据**：
- `backend/src/handlers/dye_batch_handler.rs:356-408` 函数签名缺 `AuthContext`：
```rust
pub async fn export_dye_batches(
    State(state): State<AppState>,
    Query(query): Query<DyeBatchListQuery>,  // ← 无 AuthContext
) -> Result<axum::response::Response, AppError> {
    let mut q = dye_batch::Entity::find().filter(dye_batch::Column::IsDeleted.eq(false));
    // ...
    let batches = q.all(&*state.db).await?;  // ← 全量查询无 limit
    // ...
}
```
- 无审计调用、无权限校验、无二级审批

**业务影响**：缸号含生产计划 + 配方关联，是核心技术机密；当前任何 read 权限角色可批量下载

**修复建议**：
1. handler 补 `auth: AuthContext` 参数
2. 仅 `production_manager` + 二级审批可导出
3. 补 `OperationType::Export` 审计
4. 限制导出条数 ≤ 5000

---

##### 缺陷 1-4：色卡导出无审计/审批（P1）
**风险等级：P1**
**证据**：
- `backend/src/handlers/color_card/scan_export.rs:49-113` 函数有 `_auth: AuthContext` 但未调用审计服务，未做二级审批：
```rust
pub async fn export_color_card(
    _auth: AuthContext,  // ← 有但未使用
    State(state): State<AppState>,
    Path(id): Path<i64>,
) -> Result<axum::response::Response, AppError> {
    // ...直接 build_xlsx_response 返回
    // 无 AuditLogService::record_async 调用
    // 无 approval_token 校验
}
```
- 色卡含 RGB/CMYK/LAB/Pantone/CNCS 色值 + 客户化色号映射（行 64-81），是企业色彩资产

**业务影响**：色卡数据泄露导致企业色彩资产被竞品复制

**修复建议**：
1. 补 `OperationType::Export` 审计
2. 默认禁止，仅 `dye_recipe_master` + 二级审批，或 customer 角色仅本人色卡

---

##### 缺陷 1-5：5 个 print_html handler 无审计接入（P1）
**风险等级：P1**
**证据**：
- `backend/src/handlers/print_handler.rs:19-52` 5 个 handler 全部仅 `Path` + `State`，无 `AuthContext`、无 `AuditLogService::record_async` 调用
- `backend/src/services/print_service.rs:36-55` PrintService 也无任何 audit_log_service 引用

**业务影响**：用户打印销售订单/合同/采购单等业务单据完全无审计追溯

**修复建议**：
1. 补 `AuthContext` 参数
2. 补 `OperationType::Print` 审计（需先扩展枚举）
3. 补 print 专属权限码校验

---

##### 缺陷 1-6：MRP/AR 对账单导出仅 tracing::info 不落审计库（P1）
**风险等级：P1**
**证据**：
- `backend/src/handlers/mrp_handler.rs:304-317` `export_calculation` 无 AuditLogService 调用
- `backend/src/handlers/ar_reconciliation_handler.rs:565-587` `export_reconciliation_pdf` 仅 `info!`：
```rust
info!("用户 {} 导出对账单PDF，ID: {}", auth.username, id);
// ...无 AuditLogService::record_async 调用
```

**业务影响**：MRP 计算结果与 AR 对账单导出操作无法在 audit_logs 表追溯，仅 stdout 日志

**修复建议**：补 `OperationType::Export` 审计 + resource_type/resource_id 字段

---

##### 缺陷 1-7：销售/采购订单导出无审计（P1）
**风险等级：P1**
**证据**：
- `backend/src/handlers/sales_order_handler.rs:405-444` `export_orders` 有 `_auth: AuthContext` 但未调用 AuditLogService
- `backend/src/handlers/purchase_order_handler.rs:496-534` `export_orders` 同上

**业务影响**：销售/采购订单批量导出无法在 audit_logs 表追溯

**修复建议**：补 `OperationType::Export` 审计 + 导出条数 + 查询条件

---

##### 缺陷 1-8：CRM 线索/商机导出无审计（P1）
**风险等级：P1**
**证据**：
- `backend/src/handlers/crm_handler.rs:93-101` `export_leads`、`282-290` `export_opportunities` 均仅 `_auth: AuthContext` 但未调用审计服务

**业务影响**：CRM 客户线索/商机数据批量导出无审计追溯

**修复建议**：补 `OperationType::Export` 审计

---

##### 缺陷 1-9：产品/登录日志/增强审计日志导出无审计（P2）
**风险等级：P2**
**证据**：
- `backend/src/handlers/product_handler.rs:427-466` `export_products` 无审计
- `backend/src/handlers/login_security_handler.rs:419-476` `export_login_logs` 无审计
- `backend/src/handlers/audit_enhanced_handler.rs:124-147` `export_audit_logs` 仅返回 JSON 占位 `download_url`，未真正生成文件、未审计
- `backend/src/handlers/report_enhanced_handler.rs:172-258` `export_pdf`、`export_excel` 仅 `tracing::info!` 不落库审计
- `backend/src/handlers/report_engine_handler.rs:180-238` `export_report` 无审计
- `backend/src/handlers/advanced/analytics.rs:116-142` `export_report` 无审计
- `backend/src/handlers/sales_analysis_handler.rs:188+` `export_analysis` 无审计

**业务影响**：8 个导出端点全部缺业务级审计，仅靠 omni 中间件记录通用 API_CALL

**修复建议**：统一补 `OperationType::Export` 审计落库

---

##### 缺陷 1-10：缺失端点未补齐（P2）
**风险等级：P2**
**证据**：
- 销售报价单（quotations）无导出端点（V15 计划要求补齐）
- 验布记录（fabric_inspection）无导出端点（V15 计划要求补齐）
- 产量工资核算无导出端点（V15 计划要求补齐，仅 HR 角色）
- 能耗管理报表无导出端点（V15 计划要求补齐）
- 化验室打样、大货处方、流转卡无明确禁止前端本地导出按钮的措施

**业务影响**：业务对账、质检报告交付客户、工资单发放、能耗分析等场景无导出能力

**修复建议**：按 V15 计划补齐 4 个端点 + 补审计 + 补专属权限码

---

## 维度 2：13.2 打印导出角色权限矩阵

### 检查方法
- Read `backend/src/middleware/permission.rs:119-129` 验证 `method_to_action` 是否识别 print/export
- Read `backend/database/init_admin_permissions.sql` 验证权限码表是否含 print/export 维度
- Grep `v-permission.*print|v-permission.*export` 在 frontend 验证按钮权限指令
- Grep `'customer.export'`、`'audit.log.export'`、`'dye.recipe.export'` 验证权限码使用

### 发现

#### ✅ 已落实的项

1. **权限中间件基础架构存在**：`backend/src/middleware/permission.rs:64-71` 调用 `check_permission` 校验 `role_id × resource_type × action`
2. **权限缓存机制存在**：`permission.rs:154-208` 使用 `DashMap` + 5 分钟 TTL 缓存
3. **admin 角色短路**：`permission.rs:172-174` admin 角色直接返回 true，符合计划 admin 全权限设计
4. **审计日志导出 admin 限制**：`audit_log_handler.rs:257` 通过 `require_admin_role` 强制 admin

#### ❌ 缺陷项

##### 缺陷 2-1：method_to_action 不识别 print/export（P0）
**风险等级：P0**
**证据**：
- `backend/src/middleware/permission.rs:119-129`：
```rust
fn method_to_action(method: &Method) -> String {
    match *method {
        Method::GET => "read",       // ← print/export 都是 GET → 全识别为 read
        Method::POST => "create",
        Method::PUT => "update",
        Method::PATCH => "update",
        Method::DELETE => "delete",
        _ => "read",
    }
    .to_string()
}
```
- 未按 V15 计划 13.2.4 要求增加 `path.ends_with("/print")` / `path.contains("/export/")` 识别

**业务影响**：
- 任何拥有 `read` 权限的角色（包括销售员、采购员、仓库员）都可调用所有 print/export 端点
- 计划 13.2.1 角色权限矩阵完全无法落地（角色无 `print`/`export` action 可分配）
- 销售员可导出染色配方、仓库员可打印销售合同，严重违反业务权限

**修复建议**：
1. 升级 `method_to_action(method, path)` 双参数版本：
```rust
if path.ends_with("/print") || path.contains("/print/") { return "print"; }
if path.ends_with("/export") || path.contains("/export/") || path.ends_with("/pdf") { return "export"; }
```
2. 在 `permission.rs:69` 调用处传入 path

---

##### 缺陷 2-2：权限码表完全缺 print/export action（P0）
**风险等级：P0**
**证据**：
- `backend/database/init_admin_permissions.sql:1-66` 仅含 `read`/`create`/`update`/`delete` 四个 action
- Grep `'print'` / `'export'` 在该 SQL 文件 0 命中
- 计划 13.2.3 要求的 19 个权限码全部未落地：
  - `sales.order.print`、`sales.contract.print`、`purchase.order.print`、`purchase.receipt.print`、`inventory.transfer.print`、`finance.voucher.print`、`quality.record.print`、`production.order.print`
  - `sales.order.export`、`purchase.order.export`、`dye.recipe.export`、`dye.batch.export`、`color.card.export`、`ar.reconciliation.export`、`audit.log.export`、`salary.export`、`customer.export`、`supplier.export`、`inventory.stock.export`

**业务影响**：角色权限矩阵无法配置 print/export 差异化控制

**修复建议**：执行 V15 计划 13.2.3 的 19 条 INSERT 语句

---

##### 缺陷 2-3：前端导出按钮无 v-permission 权限指令（P1）
**风险等级：P1**
**证据**：
- Grep `v-permission.*print|v-permission.*export` 在 `frontend/src/` 仅命中 `print-templates/index.vue:86,95`（且是 `print_template:update`、`print_template:delete` 与本次审计无关）
- 所有 25+ 个导出按钮（`customer/index.vue:33`、`supplier/index.vue:74`、`inventory/index.vue:94`、`ap/tabs/InvoiceTab.vue:211`、`ar/tabs/InvoiceTab.vue:209`、`voucher/tabs/...`、`system/audit-log/index.vue:90`、`finance/tabs/SubjectTab.vue:21` 等）均无 `v-permission`
- 任何登录用户都能看到并点击这些导出按钮

**业务影响**：
- 计划 13.2.1 要求"客户列表导出仅 admin/finance_manager"、"审计日志导出仅 auditor"完全无法落地
- 销售员可在浏览器看到并点击"导出审计日志"按钮（虽然后端 admin 校验会拦截，但 UX 不友好且暴露端点存在性）

**修复建议**：
1. 在 25+ 个导出按钮添加 `v-permission="'customer.export'"` 等
2. 重构 `exportToExcel` 为接受 `resourceType` 参数的版本（见维度 5）

---

##### 缺陷 2-4：禁止打印/导出的角色清单未实现（P1）
**风险等级：P1**
**证据**：
- 计划 13.2.2 要求的 `PRINT_DENIED_ROLES`、`EXPORT_DENIED_ROLES`、`DYE_RECIPE_EXPORT_DENIED_ROLES` 常量在 backend 中 0 命中
- 当前实现仅有粗粒度 admin 短路 + 单个 admin_role 校验

**业务影响**：销售员/采购员/仓库员等业务角色可无限制调用所有 print/export

**修复建议**：在 `permission.rs` 增加 print/export action 后，配合权限码表实现精细化控制

---

## 维度 3：13.3 打印导出业务级审计补齐

### 检查方法
- Grep `OperationType::Export|OperationType::Print|OperationType::Download` 在 `backend/src/handlers/`
- Read `backend/src/models/audit_log.rs:1-148` 验证 OperationType 枚举
- Grep `export_record_count|export_query_filter|export_file_format|export_approval_token|export_watermark_user` 在 models/

### 发现

#### ✅ 已落实的项

1. **3 个 handler 接入 `OperationType::Export` 审计**：
   - `backend/src/handlers/audit_log_handler.rs:301` `export_audit_logs` 调用 `record_async`
   - `backend/src/handlers/import_export_handler.rs:259` `export_csv` 调用 `record_async`
   - `backend/src/handlers/import_export_handler.rs:314` `export_excel_type` 调用 `record_async`
   - 均记录 user_id/username/operation_type/resource_type/description/request_method/request_path/after_snapshot

2. **审计日志 model 字段较完整**：`backend/src/models/audit_log.rs:71-107` 包含 user_id/username/action/resource_type/resource_id/resource_name/description/ip_address/user_agent/request_method/request_path/request_body/response_status/duration_ms/operation_type/severity/request_id/before_snapshot/after_snapshot

#### ❌ 缺陷项

##### 缺陷 3-1：OperationType 枚举缺 Print/Download（P0）
**风险等级：P0**
**证据**：
- `backend/src/models/audit_log.rs:11-44` 枚举仅含 8 个变体：
```rust
pub enum OperationType {
    Create, Update, Delete, Login, Logout, Export, Query, Other,
    // ← 缺 Print、Download
}
```
- V15 计划 13.3.1 要求新增 `Print`、`Download` 两个变体

**业务影响**：5 个 print_html handler 即使补审计也无法用 `OperationType::Print` 区分业务语义，全部退化为 `Other`

**修复建议**：扩展枚举 + `as_str()` 实现 + 数据库序列化兼容

---

##### 缺陷 3-2：10 个 print/export handler 缺业务级审计（P0）
**风险等级：P0**
**证据**：

| Handler 文件:行号 | 函数 | 当前审计状态 |
|---|---|---|
| `print_handler.rs:19` | `sales_order_print_html` | ❌ 无 |
| `print_handler.rs:26` | `sales_contract_print_html` | ❌ 无 |
| `print_handler.rs:33` | `purchase_order_print_html` | ❌ 无 |
| `print_handler.rs:40` | `purchase_receipt_print_html` | ❌ 无 |
| `print_handler.rs:47` | `inventory_transfer_print_html` | ❌ 无 |
| `sales_order_handler.rs:405` | `export_orders` | ❌ 无 |
| `purchase_order_handler.rs:496` | `export_orders` | ❌ 无 |
| `dye_batch_handler.rs:356` | `export_dye_batches` | ❌ 无 + 缺 AuthContext |
| `dye_recipe_handler.rs:199` | `export_dye_recipes` | ❌ 无 + 缺 AuthContext |
| `mrp_handler.rs:304` | `export_calculation` | ❌ 无 |
| `color_card/scan_export.rs:49` | `export_color_card` | ❌ 无 |
| `ar_reconciliation_handler.rs:565` | `export_reconciliation_pdf` | ⚠️ 仅 `info!` |
| `crm_handler.rs:93` | `export_leads` | ❌ 无 |
| `crm_handler.rs:282` | `export_opportunities` | ❌ 无 |
| `product_handler.rs:427` | `export_products` | ❌ 无 |
| `login_security_handler.rs:419` | `export_login_logs` | ❌ 无 |
| `audit_enhanced_handler.rs:124` | `export_audit_logs` | ❌ 无 |
| `report_enhanced_handler.rs:172` | `export_pdf` | ⚠️ 仅 `info!` |
| `report_enhanced_handler.rs:224` | `export_excel` | ❌ 无 |
| `report_engine_handler.rs:180` | `export_report` | ❌ 无 |
| `advanced/analytics.rs:116` | `export_report` | ❌ 无 |
| `sales_analysis_handler.rs:188` | `export_analysis` | ❌ 无 |

**业务影响**：13/22 个端点（59%）完全无业务级审计，仅靠 omni 中间件记录匿名 `API_CALL`

**修复建议**：按 V15 计划 13.3.3 代码模板补 `AuditLogService::record_async` 调用

---

##### 缺陷 3-3：audit_logs 表缺导出专属字段（P1）
**风险等级：P1**
**证据**：
- `backend/src/models/audit_log.rs:71-107` 表字段不含 `export_record_count`、`export_query_filter`、`export_file_format`、`export_approval_token`、`export_watermark_user`
- Grep 这些字段在 `backend/migrations/` 0 命中

**业务影响**：
- 导出条数、查询条件、文件格式、审批 token、水印用户名等关键追溯信息无法落库
- 计划 13.3.4 完整性校验矩阵无法落地

**修复建议**：执行 V15 计划 13.3.3 的 ALTER TABLE 添加 5 个字段

---

## 维度 4：13.4 敏感数据导出二级审批机制

### 检查方法
- Grep `export_approval|approval_token|ExportApproval` 在 `backend/src/`
- Grep `dye_recipe_master|approval_request|export_approval_request|audit_log_export_log` 在 backend/
- Grep migrations 目录下相关建表语句

### 发现

#### ✅ 已落实的项

无。

#### ❌ 缺陷项

##### 缺陷 4-1：二级审批数据模型完全缺失（P0）
**风险等级：P0**
**证据**：
- Grep `export_approval_request` 在 `backend/` 0 命中（无表、无 model、无 handler）
- Grep `audit_log_export_log` 0 命中（审计日志导出二次审计表不存在）
- Grep `approval_token` 0 命中
- 90+ migrations 文件夹中无 `*create_export_approval*` 或 `*create_audit_log_export*` migration

**业务影响**：
- V15 计划 13.4 完整的二级审批流程无法落地
- 染色配方、缸号、色卡、AR 对账单等敏感数据导出无任何审批流程
- 审计日志本身的导出无二次审计，审计员可篡改/泄露审计记录无法追溯

**修复建议**：
1. 新增 migration 创建 `export_approval_request` 表（V15 计划 13.4.2）
2. 新增 migration 创建 `audit_log_export_log` 表（V15 计划 13.10.4）
3. 实现审批 handler：申请/审批/拒绝/查询状态
4. 实现审批 token 生成 + 10 分钟有效期校验
5. 在 4 个敏感导出 handler 中强制校验 approval_token

---

##### 缺陷 4-2：dye_recipe_master 角色未创建（P0）
**风险等级：P0**
**证据**：
- Grep `dye_recipe_master` 在 backend/ 0 命中
- 当前角色体系无染色配方主管专属角色

**业务影响**：染色配方导出无法落地"仅 dye_recipe_master + 二级审批"控制

**修复建议**：新增 `dye_recipe_master` 角色 + 关联权限码 `dye.recipe.export`

---

##### 缺陷 4-3：永久禁止导出规则未实现（P1）
**风险等级：P1**
**证据**：
- 计划 13.4.4 要求化验室 OK 样配方（lab_dip）、大货处方（production_recipe）、流转卡条码（flow_card）永久禁止导出
- 当前无任何代码或配置文件声明这些资源的"永久禁止导出"状态
- 前端 lab_dip/production_recipe/flow_card 页面无禁用本地导出按钮的明确措施

**业务影响**：核心技术机密可能在新增页面时误开放导出能力

**修复建议**：在前端 utils/export.ts 增加资源黑名单校验

---

## 维度 5：13.5 前端本地导出强制走后端

### 检查方法
- Read `frontend/src/utils/export.ts` 验证是否调用后端 API
- Read `frontend/src/utils/print.ts` 验证是否调用后端 API
- Grep `exportToExcel` 调用点（25+ 页面）
- Grep `printData` 调用点
- Grep `window.print` 调用点

### 发现

#### ✅ 已落实的项

1. **导出文件格式合规**：`frontend/src/utils/export.ts:15` 注释明确"规则 3：禁止 CSV 作为最终交付"，默认 `format: 'excel'`
2. **打印 HTML 含基本样式**：`frontend/src/utils/print.ts:30-97` 包含 `@page` 设置、表格边框、表头居中
3. **XSS 防护已落实**：`print.ts:249-270` `escapeHtml` 转义 5 个特殊字符

#### ❌ 缺陷项

##### 缺陷 5-1：exportToExcel 纯前端生成文件不调用后端 API（P0）
**风险等级：P0**
**证据**：
- `frontend/src/utils/export.ts:79-89`：
```typescript
export function exportToExcel<T extends Record<string, unknown>>(options: ExportOptions<T>) {
  const { filename, columns, data } = options
  // ...
  const htmlContent = generateExcelHTML(columns, data)  // ← 纯前端 HTML
  downloadFile(htmlContent, `${filename}_${date}.xls`, 'application/vnd.ms-excel;charset=utf-8;')
  // ← 无任何 api.get('/export/...') 调用
}
```
- 25+ 个页面调用此函数（已通过 Grep 确认）：
  - `views/customer/index.vue:142` `printData` + 客户列表导出
  - `views/supplier/index.vue:74` `printData` + 供应商列表导出
  - `views/inventory/index.vue:94` 库存台账导出
  - `views/ap/tabs/InvoiceTab.vue:211` 应付发票导出
  - `views/ar/tabs/InvoiceTab.vue:209` 应收发票导出
  - `views/voucher/tabs/composables/useVchrLstProc.ts:17` 凭证导出
  - `views/voucher/tabs/composables/useVchrProc.ts:16` 凭证导出
  - `views/finance/tabs/SubjectTab.vue:133` 会计科目导出
  - `views/system/audit-log/index.vue:187` 审计日志导出
  - `views/budget/tabs/BudgetListTab.vue:135` 预算导出
  - `views/accountSubject/tabs/SubjectListTab.vue:143` 科目导出
  - `views/fixed-assets/tabs/AssetListTab.vue:278` 固定资产导出
  - `views/financeReport/tabs/ReportListTab.vue:106` + `window.print()` 财务报表
  - `views/cost/tabs/CostCollectionTab.vue:211` 成本归集导出
  - `views/quality/tabs/StandardTab.vue:105` 质量标准导出
  - `views/quality/tabs/RecordTab.vue:68` 检验记录导出
  - `views/production/composables/usePrdProc.ts:20` 生产工单导出
  - `views/sales-price/composables/useSpProc.ts:17` 销售报价导出
  - `views/sales-contract/composables/useScProc.ts:16` 销售合同导出
  - `views/purchase/composables/usePurchAct.ts:10` 采购订单导出
  - `views/warehouse/index.vue:207` 仓库列表打印

**业务影响**：
- 25+ 个页面的导出操作完全不触发后端 API → 完全无审计
- 后端 admin_only 检查完全被绕过（如审计日志导出后端有 admin 校验，但前端用本地 `exportToExcel` 绕过）
- 客户/供应商/凭证/发票等敏感数据可被任何登录用户导出

**修复建议**：
1. 重构 `exportToExcel` 接受 `resourceType` + `queryFilter` 参数，调用后端 `/export/excel/:export_type`
2. 在 25+ 个页面调用点更新参数签名
3. 在按钮上加 `v-permission="'customer.export'"` 等

---

##### 缺陷 5-2：审计日志导出"假按钮"陷阱（P0）
**风险等级：P0**
**证据**：
- `frontend/src/api/audit.ts:96-97` 后端 API 函数已实现：
```typescript
export function exportAuditLogs(params: AuditLogListParams = {}): Promise<Blob> {
  return request.get<Blob>('/audit-logs/export', { ... })
}
```
- 但 `frontend/src/views/system/audit-log/index.vue:90,187,409` 实际按钮调用的是本地 `exportToExcel`：
```typescript
import { exportToExcel } from '@/utils/export'  // ← 本地导出
// ...
const handleExport = () => {
  exportToExcel({ filename: '审计日志', ... })  // ← 完全绕过后端
}
```
- `exportAuditLogs` API 函数从未被该页面 import 或调用

**业务影响**：
- 后端 `audit_log_handler::export_audit_logs` 的 admin 校验和审计落库完全失效
- 任何登录用户都可导出审计日志 → 审计员可篡改/泄露审计记录
- 这是 V15 计划 13.5.3 标注的 P0 风险点

**修复建议**：
1. 删除 `handleExport` 中的本地 `exportToExcel` 调用
2. 改为 `import { exportAuditLogs } from '@/api/audit'` 并调用后端 API
3. 按钮添加 `v-permission="'audit.log.export'"` + 限制 auditor 角色

---

##### 缺陷 5-3：printData 纯前端 window.print 不触发后端审计（P1）
**风险等级：P1**
**证据**：
- `frontend/src/utils/print.ts:102-121`：
```typescript
export function printData<T extends Record<string, unknown>>(options: PrintOptions<T>) {
  // ...
  const html = generatePrintHTML(options)
  const printWindow = window.open('', '_blank')
  printWindow.document.write(html)
  // ← 无任何 api.post('/audit/record') 调用
}
```
- V15 计划 13.5.4 要求合理保留 `window.print` 的场景必须补前端审计埋点
- `customer/index.vue:281`、`supplier/index.vue:249`、`warehouse/index.vue:399` 都调用 `printData` 但无审计埋点

**业务影响**：客户列表/供应商列表/仓库列表打印完全无审计

**修复建议**：按 V15 计划 13.5.4 在 `printData` 中补 `api.post('/audit/record', ...)` 调用

---

## 维度 6：13.6 打印导出审计日志完整性审计

### 检查方法
- 按计划 13.6.1 矩阵逐项核对 14 个端点 + 25 前端导出的审计完整性
- 综合维度 3 的发现，评估 user_id/resource_type/resource_id/export_count/ip_address/timestamp/审批token/水印 8 个字段完整性

### 发现

#### ✅ 已落实的项

1. **3 个端点审计完整性较好**：
   - `export_csv`（`import_export_handler.rs:256-283`）：user_id ✅、resource_type ✅、ip_address ✅（通过 omni）、timestamp ✅、after_snapshot 含导出条数 + 查询条件 ✅
   - `export_excel_type`（`import_export_handler.rs:311-338`）：同上
   - `audit_log_handler::export_audit_logs`（`audit_log_handler.rs:298-313`）：user_id ✅、resource_type ✅、description 含条数 ✅

#### ❌ 缺陷项

##### 缺陷 6-1：14 个端点审计完整性矩阵（P0）

| 端点 | user_id | resource_type | resource_id | export_count | ip_address | timestamp | 二级审批 token | 水印 | 完整性 |
|------|---------|---------------|-------------|---------------|------------|-----------|---------------|------|--------|
| `sales_order_print_html` | ❌ 缺 AuthContext | ❌ 缺 | ❌ 缺 | N/A | ❌ 缺 | ✅ omni | N/A | N/A | 🔴 不完整 |
| `sales_contract_print_html` | ❌ 缺 | ❌ 缺 | ❌ 缺 | N/A | ❌ 缺 | ✅ omni | N/A | N/A | 🔴 不完整 |
| `purchase_order_print_html` | ❌ 缺 | ❌ 缺 | ❌ 缺 | N/A | ❌ 缺 | ✅ omni | N/A | N/A | 🔴 不完整 |
| `purchase_receipt_print_html` | ❌ 缺 | ❌ 缺 | ❌ 缺 | N/A | ❌ 缺 | ✅ omni | N/A | N/A | 🔴 不完整 |
| `inventory_transfer_print_html` | ❌ 缺 | ❌ 缺 | ❌ 缺 | N/A | ❌ 缺 | ✅ omni | N/A | N/A | 🔴 不完整 |
| `sales_order_handler::export_orders` | ✅ AuthContext | ❌ 缺 | N/A | ❌ 缺 | ❌ 缺 | ✅ omni | N/A | N/A | 🔴 不完整 |
| `purchase_order_handler::export_orders` | ✅ AuthContext | ❌ 缺 | N/A | ❌ 缺 | ❌ 缺 | ✅ omni | N/A | N/A | 🔴 不完整 |
| `dye_batch_handler::export_dye_batches` | ❌ **缺 AuthContext** | ❌ 缺 | N/A | ❌ 缺 | ❌ 缺 | ✅ omni | ❌ 缺 | ❌ 缺 | 🔴 极不完整 |
| `dye_recipe_handler::export_dye_recipes` | ❌ **缺 AuthContext** | ❌ 缺 | N/A | ❌ 缺 | ❌ 缺 | ✅ omni | ❌ 缺 | ❌ 缺 | 🔴 极不完整 |
| `mrp_handler::export_calculation` | ✅ AuthContext | ❌ 缺 | ✅ 路径参数 | N/A | ❌ 缺 | ✅ omni | N/A | N/A | 🟡 部分完整 |
| `color_card::scan_export::export_color_card` | ✅ AuthContext（未用） | ❌ 缺 | ✅ 路径参数 | N/A | ❌ 缺 | ✅ omni | ❌ 缺 | ❌ 缺 | 🟡 部分完整 |
| `ar_reconciliation_handler::export_reconciliation_pdf` | ✅ AuthContext | ❌ 缺 | ✅ 路径参数 | N/A | ❌ 缺 | ✅ omni | N/A | N/A | 🟡 部分完整 |
| `export_csv`（通用） | ✅ | ✅ | N/A | ❌ 缺（仅 after_snapshot） | ❌ 缺 | ✅ | N/A | N/A | 🟡 部分完整 |
| `export_excel_type`（通用） | ✅ | ✅ | N/A | ❌ 缺（仅 after_snapshot） | ❌ 缺 | ✅ | N/A | N/A | 🟡 部分完整 |
| 前端 25+ 本地导出 | ❌ **完全无审计** | ❌ | ❌ | ❌ | ❌ | ❌ | ❌ | ❌ | 🔴 完全无审计 |

**风险等级：P0**
**业务影响**：V15 计划 13.6.1 矩阵显示 14 个端点中 9 个🔴不完整/极不完整 + 4 个🟡部分完整 + 25 前端导出🔴完全无审计。整体审计完整性 23%（3/14 + 0/25），关键敏感端点（dye_batch/dye_recipe）user_id 都无法记录

**修复建议**：按 V15 计划 13.6.2 优先级队列：
1. P0：`dye_batch` + `dye_recipe` 补 AuthContext + 补审计 + 二级审批
2. P0：前端 AP/AR 发票、凭证、审计日志 4 个页面改走后端 + 补审计
3. P1：5 个 print_html handler 补 `OperationType::Print` 审计
4. P1：5 个 export handler 补 `OperationType::Export` 审计
5. P2：通用 export_csv/export_excel_type 补导出条数字段
6. P2：前端 18 个本地导出页面改走后端

---

## 维度 7：13.7 打印导出 omni_audit 中间件语义增强

### 检查方法
- Read `backend/src/middleware/omni_audit.rs:200-300` 验证 event_type 是否分类 PRINT/EXPORT
- Grep `operation_category|PRINT|EXPORT|classify` 在 omni_audit.rs

### 发现

#### ✅ 已落实的项

1. **omni_audit 中间件全量记录 API 调用**：`backend/src/middleware/omni_audit.rs:254-295` 将所有非 PUBLIC 路径的请求记录到 `omni_audit_logs` 表，含 trace_id/user_id/username/event_type/event_name/resource/action/resource_type/resource_id/ip_address/user_agent/request_method/request_path/request_body/duration_ms/status
2. **跳过敏感/无意义路径**：`omni_audit.rs:223-228` 跳过 PUBLIC_PATHS、/metrics、/health、/swagger-ui、/api-docs、/static
3. **敏感操作告警**：`omni_audit.rs:235-241` 调用 `SensitiveActionAlert::check_and_alert`

#### ❌ 缺陷项

##### 缺陷 7-1：omni_audit 全部记录为 API_CALL，无 PRINT/EXPORT 分类（P1）
**风险等级：P1**
**证据**：
- `backend/src/middleware/omni_audit.rs:258`：
```rust
event_type: "API_CALL".to_string(),  // ← 所有请求统一为 API_CALL
```
- V15 计划 13.7.1 要求的 `classify_operation(method, path)` 函数未实现
- 计划要求按路径后缀识别 `PRINT`/`EXPORT`/`DOWNLOAD`/`READ`/`CREATE`/`UPDATE`/`DELETE`

**业务影响**：
- 无法用 SQL `WHERE event_type = 'EXPORT'` 筛选导出操作
- 无法用 SQL `WHERE event_type = 'PRINT'` 筛选打印操作
- 合规审计报表无法按操作类型分类

**修复建议**：按 V15 计划 13.7.1 实现 `classify_operation` 函数 + 在 `omni_audit.rs:258` 处替换硬编码 `"API_CALL"`

---

##### 缺陷 7-2：omni_audit_logs 表缺 operation_category/export_record_count/export_approval_token 字段（P2）
**风险等级：P2**
**证据**：
- `backend/src/models/omni_audit_log.rs` 不含上述字段
- migrations 中无相关 ALTER TABLE

**业务影响**：V15 计划 13.7.2 表扩展无法落地；13.7.3 审计报表分类查询无法实现

**修复建议**：执行 V15 计划 13.7.2 的 ALTER TABLE 语句

---

## 维度 8：13.8 打印导出文件水印与防泄露

### 检查方法
- Grep `watermark|set_header|set_footer|Watermark` 在 `backend/src/`
- Read `backend/src/utils/xlsx_export.rs` 验证水印实现
- Read `frontend/src/utils/print.ts` 验证打印 HTML 水印

### 发现

#### ✅ 已落实的项

1. **打印 HTML 含基本元数据**：`frontend/src/utils/print.ts:83` 显示"打印时间"、记录数
2. **打印 HTML 含签字栏**：`print.ts:89-93` 含"打印人/审核人/日期"签字占位
3. **打印单据含制单人/审核人/收货人**：`print.ts:198-202` `printSingleDocument` 含三栏签字区
4. **xlsx 文件格式合规**：`backend/src/utils/xlsx_export.rs:103-117` 正确设置 Content-Type 和 Content-Disposition
5. **xlsx 含格式美化**：`xlsx_export.rs:39-90` 标题行加粗、边框、冻结首行、列宽自适应

#### ❌ 缺陷项

##### 缺陷 8-1：xlsx 导出文件完全无水印（P0）
**风险等级：P0**
**证据**：
- `backend/src/utils/xlsx_export.rs:32-97` `build_xlsx` 函数无 `set_header`/`set_footer` 调用
- Grep `watermark|set_header|set_footer|Watermark` 在 `backend/src/` 仅命中 `main.rs:29`（tower_http SetResponseHeaderLayer 无关）和 `event_kafka.rs:353`（Kafka high_watermark 无关）
- V15 计划 13.8.2 要求的 `build_xlsx_with_watermark` 函数不存在
- V15 计划 13.8.1 要求的"机密 - {username} - {ip} - {timestamp}"水印完全未实现

**业务影响**：
- 染色配方/缸号/色卡/审计日志等敏感数据导出后无任何标识
- 二次泄露后无法追溯到导出人
- 企业色彩资产/技术机密一旦外泄无法追责

**修复建议**：
1. 在 `xlsx_export.rs` 新增 `Watermark` 结构体和 `build_xlsx_with_watermark` 函数
2. 调用 `worksheet.set_header("&L机密&C&\"Arial,Bold\"{}&R{}", username, timestamp)`
3. 调用 `worksheet.set_footer("&LIP: {}&C导出审批: {}&R第 &P 页 / 共 &N 页", ip, approval_token_short)`
4. 13 个导出 handler 改用 `build_xlsx_with_watermark`

---

##### 缺陷 8-2：CSV 导出无首行注释水印（P2）
**风险等级：P2**
**证据**：
- 当前 `export_csv` handler（`import_export_handler.rs:237-292`）实际生成的是 xlsx 格式（规则 3），但函数名仍叫 `export_csv`
- 即使后续保留 CSV 路径，也无 `# 导出人:{username} 时间:{timestamp} IP:{ip}` 首行注释

**业务影响**：CSV 文件无法追溯导出人

**修复建议**：CSV 导出补首行注释水印（若保留 CSV 路径）

---

##### 缺陷 8-3：PDF 导出无背景水印（P2）
**风险等级：P2**
**证据**：
- `backend/src/services/export_service.rs` 的 `export_pdf` 函数未读取
- Grep `watermark|background.*watermark|半透明` 在 `backend/src/services/` 0 命中
- V15 计划 13.8.1 要求 PDF 每页背景水印

**业务影响**：AR 对账单 PDF、报表 PDF 导出后无追溯标识

**修复建议**：使用 `pdf-writer` 或后处理添加半透明大字水印

---

##### 缺陷 8-4：打印 HTML 缺用户/IP 水印（P2）
**风险等级：P2**
**证据**：
- `frontend/src/utils/print.ts:83` 仅显示"打印时间: ${printDate}"
- `print.ts:90-92` 签字栏是空白下划线，未自动填入当前用户名/IP
- V15 计划 13.8.1 要求 HTML 页眉页脚含 `{username} - {timestamp}`

**业务影响**：打印单据无法追溯到打印人

**修复建议**：在 `printData` 函数中传入当前用户信息并写入页眉

---

## 维度 9：13.9 打印导出性能与并发控制

### 检查方法
- Grep `MAX_CONCURRENT_EXPORTS|CONCURRENT_EXPORTS|stream_export|export_large_data` 在 `backend/src/`
- Read `backend/src/services/import_export_service.rs:660-670` 验证导出条数上限
- Grep `limit|page_size|MAX_` 在 dye_batch/dye_recipe/sales_order export handler

### 发现

#### ✅ 已落实的项

1. **导入侧有完整大小限制**：`backend/src/services/import_export_service.rs:32-44`
   - `MAX_CSV_BYTES = 10MB`
   - `MAX_EXCEL_ROWS = 10000`
   - `MAX_EXCEL_COLS = 100`
   - `MAX_CELL_LEN = 1024`
2. **导入侧早期校验**：`import_export_handler.rs:67-73、137-164` handler 入口校验 + DTO validate 双重防御
3. **通用导出有行数上限**：`import_export_service.rs:864` `MAX_EXPORT_ROWS: u64 = 10_000`，在 `export_products/export_customers` 等通用导出中应用
4. **登录日志导出有分页**：`login_security_handler.rs:441` `.paginate(state.db.as_ref(), 10000).fetch_page(0)` 限制 1 万行
5. **rate_limit 中间件存在**：`backend/src/middleware/rate_limit.rs` 实现 IP+用户限流，支持 Redis + 内存降级

#### ❌ 缺陷项

##### 缺陷 9-1：导出无全局并发控制（P1）
**风险等级：P1**
**证据**：
- Grep `MAX_CONCURRENT_EXPORTS|CONCURRENT_EXPORTS` 在 `backend/src/` 0 命中
- V15 计划 13.9.2 要求的全局 `AtomicUsize` 并发计数器未实现
- 计划要求 `MAX_CONCURRENT_EXPORTS = 10` 全局并发上限

**业务影响**：
- 多用户同时触发大量导出可能拖垮 DB 连接池和内存
- 大表导出（如 audit_logs 100000 条）可能 OOM

**修复建议**：在 `import_export_handler.rs` 增加 `static CONCURRENT_EXPORTS: AtomicUsize` + scopeguard 递减

---

##### 缺陷 9-2：dye_batch/dye_recipe 导出无条数上限（P0）
**风险等级：P0**
**证据**：
- `backend/src/handlers/dye_batch_handler.rs:374` `let batches = q.all(&*state.db).await?;` 全量查询无 limit
- `backend/src/handlers/dye_recipe_handler.rs:224` `let recipes = q.all(&*state.db).await?;` 全量查询无 limit
- V15 计划 13.9.1 要求染色配方"禁止批量导出（仅单条 + 二级审批）"、缸号"5000 条 + 二级审批"

**业务影响**：
- 全量查询可能返回数万条染色配方，触发 OOM
- 即使后续补二级审批，全量查询也会先消耗资源

**修复建议**：
1. 染色配方导出强制单条（路径参数 `:id`）
2. 缸号导出补 `.limit(5000)`
3. 增加导出前预估条数检查

---

##### 缺陷 9-3：sales_order/purchase_order 导出无条数上限（P1）
**风险等级：P1**
**证据**：
- `backend/src/handlers/sales_order_handler.rs:412-415` 调用 `sales_service.export_orders_to_csv` → 全量查询
- `backend/src/handlers/purchase_order_handler.rs:503-505` 同上
- V15 计划 13.9.1 要求销售/采购订单单次导出 ≤ 10000 条

**业务影响**：大客户订单超万条时导出可能 OOM

**修复建议**：在 service 层增加 `.limit(10000)`

---

##### 缺陷 9-4：大数据量导出无流式处理（P2）
**风险等级：P2**
**证据**：
- Grep `stream_export|StreamBody|export_large_data` 在 `backend/src/` 0 命中
- V15 计划 13.9.3 要求超过 5000 条的数据必须流式导出
- 当前所有导出 handler 都是先 `q.all()` 一次性加载到内存，再 `build_xlsx` 一次性序列化

**业务影响**：库存记录 50000 条、审计日志 100000 条导出可能 OOM

**修复建议**：使用 `axum::body::StreamBody` + `rust_xlsxwriter` 流式写入

---

##### 缺陷 9-5：print/export 端点未挂载 rate_limit 中间件（P2）
**风险等级：P2**
**证据**：
- Grep `rate_limit|RateLimitLayer|rate_limit_by_ip|from_fn.*rate` 在 `backend/src/routes/` 仅命中 `auth.rs:33`（登录 anti_brute_force）和 `finance.rs:802`（单个财务端点）
- 13 个 print/export 端点均无 rate_limit 挂载

**业务影响**：高频导出攻击可绕过限流

**修复建议**：在 print/export 路由组挂载 `middleware::from_fn(rate_limit::rate_limit_by_ip)`

---

## 维度 10：13.10 打印导出合规审计与定期审查

### 检查方法
- Grep `daily_export_compliance|compliance_review|export_compliance|security_alert` 在 `backend/src/`
- Grep `high_frequency_export|export_rate_limit|非工作时间|off_hours` 在 `backend/src/`
- Read `backend/src/services/business_metrics.rs` 验证是否有导出相关监控指标

### 发现

#### ✅ 已落实的项

1. **业务指标监控基础设施存在**：`backend/src/services/business_metrics.rs:203-207` 实现了 `erp_security_alerts_total` Counter
2. **业务指标告警记录函数存在**：`business_metrics.rs:345-346` `record_security_alert(alert_type)` 可记录安全告警
3. **omni_audit_handler 提供 dashboard 数据**：`backend/src/handlers/omni_audit_handler.rs:139-152` 可查询近 24 小时安全告警数
4. **报表订阅调度器存在**：`backend/src/services/report/job.rs` 支持 cron 表达式调度，可作为合规审查的调度基础设施

#### ❌ 缺陷项

##### 缺陷 10-1：每日合规审查定时任务完全缺失（P1）
**风险等级：P1**
**证据**：
- Grep `daily_export_compliance` 在 `backend/src/` 0 命中
- V15 计划 13.10.2 要求的 `daily_export_compliance_review` 函数未实现
- 计划要求每日 02:00（cron `0 2 * * *`）审查前一天所有 print/export 操作

**业务影响**：
- 异常导出行为（高频/大批量/非工作时间/离职用户/跨权限/敏感数据无审批）无法自动识别
- 安全事件无法主动发现，只能事后人工排查

**修复建议**：
1. 新增 `backend/src/services/export_compliance_service.rs`
2. 实现 `daily_export_compliance_review` 函数，按 V15 计划 13.10.2 模板
3. 使用 `tokio::spawn` + `tokio::time::interval` 启动定时任务
4. 在 `main.rs` 启动时注册

---

##### 缺陷 10-2：异常导出行为识别规则未实现（P1）
**风险等级：P1**
**证据**：
- V15 计划 13.10.1 要求的 6 类异常模式检测全部未实现：
  - 高频导出（1 小时 > 10 次）：未实现
  - 大批量导出（> 上限 80%）：未实现
  - 非工作时间导出（22:00-06:00）：未实现
  - 离职用户导出：未实现
  - 跨权限导出：未实现
  - 敏感数据无审批导出：未实现（因审批机制本身缺失）
- Grep `high_frequency_export|off_hours|非工作时间` 0 命中

**业务影响**：异常导出行为无法被自动检测和告警

**修复建议**：在 `export_compliance_service.rs` 实现六类规则检测函数 + 告警输出

---

##### 缺陷 10-3：审计日志保留期限策略未实现（P2）
**风险等级：P2**
**证据**：
- V15 计划 13.10.3 要求普通 print/export 审计保留 3 年、敏感数据 7 年、omni_audit 1 年定期归档、安全告警 7 年
- 当前无任何代码实现定期归档/清理策略
- 无 `security_alert_log` 表

**业务影响**：
- 审计日志无限增长，存储成本持续上升
- 老旧数据无归档机制，查询性能下降

**修复建议**：实现定时归档任务 + 创建 `security_alert_log` 表

---

##### 缺陷 10-4：审计日志导出二次审计机制缺失（P0）
**风险等级：P0**
**证据**：
- V15 计划 13.10.4 要求审计日志导出操作必须记录到独立表 `audit_log_export_log`，含 auditor_id/query_filter/export_record_count/export_file_hash/approval_token
- Grep `audit_log_export_log` 在 `backend/` 0 命中
- 当前审计日志导出（`audit_log_handler.rs:298-313`）仅记录到 `audit_logs` 表自身，审计员可篡改/删除自身导出记录
- 即使前端改为走后端 API（缺陷 5-2 修复后），仍缺乏独立审计表

**业务影响**：
- 审计员可导出审计日志后篡改自身审计记录，掩盖越权行为
- 审计员离职后无法追溯其历史导出行为

**修复建议**：
1. 新增 migration 创建 `audit_log_export_log` 表（V15 计划 13.10.4）
2. 在 `audit_log_handler::export_audit_logs` 中除写入 `audit_logs` 外，额外写入 `audit_log_export_log`
3. 计算导出文件 SHA256 哈希存档，便于完整性校验
4. 强制 CEO/admin 二级审批 token

---

## 审计结果汇总

| 维度 | P0 | P1 | P2 | P3 | 已落实 | 总检查项 |
|------|----|----|----|----|--------|----------|
| 13.1 打印导出端点合理性 | 3 | 4 | 3 | 0 | 5 | 15 |
| 13.2 角色权限矩阵 | 2 | 2 | 0 | 0 | 4 | 8 |
| 13.3 业务级审计补齐 | 2 | 1 | 0 | 0 | 2 | 5 |
| 13.4 二级审批机制 | 2 | 1 | 0 | 0 | 0 | 3 |
| 13.5 前端本地导出走后端 | 2 | 1 | 0 | 0 | 3 | 6 |
| 13.6 审计日志完整性 | 1 | 0 | 0 | 0 | 1 | 2 |
| 13.7 omni_audit 语义增强 | 0 | 1 | 1 | 0 | 3 | 5 |
| 13.8 文件水印防泄露 | 1 | 0 | 3 | 0 | 5 | 9 |
| 13.9 性能与并发控制 | 1 | 2 | 2 | 0 | 5 | 10 |
| 13.10 合规审计与定期审查 | 1 | 2 | 1 | 0 | 4 | 9 |
| **合计** | **15** | **14** | **10** | **0** | **32** | **72** |

**审计覆盖**：13 个后端 print/export 端点 + 25+ 前端本地导出按钮 + 8 个权限/审计基础设施文件
**已落实率**：32/72 = 44.4%
**P0 阻塞缺陷**：15 个（必须立即修复）

---

## 修复优先级队列

### P0 阻塞缺陷（15 个，必须立即修复）

1. **缺陷 1-1**：打印 HTML 返回硬编码占位数据（`backend/src/services/print_service.rs:57-142`）
2. **缺陷 1-2**：染色配方导出缺 AuthContext/审计/审批（`backend/src/handlers/dye_recipe_handler.rs:199`）
3. **缺陷 1-3**：缸号导出缺 AuthContext/审计/审批（`backend/src/handlers/dye_batch_handler.rs:356`）
4. **缺陷 2-1**：method_to_action 不识别 print/export（`backend/src/middleware/permission.rs:119`）
5. **缺陷 2-2**：权限码表完全缺 print/export action（`backend/database/init_admin_permissions.sql`）
6. **缺陷 3-1**：OperationType 枚举缺 Print/Download（`backend/src/models/audit_log.rs:11-44`）
7. **缺陷 3-2**：10 个 print/export handler 缺业务级审计（多处）
8. **缺陷 4-1**：二级审批数据模型完全缺失（无 migration）
9. **缺陷 4-2**：dye_recipe_master 角色未创建
10. **缺陷 5-1**：exportToExcel 纯前端生成文件不调用后端 API（`frontend/src/utils/export.ts:79`）
11. **缺陷 5-2**：审计日志导出"假按钮"陷阱（`frontend/src/views/system/audit-log/index.vue:409`）
12. **缺陷 6-1**：14 个端点审计完整性矩阵（覆盖全维度）
13. **缺陷 8-1**：xlsx 导出文件完全无水印（`backend/src/utils/xlsx_export.rs:32`）
14. **缺陷 9-2**：dye_batch/dye_recipe 导出无条数上限（`dye_batch_handler.rs:374`、`dye_recipe_handler.rs:224`）
15. **缺陷 10-4**：审计日志导出二次审计机制缺失（无 `audit_log_export_log` 表）

### P1 高优先级缺陷（14 个）

1. **缺陷 1-4**：色卡导出无审计/审批（`color_card/scan_export.rs:49`）
2. **缺陷 1-5**：5 个 print_html handler 无审计接入（`print_handler.rs:19-52`）
3. **缺陷 1-6**：MRP/AR 对账单导出仅 info! 不落审计库
4. **缺陷 1-7**：销售/采购订单导出无审计
5. **缺陷 1-8**：CRM 线索/商机导出无审计
6. **缺陷 2-3**：前端导出按钮无 v-permission 权限指令
7. **缺陷 2-4**：禁止打印/导出的角色清单未实现
8. **缺陷 3-3**：audit_logs 表缺导出专属字段
9. **缺陷 4-3**：永久禁止导出规则未实现
10. **缺陷 5-3**：printData 纯前端 window.print 不触发后端审计
11. **缺陷 7-1**：omni_audit 全部记录为 API_CALL，无 PRINT/EXPORT 分类
12. **缺陷 9-1**：导出无全局并发控制
13. **缺陷 9-3**：sales_order/purchase_order 导出无条数上限
14. **缺陷 10-1**：每日合规审查定时任务完全缺失
15. **缺陷 10-2**：异常导出行为识别规则未实现

### P2 中优先级缺陷（10 个）

1. **缺陷 1-9**：产品/登录日志/增强审计日志导出无审计
2. **缺陷 1-10**：缺失端点未补齐
3. **缺陷 7-2**：omni_audit_logs 表缺 operation_category 等字段
4. **缺陷 8-2**：CSV 导出无首行注释水印
5. **缺陷 8-3**：PDF 导出无背景水印
6. **缺陷 8-4**：打印 HTML 缺用户/IP 水印
7. **缺陷 9-4**：大数据量导出无流式处理
8. **缺陷 9-5**：print/export 端点未挂载 rate_limit 中间件
9. **缺陷 10-3**：审计日志保留期限策略未实现

### P3 低优先级缺陷（0 个）

无。

---

## 审计结论

V15 类十三打印导出审计与权限控制专项共审计 10 个维度 72 个检查项，发现 39 个缺陷（15 P0 + 14 P1 + 10 P2），已落实 32 项（44.4%）。

**核心问题**：
1. **业务级审计覆盖率仅 23%**：13 个后端端点仅 3 个接入 `OperationType::Export` 审计
2. **2 个敏感端点缺 AuthContext**：染色配方、缸号导出无法关联调用者
3. **25+ 前端本地导出完全无审计**：审计日志、AP/AR 发票、凭证等敏感页面绕过后端
4. **审计日志导出"假按钮"陷阱**：后端 admin 校验和审计落库完全失效
5. **权限模型缺 print/export action**：所有 print/export 请求被识别为 `read`
6. **二级审批机制完全缺失**：无表、无角色、无 token 校验
7. **导出文件无水印**：xlsx/CSV/PDF/HTML 均无用户/IP/时间标识
8. **打印 HTML 是占位假数据**：5 个 print_html handler 返回硬编码 stub

**修复路径建议**：
- **第一优先**：修复 15 个 P0 阻塞缺陷，重点是染色配方/缸号导出补 AuthContext + 前端审计日志按钮改走后端 + 权限模型补 print/export action + OperationType 枚举扩展
- **第二优先**：14 个 P1 缺陷，重点是 5 个 print_html 补审计 + 25 前端导出按钮加 v-permission + omni_audit 分类增强
- **第三优先**：10 个 P2 缺陷，重点是补齐缺失端点 + 水印补齐 + 流式导出

本审计为只读审计，未修改任何业务代码。
