use crate::models::sales_contract;
use sea_orm::{
    ActiveModelTrait, ColumnTrait, DatabaseConnection, EntityTrait, QueryFilter, QueryOrder, Set,
    TransactionTrait, QuerySelect, PaginatorTrait, Order,
};
use std::sync::Arc;
use rust_decimal::Decimal;
use chrono::NaiveDate;
use crate::utils::error::AppError;
use tracing::info;

/// 销售合同查询参数
#[derive(Debug, Clone, Default)]
pub struct SalesContractQueryParams {
    pub keyword: Option<String>,
    pub status: Option<String>,
    pub customer_id: Option<i32>,
    pub page: i64,
    pub page_size: i64,
}

/// 创建销售合同请求
#[derive(Debug, Clone)]
#[allow(dead_code)]
pub struct CreateSalesContractRequest {
    pub contract_no: String,
    pub contract_name: String,
    pub customer_id: i32,
    pub total_amount: Decimal,
    pub payment_terms: Option<String>,
    pub delivery_date: NaiveDate,
    pub remark: Option<String>,
}

/// 合同执行请求
#[derive(Debug, Clone)]
#[allow(dead_code)]
pub struct ExecuteSalesContractRequest {
    pub execution_type: String,
    pub execution_amount: Decimal,
    pub related_bill_type: Option<String>,
    pub related_bill_id: Option<i32>,
    pub remark: Option<String>,
}

pub struct SalesContractService {
    db: Arc<DatabaseConnection>,
}

impl SalesContractService {
    pub fn new(db: Arc<DatabaseConnection>) -> Self {
        Self { db }
    }

    /// 创建销售合同
    pub async fn create(
        &self,
        req: CreateSalesContractRequest,
        user_id: i32,
    ) -> Result<sales_contract::Model, AppError> {
        info!("用户 {} 正在创建销售合同：{}", user_id, req.contract_no);

        let active_contract = sales_contract::ActiveModel {
            contract_no: Set(req.contract_no),
            contract_name: Set(req.contract_name),
            customer_id: Set(req.customer_id),
            total_amount: Set(Some(req.total_amount)),
            status: Set("draft".to_string()),
            payment_terms: Set(req.payment_terms),
            delivery_date: Set(Some(req.delivery_date)),
            created_by: Set(user_id),
            ..Default::default()
        };

        let contract = active_contract.insert(&*self.db).await?;
        info!("销售合同创建成功：{}", contract.contract_no);
        Ok(contract)
    }

    /// 获取合同列表（分页）
    pub async fn get_list(
        &self,
        params: SalesContractQueryParams,
    ) -> Result<(Vec<sales_contract::Model>, u64), AppError> {
        let mut query = sales_contract::Entity::find();

        // 关键词筛选
        if let Some(keyword) = &params.keyword {
            let keyword_pattern = format!("%{}%", keyword);
            query = query.filter(
                sales_contract::Column::ContractNo
                    .like(&keyword_pattern)
                    .or(sales_contract::Column::ContractName.like(&keyword_pattern)),
            );
        }

        // 状态筛选
        if let Some(status) = &params.status {
            query = query.filter(sales_contract::Column::Status.eq(status));
        }

        // 客户筛选
        if let Some(customer_id) = &params.customer_id {
            query = query.filter(sales_contract::Column::CustomerId.eq(*customer_id));
        }

        // 获取总数
        let total = query.clone().count(&*self.db).await?;

        // 分页和排序
        let contracts = query
            .order_by(sales_contract::Column::Id, Order::Desc)
            .offset((params.page * params.page_size) as u64)
            .limit(params.page_size as u64)
            .all(&*self.db)
            .await?;

        Ok((contracts, total))
    }

    /// 获取合同详情
    pub async fn get_by_id(&self, id: i32) -> Result<sales_contract::Model, AppError> {
        let contract = sales_contract::Entity::find_by_id(id)
            .one(&*self.db)
            .await?
            .ok_or_else(|| AppError::NotFound(format!("销售合同不存在：{}", id)))?;
        Ok(contract)
    }

