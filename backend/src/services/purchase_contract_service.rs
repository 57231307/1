use crate::models::purchase_contract;
// 批次 210 P2-5 修复（v12 复审）：合同状态字符串替换为 contract 常量
use crate::models::status::contract;
use crate::models::user;
use crate::utils::error::AppError;
use crate::utils::sql_escape::safe_like_pattern;
use chrono::NaiveDate;
use rust_decimal::Decimal;
use sea_orm::{
    ActiveModelTrait, ColumnTrait, DatabaseConnection, EntityTrait, ExprTrait, FromQueryResult,
    JoinType, Order, PaginatorTrait, QueryFilter, QueryOrder, QuerySelect, RelationTrait, Set,
    TransactionTrait,
};
use serde::Serialize;
use std::sync::Arc;
use tracing::info;

/// 采购合同读模型：实体列 + LEFT JOIN 关联出的创建人姓名（实体仅有 created_by 数值）。
///
/// JOIN 名列 `Option<String>`；实体自身列（含真实列 supplier_name）按 `purchase_contract`
/// 约束保持原类型。
#[derive(Debug, Clone, Serialize, FromQueryResult)]
pub struct PurchaseContractView {
    pub id: i32,
    pub contract_no: String,
    pub contract_name: String,
    pub contract_type: Option<String>,
    pub supplier_id: i32,
    pub supplier_name: Option<String>,
    pub total_amount: Option<Decimal>,
    pub signed_date: Option<chrono::NaiveDate>,
    pub effective_date: Option<chrono::NaiveDate>,
    pub expiry_date: Option<chrono::NaiveDate>,
    pub payment_terms: Option<String>,
    pub payment_method: Option<String>,
    pub delivery_date: Option<chrono::NaiveDate>,
    pub delivery_location: Option<String>,
    pub status: String,
    pub created_by: i32,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub updated_at: chrono::DateTime<chrono::Utc>,
    pub created_by_name: Option<String>,
}

/// 解析日期筛选边界：前端下发 ISO / `YYYY-MM-DD`，取日期部分转 `NaiveDate`；解析失败视为未提供。
fn parse_date_bound(raw: &str) -> Option<chrono::NaiveDate> {
    chrono::NaiveDate::parse_from_str(&raw[..raw.len().min(10)], "%Y-%m-%d").ok()
}

/// 采购合同查询参数
#[derive(Debug, Clone, Default)]
pub struct ContractQueryParams {
    pub keyword: Option<String>,
    pub status: Option<String>,
    pub supplier_id: Option<i32>,
    /// 签订日期范围（前端 date_range 数组：[from, to]，取首/末两项）
    pub date_range: Option<Vec<String>>,
    pub page: i64,
    pub page_size: i64,
}

/// 创建采购合同请求
#[derive(Debug, Clone)]
pub struct CreateContractRequest {
    pub contract_no: String,
    pub contract_name: String,
    pub supplier_id: i32,
    pub total_amount: Decimal,
    pub payment_terms: Option<String>,
    pub delivery_date: NaiveDate,
    pub remark: Option<String>,
}

/// 合同执行请求
#[derive(Debug, Clone)]
pub struct ExecuteContractRequest {
    pub execution_type: String,
    pub execution_amount: Decimal,
    pub execution_date: chrono::NaiveDate,
    pub related_bill_type: Option<String>,
    pub related_bill_id: Option<i32>,
    pub remark: Option<String>,
}

pub struct PurchaseContractService {
    db: Arc<DatabaseConnection>,
}

impl PurchaseContractService {
    pub fn new(db: Arc<DatabaseConnection>) -> Self {
        Self { db }
    }

    /// 创建采购合同
    pub async fn create(
        &self,
        req: CreateContractRequest,
        user_id: i32,
    ) -> Result<purchase_contract::Model, AppError> {
        info!("用户 {} 正在创建采购合同：{}", user_id, req.contract_no);

        let active_contract = purchase_contract::ActiveModel {
            contract_no: Set(req.contract_no),
            contract_name: Set(req.contract_name),
            supplier_id: Set(req.supplier_id),
            total_amount: Set(Some(req.total_amount)),
            status: Set(contract::DRAFT.to_string()),
            payment_terms: Set(req.payment_terms),
            delivery_date: Set(Some(req.delivery_date)),
            created_by: Set(user_id),
            ..Default::default()
        };

        let contract = active_contract.insert(&*self.db).await?;
        info!("采购合同创建成功：{}", contract.contract_no);
        Ok(contract)
    }

