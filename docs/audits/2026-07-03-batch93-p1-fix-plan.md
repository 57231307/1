# 批次 93：v4 P1 修复规划（9 项）

**生成时间**：2026-07-03
**关联复审**：`docs/audits/2026-07-03-reaudit-v4.md` 第三节 P1 问题
**修复目标**：修复 9 项 P1 问题 — 事务边界 TOCTOU（delete 方法漏修）+ id:Set(0) 推广未完成

## 修复项清单

| # | 问题 | 文件 | 类型 | 复杂度 |
|---|------|------|------|--------|
| 1 | id: Set(0) 推广修复（13 处） | services/*.rs | 机械修复 | 低 |
| 2 | ar/recon.rs delete TOCTOU | services/ar/recon.rs | 补 txn + lock_exclusive | 中 |
| 3 | voucher_service.rs delete TOCTOU + 缺失审计 | services/voucher_service.rs | 补 txn + lock_exclusive + user_id | 中 |
| 4 | production_order_service.rs delete TOCTOU + 缺失审计 | services/production_order_service.rs | 补 txn + lock_exclusive + user_id | 中 |
| 5 | supplier_service.rs delete_supplier TOCTOU + 缺失审计 | services/supplier_service.rs | 补 txn + lock_exclusive + user_id | 中 |
| 6 | supplier_service.rs delete_supplier_contact 缺失审计 | services/supplier_service.rs | 补 txn + user_id | 中 |
| 7 | sales_return_service.rs delete_return TOCTOU + 缺失审计 | services/sales_return_service.rs | 补 txn + lock_exclusive + user_id | 中 |
| 8 | sales_return_service.rs delete_return_item TOCTOU | services/sales_return_service.rs | 状态门移入 txn | 中 |
| 9 | inventory_adjustment_service.rs delete_adjustment_item TOCTOU | services/inventory_adjustment_service.rs | 状态门移入 txn | 中 |

## 修复分批

**批次 93（本批次，一次性合并 9 项）**：
- 子批 A：id:Set(0) 推广修复（项 1，机械修复）
- 子批 B：delete 方法 TOCTOU 修复（项 2-9，补 txn + lock_exclusive + user_id）

## 详细修复方案

### 项 1：id: Set(0) 推广修复（13 处）

机械修复，将 `id: Set(0)` 改为 `id: Default::default()` 或使用 `..Default::default()` 模式。

涉及文件：
- `ar/recon.rs:37` — ReconciliationEntity create
- `ar/vfy.rs:133,172,191,227,249,274,296,539,565,586` — 10 处
- `batch_service.rs:110` — product create
- `finance_payment_service.rs:72` — payment create
- `inventory_adjustment_service.rs:82,130,515` — adjustment + item create
- `po/receipt.rs:343`、`po/price.rs:182,290`、`po/order.rs:310` — purchase_order_item create
- `product_service.rs:369` — product_color create
- `user_service.rs:143` — user create
- `inventory_stock_query.rs:43` — inventory_transaction create
- `operation_log_service.rs:53` — operation_log create
- `business_trace_service.rs:163` — business_trace_snapshot create
- `purchase_contract_service.rs:192` — purchase_contract_execution create

### 项 2：ar/recon.rs delete 补 lock_exclusive

参考同文件 update 方法（行 107-113）修复模式：
```rust
pub async fn delete(&self, id: i32, user_id: i32) -> Result<(), AppError> {
    let txn = (*self.db).begin().await?;
    let model = ReconciliationEntity::find_by_id(id)
        .lock_exclusive()
        .one(&txn).await?
        .ok_or_else(|| AppError::not_found("对账单不存在"))?;
    if model.reconciliation_status.as_deref() != Some("draft") {
        return Err(...);
    }
    // delete_with_audit(&txn, ...)
    txn.commit().await?;
    Ok(())
}
```

### 项 3：voucher_service.rs delete 补 lock_exclusive + user_id

- 签名补 `user_id: i32`
- begin txn + lock_exclusive + 状态门 + delete_with_audit(&txn)

### 项 4：production_order_service.rs delete 补 lock_exclusive + user_id

- 签名补 `user_id: i32`
- begin txn + lock_exclusive + 状态门 + update_with_audit(&txn)

### 项 5：supplier_service.rs delete_supplier 补 txn + lock_exclusive + user_id

- 签名补 `user_id: i32`
- can_delete_supplier 检查移入 txn 或在 txn 内重新校验
- get_supplier + delete 合并到 txn
- 补 delete_with_audit

### 项 6：supplier_service.rs delete_supplier_contact 补 txn + user_id

- 签名补 `user_id: i32`
- begin txn + lock_exclusive + delete_with_audit(&txn)

### 项 7：sales_return_service.rs delete_return 补 lock_exclusive + user_id

- 签名补 `user_id: i32`
- begin txn + lock_exclusive + 状态门（DRAFT）+ delete_with_audit(&txn)
- handler `sales_return_handler.rs:126` `_auth` → `auth`，注入 `auth.user_id`

### 项 8：sales_return_service.rs delete_return_item 状态门移入 txn

- 状态门（pending）移入 txn + lock_exclusive

### 项 9：inventory_adjustment_service.rs delete_adjustment_item 状态门移入 txn

- 状态门（pending）移入 txn + lock_exclusive

## CI 验证策略

- 编译错误修复后 push → CI 全绿（12 项必检，E2E continue-on-error 非阻塞）→ squash merge
- 关键检查：Rust 后端构建、Rust Clippy、Rust 单元测试、Rust 格式检查

## 进度跟踪

| 子批 | 项 | 状态 |
|------|----|----|
| A | 1 | ✅ 已完成（19 文件 39 处 + 额外发现 12 处） |
| B | 2, 3, 4, 5, 6, 7, 8, 9 | ✅ 已完成（7 service + 4 handler） |
| 提交 | PR + CI + 合并 | ✅ 已完成（main `980dec0`，PR #337） |

## 实施记录（2026-07-03）

### 子批 A — id:Set(0) 推广（项 1）

原计划 13 处，实际修复 39 处（含 v3 复审遗漏的 26 处）：

| 文件 | 处数 |
|------|------|
| ar/recon.rs | 1 |
| ar/vfy.rs | 10+ |
| batch_service.rs | 1 |
| finance_payment_service.rs | 1 |
| inventory_adjustment_service.rs | 3 |
| po/receipt.rs | 1 |
| po/price.rs | 2 |
| po/order.rs | 1 |
| product_service.rs | 1 |
| user_service.rs | 1 |
| inventory_stock_query.rs | 1 |
| operation_log_service.rs | 1 |
| business_trace_service.rs | 1 |
| purchase_contract_service.rs | 1 |
| init_service.rs（额外发现） | 6 |
| purchase_return_service.rs（额外发现） | 2 |
| inventory_stock_txn.rs（额外发现） | 1 |
| role_service.rs（额外发现） | 1 |
| assist_accounting_service.rs（额外发现） | 2 |
| purchase_inspection_service.rs（额外发现） | 1 |

修复方式：`id: Set(0)` → `id: Default::default()`（即 NotSet），避免 PostgreSQL SERIAL 列显式插入 0 覆盖默认值导致主键冲突。

### 子批 B — delete TOCTOU 修复（项 2-9）

| 项 | 文件 | 修复 |
|---|------|------|
| 2 | ar/recon.rs::delete | txn + lock_exclusive + 状态门移入 txn |
| 3 | voucher_service.rs::delete | 补 user_id + txn + lock_exclusive + 状态门 + delete_with_audit |
| 4 | production_order_service.rs::delete | 补 user_id + txn + lock_exclusive + validate_status_transition + 软删除 |
| 5 | supplier_service.rs::delete_supplier | 补 user_id + txn + lock_exclusive + delete_with_audit |
| 6 | supplier_service.rs::delete_supplier_contact | 补 user_id + txn + lock_exclusive + delete_with_audit |
| 7 | sales_return_service.rs::delete_return | 补 user_id + txn + lock_exclusive + 状态门 + delete_with_audit |
| 8 | sales_return_service.rs::delete_return_item | 状态门 + lock_exclusive 移入 txn（双锁） |
| 9 | inventory_adjustment_service.rs::delete_adjustment_item | 状态门 + lock_exclusive 移入 txn |

handler 文件修改（4 个，`_auth` → `auth` 并注入 `auth.user_id`）：
- voucher_handler.rs:373
- production_order_handler.rs:362
- supplier_handler.rs:90,193
- sales_return_handler.rs:132

### Clippy 修复（CI 阻塞修复）

CI 第一次 push 后 Rust Clippy 失败，2 条新增警告：
- `field `description` is never read`
- `fields `manager` and `capacity` are never read`

根因：批次 92 给 `CreateWarehouseRequest.description` 加 `#[allow(dead_code)]` 后，clippy 警告摘要从 "fields `manager`, `capacity`, and `description`" 变为 "fields `manager` and `capacity`"，被 baseline diff 机制判定为新增警告。

修复（按 v4 复审"占位符全部实现"指令）：
1. WarehouseService::create 接入 manager（解析为 manager_id，与 update 方法对齐）
2. WarehouseService::create 接入 description（写入 notes 列，实现原 TODO 占位）
3. 移除 description 上的 `#[allow(dead_code)]`（已真实使用）
4. CreateWarehouseRequest.capacity 加 `#[allow(dead_code)]` + TODO（warehouse 模型无 capacity 字段）
5. UpdateWarehouseRequest.capacity 加 `#[allow(dead_code)]` + TODO（同上）

### CI 结果

- 首次 push（commit `d309a85`）：Rust Clippy 失败（2 条新增警告）
- 修复 push（commit `8459ac8`）：12 项必检全绿，E2E continue-on-error 非阻塞
- Squash merge：main `980dec0`，PR #337
