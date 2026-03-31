use crate::services::api::ApiService;

/// 预算科目数据模型
#[derive(Debug, Clone, serde::Deserialize, serde::Serialize)]
pub struct BudgetItem {
    pub id: i32,
    pub item_code: String,
    pub item_name: String,
    pub item_type: String,
    pub parent_id: Option<i32>,
    pub budget_year: i32,
    pub planned_amount: String,
    pub status: Option<String>,
    pub remark: Option<String>,
    pub created_at: String,
    pub updated_at: String,
}

/// 预算方案数据模型
#[derive(Debug, Clone, serde::Deserialize, serde::Serialize)]
pub struct BudgetPlan {
    pub id: i32,
    pub plan_no: String,
    pub plan_name: String,
    pub budget_year: i32,
    pub department_id: i32,
    pub total_amount: String,
    pub start_date: String,
    pub end_date: String,
    pub status: Option<String>,
    pub remark: Option<String>,
    pub created_at: String,
    pub updated_at: String,
}

/// 预算控制数据模型
#[derive(Debug, Clone, serde::Deserialize, serde::Serialize)]
pub struct BudgetControl {
    pub id: i32,
    pub plan_id: i32,
    pub planned_amount: String,
    pub actual_amount: String,
    pub remaining_amount: String,
    pub execution_rate: String,
}

/// 预算科目查询参数
#[derive(Debug, Clone, serde::Serialize)]
pub struct BudgetItemQuery {
    pub item_type: Option<String>,
    pub status: Option<String>,
    pub page: Option<i64>,
    pub page_size: Option<i64>,
}

/// 预算科目列表响应
#[derive(Debug, Clone, serde::Deserialize)]
pub struct BudgetItemListResponse {
    pub data: Vec<BudgetItem>,
    pub total: u64,
}

/// 预算方案列表响应
#[derive(Debug, Clone, serde::Deserialize)]
pub struct BudgetPlanListResponse {
    pub data: Vec<BudgetPlan>,
    pub total: u64,
}

/// 创建预算科目请求
#[derive(Debug, Clone, serde::Serialize)]
pub struct CreateBudgetItemRequest {
    pub item_code: String,
    pub item_name: String,
    pub item_type: String,
    pub parent_id: Option<i32>,
    pub budget_year: i32,
    pub planned_amount: String,
    pub remark: Option<String>,
}

/// 更新预算科目请求
#[derive(Debug, Clone, serde::Serialize)]
pub struct UpdateBudgetItemRequest {
    pub item_name: Option<String>,
    pub item_type: Option<String>,
    pub planned_amount: Option<String>,
    pub status: Option<String>,
    pub remark: Option<String>,
}

/// 创建预算方案请求
#[derive(Debug, Clone, serde::Serialize)]
pub struct CreateBudgetPlanRequest {
    pub plan_no: String,
    pub plan_name: String,
    pub budget_year: i32,
    pub department_id: i32,
    pub total_amount: String,
    pub start_date: String,
    pub end_date: String,
    pub remark: Option<String>,
}

/// 预算执行请求
#[derive(Debug, Clone, serde::Serialize)]
pub struct BudgetExecuteRequest {
    pub actual_amount: String,
    pub expense_type: String,
    pub expense_date: String,
    pub remark: Option<String>,
}

/// 预算审批请求
#[derive(Debug, Clone, serde::Serialize)]
pub struct BudgetApproveRequest {
    pub approval_comment: Option<String>,
}

/// 预算管理服务
pub struct BudgetManagementService;

impl BudgetManagementService {
    /// 获取预算科目列表
    pub async fn list_items(query: BudgetItemQuery) -> Result<BudgetItemListResponse, String> {
        let mut url = String::from("/budget-items?");
        if let Some(item_type) = &query.item_type {
            url.push_str(&format!("item_type={}&", item_type));
        }
        if let Some(status) = &query.status {
            url.push_str(&format!("status={}&", status));
        }
        if let Some(page) = query.page {
            url.push_str(&format!("page={}&", page));
        }
        if let Some(page_size) = query.page_size {
            url.push_str(&format!("page_size={}", page_size));
        }
        ApiService::get::<BudgetItemListResponse>(&url).await
    }

    /// 获取预算科目详情
    pub async fn get_item(id: i32) -> Result<BudgetItem, String> {
        ApiService::get::<BudgetItem>(&format!("/budget-items/{}", id)).await
    }

    /// 创建预算科目
    pub async fn create_item(req: CreateBudgetItemRequest) -> Result<BudgetItem, String> {
        let payload = serde_json::to_value(&req).map_err(|e| e.to_string())?;
        ApiService::post("/budget-items", &payload).await
    }

    /// 更新预算科目
    pub async fn update_item(id: i32, req: UpdateBudgetItemRequest) -> Result<BudgetItem, String> {
        let payload = serde_json::to_value(&req).map_err(|e| e.to_string())?;
        ApiService::put(&format!("/budget-items/{}", id), &payload).await
    }

    /// 删除预算科目
    pub async fn delete_item(id: i32) -> Result<(), String> {
        ApiService::delete(&format!("/budget-items/{}", id)).await
    }

    /// 获取预算方案列表
    pub async fn list_plans(query: BudgetItemQuery) -> Result<BudgetPlanListResponse, String> {
        let mut url = String::from("/budget-plans?");
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
        ApiService::get::<BudgetPlan>(&format!("/budget-plans/{}", id)).await
    }

    /// 创建预算方案
    pub async fn create_plan(req: CreateBudgetPlanRequest) -> Result<BudgetPlan, String> {
        let payload = serde_json::to_value(&req).map_err(|e| e.to_string())?;
        ApiService::post("/budget-plans", &payload).await
    }

    /// 审批预算方案
    pub async fn approve_plan(id: i32, req: BudgetApproveRequest) -> Result<String, String> {
        let payload = serde_json::to_value(&req).map_err(|e| e.to_string())?;
        ApiService::post(&format!("/budget-plans/{}/approve", id), &payload).await
    }

    /// 执行预算方案
    pub async fn execute_plan(id: i32, req: BudgetExecuteRequest) -> Result<String, String> {
        let payload = serde_json::to_value(&req).map_err(|e| e.to_string())?;
        ApiService::post(&format!("/budget-plans/{}/execute", id), &payload).await
    }

    /// 获取预算控制情况
    pub async fn get_control(id: i32) -> Result<BudgetControl, String> {
        ApiService::get::<BudgetControl>(&format!("/budget-control/{}", id)).await
    }
}