    /// 获取合同列表（分页）
    pub async fn get_list(
        &self,
        params: ContractQueryParams,
    ) -> Result<(Vec<PurchaseContractView>, u64), AppError> {
        let mut query = purchase_contract::Entity::find();

        // 关键词筛选
        if let Some(keyword) = &params.keyword {
            let keyword_pattern = safe_like_pattern(keyword);
            query = query.filter(
                purchase_contract::Column::ContractNo
                    .like(&keyword_pattern)
                    .or(purchase_contract::Column::ContractName.like(&keyword_pattern)),
            );
        }

        // 状态筛选
        if let Some(status) = &params.status {
            query = query.filter(purchase_contract::Column::Status.eq(status));
        }

        // 供应商筛选
        if let Some(supplier_id) = params.supplier_id {
            query = query.filter(purchase_contract::Column::SupplierId.eq(supplier_id));
        }

        // 签订日期范围（date_range = [from, to]，任一端可缺省）
        if let Some(range) = &params.date_range {
            if let Some(first) = range.first().and_then(|s| parse_date_bound(s)) {
                query = query.filter(purchase_contract::Column::SignedDate.gte(first));
            }
            if let Some(last) = range
                .last()
                .filter(|_| range.len() >= 2)
                .and_then(|s| parse_date_bound(s))
            {
                query = query.filter(purchase_contract::Column::SignedDate.lte(last));
            }
        }

        // 总数在无 JOIN 的基础查询上统计：Creator JOIN 为多对一（不倍增行），单次查询无 N+1。
        let total = query.clone().count(&*self.db).await?;

        // 分页和排序（LEFT JOIN users 补创建人姓名）
        let contracts = query
            .column_as(user::Column::RealName, "created_by_name")
            .join(
                JoinType::LeftJoin,
                purchase_contract::Relation::Creator.def(),
            )
            .order_by(purchase_contract::Column::Id, Order::Desc)
            // 批次 98 P2-A 修复（v5 复审）：page clamp 防 DoS
            .offset((params.page.clamp(1, 1000).saturating_sub(1) * params.page_size) as u64)
            .limit(params.page_size as u64)
            .into_model::<PurchaseContractView>()
            .all(&*self.db)
            .await?;

        Ok((contracts, total))
    }

    /// 获取合同详情
    pub async fn get_by_id(&self, id: i32) -> Result<purchase_contract::Model, AppError> {
        let contract = purchase_contract::Entity::find_by_id(id)
            .one(&*self.db)
            .await?
            .ok_or_else(|| AppError::not_found(format!("采购合同不存在：{}", id)))?;
        Ok(contract)
    }

    /// 执行合同（入库或付款）
    pub async fn execute(
        &self,
        contract_id: i32,
        req: ExecuteContractRequest,
        user_id: i32,
    ) -> Result<(), AppError> {
        info!(
            "用户 {} 正在执行合同 {}，类型：{}",
            user_id, contract_id, req.execution_type
        );

        // 批次 26 v6 P1 修复：状态机 lock_exclusive 补全，串行化并发状态变更
        // 原实现先在事务外用 get_by_id 裸查询合同状态，再 begin() 开启事务，
        // 并发 execute 均通过状态检查后基于过期状态写入，导致状态门失效。
        let txn = (*self.db).begin().await?;
        let contract = self
            .lock_and_validate_contract_txn(&txn, contract_id, &req)
            .await?;
        // 批次 27 v7 P1 修复：事务边界内校验已执行金额（防 TOCTOU）
        let total_amount = contract.total_amount.unwrap_or(Decimal::ZERO);
        if total_amount > Decimal::ZERO {
            self.check_remaining_amount_txn(&txn, contract_id, total_amount, req.execution_amount)
                .await?;
        }
        let execution_amount = req.execution_amount;
        let execution = Self::build_execution_active_model(contract_id, req, user_id);
        execution.insert(&txn).await?;
        // 更新合同时间戳
        let mut contract_active: purchase_contract::ActiveModel = contract.into();
        contract_active.updated_at = Set(chrono::Utc::now());
        contract_active.save(&txn).await?;
        txn.commit().await?;
        info!(
            "合同 {} 执行成功，执行金额：{}",
            contract_id, execution_amount
        );
        Ok(())
    }

    /// 事务内锁定合同并校验状态与执行金额
    async fn lock_and_validate_contract_txn(
        &self,
        txn: &sea_orm::DatabaseTransaction,
        contract_id: i32,
        req: &ExecuteContractRequest,
    ) -> Result<purchase_contract::Model, AppError> {
        let contract = purchase_contract::Entity::find_by_id(contract_id)
            .lock_exclusive()
            .one(txn)
            .await?
            .ok_or_else(|| AppError::not_found(format!("采购合同不存在：{}", contract_id)))?;
        if contract.status != contract::ACTIVE {
            return Err(AppError::validation(
                "只有活跃状态的合同才能执行".to_string(),
            ));
        }
        if req.execution_amount <= Decimal::ZERO {
            return Err(AppError::validation("执行金额必须大于零"));
        }
        Ok(contract)
    }

