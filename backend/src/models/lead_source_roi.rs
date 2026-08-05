use chrono::{DateTime, NaiveDate, Utc};
use rust_decimal::Decimal;
use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

/// V15 P2 18.1-D4: 线索来源 ROI 跟踪 Entity
#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Serialize, Deserialize)]
#[sea_orm(table_name = "lead_source_roi")]
pub struct Model {
    #[sea_orm(primary_key)]
    pub id: i32,

    /// 渠道来源
    pub source: String,

    /// 统计周期开始
    pub period_start: NaiveDate,

    /// 统计周期结束
    pub period_end: NaiveDate,

    /// 渠道投入成本
    pub cost: Decimal,

    /// 线索数量
    pub lead_count: i32,

    /// 转化客户数
    pub converted_count: i32,

    /// 商机数
    pub opportunity_count: i32,

    /// 成交订单数
    pub order_count: i32,

    /// 成交金额
    pub revenue: Decimal,

    /// 转化率
    pub conversion_rate: Decimal,

    /// ROI = (收入-成本)/成本
    pub roi: Decimal,

    /// 创建时间
    pub created_at: Option<DateTime<Utc>>,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}

impl ActiveModelBehavior for ActiveModel {}
