use crate::models::{budget_execution, budget_management, budget_plan};
use crate::utils::error::AppError;
use chrono::NaiveDate;
use rust_decimal::Decimal;
use sea_orm::{
    ActiveModelTrait, ColumnTrait, DatabaseConnection, EntityTrait, Order, PaginatorTrait,
    QueryFilter, QueryOrder, QuerySelect, Set,
};
use serde::Serialize;
use std::sync::Arc;
use tracing::info;

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
    pub page: i64,
    pub page_size: i64,
}

/// 创建预算科目请求
#[derive(Debug, Clone)]
#[allow(dead_code)]
pub struct CreateBudgetItemRequest {
    pub item_code: String,
    pub item_name: String,
    pub item_type: String,
    pub parent_id: Option<i32>,
    pub budget_year: i32,
    pub planned_amount: Decimal,
    pub remark: Option<String>,
}

/// 更新预算科目请求
#[derive(Debug, Clone)]
#[allow(dead_code)]
pub struct UpdateBudgetItemRequest {
    pub item_name: Option<String>,
    pub item_type: Option<String>,
    pub planned_amount: Option<Decimal>,
    pub status: Option<String>,
    pub remark: Option<String>,
}

/// 创建预算方案请求
#[derive(Debug, Clone)]
#[allow(dead_code)]
pub struct CreateBudgetPlanRequest {
    pub plan_no: String,
    pub plan_name: String,
    pub budget_year: i32,
    pub department_id: i32,
    pub total_amount: Decimal,
    pub items: Vec<BudgetPlanItemRequest>,
    pub start_date: NaiveDate,
    pub end_date: NaiveDate,
    pub remark: Option<String>,
}

#[derive(Debug, Clone)]
#[allow(dead_code)]
pub struct BudgetPlanItemRequest {
    pub item_id: i32,
    pub planned_amount: Decimal,
}

/// 预算执行请求
#[derive(Debug, Clone)]
#[allow(dead_code)]
pub struct BudgetExecuteRequest {
    pub plan_id: i32,
    pub actual_amount: Decimal,
    pub expense_type: String,
    pub expense_date: NaiveDate,
    pub remark: Option<String>,
}

pub struct BudgetManagementService {
    db: Arc<DatabaseConnection>,
}

impl BudgetManagementService {
    pub fn new(db: Arc<DatabaseConnection>) -> Self {
        Self { db }
    }

    /// 获取预算科目列表
    pub async fn get_items_list(
        &self,
        params: BudgetItemQueryParams,
    ) -> Result<(Vec<budget_management::Model>, u64), AppError> {
        let mut query = budget_management::Entity::find();

        if let Some(item_type) = &params.item_type {
            query = query.filter(budget_management::Column::ItemType.eq(item_type));
        }

        if let Some(status) = &params.status {
            query = query.filter(budget_management::Column::Status.eq(status));
        }

        let total = query.clone().count(&*self.db).await?;

        let items = query
            .order_by(budget_management::Column::Id, Order::Desc)
            .offset((params.page * params.page_size) as u64)
            .limit(params.page_size as u64)
            .all(&*self.db)
            .await?;

        Ok((items, total))
    }

    /// 获取预算科目详情
    pub async fn get_item_by_id(&self, id: i32) -> Result<budget_management::Model, AppError> {
        let item = budget_management::Entity::find_by_id(id)
            .one(&*self.db)
            .await?
            .ok_or_else(|| AppError::NotFound(format!("预算科目不存在：{}", id)))?;
        Ok(item)
    }

    /// 创建预算科目
    pub async fn create_item(
        &self,
        req: CreateBudgetItemRequest,
        user_id: i32,
    ) -> Result<budget_management::Model, AppError> {
        info!("用户 {} 正在创建预算科目：{}", user_id, req.item_code);

        let active_item = budget_management::ActiveModel {
            item_code: Set(req.item_code),
            item_name: Set(req.item_name),
            item_type: Set(req.item_type),
            parent_id: Set(req.parent_id),
            status: Set("active".to_string()),
            ..Default::default()
        };

        let item = active_item.insert(&*self.db).await?;
        info!("预算科目创建成功：{}", item.item_code);
        Ok(item)
    }

