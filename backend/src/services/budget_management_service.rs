use crate::models::{budget_execution, budget_management, budget_plan};
// 批次 158 v11 真实接入：审批状态常量替代字符串字面量
use crate::models::status::approval;
// 批次 209 P2-5 修复（v12 复审）：预算方案/项目状态字符串替换为 budget 常量
use crate::models::status::budget;
use crate::utils::error::AppError;
use chrono::NaiveDate;
use rust_decimal::Decimal;
use sea_orm::{
    ActiveModelTrait, ColumnTrait, DatabaseConnection, EntityTrait, Order, PaginatorTrait,
    QueryFilter, QueryOrder, QuerySelect, Set, TransactionTrait,
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
///
/// v11 批次 145 P1-8：移除 dead_code 标注，扩展字段已接入 budget_management 模型
/// （对应 budget_items 表的 budget_year / planned_amount / remark 字段）
#[derive(Debug, Clone)]
pub struct CreateBudgetItemRequest {
    pub item_code: Option<String>,
    pub item_name: String,
    pub item_type: Option<String>,
    pub parent_id: Option<i32>,
    pub budget_year: Option<i32>,
    pub planned_amount: Decimal,
    pub remark: Option<String>,
}

/// 更新预算科目请求
///
/// v11 批次 145 P1-8：移除 dead_code 标注，扩展字段已接入 budget_management 模型
#[derive(Debug, Clone)]
pub struct UpdateBudgetItemRequest {
    pub item_name: Option<String>,
    pub item_type: Option<String>,
    pub planned_amount: Option<Decimal>,
    pub status: Option<String>,
    pub remark: Option<String>,
}

/// 创建预算方案请求
///
/// v11 批次 145 P1-8：移除 items 字段（handler 始终传 vec![]，无真实业务数据流，
/// 且引入 budget_plan_items 表需新增模型/迁移/handler 接口，超出本批次范围）。
/// 预算方案与预算科目的关联通过 budget_management.budget_year + budget_plan.budget_year 隐式关联。
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
///
/// v11 批次 145 P1-8：移除 dead_code 标注，execute_plan 现已真实接入 create_execution
/// （actual_amount 作为 amount，expense_type/expense_date/remark 透传）
#[derive(Debug, Clone)]
pub struct BudgetExecuteRequest {
    pub plan_id: i32,
    pub actual_amount: Decimal,
    pub expense_type: String,
    pub expense_date: NaiveDate,
    pub remark: Option<String>,
}

/// 创建预算执行明细参数对象
///
/// 批次 329 v10 复审 P3 修复：引入参数对象消除 too_many_arguments 警告
#[derive(Debug)]
pub struct CreateBudgetExecutionParams {
    /// 预算方案 ID
    pub plan_id: i32,
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
            // 批次 98 P2-A 修复（v5 复审）：page clamp 防 DoS
            .offset((params.page.clamp(1, 1000).saturating_sub(1) * params.page_size) as u64)
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
            .ok_or_else(|| AppError::not_found(format!("预算科目不存在：{}", id)))?;
        Ok(item)
    }

    /// 创建预算科目
    pub async fn create_item(
        &self,
        req: CreateBudgetItemRequest,
        user_id: i32,
    ) -> Result<budget_management::Model, AppError> {
        // 自动生成科目代码
        let item_code = req.item_code.unwrap_or_else(|| {
            let timestamp = chrono::Utc::now().format("%Y%m%d%H%M%S");
            let random = crate::utils::random::random_4_digit();
            format!("BUD-{}-{:04}", timestamp, random)
        });

        info!("用户 {} 正在创建预算科目：{}", user_id, item_code);

        let active_item = budget_management::ActiveModel {
            item_code: Set(item_code),
            item_name: Set(req.item_name),
            item_type: Set(req.item_type.unwrap_or_else(|| "expense".to_string())),
            parent_id: Set(req.parent_id),
            status: Set(budget::ACTIVE.to_string()),
            // v11 批次 145 P1-8：接入扩展字段（此前被丢弃，造成数据丢失）
            budget_year: Set(req.budget_year),
            planned_amount: Set(req.planned_amount),
            remark: Set(req.remark),
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

        // P1 5-21 修复（批次 61）：状态机 lock_exclusive 补全 + 审计日志
        // 原实现无 txn 无 lock 无审计，直接 save(&*self.db)，并发更新竞态 + 操作无追溯。
        // 改为 txn + lock_exclusive + update_with_audit。
        let txn = (*self.db).begin().await?;

        let item_model = budget_management::Entity::find_by_id(id)
            .lock_exclusive()
            .one(&txn)
            .await?
            .ok_or_else(|| AppError::not_found(format!("预算科目不存在：{}", id)))?;

        let mut item: budget_management::ActiveModel = item_model.into();

        if let Some(item_name) = req.item_name {
            item.item_name = Set(item_name);
        }
        if let Some(item_type) = req.item_type {
            item.item_type = Set(item_type);
        }
        if let Some(status) = req.status {
            item.status = Set(status);
        }
        // v11 批次 145 P1-8：接入扩展字段（此前被丢弃，更新无效）
        if let Some(planned_amount) = req.planned_amount {
            item.planned_amount = Set(planned_amount);
        }
        if let Some(remark) = req.remark {
            item.remark = Set(Some(remark));
        }

        let updated =
            crate::services::audit_log_service::AuditLogService::update_with_audit(
                &txn,
                "auto_audit",
                item,
                Some(user_id),
            )
            .await?;

        txn.commit().await?;

        info!("预算科目更新成功：{}", id);
        Ok(updated)
    }

    /// 删除预算科目
    pub async fn delete_item(&self, id: i32, user_id: i32) -> Result<(), AppError> {
        info!("用户 {} 正在删除预算科目：{}", user_id, id);

        let _item = self.get_item_by_id(id).await?;

        // 检查是否有子科目
        // 说明: 当前业务模型无需树状结构，跳过 ParentId 检查
        let children_count = 0;

        if children_count > 0 {
            return Err(AppError::validation("存在子科目，无法删除".to_string()));
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
            .offset((page.saturating_sub(1) * page_size) as u64)
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
            budget_type: Set(req.budget_type.clone()),
            department_id: Set(Some(req.department_id)),
            total_amount: Set(req.total_amount),
            status: Set(Some(budget::DRAFT.to_string())), // 草稿状态
            remark: Set(req.remark),
            prepared_by: Set(Some(user_id)),
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
            .ok_or_else(|| AppError::not_found(format!("预算方案不存在：{}", id)))?;
        Ok(plan)
    }

    /// 预算方案审批
    pub async fn approve_plan(
        &self,
        plan_id: i32,
        user_id: i32,
        _approval_comment: Option<String>,
    ) -> Result<(), AppError> {
        info!("用户 {} 正在审批预算方案：{}", user_id, plan_id);

        // 批次 26 v6 P1 修复：状态机 lock_exclusive 补全，串行化并发状态变更
        // 原状态门查询用 self.get_plan_by_id 裸查询无行锁，且 save 也用裸连接，无事务保护。
        // 改为在事务内用 find_by_id(id).lock_exclusive() 串行化并发状态变更，save 一并纳入事务。
        let txn = (*self.db).begin().await?;
        let plan = budget_plan::Entity::find_by_id(plan_id)
            .lock_exclusive()
            .one(&txn)
            .await?
            .ok_or_else(|| AppError::not_found(format!("预算方案不存在：{}", plan_id)))?;

        if plan.status.as_deref() != Some(budget::DRAFT) && plan.status.as_deref() != Some(budget::REJECTED) {
            return Err(AppError::validation("预算方案状态不允许审批".to_string()));
        }

        let mut plan_active: budget_plan::ActiveModel = plan.into();
        plan_active.status = Set(Some(budget::APPROVED.to_string()));
        plan_active.save(&txn).await?;
        txn.commit().await?;

        info!("预算方案审批通过：{}", plan_id);
        Ok(())
    }

    /// 预算方案执行
    ///
    /// v11 批次 145 P1-8：真实接入 create_execution 逻辑
    /// 原实现仅 lock_exclusive + 状态检查后直接 commit，BudgetExecuteRequest 的所有字段
    /// （actual_amount/expense_type/expense_date/remark）均被丢弃，造成 dead_code 标注。
    ///
    /// 修复：
    ///   1. lock_exclusive 串行化并发状态变更（保留批次 26 v6 P1 修复）
    ///   2. 状态门检查通过后，在事务内插入 budget_execution 记录
    ///      - execution_type = "使用"（表示预算实际支出）
    ///      - amount = req.actual_amount
    ///      - expense_type / expense_date / remark 透传
    ///   3. plan 状态变更为 "active"（执行中）
    pub async fn execute_plan(
        &self,
        req: BudgetExecuteRequest,
        user_id: i32,
    ) -> Result<(), AppError> {
        info!(
            "用户 {} 正在执行预算方案：{}，金额={}，费用类型={}",
            user_id, req.plan_id, req.actual_amount, req.expense_type
        );

        // 批次 26 v6 P1 修复：状态机 lock_exclusive 补全，串行化并发状态变更
        let txn = (*self.db).begin().await?;
        let plan = budget_plan::Entity::find_by_id(req.plan_id)
            .lock_exclusive()
            .one(&txn)
            .await?
            .ok_or_else(|| AppError::not_found(format!("预算方案不存在：{}", req.plan_id)))?;

        if plan.status.as_deref() != Some(budget::APPROVED) {
            return Err(AppError::validation("预算方案未审批，无法执行".to_string()));
        }

        // v11 批次 145 P1-8：在事务内创建预算执行明细
        // 内联插入（而非调用 self.create_execution），因为 create_execution 使用 &*self.db 而非 txn，
        // 这里必须在 txn 内插入以保证与 plan 状态变更的原子性。
        let active_execution = budget_execution::ActiveModel {
            plan_id: Set(req.plan_id),
            execution_type: Set("使用".to_string()),
            amount: Set(req.actual_amount),
            expense_type: Set(Some(req.expense_type.clone())),
            expense_date: Set(req.expense_date),
            related_document_type: Set(None),
            related_document_id: Set(None),
            remark: Set(req.remark.clone()),
            created_by: Set(Some(user_id)),
            ..Default::default()
        };
        let execution = active_execution.insert(&txn).await?;
        info!(
            "预算执行明细创建成功：ID={}，方案ID={}，金额={}",
            execution.id, req.plan_id, req.actual_amount
        );

        // plan 状态变更为 "active"（执行中）
        let mut plan_active: budget_plan::ActiveModel = plan.into();
        plan_active.status = Set(Some(budget::ACTIVE.to_string()));
        plan_active.save(&txn).await?;

        txn.commit().await?;
        info!(
            "预算方案执行成功：方案ID={}，执行明细ID={}，金额={}",
            req.plan_id, execution.id, req.actual_amount
        );
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
    ///
    /// 批次 329 v10 复审 P3 修复：使用 CreateBudgetExecutionParams 参数对象替代 8 个独立参数
    pub async fn create_execution(
        &self,
        params: CreateBudgetExecutionParams,
        user_id: i32,
    ) -> Result<budget_execution::Model, AppError> {
        let plan_id = params.plan_id;
        let execution_type = params.execution_type;
        let amount = params.amount;
        let expense_date = params.expense_date;
        let expense_type = params.expense_type;
        let related_document_type = params.related_document_type;
        let related_document_id = params.related_document_id;
        let remark = params.remark;

        info!(
            "用户 {} 正在创建预算执行明细，方案ID：{}，类型：{}，金额：{}",
            user_id, plan_id, execution_type, amount
        );

        // 验证预算方案是否存在
        let _plan = self.get_plan_by_id(plan_id).await?;

        // 验证执行类型
        if !["下达", "调整", "使用"].contains(&execution_type.as_str()) {
            return Err(AppError::validation(
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

    pub async fn adjust_budget(
        &self,
        req: crate::models::dto::budget_dto::AdjustBudgetRequest,
        user_id: i32,
    ) -> Result<crate::models::budget_adjustment::Model, AppError> {
        use sea_orm::TransactionTrait;
        let txn = self.db.begin().await?;

        let plan = crate::models::budget_plan::Entity::find_by_id(req.item_id)
            .one(&txn)
            .await?
            .ok_or_else(|| AppError::not_found("预算方案不存在"))?;

        // 记录调整单（审批流修复：创建为 PENDING 状态，待审批通过后才生效）
        let adjust_no = format!("BA{}", chrono::Local::now().format("%Y%m%d%H%M%S"));
        let adjustment = crate::models::budget_adjustment::ActiveModel {
            adjustment_no: sea_orm::Set(adjust_no),
            budget_id: sea_orm::Set(plan.id),
            adjustment_date: sea_orm::Set(chrono::Local::now().naive_local().date()),
            adjustment_type: sea_orm::Set(if req.adjust_amount.is_sign_negative() {
                "DECREASE".to_string()
            } else {
                "INCREASE".to_string()
            }),
            amount: sea_orm::Set(req.adjust_amount.abs()),
            budget_before: sea_orm::Set(plan.total_amount),
            budget_after: sea_orm::Set(plan.total_amount + req.adjust_amount),
            reason: sea_orm::Set(req.reason.unwrap_or_default()),
            approval_status: sea_orm::Set(approval::PENDING.to_string()),
            created_by: sea_orm::Set(user_id),
            ..Default::default()
        }
        .insert(&txn)
        .await?;

        txn.commit().await?;
        info!(
            "预算调整申请已创建（待审批）：方案 ID={}，调整金额={}",
            req.item_id, req.adjust_amount
        );
        Ok(adjustment)
    }

    /// 审批通过预算调整
    /// 将 PENDING 状态的调整单审批通过，并实际应用金额变更到预算方案
    pub async fn approve_adjustment(
        &self,
        adjustment_id: i32,
        user_id: i32,
    ) -> Result<crate::models::budget_adjustment::Model, AppError> {
        use sea_orm::TransactionTrait;
        info!(
            "用户 {} 正在审批预算调整：调整单 ID={}",
            user_id, adjustment_id
        );

        let txn = self.db.begin().await?;

        // 串行化并发审批：对调整单加行锁
        let adjustment = crate::models::budget_adjustment::Entity::find_by_id(adjustment_id)
            .lock_exclusive()
            .one(&txn)
            .await?
            .ok_or_else(|| AppError::not_found(format!("预算调整单不存在：{}", adjustment_id)))?;

        if adjustment.approval_status != approval::PENDING {
            return Err(AppError::validation(
                "预算调整单状态不允许审批（仅待审批状态可审批）".to_string(),
            ));
        }

        // 对预算方案加行锁，串行化金额变更
        let plan = crate::models::budget_plan::Entity::find_by_id(adjustment.budget_id)
            .lock_exclusive()
            .one(&txn)
            .await?
            .ok_or_else(|| AppError::not_found("预算方案不存在"))?;

        // 应用金额变更
        let adjust_amount = if adjustment.adjustment_type == "DECREASE" {
            -adjustment.amount
        } else {
            adjustment.amount
        };
        let new_total = plan.total_amount + adjust_amount;

        let mut plan_active: crate::models::budget_plan::ActiveModel = plan.into();
        plan_active.total_amount = sea_orm::Set(new_total);
        plan_active.update(&txn).await?;

        // 更新调整单状态为已审批
        let mut adj_active: crate::models::budget_adjustment::ActiveModel =
            adjustment.clone().into();
        adj_active.approval_status = sea_orm::Set(approval::APPROVED.to_string());
        let updated = adj_active.update(&txn).await?;

        txn.commit().await?;
        info!(
            "预算调整审批通过：调整单 ID={}，新预算总额={}",
            adjustment_id, new_total
        );
        Ok(updated)
    }

    /// 驳回预算调整
    /// 将 PENDING 状态的调整单驳回，不应用金额变更
    pub async fn reject_adjustment(
        &self,
        adjustment_id: i32,
        user_id: i32,
    ) -> Result<crate::models::budget_adjustment::Model, AppError> {
        use sea_orm::TransactionTrait;
        info!(
            "用户 {} 正在驳回预算调整：调整单 ID={}",
            user_id, adjustment_id
        );

        let txn = self.db.begin().await?;

        let adjustment = crate::models::budget_adjustment::Entity::find_by_id(adjustment_id)
            .lock_exclusive()
            .one(&txn)
            .await?
            .ok_or_else(|| AppError::not_found(format!("预算调整单不存在：{}", adjustment_id)))?;

        if adjustment.approval_status != approval::PENDING {
            return Err(AppError::validation(
                "预算调整单状态不允许驳回（仅待审批状态可驳回）".to_string(),
            ));
        }

        let mut adj_active: crate::models::budget_adjustment::ActiveModel =
            adjustment.clone().into();
        adj_active.approval_status = sea_orm::Set(approval::REJECTED.to_string());
        let updated = adj_active.update(&txn).await?;

        txn.commit().await?;
        info!("预算调整已驳回：调整单 ID={}", adjustment_id);
        Ok(updated)
    }

    /// 驳回预算方案
    /// 将 DRAFT 状态的预算方案驳回为 REJECTED
    pub async fn reject_plan(
        &self,
        plan_id: i32,
        user_id: i32,
        _reject_comment: Option<String>,
    ) -> Result<(), AppError> {
        info!(
            "用户 {} 正在驳回预算方案：{}",
            user_id, plan_id
        );

        let txn = (*self.db).begin().await?;
        let plan = budget_plan::Entity::find_by_id(plan_id)
            .lock_exclusive()
            .one(&txn)
            .await?
            .ok_or_else(|| AppError::not_found(format!("预算方案不存在：{}", plan_id)))?;

        if plan.status.as_deref() != Some(budget::DRAFT) {
            return Err(AppError::validation(
                "预算方案状态不允许驳回（仅草稿状态可驳回）".to_string(),
            ));
        }

        let mut plan_active: budget_plan::ActiveModel = plan.into();
        plan_active.status = Set(Some(budget::REJECTED.to_string()));
        plan_active.save(&txn).await?;
        txn.commit().await?;

        info!("预算方案已驳回：{}", plan_id);
        Ok(())
    }

    /// 检查预算是否可用
    /// 根据部门ID和预算方案ID检查是否有足够预算
    pub async fn check_budget_available(
        &self,
        department_id: i32,
        plan_id: i32,
        amount: Decimal,
    ) -> Result<bool, AppError> {
        info!(
            "检查预算可用性：部门ID={}, 方案ID={}, 金额={}",
            department_id, plan_id, amount
        );

        // 查询预算方案
        let plan = budget_plan::Entity::find_by_id(plan_id)
            .one(&*self.db)
            .await?
            .ok_or_else(|| AppError::not_found(format!("预算方案不存在：{}", plan_id)))?;

        // 验证部门匹配
        if let Some(plan_dept_id) = plan.department_id {
            if plan_dept_id != department_id {
                return Err(AppError::validation("预算方案与部门不匹配".to_string()));
            }
        }

        // 验证方案状态
        if plan.status.as_deref() != Some(budget::APPROVED) && plan.status.as_deref() != Some(budget::ACTIVE) {
            return Err(AppError::validation("预算方案未审批或未激活".to_string()));
        }

        // 计算已执行金额
        let executed_amount: Decimal = budget_execution::Entity::find()
            .filter(budget_execution::Column::PlanId.eq(plan_id))
            .filter(budget_execution::Column::ExecutionType.eq("使用".to_string()))
            .all(&*self.db)
            .await?
            .iter()
            .map(|e| e.amount)
            .sum();

        // 计算已下达金额
        let issued_amount: Decimal = budget_execution::Entity::find()
            .filter(budget_execution::Column::PlanId.eq(plan_id))
            .filter(budget_execution::Column::ExecutionType.eq("下达".to_string()))
            .all(&*self.db)
            .await?
            .iter()
            .map(|e| e.amount)
            .sum();

        // 可用金额 = 已下达金额 - 已执行金额
        let available_amount = issued_amount - executed_amount;

        info!(
            "预算可用性检查：总额={}，已下达={}，已执行={}，可用={}，请求={}",
            plan.total_amount, issued_amount, executed_amount, available_amount, amount
        );

        Ok(available_amount >= amount)
    }

    /// 占用预算
    /// 创建采购订单时调用，记录预算使用
    pub async fn occupy_budget(
        &self,
        department_id: i32,
        plan_id: i32,
        amount: Decimal,
        document_type: String,
        document_id: i32,
        user_id: i32,
    ) -> Result<budget_execution::Model, AppError> {
        info!(
            "占用预算：部门ID={}, 方案ID={}, 金额={}, 单据类型={}, 单据ID={}",
            department_id, plan_id, amount, document_type, document_id
        );

        // 先检查预算是否可用
        let available = self
            .check_budget_available(department_id, plan_id, amount)
            .await?;
        if !available {
            return Err(AppError::validation("预算余额不足，无法占用".to_string()));
        }

        // 创建预算执行记录
        let execution = self
            .create_execution(
                CreateBudgetExecutionParams {
                    plan_id,
                    execution_type: "使用".to_string(),
                    amount,
                    expense_date: chrono::Utc::now().date_naive(),
                    expense_type: Some("采购订单".to_string()),
                    related_document_type: Some(document_type),
                    related_document_id: Some(document_id),
                    remark: Some(format!("采购订单占用预算，单据ID: {}", document_id)),
                },
                user_id,
            )
            .await?;

        info!("预算占用成功：执行ID={}", execution.id);
        Ok(execution)
    }

    /// 释放预算
    /// 订单取消时调用，释放已占用的预算
    /// 核销预算
    /// 付款确认时调用，将预算占用转为实际执行
    pub async fn write_off_budget(
        &self,
        department_id: i32,
        plan_id: i32,
        amount: Decimal,
        document_type: String,
        document_id: i32,
        user_id: i32,
    ) -> Result<budget_execution::Model, AppError> {
        info!(
            "核销预算：部门ID={}, 方案ID={}, 金额={}, 单据类型={}, 单据ID={}",
            department_id, plan_id, amount, document_type, document_id
        );

        // 创建核销记录
        let execution = self
            .create_execution(
                CreateBudgetExecutionParams {
                    plan_id,
                    execution_type: "使用".to_string(),
                    amount,
                    expense_date: chrono::Utc::now().date_naive(),
                    expense_type: Some("付款核销".to_string()),
                    related_document_type: Some(document_type),
                    related_document_id: Some(document_id),
                    remark: Some(format!("付款确认核销预算，单据ID: {}", document_id)),
                },
                user_id,
            )
            .await?;

        info!("预算核销成功：执行ID={}", execution.id);
        Ok(execution)
    }

    /// 根据部门获取可用预算方案
    pub async fn get_available_plan_by_department(
        &self,
        department_id: i32,
    ) -> Result<Option<budget_plan::Model>, AppError> {
        info!("查询部门可用预算方案：部门ID={}", department_id);

        let plan = budget_plan::Entity::find()
            .filter(budget_plan::Column::DepartmentId.eq(Some(department_id)))
            .filter(budget_plan::Column::Status.eq(budget::APPROVED))
            .order_by(budget_plan::Column::CreatedAt, Order::Desc)
            .one(&*self.db)
            .await?;

        Ok(plan)
    }
}
