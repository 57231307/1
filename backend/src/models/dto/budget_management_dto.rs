//! 预算管理服务的数据传输对象（DTO）
//!
//! 从 services/budget_management_service.rs 迁移而来的纯数据结构

use chrono::NaiveDate;
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};

/// 预算明细期间输入（按月 '2026-01' / 季 '2026-Q1' / 年度聚合 '2026-FY' 分解的金额行）
#[derive(Debug, Clone, Deserialize)]
pub struct BudgetItemPeriodInput {
    /// 期间标识
    pub period: String,
    /// 该期间计划金额
    pub planned_amount: Decimal,
}

/// 预算控制响应数据结构
#[derive(Debug, Clone, Serialize)]
pub struct BudgetControlResponse {
    /// 预算方案ID
    pub plan_id: i32,
    /// 预算总额
    pub total_amount: Decimal,
    /// 已下达金额
    pub issued_amount: Decimal,
    /// 已执行金额
    pub executed_amount: Decimal,
    /// 可用金额
    pub available_amount: Decimal,
}

/// 预算科目查询参数
#[derive(Debug, Clone, Default)]
pub struct BudgetItemQueryParams {
    pub item_type: Option<String>,
    pub status: Option<String>,
    /// 按所属预算方案筛选（并入 plan 主线后新增）
    pub plan_id: Option<i32>,
    pub page: i64,
    pub page_size: i64,
}

/// 预算科目详情出参：明细主体（含 plan_id）+ 按月/季分解的期间数组
#[derive(Debug, Clone, Serialize)]
pub struct BudgetItemWithPeriods {
    /// 预算明细主体（budget_items 全列，键名 snake_case，含 NOT NULL 的 plan_id）
    #[serde(flatten)]
    pub item: crate::models::budget_management::Model,
    /// 期间分解明细（budget_item_periods，键名 snake_case）
    pub periods: Vec<crate::models::budget_item_periods::Model>,
}

/// 创建预算科目请求
/// v11 批次 145 P1-8：移除 dead_code 标注，扩展字段已接入 budget_management 模型；（对应 budget_items 表的 budget_year / planned_amount / remark 字段）
#[derive(Debug, Clone)]
pub struct CreateBudgetItemRequest {
    pub item_code: Option<String>,
    pub item_name: String,
    pub item_type: Option<String>,
    pub parent_id: Option<i32>,
    /// 所属预算方案 ID（NOT NULL，并入 plan 主线）
    pub plan_id: i32,
    pub budget_year: Option<i32>,
    pub planned_amount: Decimal,
    pub remark: Option<String>,
    /// P2-14：预算科目-会计科目映射
    pub account_subject_id: Option<i32>,
    /// 期间分解（为空时服务层按年度生成单条 'FY' 合计行）
    pub periods: Vec<BudgetItemPeriodInput>,
}

/// 更新预算科目请求（v11 批次 145 P1-8：移除 dead_code 标注，扩展字段已接入 budget_management 模型）
#[derive(Debug, Clone)]
pub struct UpdateBudgetItemRequest {
    pub item_name: Option<String>,
    pub item_type: Option<String>,
    pub planned_amount: Option<Decimal>,
    pub status: Option<String>,
    pub remark: Option<String>,
    /// P2-14：预算科目-会计科目映射
    pub account_subject_id: Option<Option<i32>>,
    /// 期间分解：Some(..) 时整体替换该明细的期间行并重新聚合校验；None 时保持原期间不变
    pub periods: Option<Vec<BudgetItemPeriodInput>>,
}

/// 创建预算方案请求
/// v11 批次 145 P1-8：移除 items 字段（handler 始终传 vec![]，无真实业务数据流，；且引入 budget_plan_items 表需新增模型/迁移/handler 接口，超出本批次范围）。；预算方案与预算科目的关联通过 budget_management.budget_year + budget_plan.budget_year 隐式关联。
#[derive(Debug, Clone)]
pub struct CreateBudgetPlanRequest {
    pub plan_no: String,
    pub plan_name: String,
    pub budget_year: i32,
    pub budget_type: String,
    pub department_id: i32,
    pub total_amount: Decimal,
    pub remark: Option<String>,
}

/// 预算执行请求
/// v11 批次 145 P1-8：移除 dead_code 标注，execute_plan 现已真实接入 create_execution；（actual_amount 作为 amount，expense_type/expense_date/remark 透传）
#[derive(Debug, Clone)]
pub struct BudgetExecuteRequest {
    pub plan_id: i32,
    pub actual_amount: Decimal,
    pub expense_type: String,
    pub expense_date: NaiveDate,
    pub remark: Option<String>,
}

/// 创建预算执行明细参数对象（批次 329 v10 复审 P3 修复：引入参数对象消除 too_many_arguments 警告）
#[derive(Debug)]
pub struct CreateBudgetExecutionParams {
    /// 预算方案 ID
    pub plan_id: i32,
    /// 预算明细科目 ID（可空：方案级下达/调整不绑定具体明细时为 None）
    pub item_id: Option<i32>,
    /// 执行类型（下达/调整/使用）
    pub execution_type: String,
    /// 金额
    pub amount: Decimal,
    /// 费用日期
    pub expense_date: NaiveDate,
    /// 费用类型
    pub expense_type: Option<String>,
    /// 关联单据类型
    pub related_document_type: Option<String>,
    /// 关联单据 ID
    pub related_document_id: Option<i32>,
    /// 备注
    pub remark: Option<String>,
}

/// V15 P1 17.7-D2：预算差异分析条目
#[derive(Debug, Clone, Serialize)]
pub struct BudgetVarianceItem {
    pub plan_id: i32,
    pub plan_no: String,
    pub plan_name: String,
    pub department_id: Option<i32>,
    pub budget_year: i32,
    pub budget_type: String,
    /// 预算总额
    pub total_amount: Decimal,
    /// 已下达金额
    pub issued_amount: Decimal,
    /// 已执行金额
    pub executed_amount: Decimal,
    /// 差异 = 已下达 - 已执行（正数=未执行完，负数=超支）
    pub variance: Decimal,
    /// 差异率 = 差异 / 已下达 × 100%
    pub variance_rate: Option<Decimal>,
    /// 状态：normal / near_limit / over_budget / no_issued
    pub status: String,
}

/// V15 P1 17.7-D4：预算预警
#[derive(Debug, Clone, Serialize)]
pub struct BudgetWarning {
    pub plan_id: i32,
    pub plan_no: String,
    pub plan_name: String,
    pub department_id: Option<i32>,
    pub budget_year: i32,
    /// 已下达金额
    pub issued_amount: Decimal,
    /// 已执行金额
    pub executed_amount: Decimal,
    /// 可用金额 = 已下达 - 已执行
    pub available_amount: Decimal,
    /// 执行率 = 已执行 / 已下达 × 100%
    pub execution_rate: Decimal,
    /// 预警级别：yellow（≥80%）/ red（≥100%）
    pub warning_level: String,
}

/// 预算考核汇总
#[derive(Debug, Clone, Serialize)]
pub struct BudgetAssessmentSummary {
    /// 总预算金额
    pub total_budget: Decimal,
    /// 已执行金额
    pub total_executed: Decimal,
    /// 执行率
    pub execution_rate: Decimal,
    /// 预算方案数量
    pub plan_count: i64,
    /// 预警数量
    pub warning_count: i64,
}