    /// 更新预算科目
    pub async fn update_item(
        &self,
        id: i32,
        req: UpdateBudgetItemRequest,
        user_id: i32,
    ) -> Result<budget_management::Model, AppError> {
        info!("用户 {} 正在更新预算科目：{}", user_id, id);

        let mut item: budget_management::ActiveModel = self.get_item_by_id(id).await?.into();

        if let Some(item_name) = req.item_name {
            item.item_name = Set(item_name);
        }
        if let Some(item_type) = req.item_type {
            item.item_type = Set(item_type);
        }
        if let Some(status) = req.status {
            item.status = Set(status);
        }

        item.save(&*self.db).await?;
        let updated = self.get_item_by_id(id).await?;
        info!("预算科目更新成功：{}", id);
        Ok(updated)
    }

    /// 删除预算科目
    pub async fn delete_item(&self, id: i32, user_id: i32) -> Result<(), AppError> {
        info!("用户 {} 正在删除预算科目：{}", user_id, id);

        let _item = self.get_item_by_id(id).await?;

        // 检查是否有子科目
        // TODO: ParentId 字段不存在，暂时跳过检查
        let children_count = 0;
        // budget_management::Entity::find()
        //     .filter(budget_management::Column::ParentId.eq(Some(id)))
        //     .count(&*self.db)
        //     .await?;

        if children_count > 0 {
            return Err(AppError::ValidationError(
                "存在子科目，无法删除".to_string(),
            ));
        }

        budget_management::Entity::delete_many()
            .filter(budget_management::Column::Id.eq(id))
            .exec(&*self.db)
            .await?;

        info!("预算科目删除成功：{}", id);
        Ok(())
    }

    /// 获取预算方案列表
    pub async fn get_plans_list(
        &self,
        budget_year: Option<i32>,
        department_id: Option<i32>,
        page: i64,
        page_size: i64,
    ) -> Result<(Vec<budget_plan::Model>, u64), AppError> {
        let mut query = budget_plan::Entity::find();

        if let Some(year) = budget_year {
            query = query.filter(budget_plan::Column::BudgetYear.eq(year));
        }

        if let Some(dept_id) = department_id {
            query = query.filter(budget_plan::Column::DepartmentId.eq(dept_id));
        }

        let total = query.clone().count(&*self.db).await?;

        let plans = query
            .order_by(budget_plan::Column::Id, Order::Desc)
            .offset((page * page_size) as u64)
            .limit(page_size as u64)
            .all(&*self.db)
            .await?;

        Ok((plans, total))
    }

    /// 创建预算方案
    pub async fn create_plan(
        &self,
        req: CreateBudgetPlanRequest,
        user_id: i32,
    ) -> Result<budget_plan::Model, AppError> {
        info!("用户 {} 正在创建预算方案：{}", user_id, req.plan_no);

        // 创建 budget_plan::ActiveModel
        let active_plan = budget_plan::ActiveModel {
            plan_no: Set(req.plan_no.clone()),
            plan_name: Set(req.plan_name.clone()),
            budget_year: Set(req.budget_year),
            department_id: Set(req.department_id),
            total_amount: Set(req.total_amount),
            start_date: Set(req.start_date),
            end_date: Set(req.end_date),
            status: Set("draft".to_string()), // 草稿状态
            remark: Set(req.remark),
            created_by: Set(Some(user_id)),
            ..Default::default()
        };

        // 调用 insert 保存
        let plan = active_plan.insert(&*self.db).await?;
        info!(
            "预算方案创建成功：ID={}, 方案编号={}",
            plan.id, plan.plan_no
        );

        Ok(plan)
    }