    /// 校验执行金额不超过合同剩余可执行金额（防 TOCTOU，事务内查询）
    async fn check_remaining_amount_txn(
        &self,
        txn: &sea_orm::DatabaseTransaction,
        contract_id: i32,
        total_amount: Decimal,
        execution_amount: Decimal,
    ) -> Result<(), AppError> {
        let executed_amount = crate::models::purchase_contract_execution::Entity::find()
            .filter(crate::models::purchase_contract_execution::Column::ContractId.eq(contract_id))
            .all(txn)
            .await?
            .iter()
            .map(|e| e.amount)
            .fold(Decimal::ZERO, |acc, x| acc + x);
        let remaining = total_amount - executed_amount;
        if execution_amount > remaining {
            return Err(AppError::validation(format!(
                "执行金额 {} 超过合同剩余可执行金额 {}（合同总额 {}，已执行 {}）",
                execution_amount, remaining, total_amount, executed_amount
            )));
        }
        Ok(())
    }

    /// 构造采购合同执行记录 ActiveModel
    fn build_execution_active_model(
        contract_id: i32,
        req: ExecuteContractRequest,
        user_id: i32,
    ) -> crate::models::purchase_contract_execution::ActiveModel {
        crate::models::purchase_contract_execution::ActiveModel {
            id: Default::default(),
            contract_id: Set(contract_id),
            execution_no: Set(format!(
                "PCE{}{}",
                chrono::Utc::now().format("%Y%m%d%H%M%S"),
                contract_id
            )),
            execution_type: Set(req.execution_type),
            execution_date: Set(req.execution_date),
            quantity: Set(req.execution_amount),
            amount: Set(req.execution_amount),
            status: Set("COMPLETED".to_string()),
            remarks: Set(req.remark),
            created_by: Set(user_id),
            created_at: Set(chrono::Utc::now()),
            updated_at: Set(chrono::Utc::now()),
        }
    }

    /// 审核合同
    pub async fn approve(&self, contract_id: i32, user_id: i32) -> Result<(), AppError> {
        info!("用户 {} 正在审核合同 {}", user_id, contract_id);

        // 批次 25 v6 P0 修复：状态机 lock_exclusive 补全，串行化并发状态变更
        // 原实现无事务、无行锁，并发审核会基于过期状态通过状态检查后重复写入。
        let txn = (*self.db).begin().await?;

        // 1. 加 lock_exclusive 串行化并发状态变更
        let contract = purchase_contract::Entity::find_by_id(contract_id)
            .lock_exclusive()
            .one(&txn)
            .await?
            .ok_or_else(|| AppError::not_found(format!("采购合同不存在：{}", contract_id)))?;

        // 2. 检查状态
        if contract.status != contract::DRAFT {
            return Err(AppError::validation(
                "只有草稿状态的合同才能审核".to_string(),
            ));
        }

        // 3. 更新状态 + 审计日志（事务内原子提交）
        let mut contract_active: purchase_contract::ActiveModel = contract.into();
        contract_active.status = Set(contract::ACTIVE.to_string());
        contract_active.updated_at = Set(chrono::Utc::now());

        crate::services::audit_log_service::AuditLogService::update_with_audit(
            &txn,
            "auto_audit",
            contract_active,
            Some(user_id),
        )
        .await?;

        txn.commit().await?;

        info!("合同 {} 审核成功", contract_id);
        Ok(())
    }

    /// 取消合同
    pub async fn cancel(
        &self,
        contract_id: i32,
        user_id: i32,
        reason: String,
    ) -> Result<(), AppError> {
        info!(
            "用户 {} 正在取消合同 {}，原因：{}",
            user_id, contract_id, reason
        );

        // 批次 25 v6 P0 修复：状态机 lock_exclusive 补全，串行化并发状态变更
        // 原实现无事务、无行锁，并发取消会基于过期状态通过状态检查后重复写入。
        let txn = (*self.db).begin().await?;

        // 1. 加 lock_exclusive 串行化并发状态变更
        let contract = purchase_contract::Entity::find_by_id(contract_id)
            .lock_exclusive()
            .one(&txn)
            .await?
            .ok_or_else(|| AppError::not_found(format!("采购合同不存在：{}", contract_id)))?;

        // 2. 检查状态
        if contract.status != contract::ACTIVE && contract.status != contract::DRAFT {
            return Err(AppError::validation(
                "只能取消活跃或草稿状态的合同".to_string(),
            ));
        }

        // 3. 更新状态 + 审计日志（事务内原子提交）
        let mut contract_active: purchase_contract::ActiveModel = contract.into();
        contract_active.status = Set(contract::CANCELLED.to_string());
        contract_active.updated_at = Set(chrono::Utc::now());

        crate::services::audit_log_service::AuditLogService::update_with_audit(
            &txn,
            "auto_audit",
            contract_active,
            Some(user_id),
        )
        .await?;

        txn.commit().await?;

        info!("合同 {} 取消成功", contract_id);
        Ok(())
    }
}
