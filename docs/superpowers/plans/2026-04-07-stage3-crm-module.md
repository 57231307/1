# CRM 客户关系管理 Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** 基于已有的 `crm_lead` 和 `crm_opportunity` 数据库模型，从零开始完整实现 CRM 模块的后端接口（DTOs, Service, Handler）和前端页面（Service, Yew Component），打通线索跟进与商机转化的售前管理全链路。

**Architecture:** 
1. **后端 (Backend)**：在 `backend/src/models/dto/` 下创建 `crm_dto.rs`，在 `backend/src/services/` 下创建 `crm_service.rs` 处理业务逻辑（如线索转商机），在 `backend/src/handlers/` 下创建 `crm_handler.rs` 暴露 RESTful API，并挂载到 `routes/mod.rs`。
2. **前端 (Frontend)**：在 `frontend/src/services/` 下创建 `crm_service.rs` 封装 HTTP 请求，在 `frontend/src/pages/` 下创建 `crm_lead.rs` 和 `crm_opportunity.rs` 两个 Yew 页面组件，并在 `frontend/src/app/mod.rs` 中注册对应的路由。

**Tech Stack:** Rust (Axum, SeaORM), WebAssembly (Yew), reqwasm

---

### Task 1: 创建后端 CRM DTOs

**Files:**
- Create: `backend/src/models/dto/crm_dto.rs`
- Modify: `backend/src/models/dto/mod.rs`

- [ ] **Step 1: 创建 CRM DTO 文件**
创建 `backend/src/models/dto/crm_dto.rs`：

```rust
use serde::{Deserialize, Serialize};
use rust_decimal::Decimal;
use chrono::NaiveDate;

#[derive(Debug, Deserialize)]
pub struct CreateLeadRequest {
    pub name: String,
    pub customer_name: Option<String>,
    pub contact_person: Option<String>,
    pub contact_phone: Option<String>,
    pub email: Option<String>,
    pub address: Option<String>,
    pub source: String,
    pub remarks: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct CreateOpportunityRequest {
    pub name: String,
    pub customer_id: Option<i32>,
    pub lead_id: Option<i32>,
    pub amount: Decimal,
    pub expected_close_date: Option<NaiveDate>,
    pub stage: String,
    pub source: Option<String>,
    pub remarks: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct LeadQuery {
    pub status: Option<String>,
    pub page: Option<u64>,
    pub page_size: Option<u64>,
}

#[derive(Debug, Deserialize)]
pub struct OpportunityQuery {
    pub stage: Option<String>,
    pub page: Option<u64>,
    pub page_size: Option<u64>,
}
```

- [ ] **Step 2: 在 DTO 模块中导出**
打开 `backend/src/models/dto/mod.rs`，添加 `pub mod crm_dto;`

- [ ] **Step 3: 运行检查**
```bash
cd backend && cargo check --lib
```
Expected: PASS

- [ ] **Step 4: Commit**
```bash
git add backend/src/models/dto/
git commit -m "feat: add DTOs for CRM lead and opportunity"
```

---

### Task 2: 实现后端 CrmService 业务逻辑

**Files:**
- Create: `backend/src/services/crm_service.rs`
- Modify: `backend/src/services/mod.rs`

- [ ] **Step 1: 创建 CrmService 并实现线索 (Lead) 的增删改查**
创建 `backend/src/services/crm_service.rs`：