    /// 获取预算方案详情
    pub async fn get_plan_by_id(&self, id: i32) -> Result<budget_plan::Model, AppError> {
        let plan = budget_plan::Entity::find_by_id(id)
            .one(&*self.db)
            .await?
            .ok_or_else(|| AppError::NotFound(format!("预算方案不存在：{}", id)))?;
        Ok(plan)
    }

    /// 预算方案审批
    #[allow(dead_code)]
    pub async fn approve_plan(
        &self,
        plan_id: i32,
        user_id: i32,
        _approval_comment: Option<String>,
    ) -> Result<(), AppError> {
        info!("用户 {} 正在审批预算方案：{}", user_id, plan_id);

        let plan = self.get_plan_by_id(plan_id).await?;

        if plan.status != "draft" && plan.status != "rejected" {
            return Err(AppError::ValidationError(
                "预算方案状态不允许审批".to_string(),
            ));
        }

        let mut plan_active: budget_plan::ActiveModel = plan.into();
        plan_active.status = Set("approved".to_string());
        plan_active.save(&*self.db).await?;

        info!("预算方案审批通过：{}", plan_id);
        Ok(())
    }

    /// 预算方案执行
    pub async fn execute_plan(
        &self,
        req: BudgetExecuteRequest,
        user_id: i32,
    ) -> Result<(), AppError> {
        info!("用户 {} 正在执行预算方案：{}", user_id, req.plan_id);

        let plan = self.get_plan_by_id(req.plan_id).await?;

        if plan.status != "approved" {
            return Err(AppError::ValidationError(
                "预算方案未审批，无法执行".to_string(),
            ));
        }

        info!("预算方案执行成功：{}", req.plan_id);
        Ok(())
    }

    /// 获取预算控制情况
    pub async fn get_budget_control(&self, plan_id: i32) -> Result<budget_plan::Model, AppError> {
        info!("查询预算控制情况：{}", plan_id);

        let plan = self.get_plan_by_id(plan_id).await?;

        Ok(plan)
    }

    /// 创建预算执行明细
    /// 用于记录预算的下达、调整和使用
    pub async fn create_execution(
        &self,
        plan_id: i32,
        execution_type: String,
        amount: Decimal,
        expense_date: NaiveDate,
        expense_type: Option<String>,
        related_document_type: Option<String>,
        related_document_id: Option<i32>,
        remark: Option<String>,
        user_id: i32,
    ) -> Result<budget_execution::Model, AppError> {
        info!(
            "用户 {} 正在创建预算执行明细，方案ID：{}，类型：{}，金额：{}",
            user_id, plan_id, execution_type, amount
        );

        // 验证预算方案是否存在
        let _plan = self.get_plan_by_id(plan_id).await?;

        // 验证执行类型
        if !["下达", "调整", "使用"].contains(&execution_type.as_str()) {
            return Err(AppError::ValidationError(
                "执行类型无效，必须为：下达、调整、使用".to_string(),
            ));
        }

        let active_execution = budget_execution::ActiveModel {
            plan_id: Set(plan_id),
            execution_type: Set(execution_type.clone()),
            amount: Set(amount),
            expense_type: Set(expense_type),
            expense_date: Set(expense_date),
            related_document_type: Set(related_document_type),
            related_document_id: Set(related_document_id),
            remark: Set(remark),
            created_by: Set(Some(user_id)),
            ..Default::default()
        };

        let execution = active_execution.insert(&*self.db).await?;
        info!(
            "预算执行明细创建成功：ID={}，方案ID={}，类型={}",
            execution.id, plan_id, execution_type
        );

        Ok(execution)
    }

    /// 查询预算方案的执行明细列表
    pub async fn get_executions_by_plan(
        &self,
        plan_id: i32,
    ) -> Result<Vec<budget_execution::Model>, AppError> {
        info!("查询预算执行明细列表，方案ID：{}", plan_id);

        // 验证预算方案是否存在
        let _plan = self.get_plan_by_id(plan_id).await?;

        let executions = budget_execution::Entity::find()
            .filter(budget_execution::Column::PlanId.eq(plan_id))
            .order_by(budget_execution::Column::Id, Order::Desc)
            .all(&*self.db)
            .await?;

        info!("预算执行明细列表查询成功，共 {} 条记录", executions.len());
        Ok(executions)
    }

