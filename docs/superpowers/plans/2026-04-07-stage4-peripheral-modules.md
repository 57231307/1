# 第四阶段：外围高级模块 Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** 完成 ERP 系统的最后一块功能拼图，实现资金调拨（Fund Transfer）、预算调整（Budget Adjustment）和供应商评估（Supplier Evaluation）的后端接口及前端简单集成。

**Architecture:** 
1. **Fund Transfer**: 基于 `fund_transfer_record` 表，实现账户间资金互转逻辑。
2. **Budget Adjustment**: 基于 `budget_adjustment` 表，实现预算金额调整审批。
3. **Supplier Evaluation**: 修复并暴露已存在但未挂载的 `supplier_evaluation_service` 接口（如 `get_indicators_list`）。
4. **Frontend**: 在现有对应的页面或新页面中绑定这些接口，解除死代码。

**Tech Stack:** Rust (Axum, SeaORM), WebAssembly (Yew)

---

### Task 1: 资金调拨 (Fund Transfer) 后端实现

**Files:**
- Create: `backend/src/models/dto/fund_dto.rs`
- Modify: `backend/src/models/dto/mod.rs`
- Modify: `backend/src/services/fund_management_service.rs`
- Modify: `backend/src/handlers/fund_management_handler.rs`
- Modify: `backend/src/routes/mod.rs`

- [ ] **Step 1: 创建 DTO**
创建 `backend/src/models/dto/fund_dto.rs`，并在 `dto/mod.rs` 导出 `pub mod fund_dto;`:
```rust
use serde::{Deserialize, Serialize};
use rust_decimal::Decimal;

#[derive(Debug, Deserialize)]
pub struct TransferFundRequest {
    pub from_account_id: i32,
    pub to_account_id: i32,
    pub amount: Decimal,
    pub fee: Option<Decimal>,
    pub reason: Option<String>,
}
```

- [ ] **Step 2: 实现转账 Service**
在 `fund_management_service.rs` 中引入 `fund_transfer_record` 和 `TransferFundRequest`，并实现 `transfer_fund`：
```rust
// 补充在 impl FundManagementService 中:
    pub async fn transfer_fund(&self, req: crate::models::dto::fund_dto::TransferFundRequest, user_id: i32) -> Result<crate::models::fund_transfer_record::Model, AppError> {
        let txn = self.db.begin().await.map_err(|e| AppError::DatabaseError(e.to_string()))?;
        
        // 1. 扣减转出账户
        let from_acc = crate::models::fund_management::Entity::find_by_id(req.from_account_id)
            .one(&txn).await.map_err(|e| AppError::DatabaseError(e.to_string()))?.ok_or_else(|| AppError::NotFound("From account not found".into()))?;
        if from_acc.balance < req.amount + req.fee.unwrap_or_default() {
            return Err(AppError::ValidationError("Insufficient balance".into()));
        }
        let mut from_active: crate::models::fund_management::ActiveModel = from_acc.into();
        from_active.balance = sea_orm::Set(from_active.balance.unwrap() - req.amount - req.fee.unwrap_or_default());
        from_active.update(&txn).await.map_err(|e| AppError::DatabaseError(e.to_string()))?;

        // 2. 增加转入账户
        let to_acc = crate::models::fund_management::Entity::find_by_id(req.to_account_id)
            .one(&txn).await.map_err(|e| AppError::DatabaseError(e.to_string()))?.ok_or_else(|| AppError::NotFound("To account not found".into()))?;
        let mut to_active: crate::models::fund_management::ActiveModel = to_acc.into();
        to_active.balance = sea_orm::Set(to_active.balance.unwrap() + req.amount);
        to_active.update(&txn).await.map_err(|e| AppError::DatabaseError(e.to_string()))?;

        // 3. 记录 Transfer
        let transfer_no = format!("TR{}", chrono::Local::now().format("%Y%m%d%H%M%S"));
        let record = crate::models::fund_transfer_record::ActiveModel {
            transfer_no: sea_orm::Set(transfer_no),
            from_account_id: sea_orm::Set(req.from_account_id),
            to_account_id: sea_orm::Set(req.to_account_id),
            amount: sea_orm::Set(req.amount),
            fee: sea_orm::Set(req.fee.unwrap_or_default()),
            status: sea_orm::Set("COMPLETED".to_string()),
            reason: sea_orm::Set(req.reason),
            created_by: sea_orm::Set(user_id),
            ..Default::default()
        }.insert(&txn).await.map_err(|e| AppError::DatabaseError(e.to_string()))?;

        txn.commit().await.map_err(|e| AppError::DatabaseError(e.to_string()))?;
        Ok(record)
    }
```

