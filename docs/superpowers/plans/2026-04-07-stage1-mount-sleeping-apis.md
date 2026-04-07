# 第一阶段：挂载沉睡 API 并打通核心单据高级操作

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** 将后端已经实现但未挂载的采购、应付账款、财务分析等核心业务 API 挂载到 Axum 路由，并解除前端相关组件的 "开发中" 限制，实现真正的全链路操作。

**Architecture:** 
由于底层 Service 和 Handler 大多已实现完毕并保留在项目中，我们主要的工作是在 `routes/mod.rs` 中补全路由映射，并在前端的 `services/` 下封装对应的 API 请求，最后在 `pages/` 和 `components/` 触发真实的 `Msg` 代替原本占位的 `{"功能开发中"}` 提示。

**Tech Stack:** Rust (Axum, SeaORM), WebAssembly (Yew), reqwasm

---

### Task 1: 挂载后端沉睡 API 到路由

**Files:**
- Modify: `backend/src/routes/mod.rs`

- [ ] **Step 1: 补全采购订单高级操作路由**
打开 `backend/src/routes/mod.rs`，在 `purchase_order_routes` 中添加 `submit`, `reject`, `close` 操作的路由。

```rust
// 搜索 let purchase_order_routes = Router::new() 并补充:
        .route("/:id/submit", post(purchase_order_handler::submit_order))
        .route("/:id/reject", post(purchase_order_handler::reject_order))
        .route("/:id/close", post(purchase_order_handler::close_order))
```

- [ ] **Step 2: 补全应付账款 (AP Invoice) 高级操作路由**
在 `ap_routes` 的 `invoice` 模块下挂载审核、作废等路由。

```rust
// 搜索 ap_invoice_handler 相关的路由并补充:
        .route("/invoices/:id/approve", post(ap_invoice_handler::approve_invoice))
        .route("/invoices/:id/cancel", post(ap_invoice_handler::cancel_invoice))
        .route("/invoices/auto-generate", post(ap_invoice_handler::auto_generate))
        .route("/invoices/aging", get(ap_invoice_handler::get_aging_analysis))
```

- [ ] **Step 3: 补全财务分析 (Financial Analysis) 路由**
在 `financial_analysis_routes` 中补全趋势分析和新建指标的路由。

```rust
// 补充 financial_analysis_routes:
        .route("/indicators", post(financial_analysis_handler::create_indicator))
        .route("/trends", get(financial_analysis_handler::get_trends))
```

- [ ] **Step 4: 运行检查**

```bash
cd backend && cargo check --bin server
```
Expected: PASS 无路由编译错误。

- [ ] **Step 5: Commit**

```bash
git add backend/src/routes/mod.rs
git commit -m "feat: mount sleeping APIs for purchase orders, AP invoices, and financial analysis"
```

---

### Task 2: 完善前端采购订单服务与界面对接

**Files:**
- Modify: `frontend/src/services/purchase_order_service.rs`
- Modify: `frontend/src/pages/purchase/purchase_order_detail.rs`

- [ ] **Step 1: 在 Service 中封装高级 API 请求**

```rust
// 在 purchase_order_service.rs 中添加
pub async fn submit_order(id: i32) -> Result<ApiResponse<()>, AppError> {
    request::post(&format!("/api/v1/erp/purchases/orders/{}/submit", id), &serde_json::Value::Null).await
}

pub async fn reject_order(id: i32, reason: &str) -> Result<ApiResponse<()>, AppError> {
    let body = serde_json::json!({ "reason": reason });
    request::post(&format!("/api/v1/erp/purchases/orders/{}/reject", id), &body).await
}
```

- [ ] **Step 2: 在详情页移除“开发中”限制并对接真实事件**
打开 `purchase_order_detail.rs`，找到对应的按钮（如“提交审核”、“驳回”），将它们绑定到组件的 `update` 函数中调用上面封装的 API。

- [ ] **Step 3: 运行检查**

```bash
cd frontend && cargo check
```
Expected: PASS

- [ ] **Step 4: Commit**

```bash
git add frontend/src/services/purchase_order_service.rs frontend/src/pages/purchase/purchase_order_detail.rs
git commit -m "feat: connect frontend to purchase order advanced APIs"
```

---

### Task 3: 完善前端应付账款与财务分析界面对接

**Files:**
- Modify: `frontend/src/services/ap_invoice_service.rs` (创建或修改)
- Modify: `frontend/src/pages/finance/ap_invoice_list.rs`

- [ ] **Step 1: 封装 AP Invoice API**

```rust
// 在 ap_invoice_service.rs 中添加
pub async fn approve_invoice(id: i32) -> Result<ApiResponse<()>, AppError> {
    request::post(&format!("/api/v1/erp/ap/invoices/{}/approve", id), &serde_json::Value::Null).await
}

pub async fn get_aging_analysis() -> Result<ApiResponse<serde_json::Value>, AppError> {
    request::get("/api/v1/erp/ap/invoices/aging").await
}
```

- [ ] **Step 2: 替换财务页面的占位符**
找到财务列表中硬编码的 `{"message": "统计报表功能开发中"}` 的相关 UI 触发点，替换为调用 `get_aging_analysis`。

- [ ] **Step 3: 运行检查**

```bash
cd frontend && cargo check
```
Expected: PASS

- [ ] **Step 4: Commit**

```bash
git add frontend/src/services/ap_invoice_service.rs frontend/src/pages/finance/ap_invoice_list.rs
git commit -m "feat: connect frontend to AP invoice and financial analysis APIs"
```