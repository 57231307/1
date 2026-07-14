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

    /// 分页页码安全约束（防 DoS）
    ///
    /// 批次 409 提取：原 get_list 方法内联的 page.clamp(1, 1000) 逻辑，
    /// 提取为独立纯函数便于单元测试。
    /// - 页码 < 1 → 1
    /// - 页码 > 1000 → 1000
    /// - 其他 → 原值
    pub(crate) fn clamp_page(page: i64) -> i64 {
        page.clamp(1, 1000)
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

    /// P2 3-20 修复：事务内查询客户信用评级，避免 TOCTOU
    ///
    /// 原 check_credit_available 用 self.db 查询信用额度，与订单提交事务隔离，
    /// 并发场景下可能在查询后、提交前信用额度被调整，导致超卖。
    /// 新增 _txn 变体，在订单提交事务内查询信用额度。
    pub async fn get_by_customer_id_txn(
        &self,
        txn: &sea_orm::DatabaseTransaction,
        customer_id: i32,
    ) -> Result<Option<customer_credit::Model>, AppError> {
        let credit = customer_credit::Entity::find()
            .filter(customer_credit::Column::CustomerId.eq(customer_id))
            .one(txn)
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
            // 批次 98 P2-A 修复（v5 复审）：page clamp 防 DoS
            .offset((Self::clamp_page(params.page).saturating_sub(1) * params.page_size) as u64)
            .limit(params.page_size as u64)
            .all(&*self.db)
            .await?;

        Ok((credits, total))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // ========== clamp_page 纯函数测试 ==========

    /// 测试_页码约束_零值被限制为1
    ///
    /// 验证 page=0（页码小于下界）被 clamp 到 1。
    #[test]
    fn 测试_页码约束_零值被限制为1() {
        assert_eq!(CustomerCreditService::clamp_page(0), 1);
    }

    /// 测试_页码约束_页码为1_边界返回1
    ///
    /// 验证 page=1（下界边界）原值返回 1。
    #[test]
    fn 测试_页码约束_页码为1_边界返回1() {
        assert_eq!(CustomerCreditService::clamp_page(1), 1);
    }

    /// 测试_页码约束_页码为500_正常范围原值返回
    ///
    /// 验证 page=500（在 [1, 1000] 正常范围内）原值返回 500。
    #[test]
    fn 测试_页码约束_页码为500_正常范围原值返回() {
        assert_eq!(CustomerCreditService::clamp_page(500), 500);
    }

    /// 测试_页码约束_页码为1000_边界返回1000
    ///
    /// 验证 page=1000（上界边界）原值返回 1000。
    #[test]
    fn 测试_页码约束_页码为1000_边界返回1000() {
        assert_eq!(CustomerCreditService::clamp_page(1000), 1000);
    }

    /// 测试_页码约束_页码为1001_超出上界被限制为1000
    ///
    /// 验证 page=1001（超出上界）被 clamp 到 1000，防止超大偏移量 DoS。
    #[test]
    fn 测试_页码约束_页码为1001_超出上界被限制为1000() {
        assert_eq!(CustomerCreditService::clamp_page(1001), 1000);
    }

    /// 测试_页码约束_负值被限制为1
    ///
    /// 验证 page=-5（负数）被 clamp 到 1，防止负偏移量。
    #[test]
    fn 测试_页码约束_负值被限制为1() {
        assert_eq!(CustomerCreditService::clamp_page(-5), 1);
    }

    /// 测试_页码约束_i64最大值被限制为1000
    ///
    /// 验证 page=i64::MAX（极端大值）被 clamp 到 1000，防止溢出与 DoS。
    #[test]
    fn 测试_页码约束_i64最大值被限制为1000() {
        assert_eq!(CustomerCreditService::clamp_page(i64::MAX), 1000);
    }

    /// 测试_页码约束_i64最小值被限制为1
    ///
    /// 验证 page=i64::MIN（极端小值）被 clamp 到 1，防止溢出与负偏移。
    #[test]
    fn 测试_页码约束_i64最小值被限制为1() {
        assert_eq!(CustomerCreditService::clamp_page(i64::MIN), 1);
    }

    // ========== CreditQueryParams Default 实现测试 ==========

    /// 测试_查询参数_Default实现_全部字段为空或零
    ///
    /// 验证 CreditQueryParams::default() 返回的结构体：
    /// - customer_id=None（未指定客户筛选）
    /// - credit_level=None（未指定信用等级筛选）
    /// - status=None（未指定状态筛选）
    /// - page=0（零页码，需配合 clamp_page 使用）
    /// - page_size=0（零页大小）
    #[test]
    fn 测试_查询参数_Default实现_全部字段为空或零() {
        let params = CreditQueryParams::default();
        assert!(params.customer_id.is_none(), "默认 customer_id 应为 None");
        assert!(params.credit_level.is_none(), "默认 credit_level 应为 None");
        assert!(params.status.is_none(), "默认 status 应为 None");
        assert_eq!(params.page, 0, "默认 page 应为 0");
        assert_eq!(params.page_size, 0, "默认 page_size 应为 0");
    }
}
