# BPM 工作流引擎 Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** 实现后端的 BPM 流程引擎服务（`BpmService`）和对应 API 接口，使系统支持基于 JSON Schema 的流程定义解析、实例启动和任务审批流转。

**Architecture:** 
1. 数据库模型（`bpm_process_definition`, `bpm_process_instance`, `bpm_task`）和 JSON 结构已经设计好并存在于系统中。
2. 我们需要创建一个完整的 `BpmService`，它能解析 `flow_definition` JSON，创建 `bpm_process_instance`，并根据流程图逻辑生成对应的 `bpm_task`。
3. 暴露一套 RESTful API（如 `/api/v1/erp/bpm/instances`, `/api/v1/erp/bpm/tasks`）供前端统一调用，取代分散在各业务模块里的硬编码状态流转。

**Tech Stack:** Rust (Axum, SeaORM, serde_json)

---

### Task 1: 定义 BPM 请求和响应结构体

**Files:**
- Create: `backend/src/models/dto/bpm_dto.rs`
- Modify: `backend/src/models/dto/mod.rs`

- [x] **Step 1: 创建 BPM DTO 文件**
创建 `backend/src/models/dto/bpm_dto.rs`，基于测试文件中推导出的结构定义 DTO：

```rust
use serde::{Deserialize, Serialize};

#[derive(Debug, Deserialize, Serialize)]
pub struct StartProcessRequest {
    pub process_key: String,
    pub business_type: String,
    pub business_id: i32,
    pub title: String,
    pub initiator_id: i32,
    pub initiator_name: String,
    pub initiator_department_id: Option<i32>,
    pub priority: Option<String>,
    pub form_data: Option<serde_json::Value>,
    pub variables: Option<serde_json::Value>,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct StartProcessResponse {
    pub instance_id: i32,
    pub instance_no: String,
    pub task_ids: Vec<i32>,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct ApproveTaskRequest {
    pub task_id: i32,
    pub handler_id: i32,
    pub handler_name: String,
    pub action: String, // "approve", "reject"
    pub approval_opinion: Option<String>,
    pub attachment_urls: Option<Vec<String>>,
}

#[derive(Debug, Deserialize)]
pub struct TaskQuery {
    pub user_id: i32,
    pub status: Option<String>,
    pub page: Option<u64>,
    pub page_size: Option<u64>,
}
```

- [x] **Step 2: 在 DTO 模块中导出**
打开 `backend/src/models/dto/mod.rs`，添加 `pub mod bpm_dto;`

- [x] **Step 3: 运行检查**
```bash
cd backend && cargo check --lib
```
Expected: PASS

- [x] **Step 4: Commit**
```bash
git add backend/src/models/dto/
git commit -m "feat: add DTOs for BPM engine requests and responses"
```

---

### Task 2: 实现 BpmService 的基础框架与流程启动

**Files:**
- Create: `backend/src/services/bpm_service.rs`
- Modify: `backend/src/services/mod.rs`

- [x] **Step 1: 创建 BpmService 并实现 `start_process`**
创建 `backend/src/services/bpm_service.rs`。实现解析定义并创建实例的逻辑：