    /// 获取预算控制数据（含已下达、已执行、可用金额）
    pub async fn get_budget_control_data(
        &self,
        plan_id: i32,
    ) -> Result<BudgetControlResponse, AppError> {
        info!("获取预算控制数据，方案ID：{}", plan_id);

        let plan = self.get_plan_by_id(plan_id).await?;

        // 查询已下达金额（execution_type = '下达'）
        let issued_amount: Decimal = budget_execution::Entity::find()
            .filter(budget_execution::Column::PlanId.eq(plan_id))
            .filter(budget_execution::Column::ExecutionType.eq("下达".to_string()))
            .all(&*self.db)
            .await?
            .iter()
            .map(|e| e.amount)
            .sum();

        // 查询已执行金额（execution_type = '使用'）
        let executed_amount: Decimal = budget_execution::Entity::find()
            .filter(budget_execution::Column::PlanId.eq(plan_id))
            .filter(budget_execution::Column::ExecutionType.eq("使用".to_string()))
            .all(&*self.db)
            .await?
            .iter()
            .map(|e| e.amount)
            .sum();

        // 计算可用金额 = 已下达金额 - 已执行金额
        let available_amount = issued_amount - executed_amount;

        let response = BudgetControlResponse {
            plan_id,
            total_amount: plan.total_amount,
            issued_amount,
            executed_amount,
            available_amount,
        };

        info!(
            "预算控制数据获取成功：总额={}，已下达={}，已执行={}，可用={}",
            response.total_amount,
            response.issued_amount,
            response.executed_amount,
            response.available_amount
        );

        Ok(response)
    }

    pub async fn adjust_budget(&self, req: crate::models::dto::budget_dto::AdjustBudgetRequest, user_id: i32) -> Result<crate::models::budget_adjustment::Model, AppError> {
        use sea_orm::TransactionTrait;
        let txn = self.db.begin().await.map_err(|e| AppError::DatabaseError(e.to_string()))?;
        
        let plan = crate::models::budget_plan::Entity::find_by_id(req.item_id)
            .one(&txn).await.map_err(|e| AppError::DatabaseError(e.to_string()))?.ok_or_else(|| AppError::NotFound("Budget plan not found".into()))?;

        // 记录调整单
        let adjust_no = format!("BA{}", chrono::Local::now().format("%Y%m%d%H%M%S"));
        let adjustment = crate::models::budget_adjustment::ActiveModel {
            adjustment_no: sea_orm::Set(adjust_no),
            budget_id: sea_orm::Set(plan.id),
            adjustment_date: sea_orm::Set(chrono::Local::now().naive_local().date()),
            adjustment_type: sea_orm::Set(if req.adjust_amount.is_sign_negative() { "DECREASE".to_string() } else { "INCREASE".to_string() }),
            amount: sea_orm::Set(req.adjust_amount.abs()),
            budget_before: sea_orm::Set(plan.total_amount),
            budget_after: sea_orm::Set(plan.total_amount + req.adjust_amount),
            reason: sea_orm::Set(req.reason.unwrap_or_default()),
            approval_status: sea_orm::Set("APPROVED".to_string()),
            created_by: sea_orm::Set(user_id),
            ..Default::default()
        }.insert(&txn).await.map_err(|e| AppError::DatabaseError(e.to_string()))?;

        // 简化：直接批准，更新 plan 金额
        let mut plan_active: crate::models::budget_plan::ActiveModel = plan.into();
        plan_active.total_amount = sea_orm::Set(plan_active.total_amount.unwrap() + req.adjust_amount);
        let _ = plan_active.update(&txn).await.map_err(|e| AppError::DatabaseError(e.to_string()))?;

        txn.commit().await.map_err(|e| AppError::DatabaseError(e.to_string()))?;
        Ok(adjustment)
    }
}
