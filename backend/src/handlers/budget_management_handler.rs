
use crate::middleware::auth_context::AuthContext;
use crate::models::{budget_execution, budget_management, budget_plan};
use crate::services::budget_management_service::{BudgetControlResponse, BudgetManagementService};
use crate::utils::app_state::AppState;
use crate::utils::error::AppError;
use crate::utils::ApiResponse;
use axum::{
    extract::{Path, Query, State},
    Json,
};
use chrono::NaiveDate;
use rust_decimal::Decimal;
use serde::Deserialize;
use tracing::info;

/// 预算科目查询参数 DTO
#[derive(Debug, Deserialize)]
pub struct BudgetItemQuery {
    pub item_type: Option<String>,
    pub status: Option<String>,
    pub page: Option<i64>,
    pub page_size: Option<i64>,
}

/// 创建预算科目请求 DTO
#[derive(Debug, Deserialize)]

pub struct CreateBudgetItemRequest {
    pub item_code: Option<String>,
    pub item_name: String,
    pub item_type: Option<String>,
    pub parent_id: Option<i32>,
    pub budget_year: Option<i32>,
    pub planned_amount: Decimal,
    pub remark: Option<String>,
}

/// 更新预算科目请求 DTO
#[derive(Debug, Deserialize)]

pub struct UpdateBudgetItemRequest {
    pub item_name: Option<String>,
    pub item_type: Option<String>,
    pub planned_amount: Option<Decimal>,
    pub status: Option<String>,
    pub remark: Option<String>,
}

/// 创建预算方案请求 DTO
#[derive(Debug, Deserialize)]

pub struct CreateBudgetPlanRequest {
    pub plan_no: Option<String>,
    pub plan_name: Option<String>,
    pub budget_year: Option<i32>,
    pub budget_type: Option<String>,
    pub department_id: Option<i32>,
    pub total_amount: Option<Decimal>,
    pub remark: Option<String>,
}

/// 预算执行请求 DTO
#[derive(Debug, Deserialize)]

pub struct BudgetExecuteRequest {
    pub plan_id: i32,
    pub actual_amount: Decimal,
    pub expense_type: String,
    pub expense_date: String,
    pub remark: Option<String>,
}

/// 预算审批请求 DTO
#[derive(Debug, Deserialize)]

pub struct BudgetApproveRequest {
    pub approval_comment: Option<String>,
}

/// 创建预算执行明细请求 DTO
#[derive(Debug, Deserialize)]

pub struct CreateBudgetExecutionRequest {
    pub execution_type: String,
    pub amount: Decimal,
    pub expense_type: Option<String>,
    pub expense_date: String,
    pub related_document_type: Option<String>,
    pub related_document_id: Option<i32>,
    pub remark: Option<String>,
}

/// 获取预算科目列表
pub async fn list_budget_items(
    Query(params): Query<BudgetItemQuery>,
    State(state): State<AppState>,
    auth: AuthContext,
) -> Result<Json<ApiResponse<Vec<budget_management::Model>>>, AppError> {
    info!("用户 {} 正在查询预算科目列表", auth.username);

    let service = BudgetManagementService::new(state.db.clone());
    let query_params = crate::services::budget_management_service::BudgetItemQueryParams {
        item_type: params.item_type,
        status: params.status,
        page: params.page.unwrap_or_default(),
        page_size: params.page_size.unwrap_or(10),
    };

    let (items, _total) = service.get_items_list(query_params).await?;
    info!("预算科目列表查询成功，共 {} 条记录", items.len());

    Ok(Json(ApiResponse::success(items)))
}

/// 创建预算科目
#[axum::debug_handler]
pub async fn create_budget_item(
    State(state): State<AppState>,
    auth: AuthContext,
    Json(req): Json<CreateBudgetItemRequest>,
) -> Result<Json<ApiResponse<budget_management::Model>>, AppError> {
    info!(
        "用户 {} 正在创建预算科目：{}",
        auth.username,
        req.item_code.as_deref().unwrap_or("自动生成")
    );

    let service = BudgetManagementService::new(state.db.clone());
    let item = service
        .create_item(
            crate::services::budget_management_service::CreateBudgetItemRequest {
                item_code: req.item_code,
                item_name: req.item_name,
                item_type: req.item_type,
                parent_id: req.parent_id,
                budget_year: req.budget_year,
                planned_amount: req.planned_amount,
                remark: req.remark,
            },
            auth.user_id,
        )
        .await?;

    info!("预算科目创建成功：{}", item.item_code);
    Ok(Json(ApiResponse::success(item)))
}

