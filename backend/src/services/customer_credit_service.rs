use crate::models::customer_credit;
use crate::utils::error::AppError;
use rust_decimal::Decimal;
use sea_orm::{
    ColumnTrait, DatabaseConnection, EntityTrait, Order, PaginatorTrait, QueryFilter, QueryOrder,
    QuerySelect,
};
use std::sync::Arc;
/// 信用评估结果
#[derive(Debug, Clone, serde::Serialize)]
pub struct CreditEvaluationResult {
    pub customer_id: i32,
    pub customer_name: String,
    pub credit_score: i32,
    pub credit_rating: String,
    pub recommended_limit: Decimal,
    pub evaluation_factors: Vec<EvaluationFactor>,
    pub evaluation_date: String,
}

/// 评估因子
#[derive(Debug, Clone, serde::Serialize)]
pub struct EvaluationFactor {
    pub factor_name: String,
    pub weight: f64,
    pub score: i32,
    pub description: String,
}

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
pub struct CreditRatingRequest {
    pub customer_id: i32,
    pub credit_level: Option<String>,
    pub credit_score: Option<i32>,
    pub credit_limit: Decimal,
    pub credit_days: Option<i32>,
    pub remark: Option<String>,
}

/// 信用额度调整请求
#[derive(Debug, Clone)]
pub struct CreditLimitAdjustmentRequest {
    pub customer_id: i32,
    pub adjustment_type: String,
    pub amount: Decimal,
    pub reason: String,
}

pub struct CustomerCreditService {
    pub db: Arc<DatabaseConnection>,
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
            .offset((params.page.saturating_sub(1) * params.page_size) as u64)
            .limit(params.page_size as u64)
            .all(&*self.db)
            .await?;

        Ok((credits, total))
    }
}
