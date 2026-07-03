# 批次 94：v4 P2 修复规划（15 项）

**生成时间**：2026-07-03
**关联复审**：`docs/audits/2026-07-03-reaudit-v4.md` 第四节 P2 问题
**修复目标**：修复 15 项 P2 问题 — SQL 注入 + N+1 + 审计日志占位 user_id + handler auth 未用 + 错误吞没 + 占位符功能

## 修复项清单

| # | 问题 | 文件 | 类型 | 复杂度 |
|---|------|------|------|--------|
| 1 | audit_cleanup_service.rs format! 拼接 SQL | services/audit_cleanup_service.rs | SQL 参数化 | 中 |
| 2 | LIKE 模式注入 - services 层 5 处 | color_card_crud_service.rs、quotation_service.rs、import_export_service.rs(3处)、custom_order_crud_service.rs、mrp_engine_service.rs | safe_like_pattern | 中 |
| 3 | LIKE 模式注入 - handlers 层 3 处 | omni_audit_handler.rs、audit_log_handler.rs、slow_query_handler.rs | safe_like_pattern | 低 |
| 4 | batch_service.rs N+1 查询 | services/batch_service.rs:249-253 | is_in 批量查询 | 中 |
| 5 | color_price_seasonal_service.rs delete 缺失审计 | services/color_price_seasonal_service.rs | 补 user_id + delete_with_audit | 中 |
| 6 | sales_return_handler.rs delete_sales_return `_auth` 未用 | handlers/sales_return_handler.rs | `_auth` → `auth` | 低 |
| 7 | ar_report_handler.rs 4 个接口 `_auth` 未用 | handlers/ar_report_handler.rs | 评估数据权限 | 低 |
| 8 | material_shortage_handler.rs `_auth` 未用 | handlers/material_shortage_handler.rs | 评估数据权限 | 低 |
| 9 | fixed_asset_handler batch_depreciate 客户端 user_id | handlers/fixed_asset_handler.rs | DTO 移除 user_id，用 auth.user_id | 中 |
| 10 | 审计日志 Some(0) 占位 user_id（约 40 处） | 多个 service + handler | service 补 user_id，handler 注入 auth.user_id | 高 |
| 11 | 事件发布/审计 let _ = 吞错（约 20 处） | 多个 service + handler | 改为 ? 传播或 warn 日志 | 中 |
| 12 | 前端 P1 占位功能 | 6 个前端文件 | 真实实现 | 高 |
| 13 | quotation_handler/quotation_service 审计 user_id 占位 | quotation_handler.rs、quotation_service.rs | 接入 AuthContext | 中 |
| 14 | custom_order_crud_service cancel reason 占位 | custom_order_crud_service.rs:255 | 记录到 process_log | 中 |
| 15 | custom_order_quality_service operator_id 占位 | custom_order_quality_service.rs:129 | 记录到 audit_log | 中 |

## 修复分批

**批次 94（本批次，一次性合并 15 项）**：
- 子批 A：SQL 注入修复（项 1, 2, 3）+ N+1（项 4）
- 子批 B：handler auth 接入（项 6, 7, 8, 9）+ service 审计补全（项 5, 13）
- 子批 C：审计日志 Some(0) 推广（项 10）— 大批量
- 子批 D：错误吞没（项 11）+ 占位符（项 14, 15）
- 子批 E：前端占位功能（项 12）

## 详细修复方案

### 项 1：audit_cleanup_service.rs format! 拼接 SQL

- 位置：`backend/src/services/audit_cleanup_service.rs:52-55, 74-77`
- 问题：`format!("DELETE FROM ... WHERE created_at < '{}'", cutoff)` 拼接 SQL，存在注入风险
- 修复：改用 `Statement::from_sql_and_values` 参数化绑定
- 模式：
  ```rust
  let stmt = Statement::from_sql_and_values(
      DatabaseBackend::Postgres,
      r#"DELETE FROM audit_logs WHERE created_at < $1"#,
      [cutoff.into()],
  );
  db.execute(stmt).await?;
  ```

### 项 2：LIKE 模式注入 - services 层（6 文件 8 处）

- 位置：
  - `color_card_crud_service.rs:109`
  - `quotation_service.rs:191`
  - `import_export_service.rs:610,673,733`
  - `custom_order_crud_service.rs:144`
  - `mrp_engine_service.rs:746`