- [ ] **Step 3: 暴露 Handler 和 Route**
在 `fund_management_handler.rs` 中添加 `transfer` 方法：
```rust
use crate::models::dto::fund_dto::TransferFundRequest;

pub async fn transfer(
    axum::extract::State(state): axum::extract::State<crate::utils::app_state::AppState>,
    axum::Json(req): axum::Json<TransferFundRequest>,
) -> Result<axum::Json<crate::models::api_response::ApiResponse<serde_json::Value>>, AppError> {
    let service = FundManagementService::new(state.db.clone());
    let res = service.transfer_fund(req, 1).await?;
    Ok(axum::Json(crate::models::api_response::ApiResponse::success(serde_json::to_value(res)?)))
}
```
在 `routes/mod.rs` 中将其挂载到 `/api/v1/erp/fund-management/transfer`：
```rust
// 搜索 fund_management_routes 补充:
        .route("/transfer", post(fund_management_handler::transfer))
```

- [ ] **Step 4: 运行检查**
```bash
cd backend && cargo check --bin server
```
Expected: PASS

- [ ] **Step 5: Commit**
```bash
git add backend/
git commit -m "feat: implement fund transfer logic and API"
```

---

### Task 2: 预算调整 (Budget Adjustment) 后端实现

**Files:**
- Create: `backend/src/models/dto/budget_dto.rs`
- Modify: `backend/src/models/dto/mod.rs`
- Modify: `backend/src/services/budget_management_service.rs`
- Modify: `backend/src/handlers/budget_management_handler.rs`
- Modify: `backend/src/routes/mod.rs`

- [ ] **Step 1: 创建 DTO**
创建 `backend/src/models/dto/budget_dto.rs` 并在 `dto/mod.rs` 中导出：
```rust
use serde::Deserialize;
use rust_decimal::Decimal;

#[derive(Debug, Deserialize)]
pub struct AdjustBudgetRequest {
    pub item_id: i32,
    pub adjust_amount: Decimal,
    pub reason: Option<String>,
}
```

- [ ] **Step 2: 实现预算调整 Service**
在 `budget_management_service.rs` 中实现：
```rust
// 补充在 impl BudgetManagementService 中:
    pub async fn adjust_budget(&self, req: crate::models::dto::budget_dto::AdjustBudgetRequest, user_id: i32) -> Result<crate::models::budget_adjustment::Model, AppError> {
        let txn = self.db.begin().await.map_err(|e| AppError::DatabaseError(e.to_string()))?;
        
        let item = crate::models::budget_item::Entity::find_by_id(req.item_id)
            .one(&txn).await.map_err(|e| AppError::DatabaseError(e.to_string()))?.ok_or_else(|| AppError::NotFound("Budget item not found".into()))?;

        // 记录调整单
        let adjust_no = format!("BA{}", chrono::Local::now().format("%Y%m%d%H%M%S"));
        let adjustment = crate::models::budget_adjustment::ActiveModel {
            adjustment_no: sea_orm::Set(adjust_no),
            plan_id: sea_orm::Set(item.plan_id),
            item_id: sea_orm::Set(item.id),
            original_amount: sea_orm::Set(item.total_amount),
            adjust_amount: sea_orm::Set(req.adjust_amount),
            new_amount: sea_orm::Set(item.total_amount + req.adjust_amount),
            reason: sea_orm::Set(req.reason),
            status: sea_orm::Set("APPROVED".to_string()),
            created_by: sea_orm::Set(user_id),
            ..Default::default()
        }.insert(&txn).await.map_err(|e| AppError::DatabaseError(e.to_string()))?;

        // 简化：直接批准，更新 item 金额
        let mut item_active: crate::models::budget_item::ActiveModel = item.into();
        item_active.total_amount = sea_orm::Set(item_active.total_amount.unwrap() + req.adjust_amount);
        item_active.update(&txn).await.map_err(|e| AppError::DatabaseError(e.to_string()))?;

        txn.commit().await.map_err(|e| AppError::DatabaseError(e.to_string()))?;
        Ok(adjustment)
    }
```