/// 获取预算科目详情
pub async fn get_budget_item(
    Path(id): Path<i32>,
    State(state): State<AppState>,
    auth: AuthContext,
) -> Result<Json<ApiResponse<budget_management::Model>>, AppError> {
    info!("用户 {} 正在查询预算科目详情：{}", auth.username, id);

    let service = BudgetManagementService::new(state.db.clone());
    let item = service.get_item_by_id(id).await?;

    info!("预算科目详情查询成功：{}", item.item_code);
    Ok(Json(ApiResponse::success(item)))
}

/// 更新预算科目
#[axum::debug_handler]
pub async fn update_budget_item(
    Path(id): Path<i32>,
    State(state): State<AppState>,
    auth: AuthContext,
    Json(req): Json<UpdateBudgetItemRequest>,
) -> Result<Json<ApiResponse<budget_management::Model>>, AppError> {
    info!("用户 {} 正在更新预算科目：{}", auth.username, id);

    let service = BudgetManagementService::new(state.db.clone());
    let item = service
        .update_item(
            id,
            crate::services::budget_management_service::UpdateBudgetItemRequest {
                item_name: req.item_name,
                item_type: req.item_type,
                planned_amount: req.planned_amount,
                status: req.status,
                remark: req.remark,
            },
            auth.user_id,
        )
        .await?;

    info!("预算科目更新成功：{}", id);
    Ok(Json(ApiResponse::success(item)))
}

/// 删除预算科目
pub async fn delete_budget_item(
    Path(id): Path<i32>,
    State(state): State<AppState>,
    auth: AuthContext,
) -> Result<Json<ApiResponse<String>>, AppError> {
    info!("用户 {} 正在删除预算科目：{}", auth.username, id);

    let service = BudgetManagementService::new(state.db.clone());
    service.delete_item(id, auth.user_id).await?;

    info!("预算科目删除成功：{}", id);
    Ok(Json(ApiResponse::success("删除成功".to_string())))
}

/// 获取预算方案列表
pub async fn list_plans(
    Query(params): Query<BudgetItemQuery>,
    State(state): State<AppState>,
    auth: AuthContext,
) -> Result<Json<ApiResponse<Vec<budget_plan::Model>>>, AppError> {
    info!("用户 {} 正在查询预算方案列表", auth.username);

    let service = BudgetManagementService::new(state.db.clone());
    let (plans, _total) = service
        .get_plans_list(
            None,
            None,
            params.page.unwrap_or_default(),
            params.page_size.unwrap_or(10),
        )
        .await?;

    info!("预算方案列表查询成功，共 {} 条记录", plans.len());
    Ok(Json(ApiResponse::success(plans)))
}

/// 创建预算方案
#[axum::debug_handler]
pub async fn create_plan(
    State(state): State<AppState>,
    auth: AuthContext,
    Json(req): Json<CreateBudgetPlanRequest>,
) -> Result<Json<ApiResponse<budget_plan::Model>>, AppError> {
    info!("用户 {} 正在创建预算方案：{:?}", auth.username, req.plan_no);

    let service = BudgetManagementService::new(state.db.clone());

    let plan = service
        .create_plan(
            crate::services::budget_management_service::CreateBudgetPlanRequest {
                plan_no: req.plan_no.unwrap_or_default(),
                plan_name: req.plan_name.unwrap_or_default(),
                budget_year: req.budget_year.unwrap_or_else(|| {
                    use chrono::Datelike;
                    chrono::Utc::now().naive_utc().year()
                }),
                budget_type: req.budget_type.unwrap_or_else(|| "年度预算".to_string()),
                // 部门 ID 缺失时返回 4xx 错误，避免脏 department_id=0 记录
                department_id: req.department_id.ok_or_else(|| {
                    AppError::validation("预算编制请求缺少部门ID")
                })?,
                total_amount: req.total_amount.unwrap_or(Decimal::ZERO),
                items: vec![],
                remark: req.remark,
            },
            auth.user_id,
        )
        .await?;

    info!("预算方案创建成功：ID={}", plan.id);
    Ok(Json(ApiResponse::success(plan)))
}

