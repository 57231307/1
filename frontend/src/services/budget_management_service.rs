use crate::models::budget_management::{
    BudgetApproveRequest, BudgetControl, BudgetExecuteRequest, BudgetItem, BudgetItemListResponse,
    BudgetItemQuery, BudgetPlan, BudgetPlanListResponse, CreateBudgetItemRequest,
    CreateBudgetPlanRequest, UpdateBudgetItemRequest,
};
use crate::services::api::ApiService;
use crate::services::crud_service::CrudService;

/// 预算管理服务
pub struct BudgetManagementService;

impl CrudService for BudgetManagementService {
    type Model = BudgetItem;
    type ListResponse = BudgetItemListResponse;
    type CreateRequest = CreateBudgetItemRequest;
    type UpdateRequest = UpdateBudgetItemRequest;

    fn base_path() -> &'static str {
        "/budgets/items"
    }
}


impl BudgetManagementService {
    /// 获取预算科目列表
    pub async fn list_items(query: BudgetItemQuery) -> Result<BudgetItemListResponse, String> {
        <Self as CrudService>::list_with_query(&query).await
    }

    /// 获取预算科目详情
    pub async fn get_item(id: i32) -> Result<BudgetItem, String> {
        <Self as CrudService>::get(id).await
    }

    /// 创建预算科目
    pub async fn create_item(req: CreateBudgetItemRequest) -> Result<BudgetItem, String> {
        <Self as CrudService>::create(req).await
    }

    /// 更新预算科目
    pub async fn update_item(id: i32, req: UpdateBudgetItemRequest) -> Result<BudgetItem, String> {
        <Self as CrudService>::update(id, req).await
    }

    /// 删除预算科目
    pub async fn delete_item(id: i32) -> Result<(), String> {
        <Self as CrudService>::delete(id).await
    }

    /// 获取预算方案列表
    pub async fn list_plans(query: BudgetItemQuery) -> Result<BudgetPlanListResponse, String> {
        let mut url = String::from("/budgets/plans?");
        if let Some(page) = query.page {
            url.push_str(&format!("page={}&", page));
        }
        if let Some(page_size) = query.page_size {
            url.push_str(&format!("page_size={}", page_size));
        }
        ApiService::get::<BudgetPlanListResponse>(&url).await
    }

    /// 获取预算方案详情
    pub async fn get_plan(id: i32) -> Result<BudgetPlan, String> {
        ApiService::get::<BudgetPlan>(&format!("/budgets/plans/{}", id)).await
    }

    /// 创建预算方案
    pub async fn create_plan(req: CreateBudgetPlanRequest) -> Result<BudgetPlan, String> {
        let payload = serde_json::to_value(&req).map_err(|e| e.to_string())?;
        ApiService::post("/budgets/plans", &payload).await
    }

    /// 审批预算方案
    pub async fn approve_plan(id: i32, req: BudgetApproveRequest) -> Result<String, String> {
        let payload = serde_json::to_value(&req).map_err(|e| e.to_string())?;
        ApiService::post(&format!("/budgets/plans/{}/approve", id), &payload).await
    }

    /// 执行预算方案
    pub async fn execute_plan(id: i32, req: BudgetExecuteRequest) -> Result<String, String> {
        let payload = serde_json::to_value(&req).map_err(|e| e.to_string())?;
        ApiService::post(&format!("/budgets/plans/{}/execute", id), &payload).await
    }

    /// 获取预算控制情况
    pub async fn get_control(id: i32) -> Result<BudgetControl, String> {
        ApiService::get::<BudgetControl>(&format!("/budgets/control/{}", id)).await
    }

    /// 预算调整
    pub async fn adjust_budget(item_id: i32, adjust_amount: String, reason: Option<String>) -> Result<serde_json::Value, String> {
        #[derive(Debug, Clone, serde::Serialize)]
        struct AdjustRequest {
            item_id: i32,
            adjust_amount: String,
            reason: Option<String>,
        }
        let req = AdjustRequest { item_id, adjust_amount, reason };
        let payload = serde_json::to_value(&req).map_err(|e| e.to_string())?;
        ApiService::post("/budgets/adjust", &payload).await
    }
}