- [ ] **Step 3: 暴露 Handler 和 Route**
在 `budget_management_handler.rs` 中添加 `adjust_budget`：
```rust
use crate::models::dto::budget_dto::AdjustBudgetRequest;

pub async fn adjust_budget(
    axum::extract::State(state): axum::extract::State<crate::utils::app_state::AppState>,
    axum::Json(req): axum::Json<AdjustBudgetRequest>,
) -> Result<axum::Json<crate::models::api_response::ApiResponse<serde_json::Value>>, AppError> {
    let service = BudgetManagementService::new(state.db.clone());
    let res = service.adjust_budget(req, 1).await?;
    Ok(axum::Json(crate::models::api_response::ApiResponse::success(serde_json::to_value(res)?)))
}
```
在 `routes/mod.rs` 的 `budget_management_routes` 补充：
```rust
        .route("/adjust", post(budget_management_handler::adjust_budget))
```

- [ ] **Step 4: 运行检查**
```bash
cd backend && cargo check --bin server
```
Expected: PASS

- [ ] **Step 5: Commit**
```bash
git add backend/
git commit -m "feat: implement budget adjustment logic and API"
```

---

### Task 3: 唤醒供应商评估沉睡 API

**Files:**
- Modify: `backend/src/routes/mod.rs`

- [ ] **Step 1: 将供应商评估指标等闲置 API 挂载**
在 `routes/mod.rs` 中的 `supplier_evaluation_routes` 补充路由：
```rust
// 寻找 supplier_evaluation_handler 的相关注册，补充未使用的 get_indicators_list, get_rankings, list_evaluation_records:
        .route("/indicators", get(supplier_evaluation_handler::list_indicators))
        .route("/rankings", get(supplier_evaluation_handler::get_rankings))
        .route("/records", get(supplier_evaluation_handler::list_evaluation_records))
```

- [ ] **Step 2: 运行检查**
```bash
cd backend && cargo check --bin server
```
Expected: PASS

- [ ] **Step 3: Commit**
```bash
git add backend/src/routes/mod.rs
git commit -m "feat: mount sleeping APIs for supplier evaluation"
```

---

### Task 4: 前端占位解除与基础对接

**Files:**
- Modify: `frontend/src/services/fund_management_service.rs` (创建或修改)
- Modify: `frontend/src/services/budget_management_service.rs` (创建或修改)
- Modify: `frontend/src/pages/fund_management.rs` (或对应列表)
- Modify: `frontend/src/pages/budget_management.rs`

- [ ] **Step 1: 前端 Service 封装**
在对应的 `service.rs` 中，利用 `request::post` 封装 `/transfer` 和 `/adjust` 接口调用。

- [ ] **Step 2: 解除前端占位**
在 Yew 组件中，寻找 `{"功能开发中..."}` 的占位符（如果存在），将其替换为一个简单的转账或调整按钮，并绑定刚封装的服务。

- [ ] **Step 3: 运行检查**
```bash
cd frontend && cargo check
```
Expected: PASS

- [ ] **Step 4: Commit**
```bash
git add frontend/
git commit -m "feat: connect frontend to fund transfer and budget adjustment APIs"
```