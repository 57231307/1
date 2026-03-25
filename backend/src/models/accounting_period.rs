use chrono::{DateTime, Utc};
use sea_orm::entity::prelude::*;

/// 会计期间实体模型
/// 管理会计年度和期间
#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Eq)]
#[sea_orm(table_name = "accounting_periods")]
pub struct Model {
    /// 期间 ID（主键）
    #[sea_orm(primary_key)]
    pub id: i32,
    /// 会计年度
    pub year: i32,
    /// 会计期间（1-12）
    pub period: i32,
    /// 期间名称（如：2026 年 03 月）
    pub period_name: String,
    /// 开始日期
    pub start_date: DateTime<Utc>,
    /// 结束日期
    pub end_date: DateTime<Utc>,
    /// 状态：OPEN/CLOSING/CLOSED
    pub status: String,
    /// 结账时间
    pub closed_at: Option<DateTime<Utc>>,
    /// 结账人 ID
    pub closed_by: Option<i32>,
    /// 创建时间
    pub created_at: DateTime<Utc>,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}

impl ActiveModelBehavior for ActiveModel {}