```rust
use sea_orm::*;
use std::sync::Arc;
use crate::models::{crm_lead, crm_opportunity};
use crate::models::dto::crm_dto::{CreateLeadRequest, CreateOpportunityRequest, LeadQuery, OpportunityQuery};
use crate::models::api_response::PaginatedResponse;
use crate::error::AppError;

pub struct CrmService {
    db: Arc<DatabaseConnection>,
}

impl CrmService {
    pub fn new(db: Arc<DatabaseConnection>) -> Self {
        Self { db }
    }

    // --- Lead Methods ---
    pub async fn create_lead(&self, req: CreateLeadRequest, user_id: i32) -> Result<crm_lead::Model, AppError> {
        let lead_no = format!("LD{}", chrono::Local::now().format("%Y%m%d%H%M%S"));
        
        let model = crm_lead::ActiveModel {
            lead_no: Set(lead_no),
            name: Set(req.name),
            customer_name: Set(req.customer_name),
            contact_person: Set(req.contact_person),
            contact_phone: Set(req.contact_phone),
            email: Set(req.email),
            address: Set(req.address),
            source: Set(req.source),
            status: Set("NEW".to_string()),
            remarks: Set(req.remarks),
            created_by: Set(user_id),
            ..Default::default()
        };

        model.insert(&*self.db).await.map_err(|e| AppError::DatabaseError(e.to_string()))
    }

    pub async fn list_leads(&self, query: LeadQuery) -> Result<PaginatedResponse<crm_lead::Model>, AppError> {
        let page = query.page.unwrap_or(1);
        let page_size = query.page_size.unwrap_or(10);
        
        let mut stmt = crm_lead::Entity::find().order_by_desc(crm_lead::Column::CreatedAt);
        
        if let Some(status) = query.status {
            stmt = stmt.filter(crm_lead::Column::Status.eq(status));
        }
        
        let paginator = stmt.paginate(&*self.db, page_size);
        let total = paginator.num_items().await.map_err(|e| AppError::DatabaseError(e.to_string()))?;
        let items = paginator.fetch_page(page - 1).await.map_err(|e| AppError::DatabaseError(e.to_string()))?;
        
        Ok(PaginatedResponse { list: items, total, page, page_size })
    }

    pub async fn update_lead_status(&self, id: i32, status: &str) -> Result<(), AppError> {
        let lead = crm_lead::Entity::find_by_id(id).one(&*self.db).await
            .map_err(|e| AppError::DatabaseError(e.to_string()))?
            .ok_or_else(|| AppError::NotFound("Lead not found".to_string()))?;

        let mut active: crm_lead::ActiveModel = lead.into();
        active.status = Set(status.to_string());
        active.updated_at = Set(chrono::Utc::now());
        active.update(&*self.db).await.map_err(|e| AppError::DatabaseError(e.to_string()))?;
        Ok(())
    }

    // --- Opportunity Methods ---
    pub async fn create_opportunity(&self, req: CreateOpportunityRequest, user_id: i32) -> Result<crm_opportunity::Model, AppError> {
        let opp_no = format!("OPP{}", chrono::Local::now().format("%Y%m%d%H%M%S"));
        
        let txn = self.db.begin().await.map_err(|e| AppError::DatabaseError(e.to_string()))?;

        let model = crm_opportunity::ActiveModel {
            opportunity_no: Set(opp_no),
            name: Set(req.name),
            customer_id: Set(req.customer_id),
            lead_id: Set(req.lead_id),
            amount: Set(req.amount),
            expected_close_date: Set(req.expected_close_date),
            stage: Set(req.stage),
            source: Set(req.source),
            remarks: Set(req.remarks),
            created_by: Set(user_id),
            ..Default::default()
        };

        let opp = model.insert(&txn).await.map_err(|e| AppError::DatabaseError(e.to_string()))?;

        // 如果是从线索转化的，更新线索状态
        if let Some(lead_id) = req.lead_id {
            if let Some(lead) = crm_lead::Entity::find_by_id(lead_id).one(&txn).await.map_err(|e| AppError::DatabaseError(e.to_string()))? {
                let mut active_lead: crm_lead::ActiveModel = lead.into();
                active_lead.status = Set("CONVERTED".to_string());
                active_lead.update(&txn).await.map_err(|e| AppError::DatabaseError(e.to_string()))?;
            }
        }

        txn.commit().await.map_err(|e| AppError::DatabaseError(e.to_string()))?;
        Ok(opp)
    }

    pub async fn list_opportunities(&self, query: OpportunityQuery) -> Result<PaginatedResponse<crm_opportunity::Model>, AppError> {
        let page = query.page.unwrap_or(1);
        let page_size = query.page_size.unwrap_or(10);
        
        let mut stmt = crm_opportunity::Entity::find().order_by_desc(crm_opportunity::Column::CreatedAt);
        
        if let Some(stage) = query.stage {
            stmt = stmt.filter(crm_opportunity::Column::Stage.eq(stage));
        }
        
        let paginator = stmt.paginate(&*self.db, page_size);
        let total = paginator.num_items().await.map_err(|e| AppError::DatabaseError(e.to_string()))?;
        let items = paginator.fetch_page(page - 1).await.map_err(|e| AppError::DatabaseError(e.to_string()))?;
        
        Ok(PaginatedResponse { list: items, total, page, page_size })
    }
}
```

- [ ] **Step 2: 导出服务**
打开 `backend/src/services/mod.rs`，添加 `pub mod crm_service;`

- [ ] **Step 3: 运行检查**
```bash
cd backend && cargo check --lib
```
Expected: PASS

- [ ] **Step 4: Commit**
```bash
git add backend/src/services/crm_service.rs backend/src/services/mod.rs
git commit -m "feat: implement CrmService with lead and opportunity management"
```

---

### Task 3: 暴露后端 CRM RESTful API 路由

**Files:**
- Create: `backend/src/handlers/crm_handler.rs`
- Modify: `backend/src/handlers/mod.rs`
- Modify: `backend/src/routes/mod.rs`

- [ ] **Step 1: 创建 crm_handler**
创建 `backend/src/handlers/crm_handler.rs`：

