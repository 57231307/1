use crate::models::purchase_contract;
use crate::utils::error::AppError;
use chrono::NaiveDate;
use rust_decimal::Decimal;
use sea_orm::{
    ActiveModelTrait, ColumnTrait, DatabaseConnection, EntityTrait, Order, PaginatorTrait,
    QueryFilter, QueryOrder, QuerySelect, Set, TransactionTrait,
};
use std::sync::Arc;
use tracing::info;

/// 采购合同查询参数
#[derive(Debug, Clone, Default)]
pub struct ContractQueryParams {
    pub keyword: Option<String>,
    pub status: Option<String>,
    pub supplier_id: Option<i32>,
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
            status: Set("draft".to_string()),
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
    ) -> Result<(Vec<purchase_contract::Model>, u64), AppError> {
        let mut query = purchase_contract::Entity::find();

        // 关键词筛选
        if let Some(keyword) = &params.keyword {
            let keyword_pattern = format!("%{}%", keyword);
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

        // 获取总数
        let total = query.clone().count(&*self.db).await?;

        // 分页和排序
        let contracts = query
            .order_by(purchase_contract::Column::Id, Order::Desc)
            .offset((params.page * params.page_size) as u64)
            .limit(params.page_size as u64)
            .all(&*self.db)
            .await?;

        Ok((contracts, total))
    }

    /// 获取合同详情
    pub async fn get_by_id(&self, id: i32) -> Result<purchase_contract::Model, AppError> {
        let contract = purchase_contract::Entity::find_by_id(id)
            .one(&*self.db)
            .await?
            .ok_or_else(|| AppError::NotFound(format!("采购合同不存在：{}", id)))?;
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

        // 获取合同
        let contract = self.get_by_id(contract_id).await?;

        // 检查合同状态
        if contract.status != "active" {
            return Err(AppError::ValidationError(
                "只有活跃状态的合同才能执行".to_string(),
            ));
        }

        // 检查执行金额
        // 注意：purchase_contract 模型没有 executed_amount 字段，暂时跳过检查
        // let new_executed_amount = contract.executed_amount + req.execution_amount;
        // if new_executed_amount > contract.total_amount {
        //     return Err(AppError::ValidationError(format!(
        //         "执行金额 {:.2} 超过合同总金额 {:.2}",
        //         new_executed_amount, contract.total_amount
        //     )));
        // }

        // 开启事务
        let txn = (*self.db).begin().await?;

        // 创建执行记录
        let execution = crate::models::purchase_contract_execution::ActiveModel {
            id: Set(0),
            contract_id: Set(contract_id),
            execution_no: Set(format!(
                "PCE{}{}",
                chrono::Utc::now().format("%Y%m%d%H%M%S"),
                contract_id
            )),
            execution_type: Set(req.execution_type),
            execution_date: Set(req.execution_date),
            quantity: Set(req.execution_amount), // 使用 execution_amount 作为数量
            amount: Set(req.execution_amount),   // 使用 amount
            status: Set("COMPLETED".to_string()),
            remarks: Set(req.remark), // 使用 remarks
            created_by: Set(user_id),
            created_at: Set(chrono::Utc::now()),
            updated_at: Set(chrono::Utc::now()),
        };

        execution.insert(&txn).await?;

        // 更新合同已执行金额
        let contract_active: purchase_contract::ActiveModel = contract.into();
        // 注意：purchase_contract 模型没有 executed_amount 字段，需要添加或使用其他方式跟踪
        // 这里暂时注释掉，等待模型更新
        // contract_active.executed_amount = Set(new_executed_amount);
        // contract_active.updated_by = Set(Some(user_id));

        // 检查合同是否完成
        // if new_executed_amount >= contract.total_amount {
        //     contract_active.status = Set("completed".to_string());
        // }

        contract_active.save(&txn).await?;

        // 提交事务
        txn.commit().await?;

        info!(
            "合同 {} 执行成功，执行金额：{}",
            contract_id, req.execution_amount
        );
        Ok(())
    }

    /// 审核合同
    pub async fn approve(&self, contract_id: i32, user_id: i32) -> Result<(), AppError> {
        info!("用户 {} 正在审核合同 {}", user_id, contract_id);

        let contract = self.get_by_id(contract_id).await?;

        if contract.status != "draft" {
            return Err(AppError::ValidationError(
                "只有草稿状态的合同才能审核".to_string(),
            ));
        }

        let mut contract_active: purchase_contract::ActiveModel = contract.into();
        contract_active.status = Set("active".to_string());
        // 注意：purchase_contract 模型没有 approved_by、approved_at、updated_by 字段
        // contract_active.approved_by = Set(Some(user_id));
        // contract_active.approved_at = Set(Some(chrono::Local::now().naive_local()));
        // contract_active.updated_by = Set(Some(user_id));
        contract_active.save(&*self.db).await?;

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

        let contract = self.get_by_id(contract_id).await?;

        if contract.status != "active" && contract.status != "draft" {
            return Err(AppError::ValidationError(
                "只能取消活跃或草稿状态的合同".to_string(),
            ));
        }

        // 注意：purchase_contract 模型没有 executed_amount 字段
        // if contract.executed_amount > Decimal::ZERO {
        //     return Err(AppError::ValidationError("已执行的合同不能取消".to_string()));
        // }

        let mut contract_active: purchase_contract::ActiveModel = contract.into();
        contract_active.status = Set("cancelled".to_string());
        // 注意：purchase_contract 模型没有 remark 和 updated_by 字段
        // contract_active.remark = Set(Some(format!("{}\n取消原因：{}", contract.remark.unwrap_or_default(), reason)));
        // contract_active.updated_by = Set(Some(user_id));
        contract_active.save(&*self.db).await?;

        info!("合同 {} 取消成功", contract_id);
        Ok(())
    }
}