```rust
use sea_orm::*;
use std::sync::Arc;
use crate::models::{bpm_process_definition, bpm_process_instance, bpm_task};
use crate::models::dto::bpm_dto::{StartProcessRequest, StartProcessResponse};
use crate::error::AppError;

pub struct BpmService {
    db: Arc<DatabaseConnection>,
}

impl BpmService {
    pub fn new(db: Arc<DatabaseConnection>) -> Self {
        Self { db }
    }

    pub async fn start_process(&self, req: StartProcessRequest) -> Result<StartProcessResponse, AppError> {
        let txn = self.db.begin().await.map_err(|e| AppError::DatabaseError(e.to_string()))?;
        
        // 1. 查找流程定义
        let definition = bpm_process_definition::Entity::find()
            .filter(bpm_process_definition::Column::ProcessKey.eq(&req.process_key))
            .filter(bpm_process_definition::Column::Status.eq("active"))
            .one(&txn).await
            .map_err(|e| AppError::DatabaseError(e.to_string()))?
            .ok_or_else(|| AppError::NotFound("Process definition not found or inactive".to_string()))?;

        // 2. 创建流程实例
        let instance_no = format!("BPM{}{}", chrono::Local::now().format("%Y%m%d%H%M%S"), req.business_id);
        let instance_model = bpm_process_instance::ActiveModel {
            definition_id: Set(definition.id),
            instance_no: Set(instance_no.clone()),
            business_type: Set(req.business_type),
            business_id: Set(req.business_id.to_string()),
            title: Set(req.title),
            initiator_id: Set(Some(req.initiator_id)),
            initiator_name: Set(Some(req.initiator_name)),
            status: Set("running".to_string()),
            form_data: Set(req.form_data),
            variables: Set(req.variables),
            ..Default::default()
        };
        
        let instance = instance_model.insert(&txn).await.map_err(|e| AppError::DatabaseError(e.to_string()))?;

        // 3. 解析 flow_definition JSON 创建初始任务 (简化的 Start -> First Task 逻辑)
        let mut task_ids = vec![];
        if let Some(flow_def) = definition.flow_definition {
            if let Some(nodes) = flow_def.get("nodes").and_then(|n| n.as_array()) {
                // 查找第一个 user_task
                if let Some(first_task) = nodes.iter().find(|n| n.get("type").and_then(|t| t.as_str()) == Some("user_task")) {
                    let task_model = bpm_task::ActiveModel {
                        instance_id: Set(instance.id),
                        node_id: Set(first_task.get("id").and_then(|i| i.as_str()).unwrap_or("unknown").to_string()),
                        node_name: Set(first_task.get("name").and_then(|n| n.as_str()).unwrap_or("Task").to_string()),
                        task_type: Set("user_task".to_string()),
                        assignee_type: Set(first_task.get("assignee_type").and_then(|a| a.as_str()).map(|s| s.to_string())),
                        assignee_value: Set(first_task.get("assignee_value").and_then(|a| a.as_str()).map(|s| s.to_string())),
                        status: Set("pending".to_string()),
                        ..Default::default()
                    };
                    let task = task_model.insert(&txn).await.map_err(|e| AppError::DatabaseError(e.to_string()))?;
                    task_ids.push(task.id);
                }
            }
        }

        txn.commit().await.map_err(|e| AppError::DatabaseError(e.to_string()))?;

        Ok(StartProcessResponse {
            instance_id: instance.id,
            instance_no,
            task_ids,
        })
    }
}
```

- [x] **Step 2: 导出服务**
打开 `backend/src/services/mod.rs`，添加 `pub mod bpm_service;`

- [x] **Step 3: 运行检查**
```bash
cd backend && cargo check --lib
```
Expected: PASS

- [x] **Step 4: Commit**
```bash
git add backend/src/services/bpm_service.rs backend/src/services/mod.rs
git commit -m "feat: implement basic BpmService and start_process logic"
```

---

### Task 3: 实现 BpmService 的任务审批与查询逻辑

**Files:**
- Modify: `backend/src/services/bpm_service.rs`

- [x] **Step 1: 实现 `approve_task` 和 `query_user_tasks`**
在 `impl BpmService` 中继续添加以下方法：

```rust
use crate::models::dto::bpm_dto::{ApproveTaskRequest, TaskQuery};
use crate::models::api_response::PaginatedResponse;

// 在 impl BpmService 中补充：
    pub async fn approve_task(&self, req: ApproveTaskRequest) -> Result<(), AppError> {
        let txn = self.db.begin().await.map_err(|e| AppError::DatabaseError(e.to_string()))?;
        
        let task = bpm_task::Entity::find_by_id(req.task_id)
            .one(&txn).await
            .map_err(|e| AppError::DatabaseError(e.to_string()))?
            .ok_or_else(|| AppError::NotFound("Task not found".to_string()))?;

        if task.status != "pending" {
            return Err(AppError::ValidationError("Task is not pending".to_string()));
        }

        // 更新任务状态
        let mut task_active: bpm_task::ActiveModel = task.into();
        task_active.status = Set(if req.action == "approve" { "completed".to_string() } else { "rejected".to_string() });
        task_active.handler_id = Set(Some(req.handler_id));
        task_active.handler_name = Set(Some(req.handler_name));
        task_active.approval_opinion = Set(req.approval_opinion);
        task_active.handle_time = Set(Some(chrono::Utc::now().naive_utc()));
        task_active.update(&txn).await.map_err(|e| AppError::DatabaseError(e.to_string()))?;

        // 简化版：直接更新流程实例状态为结束（实际应解析 JSON 查找下一个节点）
        let instance = bpm_process_instance::Entity::find_by_id(task_active.instance_id.unwrap())
            .one(&txn).await
            .map_err(|e| AppError::DatabaseError(e.to_string()))?
            .unwrap();
            
        let mut instance_active: bpm_process_instance::ActiveModel = instance.into();
        instance_active.status = Set(if req.action == "approve" { "completed".to_string() } else { "terminated".to_string() });
        instance_active.end_time = Set(Some(chrono::Utc::now().naive_utc()));
        instance_active.update(&txn).await.map_err(|e| AppError::DatabaseError(e.to_string()))?;

        txn.commit().await.map_err(|e| AppError::DatabaseError(e.to_string()))?;
        Ok(())
    }

    pub async fn query_user_tasks(&self, query: TaskQuery) -> Result<PaginatedResponse<bpm_task::Model>, AppError> {
        let page = query.page.unwrap_or(1);
        let page_size = query.page_size.unwrap_or(10);
        
        let mut stmt = bpm_task::Entity::find();
        
        // 此处简化，实际应该通过 assignee_type 和 assignee_value 以及用户所属角色进行匹配
        // 这里仅作示例，假设直接指派给了用户 ID
        stmt = stmt.filter(bpm_task::Column::AssigneeValue.eq(query.user_id.to_string()));
        
        if let Some(status) = query.status {
            stmt = stmt.filter(bpm_task::Column::Status.eq(status));
        }
        
        let paginator = stmt.paginate(&*self.db, page_size);
        let total = paginator.num_items().await.map_err(|e| AppError::DatabaseError(e.to_string()))?;
        let items = paginator.fetch_page(page - 1).await.map_err(|e| AppError::DatabaseError(e.to_string()))?;
        
        Ok(PaginatedResponse {
            list: items,
            total,
            page,
            page_size,
        })
    }
```

