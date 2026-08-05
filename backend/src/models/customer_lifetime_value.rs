use chrono::{DateTime, NaiveDate, Utc};
use rust_decimal::Decimal;
use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

/// V15 P2 18.5-D5: 客户全生命周期价值（CLV）Entity
#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Serialize, Deserialize)]
#[sea_orm(table_name = "customer_lifetime_value")]
pub struct Model {
    #[sea_orm(primary_key)]
    pub id: i32,

    /// 客户ID
    pub customer_id: i32,

    /// 总订单数
    pub total_orders: i32,

    /// 总收入
    pub total_revenue: Decimal,

    /// 平均订单金额
    pub avg_order_value: Decimal,

    /// 首次订单日期
    pub first_order_date: Option<NaiveDate>,

    /// 最近订单日期
    pub last_order_date: Option<NaiveDate>,

    /// 客户生命周期天数
    pub customer_lifespan_days: i32,

    /// 购买频率（订单数/年）
    pub purchase_frequency: Decimal,

    /// CLV评分
    pub clv_score: Decimal,

    /// 客户分层：champion/loyal/potential/at_risk/lost
    pub segment: Option<String>,

    /// 计算时间
    pub calculated_at: Option<DateTime<Utc>>,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}

impl ActiveModelBehavior for ActiveModel {}
