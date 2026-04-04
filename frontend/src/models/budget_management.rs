//! 预算管理模型
//!
//! 预算管理相关的数据结构

use serde::{Deserialize, Serialize};

/// 预算科目数据模型
#[derive(Debug, Clone, Deserialize, Serialize)]
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
#[derive(Debug, Clone, Deserialize, Serialize)]
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
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct BudgetControl {
    pub id: i32,
    pub plan_id: i32,
    pub planned_amount: String,
    pub actual_amount: String,
    pub remaining_amount: String,
    pub execution_rate: String,
}

/// 预算科目查询参数
#[derive(Debug, Clone, Serialize)]
pub struct BudgetItemQuery {
    pub item_type: Option<String>,
    pub status: Option<String>,
    pub page: Option<i64>,
    pub page_size: Option<i64>,
}

/// 预算科目列表响应
#[derive(Debug, Clone, Deserialize)]
pub struct BudgetItemListResponse {
    pub data: Vec<BudgetItem>,
    pub total: u64,
}

/// 预算方案列表响应
#[derive(Debug, Clone, Deserialize)]
pub struct BudgetPlanListResponse {
    pub data: Vec<BudgetPlan>,
    pub total: u64,
}

/// 创建预算科目请求
#[derive(Debug, Clone, Serialize)]
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
#[derive(Debug, Clone, Serialize)]
pub struct UpdateBudgetItemRequest {
    pub item_name: Option<String>,
    pub item_type: Option<String>,
    pub planned_amount: Option<String>,
    pub status: Option<String>,
    pub remark: Option<String>,
}

/// 创建预算方案请求
#[derive(Debug, Clone, Serialize)]
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
#[derive(Debug, Clone, Serialize)]
pub struct BudgetExecuteRequest {
    pub actual_amount: String,
    pub expense_type: String,
    pub expense_date: String,
    pub remark: Option<String>,
}

/// 预算审批请求
#[derive(Debug, Clone, Serialize)]
pub struct BudgetApproveRequest {
    pub approval_comment: Option<String>,
}