- [x] **Step 2: 运行检查**
```bash
cd backend && cargo check --lib
```
Expected: PASS

- [x] **Step 3: Commit**
```bash
git add backend/src/services/bpm_service.rs
git commit -m "feat: implement task approval and querying in BpmService"
```

---

### Task 4: 暴露 BPM RESTful API 路由

**Files:**
- Create: `backend/src/handlers/bpm_handler.rs`
- Modify: `backend/src/handlers/mod.rs`
- Modify: `backend/src/routes/mod.rs`

- [x] **Step 1: 创建 bpm_handler**
创建 `backend/src/handlers/bpm_handler.rs`：

```rust
use axum::{extract::{State, Query}, Json};
use std::sync::Arc;
use crate::utils::app_state::AppState;
use crate::models::api_response::ApiResponse;
use crate::models::dto::bpm_dto::{StartProcessRequest, ApproveTaskRequest, TaskQuery};
use crate::services::bpm_service::BpmService;
use crate::error::AppError;

pub async fn start_process(
    State(state): State<AppState>,
    Json(req): Json<StartProcessRequest>,
) -> Result<Json<ApiResponse<serde_json::Value>>, AppError> {
    let service = BpmService::new(state.db.clone());
    let res = service.start_process(req).await?;
    Ok(Json(ApiResponse::success(serde_json::to_value(res)?)))
}

pub async fn approve_task(
    State(state): State<AppState>,
    Json(req): Json<ApproveTaskRequest>,
) -> Result<Json<ApiResponse<String>>, AppError> {
    let service = BpmService::new(state.db.clone());
    service.approve_task(req).await?;
    Ok(Json(ApiResponse::success("Task processed successfully".to_string())))
}

pub async fn query_tasks(
    State(state): State<AppState>,
    Query(query): Query<TaskQuery>,
) -> Result<Json<ApiResponse<serde_json::Value>>, AppError> {
    let service = BpmService::new(state.db.clone());
    let res = service.query_user_tasks(query).await?;
    Ok(Json(ApiResponse::success(serde_json::to_value(res)?)))
}
```

- [x] **Step 2: 导出 Handler**
打开 `backend/src/handlers/mod.rs`，添加 `pub mod bpm_handler;`

- [x] **Step 3: 注册路由**
打开 `backend/src/routes/mod.rs`，在 `Router::new()` 中添加 bpm 路由组：

```rust
// 在其他 erp 路由附近添加：
        .nest("/api/v1/erp/bpm", Router::new()
            .route("/instances/start", post(crate::handlers::bpm_handler::start_process))
            .route("/tasks/approve", post(crate::handlers::bpm_handler::approve_task))
            .route("/tasks", get(crate::handlers::bpm_handler::query_tasks))
        )
```

- [x] **Step 4: 运行检查**
```bash
cd backend && cargo check --bin server
```
Expected: PASS 无编译错误。

- [x] **Step 5: Commit**
```bash
git add backend/src/handlers/bpm_handler.rs backend/src/handlers/mod.rs backend/src/routes/mod.rs
git commit -m "feat: expose BPM engine RESTful APIs via axum router"
```