- 问题：`format!("%{}%", keyword)` 直接拼接 LIKE 模式，未转义 `%` `_` `\` 特殊字符
- 修复：改用 `crate::utils::sql_escape::safe_like_pattern(&keyword)` 生成安全模式
- 模式：
  ```rust
  let pattern = safe_like_pattern(&keyword);
  q.filter(Column::Name.like(&pattern))
  ```

### 项 3：LIKE 模式注入 - handlers 层（3 处）

- 位置：`omni_audit_handler.rs:226`、`audit_log_handler.rs:172`、`slow_query_handler.rs:111`
- 修复：同项 2

### 项 4：batch_service.rs N+1 查询

- 位置：`backend/src/services/batch_service.rs:249-253`
- 问题：循环内 `find_by_id` 逐条查询，N 个批次触发 N 次查询
- 修复：改为 `is_in` 批量查询 + HashMap 索引
- 模式：
  ```rust
  let batches = batch::Entity::find()
      .filter(batch::Column::Id.is_in(batch_ids.clone()))
      .all(&*self.db)
      .await?;
  let batch_map: HashMap<i32, batch::Model> = batches.into_iter().map(|b| (b.id, b)).collect();
  // 循环内用 batch_map.get(&id) 替代 find_by_id
  ```

### 项 5：color_price_seasonal_service.rs delete 缺失审计

- 位置：`backend/src/services/color_price_seasonal_service.rs:168-172`
- 修复：补 `user_id: i32` 参数 + `delete_with_audit(&*self.db, ..., Some(user_id))`

### 项 6：sales_return_handler.rs delete_sales_return `_auth` 未用

- 位置：`backend/src/handlers/sales_return_handler.rs:126-137`
- 修复：`_auth` → `auth`，注入 `auth.user_id` 到 service 调用

### 项 7：ar_report_handler.rs 4 个接口 `_auth` 未用

- 位置：`backend/src/handlers/ar_report_handler.rs:23,40,57,74`
- 修复：评估数据权限过滤需求；若报表无数据权限需求，`_auth` 改 `auth` 并记录访问日志

### 项 8：material_shortage_handler.rs `_auth` 未用

- 位置：`backend/src/handlers/material_shortage_handler.rs:61`
- 修复：同项 7

### 项 9：fixed_asset_handler batch_depreciate 客户端 user_id

- 位置：`backend/src/handlers/fixed_asset_handler.rs:314,325`
- 问题：DTO 中 user_id 由客户端传入，可伪造操作人
- 修复：DTO 移除 user_id 字段，handler 使用 `auth.user_id`

### 项 10：审计日志 Some(0) 占位 user_id（约 40 处）

- 位置：bom_service、finance_invoice_service、role_permission_service、sales_return_service、crm/lead、crm/opp、crm/recycle_rule、purchase_receipt_private、po/receipt、ap_payment_request_service、product_service、product_category_service、color_price_tier_service、inventory_adjustment_service、inv/inventory_move、supplier_service、sales_fabric_order_handler、warehouse_handler、inventory_batch_handler、logistics_handler 等
- 问题：审计日志 user_id 传 `Some(0)`，无法追踪真实操作人
- 修复：service 方法签名补 `user_id: i32`，handler 注入 `auth.user_id`
- 模式：
  ```rust
  // service
  pub async fn create(&self, req: Dto, user_id: i32) -> Result<...> {
      AuditLogService::create_with_audit(&*self.db, "table", active_model, Some(user_id)).await
  }
  // handler
  pub async fn create(State(state): State<AppState>, auth: AuthContext, Json(req): Json<Dto>) {
      let result = service.create(req, auth.user_id).await?;
  }
  ```

### 项 11：事件发布/审计 let _ = 吞错（约 20 处）

- 位置：purchase_price_service、sales_price_service、sales_contract_service、bpm_service、sales_order_handler、inventory_stock_handler 等
- 问题：`let _ = publish_event(...)` 静默吞错，事件发布失败无感知
- 修复：改为 `?` 传播（关键路径）或 `if let Err(e) = ... { warn!(...) }`（非关键路径）

### 项 12：前端 P1 占位功能

| 文件 | 占位 | 修复 |
|------|------|------|
| quality/index.vue:456 | 更新功能未实现 | 实现更新对话框 + API 调用 |
| inventory/tabs/StockTab.vue | 4 处占位（编辑/删除/批量调整/批量删除） | 实现对应功能 |
| purchase-contract/composables/usePcProc.ts:74 | 导出假成功 | 接入真实导出 API |
| crm/leads/index.vue:366 | 导出假成功 | 接入真实导出 API |
| security/two-factor/composables/useTfaProc.ts:79,110 | 恢复码客户端生成 | 改为服务端生成 |
| custom-orders/list.vue:202 | 操作人 ID 占位为 1 | 接入真实 user_id |

### 项 13：quotation_handler/quotation_service 审计 user_id 占位

- 位置：`quotation_handler.rs:199` `let _ = auth;`、`quotation_service.rs:406` `let _ = user_id;`
- 修复：接入 AuthContext 注入 user_id 到审计日志

### 项 14：custom_order_crud_service cancel reason 占位

- 位置：`custom_order_crud_service.rs:255` `let _ = dto.reason;`
- 修复：记录 cancel reason 到 process_log 或 custom_order.notes

### 项 15：custom_order_quality_service operator_id 占位

- 位置：`custom_order_quality_service.rs:129` `let _ = dto.operator_id;`
- 修复：记录 operator_id 到 audit_log 或 quality_issue 记录

## CI 验证策略

- 编译错误修复后 push → CI 全绿（12 项必检，E2E continue-on-error 非阻塞）→ squash merge
- 关键检查：Rust 后端构建、Rust Clippy、Rust 单元测试、Rust 格式检查
- 注意：项 10 涉及约 40 处 service 签名变更，可能引发编译错误，需仔细验证

## 进度跟踪

| 子批 | 项 | 状态 |
|------|----|----|
| A | 1, 2, 3, 4 | ⬜ 待修复 |
| B | 5, 6, 7, 8, 9, 13 | ⬜ 待修复 |
| C | 10 | ⬜ 待修复 |
| D | 11, 14, 15 | ⬜ 待修复 |
| E | 12 | ⬜ 待修复 |
| 提交 | PR + CI + 合并 | ⬜ 待执行 |