/// 获取预算方案详情
pub async fn get_plan(
    Path(id): Path<i32>,
    State(state): State<AppState>,
    auth: AuthContext,
) -> Result<Json<ApiResponse<budget_plan::Model>>, AppError> {
    info!("用户 {} 正在查询预算方案详情：{}", auth.username, id);

    let service = BudgetManagementService::new(state.db.clone());
    let plan = service.get_plan_by_id(id).await?;

    info!("预算方案详情查询成功：ID={}", plan.id);
    Ok(Json(ApiResponse::success(plan)))
}

/// 预算方案审批
pub async fn approve_plan(
    State(state): State<AppState>,
    auth: AuthContext,
    Path(id): Path<i32>,
    Json(req): Json<BudgetApproveRequest>,
) -> Result<Json<ApiResponse<String>>, AppError> {
    info!("用户 {} 正在审批预算方案：{}", auth.username, id);

    let service = BudgetManagementService::new(state.db.clone());
    service
        .approve_plan(id, auth.user_id, req.approval_comment)
        .await?;

    info!("预算方案审批通过：{}", id);
    Ok(Json(ApiResponse::success("审批通过".to_string())))
}

/// 预算方案执行
#[axum::debug_handler]
pub async fn execute_plan(
    Path(id): Path<i32>,
    State(state): State<AppState>,
    auth: AuthContext,
    Json(req): Json<BudgetExecuteRequest>,
) -> Result<Json<ApiResponse<String>>, AppError> {
    info!("用户 {} 正在执行预算方案：{}", auth.username, id);

    let expense_date = NaiveDate::parse_from_str(&req.expense_date, "%Y-%m-%d")
        .map_err(|e| AppError::validation(format!("日期格式错误：{}", e)))?;

    let service = BudgetManagementService::new(state.db.clone());
    service
        .execute_plan(
            crate::services::budget_management_service::BudgetExecuteRequest {
                plan_id: id,
                actual_amount: req.actual_amount,
                expense_type: req.expense_type,
                expense_date,
                remark: req.remark,
            },
            auth.user_id,
        )
        .await?;

    info!("预算方案执行成功：{}", id);
    Ok(Json(ApiResponse::success("执行成功".to_string())))
}

/// 获取预算控制情况
pub async fn get_control(
    Path(id): Path<i32>,
    State(state): State<AppState>,
    auth: AuthContext,
) -> Result<Json<ApiResponse<budget_plan::Model>>, AppError> {
    info!("用户 {} 正在查询预算控制情况：{}", auth.username, id);

    let service = BudgetManagementService::new(state.db.clone());
    let control = service.get_budget_control(id).await?;

    info!("预算控制情况查询成功：{}", id);
    Ok(Json(ApiResponse::success(control)))
}

/// 获取预算控制数据（含已下达、已执行、可用金额）
pub async fn get_budget_control_data(
    Path(id): Path<i32>,
    State(state): State<AppState>,
    auth: AuthContext,
) -> Result<Json<ApiResponse<BudgetControlResponse>>, AppError> {
    info!("用户 {} 正在查询预算控制数据：{}", auth.username, id);

    let service = BudgetManagementService::new(state.db.clone());
    let control_data = service.get_budget_control_data(id).await?;

    info!("预算控制数据查询成功：{}", id);
    Ok(Json(ApiResponse::success(control_data)))
}