```rust
use axum::{extract::{State, Query, Path}, Json};
use crate::utils::app_state::AppState;
use crate::models::api_response::ApiResponse;
use crate::models::dto::crm_dto::{CreateLeadRequest, CreateOpportunityRequest, LeadQuery, OpportunityQuery};
use crate::services::crm_service::CrmService;
use crate::error::AppError;

pub async fn create_lead(
    State(state): State<AppState>,
    Json(req): Json<CreateLeadRequest>,
) -> Result<Json<ApiResponse<serde_json::Value>>, AppError> {
    let service = CrmService::new(state.db.clone());
    let res = service.create_lead(req, 1).await?; // TODO: Auth extraction
    Ok(Json(ApiResponse::success(serde_json::to_value(res)?)))
}

pub async fn list_leads(
    State(state): State<AppState>,
    Query(query): Query<LeadQuery>,
) -> Result<Json<ApiResponse<serde_json::Value>>, AppError> {
    let service = CrmService::new(state.db.clone());
    let res = service.list_leads(query).await?;
    Ok(Json(ApiResponse::success(serde_json::to_value(res)?)))
}

pub async fn update_lead_status(
    State(state): State<AppState>,
    Path(id): Path<i32>,
    Json(payload): Json<serde_json::Value>,
) -> Result<Json<ApiResponse<String>>, AppError> {
    let status = payload.get("status").and_then(|s| s.as_str()).unwrap_or("NEW");
    let service = CrmService::new(state.db.clone());
    service.update_lead_status(id, status).await?;
    Ok(Json(ApiResponse::success("Status updated".to_string())))
}

pub async fn create_opportunity(
    State(state): State<AppState>,
    Json(req): Json<CreateOpportunityRequest>,
) -> Result<Json<ApiResponse<serde_json::Value>>, AppError> {
    let service = CrmService::new(state.db.clone());
    let res = service.create_opportunity(req, 1).await?;
    Ok(Json(ApiResponse::success(serde_json::to_value(res)?)))
}

pub async fn list_opportunities(
    State(state): State<AppState>,
    Query(query): Query<OpportunityQuery>,
) -> Result<Json<ApiResponse<serde_json::Value>>, AppError> {
    let service = CrmService::new(state.db.clone());
    let res = service.list_opportunities(query).await?;
    Ok(Json(ApiResponse::success(serde_json::to_value(res)?)))
}
```

- [ ] **Step 2: 导出 Handler**
打开 `backend/src/handlers/mod.rs`，添加 `pub mod crm_handler;`

- [ ] **Step 3: 注册路由**
打开 `backend/src/routes/mod.rs`，在 `Router::new()` 中添加 crm 路由组：

```rust
// 搜索 erp 路由并在附近添加：
        .nest("/api/v1/erp/crm", Router::new()
            .route("/leads", post(crate::handlers::crm_handler::create_lead).get(crate::handlers::crm_handler::list_leads))
            .route("/leads/:id/status", put(crate::handlers::crm_handler::update_lead_status))
            .route("/opportunities", post(crate::handlers::crm_handler::create_opportunity).get(crate::handlers::crm_handler::list_opportunities))
        )
```

- [ ] **Step 4: 运行检查**
```bash
cd backend && cargo check --bin server
```
Expected: PASS 无编译错误。

- [ ] **Step 5: Commit**
```bash
git add backend/src/handlers/crm_handler.rs backend/src/handlers/mod.rs backend/src/routes/mod.rs
git commit -m "feat: expose CRM RESTful APIs for leads and opportunities"
```

---

### Task 4: 创建前端 CRM Service 与页面组件

**Files:**
- Create: `frontend/src/services/crm_service.rs`
- Create: `frontend/src/pages/crm_lead.rs`
- Create: `frontend/src/pages/crm_opportunity.rs`
- Modify: `frontend/src/services/mod.rs`
- Modify: `frontend/src/pages/mod.rs`
- Modify: `frontend/src/app/mod.rs`

- [ ] **Step 1: 封装前端 CRM API 请求**
创建 `frontend/src/services/crm_service.rs` (使用项目已有的 `request` 工具发起 HTTP 请求)。

- [ ] **Step 2: 编写 Yew 页面组件**
创建 `frontend/src/pages/crm_lead.rs` 和 `frontend/src/pages/crm_opportunity.rs`。
由于 Yew 组件样板代码较长，只需使用基础的 HTML Table 结构展示通过 `crm_service` 获取到的列表数据，并保留“新建”按钮的事件存根（Stub）。

- [ ] **Step 3: 在前端路由中注册**
在 `frontend/src/app/mod.rs` 的 `Route` 枚举中添加：
```rust
    #[at("/crm/leads")]
    CrmLeads,
    #[at("/crm/opportunities")]
    CrmOpportunities,
```
并在 `switch` 函数中路由到刚刚创建的 `CrmLeadPage` 和 `CrmOpportunityPage` 组件。

- [ ] **Step 4: 运行检查**
```bash
cd frontend && cargo check
```
Expected: PASS 无编译错误。

- [ ] **Step 5: Commit**
```bash
git add frontend/src/
git commit -m "feat: add frontend CRM pages and route integration"
```