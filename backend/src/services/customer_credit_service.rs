use crate::models::customer;
use crate::models::customer_credit;
use crate::utils::error::AppError;
use rust_decimal::Decimal;
use sea_orm::{
    ColumnTrait, DatabaseConnection, EntityTrait, FromQueryResult, JoinType, Order, PaginatorTrait,
    QueryFilter, QueryOrder, QuerySelect, RelationDef, Select,
};
use serde::Serialize;
use std::sync::Arc;
/// 信用评估结果
#[derive(Debug, Clone, Serialize)]
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
#[derive(Debug, Clone, Serialize)]
pub struct EvaluationFactor {
    pub factor_name: String,
    pub weight: f64,
    pub score: i32,
    pub description: String,
}

/// 客户信用读模型：customer_credit_ratings 全部实体列 + 按 customer_id
/// LEFT JOIN customers 富化出的真实客户名。
///
/// 出参键集与 customer_credit::Model 逐键同源（键名、类型、可空性一致），前端按
/// snake_case 直映。customer_name 的唯一取值来源是 JOIN 结果：本表同名的冗余列
/// 在写入路径从不赋值（信用评级只有等级/分数/额度/账期四个写入项），所以投影显式
/// 剔除该列、不让它进结果集，避免同名列出现两次后取值来源随驱动列序漂移。
///
/// 客户行缺失时 customer_name 如实为 null（键必出），禁止用 customer_id 拼装
/// 或 unwrap_or_default 伪装成空名字——界面据此区分“无名字”与“字段缺失”。
#[derive(Debug, Clone, Serialize, FromQueryResult)]
pub struct CustomerCreditView {
    pub id: i32,
    pub customer_id: i32,
    pub customer_name: Option<String>,
    pub credit_level: Option<String>,
    pub credit_score: Option<i32>,
    pub credit_limit: Decimal,
    pub used_credit: Decimal,
    pub available_credit: Decimal,
    pub credit_days: Option<i32>,
    pub last_assessment_date: Option<chrono::NaiveDate>,
    pub next_assessment_date: Option<chrono::NaiveDate>,
    pub status: String,
    pub created_by: Option<i32>,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub updated_at: chrono::DateTime<chrono::Utc>,
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

/// 创建/更新信用评级请求（批次 414 技术债务修复：credit_limit 改为 Option<Decimal>，；区分"未提供"（None，更新时保持原值）与"显式置 0"（Some(0)）两种语义。）
#[derive(Debug, Clone)]
pub struct CreditRatingRequest {
    pub customer_id: i32,
    pub credit_level: Option<String>,
    pub credit_score: Option<i32>,
    /// 信用额度。None 表示更新时保持原值；Some(v) 表示显式设置（含 Some(0)）。
    pub credit_limit: Option<Decimal>,
    pub credit_days: Option<i32>,
    #[allow(dead_code, reason = "预留")]
    pub remark: Option<String>,
}

/// 信用额度调整请求
#[derive(Debug, Clone)]
pub struct CreditLimitAdjustmentRequest {
    pub customer_id: i32,
    pub adjustment_type: String,
    pub amount: Decimal,
    #[allow(dead_code, reason = "预留")]
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
    /// 批次 409 提取：原 get_list 方法内联的 page.clamp(1, 1000) 逻辑，；提取为独立纯函数便于单元测试。；页码 < 1 → 1；页码 > 1000 → 1000；其他 → 原值
    pub fn clamp_page(page: i64) -> i64 {
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
    /// 原 check_credit_available 用 self.db 查询信用额度，与订单提交事务隔离，；并发场景下可能在查询后、提交前信用额度被调整，导致超卖。；新增 _txn 变体，在订单提交事务内查询信用额度。
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
    ///
    /// 出参是 [CustomerCreditView]（与详情同一 JOIN 基底），customer_name 恒为
    /// customers 表真值；分页语义（page clamp、offset/limit、按 id 倒序）与总数口径不变。
    pub async fn get_list(
        &self,
        params: CreditQueryParams,
    ) -> Result<(Vec<CustomerCreditView>, u64), AppError> {
        let mut query = Self::view_select();

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

        // 获取总数：富化基底是“多对一 LEFT JOIN”（customers.id 为主键），不倍增行，
        // 因此总数与筛选后的评级行数一致，无需另建一条不含 JOIN 的计数查询。
        let total = query.clone().count(&*self.db).await?;

        // 分页和排序
        let credits = query
            .order_by(customer_credit::Column::Id, Order::Desc)
            // page clamp 防 DoS：超大页码不产生超大偏移量查询
            .offset((Self::clamp_page(params.page).saturating_sub(1) * params.page_size) as u64)
            .limit(params.page_size as u64)
            .into_model::<CustomerCreditView>()
            .all(&*self.db)
            .await?;

        Ok((credits, total))
    }

    /// 单行富化读（按客户自然键）：与列表同一条 JOIN 基底，保证详情的
    /// customer_name 不缺来源。本表 customer_id 由迁移 m0082 施加全表唯一，
    /// 每客户恒一行，.one() 取单行成立。
    pub async fn get_view_by_customer_id(
        &self,
        customer_id: i32,
    ) -> Result<Option<CustomerCreditView>, AppError> {
        Ok(Self::view_select()
            .filter(customer_credit::Column::CustomerId.eq(customer_id))
            .into_model::<CustomerCreditView>()
            .one(&*self.db)
            .await?)
    }

    /// 单行富化读（按行主键 customer_credit_ratings.id）：与列表/详情同一条 JOIN
    /// 基底，供路径按行主键定位本资源的出口（打印数据装配）复用，避免第二套名称来源。
    pub async fn get_view_by_row_id(
        &self,
        id: i32,
    ) -> Result<Option<CustomerCreditView>, AppError> {
        Ok(Self::view_select()
            .filter(customer_credit::Column::Id.eq(id))
            .into_model::<CustomerCreditView>()
            .one(&*self.db)
            .await?)
    }

    /// 富化查询基底：显式列举 customer_credit_ratings 实体列（剔除本表冗余的
    /// customer_name 列，见 [CustomerCreditView] 取值来源说明）+ 一次
    /// LEFT JOIN customers 取真实客户名，别名与前端契约键逐字同为 customer_name。
    ///
    /// customer_credit::Relation 为空（本实体无声明式关系），故按仓内既有写法内联
    /// 构造 RelationDef（范式同 services/supplier_evaluation_service.rs 的
    /// record_view_select 与 services/po/order_ops/crud.rs 的 list_orders）：单次查询，禁止逐行再查。
    fn view_select() -> Select<customer_credit::Entity> {
        let customer_rel: RelationDef = customer_credit::Entity::belongs_to(customer::Entity)
            .from(customer_credit::Column::CustomerId)
            .to(customer::Column::Id)
            .into();

        customer_credit::Entity::find()
            .select_only()
            .column(customer_credit::Column::Id)
            .column(customer_credit::Column::CustomerId)
            .column(customer_credit::Column::CreditLevel)
            .column(customer_credit::Column::CreditScore)
            .column(customer_credit::Column::CreditLimit)
            .column(customer_credit::Column::UsedCredit)
            .column(customer_credit::Column::AvailableCredit)
            .column(customer_credit::Column::CreditDays)
            .column(customer_credit::Column::LastAssessmentDate)
            .column(customer_credit::Column::NextAssessmentDate)
            .column(customer_credit::Column::Status)
            .column(customer_credit::Column::CreatedBy)
            .column(customer_credit::Column::CreatedAt)
            .column(customer_credit::Column::UpdatedAt)
            .column_as(customer::Column::CustomerName, "customer_name")
            .join(JoinType::LeftJoin, customer_rel)
    }
}