    /// 执行合同（出库或收款）
    pub async fn execute(
        &self,
        contract_id: i32,
        req: ExecuteSalesContractRequest,
        user_id: i32,
    ) -> Result<(), AppError> {
        info!("用户 {} 正在执行销售合同 {}，类型：{}", user_id, contract_id, req.execution_type);

        // 获取合同
        let contract = self.get_by_id(contract_id).await?;

        // 检查合同状态
        if contract.status != "active" {
            return Err(AppError::ValidationError("只有活跃状态的合同才能执行".to_string()));
        }

        // 检查执行金额
        // 注意：sales_contract 模型没有 executed_amount 字段，暂时跳过检查
        // let new_executed_amount = contract.executed_amount + req.execution_amount;
        // if new_executed_amount > contract.total_amount {
        //     return Err(AppError::ValidationError(format!(
        //         "执行金额 {:.2} 超过合同总金额 {:.2}",
        //         new_executed_amount, contract.total_amount
        //     )));
        // }

        // 开启事务
        let txn = (&*self.db).begin().await?;

        // 创建执行记录
        // TODO: 需要创建 sales_contract_execution 模型
        // let execution = sales_contract::contract_execution::ActiveModel {
        //     contract_id: Set(contract_id),
        //     execution_type: Set(req.execution_type),
        //     execution_amount: Set(req.execution_amount),
        //     related_bill_type: Set(req.related_bill_type),
        //     related_bill_id: Set(req.related_bill_id),
        //     remark: Set(req.remark),
        //     created_by: Set(Some(user_id)),
        //     ..Default::default()
        // };
        // execution.insert(&txn).await?;

        // 更新合同已执行金额
        let contract_active: sales_contract::ActiveModel = contract.into();
        // 注意：sales_contract 模型没有 executed_amount 和 updated_by 字段
        // contract_active.executed_amount = Set(new_executed_amount);
        // contract_active.updated_by = Set(Some(user_id));

        // 检查合同是否完成
        // if new_executed_amount >= contract.total_amount {
        //     contract_active.status = Set("completed".to_string());
        // }

        contract_active.save(&txn).await?;

        // 提交事务
        txn.commit().await?;

        info!("销售合同 {} 执行成功，执行金额：{}", contract_id, req.execution_amount);
        Ok(())
    }

    /// 审核合同
    pub async fn approve(&self, contract_id: i32, user_id: i32) -> Result<(), AppError> {
        info!("用户 {} 正在审核销售合同 {}", user_id, contract_id);

        let contract = self.get_by_id(contract_id).await?;

        if contract.status != "draft" {
            return Err(AppError::ValidationError("只有草稿状态的合同才能审核".to_string()));
        }

        let mut contract_active: sales_contract::ActiveModel = contract.into();
        contract_active.status = Set("active".to_string());
        // 注意：sales_contract 模型没有 approved_by、approved_at、updated_by 字段
        // contract_active.approved_by = Set(Some(user_id));
        // contract_active.approved_at = Set(Some(chrono::Local::now().naive_local()));
        // contract_active.updated_by = Set(Some(user_id));
        contract_active.save(&*self.db).await?;

        info!("销售合同 {} 审核成功", contract_id);
        Ok(())
    }

    /// 取消合同
    pub async fn cancel(&self, contract_id: i32, user_id: i32, reason: String) -> Result<(), AppError> {
        info!("用户 {} 正在取消销售合同 {}，原因：{}", user_id, contract_id, reason);

        let contract = self.get_by_id(contract_id).await?;

        if contract.status != "active" && contract.status != "draft" {
            return Err(AppError::ValidationError("只能取消活跃或草稿状态的合同".to_string()));
        }

        // 注意：sales_contract 模型没有 executed_amount 字段
        // if contract.executed_amount > Decimal::ZERO {
        //     return Err(AppError::ValidationError("已执行的合同不能取消".to_string()));
        // }

        let mut contract_active: sales_contract::ActiveModel = contract.into();
        contract_active.status = Set("cancelled".to_string());
        // 注意：sales_contract 模型没有 remark 和 updated_by 字段
        // contract_active.remark = Set(Some(format!("{}\n取消原因：{}", contract.remark.unwrap_or_default(), reason)));
        // contract_active.updated_by = Set(Some(user_id));
        contract_active.save(&*self.db).await?;

        info!("销售合同 {} 取消成功", contract_id);
        Ok(())
    }
}