/// 创建预算执行明细
#[axum::debug_handler]
pub async fn create_execution(
    Path(id): Path<i32>,
    State(state): State<AppState>,
    auth: AuthContext,
    Json(req): Json<CreateBudgetExecutionRequest>,
) -> Result<Json<ApiResponse<budget_execution::Model>>, AppError> {
    info!(
        "用户 {} 正在创建预算执行明细，方案ID：{}",
        auth.username, id
    );

    let expense_date = NaiveDate::parse_from_str(&req.expense_date, "%Y-%m-%d")
        .map_err(|e| AppError::validation(format!("日期格式错误：{}", e)))?;

    let service = BudgetManagementService::new(state.db.clone());
    let execution = service
        .create_execution(
            id,
            req.execution_type,
            req.amount,
            expense_date,
            req.expense_type,
            req.related_document_type,
            req.related_document_id,
            req.remark,
            auth.user_id,
        )
        .await?;

    info!("预算执行明细创建成功：ID={}", execution.id);
    Ok(Json(ApiResponse::success(execution)))
}

/// 获取预算执行明细列表
pub async fn get_plan_executions(
    Path(id): Path<i32>,
    State(state): State<AppState>,
    auth: AuthContext,
) -> Result<Json<ApiResponse<Vec<budget_execution::Model>>>, AppError> {
    info!(
        "用户 {} 正在查询预算执行明细列表，方案ID：{}",
        auth.username, id
    );

    let service = BudgetManagementService::new(state.db.clone());
    let executions = service.get_executions_by_plan(id).await?;

    info!("预算执行明细列表查询成功，共 {} 条记录", executions.len());
    Ok(Json(ApiResponse::success(executions)))
}

/// GET /api/v1/erp/budgets - 预算列表查询
pub async fn list_budgets(
    Query(params): Query<serde_json::Value>,
    State(state): State<AppState>,
    auth: AuthContext,
) -> Result<Json<ApiResponse<serde_json::Value>>, AppError> {
    info!("用户 {} 查询预算列表", auth.username);

    let service = BudgetManagementService::new(state.db.clone());

    let page = params.get("page").and_then(|v| v.as_i64()).unwrap_or(1);

    let page_size = params
        .get("page_size")
        .and_then(|v| v.as_i64())
        .unwrap_or(20);

    let query = crate::services::budget_management_service::BudgetItemQueryParams {
        item_type: params
            .get("item_type")
            .and_then(|v| v.as_str())
            .map(|s| s.to_string()),
        status: params
            .get("status")
            .and_then(|v| v.as_str())
            .map(|s| s.to_string()),
        page,
        page_size,
    };

    let (items, total) = service.get_items_list(query).await?;

    Ok(Json(ApiResponse::success(serde_json::json!({
        "items": items,
        "total": total,
        "page": page,
        "page_size": page_size,
    }))))
}

/// POST /api/v1/erp/budgets - 创建预算
pub async fn create_budget(
    State(state): State<AppState>,
    auth: AuthContext,
    Json(req): Json<serde_json::Value>,
) -> Result<Json<ApiResponse<serde_json::Value>>, AppError> {
    info!("用户 {} 创建预算", auth.username);

    let service = BudgetManagementService::new(state.db.clone());

    let item_code = req
        .get("item_code")
        .and_then(|v| v.as_str())
        .map(|s| s.to_string());

    let item_name = req
        .get("item_name")
        .and_then(|v| v.as_str())
        .unwrap_or("")
        .to_string();

    let item_type = req
        .get("item_type")
        .and_then(|v| v.as_str())
        .map(|s| s.to_string());

    let budget_year = req
        .get("budget_year")
        .and_then(|v| v.as_i64())
        .map(|y| y as i32);

    let planned_amount = req
        .get("planned_amount")
        .and_then(|v| v.as_f64())
        .map(|f| rust_decimal::Decimal::from_f64_retain(f).unwrap_or_default())
        .unwrap_or_default();

    let remark = req
        .get("remark")
        .and_then(|v| v.as_str())
        .map(|s| s.to_string());

    let create_req = crate::services::budget_management_service::CreateBudgetItemRequest {
        item_code,
        item_name,
        item_type,
        parent_id: None,
        budget_year,
        planned_amount,
        remark,
    };

    let item = service.create_item(create_req, auth.user_id).await?;

    Ok(Json(ApiResponse::success_with_message(
        serde_json::to_value(item)?,
        "预算创建成功",
    )))
}

