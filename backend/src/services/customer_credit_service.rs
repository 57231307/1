use crate::models::customer_credit;
use crate::utils::error::AppError;
use rust_decimal::Decimal;
use sea_orm::{
    ActiveModelTrait, ColumnTrait, DatabaseConnection, EntityTrait, Order, PaginatorTrait,
    QueryFilter, QueryOrder, QuerySelect, Set, TransactionTrait,
};
use std::sync::Arc;
use tracing::info;

/// 客户信用查询参数
#[derive(Debug, Clone, Default)]
pub struct CreditQueryParams {
    pub customer_id: Option<i32>,
    pub credit_level: Option<String>,
    pub status: Option<String>,
    pub page: i64,
    pub page_size: i64,
}

/// 创建/更新信用评级请求
#[derive(Debug, Clone)]
#[allow(dead_code)]
pub struct CreditRatingRequest {
    pub customer_id: i32,
    pub credit_level: String,
    pub credit_score: i32,
    pub credit_limit: Decimal,
    pub credit_days: i32,
    pub remark: Option<String>,
}

/// 信用额度调整请求
#[derive(Debug, Clone)]
#[allow(dead_code)]
pub struct CreditLimitAdjustmentRequest {
    pub customer_id: i32,
    pub adjustment_type: String,
    pub amount: Decimal,
    pub reason: String,
}

pub struct CustomerCreditService {
    db: Arc<DatabaseConnection>,
}

impl CustomerCreditService {
    pub fn new(db: Arc<DatabaseConnection>) -> Self {
        Self { db }
    }

    /// 获取客户信用评级
    pub async fn get_by_customer_id(
        &self,
        customer_id: i32,
    ) -> Result<Option<customer_credit::Model>, AppError> {
        let credit = customer_credit::Entity::find()
            .filter(customer_credit::Column::CustomerId.eq(customer_id))
            .one(&*self.db)
            .await?;
        Ok(credit)
    }

    /// 获取信用评级列表（分页）
    pub async fn get_list(
        &self,
        params: CreditQueryParams,
    ) -> Result<(Vec<customer_credit::Model>, u64), AppError> {
        let mut query = customer_credit::Entity::find();

        // 客户筛选
        if let Some(customer_id) = &params.customer_id {
            query = query.filter(customer_credit::Column::CustomerId.eq(*customer_id));
        }

        // 信用等级筛选
        if let Some(credit_level) = &params.credit_level {
            query = query.filter(customer_credit::Column::CreditLevel.eq(credit_level));
        }

        // 状态筛选
        if let Some(status) = &params.status {
            query = query.filter(customer_credit::Column::Status.eq(status));
        }

        // 获取总数
        let total = query.clone().count(&*self.db).await?;

        // 分页和排序
        let credits = query
            .order_by(customer_credit::Column::Id, Order::Desc)
            .offset((params.page * params.page_size) as u64)
            .limit(params.page_size as u64)
            .all(&*self.db)
            .await?;

        Ok((credits, total))
    }

    /// 创建/更新客户信用评级
    pub async fn set_credit_rating(
        &self,
        req: CreditRatingRequest,
        user_id: i32,
    ) -> Result<customer_credit::Model, AppError> {
        info!(
            "用户 {} 正在设置客户 {} 的信用评级",
            user_id, req.customer_id
        );

        // 检查客户是否已有信用评级
        let existing = self.get_by_customer_id(req.customer_id).await?;

        let credit = match existing {
            Some(credit) => {
                // 更新现有评级
                let used_credit = credit.used_credit;
                let mut credit_active: customer_credit::ActiveModel = credit.into();
                credit_active.credit_level = Set(Some(req.credit_level));
                credit_active.credit_score = Set(Some(req.credit_score));
                credit_active.credit_limit = Set(req.credit_limit);
                credit_active.credit_days = Set(Some(req.credit_days));
                credit_active.available_credit = Set(req.credit_limit - used_credit);
                credit_active.update(&*self.db).await?
            }
            None => {
                // 创建新评级
                let active_credit = customer_credit::ActiveModel {
                    customer_id: Set(req.customer_id),
                    credit_level: Set(Some(req.credit_level)),
                    credit_score: Set(Some(req.credit_score)),
                    credit_limit: Set(req.credit_limit),
                    used_credit: Set(Decimal::ZERO),
                    available_credit: Set(req.credit_limit),
                    credit_days: Set(Some(req.credit_days)),
                    status: Set("active".to_string()),
                    ..Default::default()
                };
                active_credit.insert(&*self.db).await?
            }
        };

        info!(
            "客户 {} 信用评级设置成功，等级：{:?}",
            req.customer_id, credit.credit_level
        );
        Ok(credit)
    }

    /// 占用信用额度
    pub async fn occupy_credit(
        &self,
        customer_id: i32,
        amount: Decimal,
        user_id: i32,
    ) -> Result<(), AppError> {
        info!(
            "用户 {} 正在占用客户 {} 的信用额度 {:.2}",
            user_id, customer_id, amount
        );

        let credit = self
            .get_by_customer_id(customer_id)
            .await?
            .ok_or_else(|| AppError::NotFound(format!("客户 {} 的信用评级不存在", customer_id)))?;

        if credit.status != "active" {
            return Err(AppError::ValidationError("客户信用状态非活跃".to_string()));
        }

        if amount > credit.available_credit {
            return Err(AppError::ValidationError(format!(
                "可用额度不足：请求 {:.2}，可用 {:.2}",
                amount, credit.available_credit
            )));
        }

        let mut credit_active: customer_credit::ActiveModel = credit.clone().into();
        credit_active.used_credit = Set(credit.used_credit + amount);
        credit_active.available_credit = Set(credit.available_credit - amount);
        // 注意：customer_credit 模型没有 updated_by 字段
        // credit_active.updated_by = Set(Some(user_id));
        credit_active.save(&*self.db).await?;

        info!(
            "客户 {} 信用额度占用成功，已占用：{:.2}",
            customer_id,
            credit.used_credit + amount
        );
        Ok(())
    }

    /// 释放信用额度
    pub async fn release_credit(
        &self,
        customer_id: i32,
        amount: Decimal,
        user_id: i32,
    ) -> Result<(), AppError> {
        info!(
            "用户 {} 正在释放客户 {} 的信用额度 {:.2}",
            user_id, customer_id, amount
        );

        let credit = self
            .get_by_customer_id(customer_id)
            .await?
            .ok_or_else(|| AppError::NotFound(format!("客户 {} 的信用评级不存在", customer_id)))?;

        if amount > credit.used_credit {
            return Err(AppError::ValidationError(
                "释放额度超过已占用额度".to_string(),
            ));
        }

        let mut credit_active: customer_credit::ActiveModel = credit.clone().into();
        credit_active.used_credit = Set(credit.used_credit - amount);
        credit_active.available_credit = Set(credit.available_credit + amount);
        // 注意：customer_credit 模型没有 updated_by 字段
        // credit_active.updated_by = Set(Some(user_id));
        credit_active.save(&*self.db).await?;

        info!(
            "客户 {} 信用额度释放成功，已占用：{:.2}",
            customer_id,
            credit.used_credit - amount
        );
        Ok(())
    }

    /// 调整信用额度
    pub async fn adjust_credit_limit(
        &self,
        req: CreditLimitAdjustmentRequest,
        user_id: i32,
    ) -> Result<(), AppError> {
        info!(
            "用户 {} 正在调整客户 {} 的信用额度，类型：{}",
            user_id, req.customer_id, req.adjustment_type
        );

        let credit = self
            .get_by_customer_id(req.customer_id)
            .await?
            .ok_or_else(|| {
                AppError::NotFound(format!("客户 {} 的信用评级不存在", req.customer_id))
            })?;

        let new_limit = match req.adjustment_type.as_str() {
            "increase" => credit.credit_limit + req.amount,
            "decrease" => {
                if req.amount > credit.credit_limit {
                    return Err(AppError::ValidationError(
                        "降低后的额度不能为负".to_string(),
                    ));
                }
                credit.credit_limit - req.amount
            }
            _ => return Err(AppError::ValidationError("无效的额度调整类型".to_string())),
        };

        let new_available = new_limit - credit.used_credit;

        // 开启事务
        let txn = (*self.db).begin().await?;

        // 更新信用额度
        let mut credit_active: customer_credit::ActiveModel = credit.into();
        credit_active.credit_limit = Set(new_limit);
        credit_active.available_credit = Set(new_available);
        // 注意：customer_credit 模型没有 updated_by 字段
        // credit_active.updated_by = Set(Some(user_id));
        credit_active.save(&txn).await?;

        // 记录变更历史
        // TODO: 需要创建 customer_credit_change 模型
        // let change_record = customer_credit::credit_change::ActiveModel {
        //     customer_id: Set(req.customer_id),
        //     change_type: Set(format!("credit_limit_{}", req.adjustment_type)),
        //     old_value: Set(credit.credit_limit.to_string()),
        //     new_value: Set(new_limit.to_string()),
        //     reason: Set(req.reason),
        //     created_by: Set(Some(user_id)),
        //     ..Default::default()
        // };
        // change_record.insert(&txn).await?;

        txn.commit().await?;

        info!(
            "客户 {} 信用额度调整成功，新额度：{}",
            req.customer_id, new_limit
        );
        Ok(())
    }

    /// 停用客户信用
    pub async fn deactivate(&self, customer_id: i32, user_id: i32) -> Result<(), AppError> {
        info!("用户 {} 正在停用客户 {} 的信用", user_id, customer_id);

        let credit = self
            .get_by_customer_id(customer_id)
            .await?
            .ok_or_else(|| AppError::NotFound(format!("客户 {} 的信用评级不存在", customer_id)))?;

        if credit.used_credit > Decimal::ZERO {
            return Err(AppError::ValidationError(
                "客户仍有占用额度，无法停用".to_string(),
            ));
        }

        let mut credit_active: customer_credit::ActiveModel = credit.into();
        credit_active.status = Set("inactive".to_string());
        // 注意：customer_credit 模型没有 updated_by 字段
        // credit_active.updated_by = Set(Some(user_id));
        credit_active.save(&*self.db).await?;

        info!("客户 {} 信用停用成功", customer_id);
        Ok(())
    }
}