/// PUT /api/v1/erp/budgets/:id - 更新预算
pub async fn update_budget(
    Path(id): Path<i32>,
    State(state): State<AppState>,
    auth: AuthContext,
    Json(req): Json<serde_json::Value>,
) -> Result<Json<ApiResponse<serde_json::Value>>, AppError> {
    info!("用户 {} 更新预算: ID={}", auth.username, id);

    let service = BudgetManagementService::new(state.db.clone());

    let item_name = req
        .get("item_name")
        .and_then(|v| v.as_str())
        .map(|s| s.to_string());

    let item_type = req
        .get("item_type")
        .and_then(|v| v.as_str())
        .map(|s| s.to_string());

    let planned_amount = req
        .get("planned_amount")
        .and_then(|v| v.as_f64())
        .map(|f| rust_decimal::Decimal::from_f64_retain(f).unwrap_or_default());

    let status = req
        .get("status")
        .and_then(|v| v.as_str())
        .map(|s| s.to_string());

    let remark = req
        .get("remark")
        .and_then(|v| v.as_str())
        .map(|s| s.to_string());

    let update_req = crate::services::budget_management_service::UpdateBudgetItemRequest {
        item_name,
        item_type,
        planned_amount,
        status,
        remark,
    };

    let item = service.update_item(id, update_req, auth.user_id).await?;

    Ok(Json(ApiResponse::success_with_message(
        serde_json::to_value(item)?,
        "预算更新成功",
    )))
}

/// DELETE /api/v1/erp/budgets/:id - 删除预算
pub async fn delete_budget(
    Path(id): Path<i32>,
    State(state): State<AppState>,
    auth: AuthContext,
) -> Result<Json<ApiResponse<()>>, AppError> {
    info!("用户 {} 删除预算: ID={}", auth.username, id);

    let service = BudgetManagementService::new(state.db.clone());
    service.delete_item(id, auth.user_id).await?;

    Ok(Json(ApiResponse::success_with_message((), "预算已删除")))
}

/// GET /api/v1/erp/budgets/:id - 获取预算详情
pub async fn get_budget(
    Path(id): Path<i32>,
    State(state): State<AppState>,
    auth: AuthContext,
) -> Result<Json<ApiResponse<serde_json::Value>>, AppError> {
    info!("用户 {} 获取预算详情: ID={}", auth.username, id);

    let service = BudgetManagementService::new(state.db.clone());
    let item = service.get_item_by_id(id).await?;

    Ok(Json(ApiResponse::success(serde_json::to_value(item)?)))
}

/// POST /api/v1/erp/budgets/:id/approve - 审批预算
pub async fn approve_budget(
    Path(id): Path<i32>,
    State(state): State<AppState>,
    auth: AuthContext,
    Json(req): Json<serde_json::Value>,
) -> Result<Json<ApiResponse<serde_json::Value>>, AppError> {
    info!("用户 {} 审批预算: ID={}", auth.username, id);

    let service = BudgetManagementService::new(state.db.clone());

    let opinion = req
        .get("opinion")
        .and_then(|v| v.as_str())
        .map(|s| s.to_string());

    service.approve_plan(id, auth.user_id, opinion).await?;

    Ok(Json(ApiResponse::success_with_message(
        serde_json::json!({"id": id}),
        "预算审批成功",
    )))
}

pub async fn adjust_budget(
    State(state): State<AppState>,
    auth: AuthContext,
    Json(req): Json<crate::models::dto::budget_dto::AdjustBudgetRequest>,
) -> Result<Json<ApiResponse<serde_json::Value>>, AppError> {
    info!("用户 {} 正在发起预算调整", auth.username);
    let service = BudgetManagementService::new(state.db.clone());
    let res = service.adjust_budget(req, auth.user_id).await?;
    Ok(Json(ApiResponse::success(
        serde_json::to_value(res).map_err(|e| AppError::internal(format!("序列化失败: {}", e)))?,
    )))
}
