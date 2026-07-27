use chrono::{DateTime, Utc};
use rust_decimal::Decimal;
use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

/// 售后工单实体
#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Eq, Serialize, Deserialize)]
#[sea_orm(table_name = "after_sales")]
pub struct Model {
    #[sea_orm(primary_key)]
    pub id: i64,
    pub custom_order_id: i64,
    pub issue_type: String,
    pub customer_id: i64,
    pub description: String,
    pub status: String,
    pub opened_at: DateTime<Utc>,
    pub closed_at: Option<DateTime<Utc>>,
    pub resolution: Option<String>,
    pub refund_amount: Option<Decimal>,
    /// V15 P0-B12：关联质量异常 ID，售后创建时可选关联或调用 trigger_quality_investigation 自动创建
    pub quality_issue_id: Option<i64>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {
    #[sea_orm(
        belongs_to = "super::custom_order::Entity",
        from = "Column::CustomOrderId",
        to = "super::custom_order::Column::Id"
    )]
    CustomOrder,
    #[sea_orm(
        belongs_to = "super::customer::Entity",
        from = "Column::CustomerId",
        to = "super::customer::Column::Id"
    )]
    Customer,
    /// V15 P0-B12：关联质量异常（quality_issues）
    #[sea_orm(
        belongs_to = "super::quality_issue::Entity",
        from = "Column::QualityIssueId",
        to = "super::quality_issue::Column::Id"
    )]
    QualityIssue,
}

impl Related<super::custom_order::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::CustomOrder.def()
    }
}

impl ActiveModelBehavior for ActiveModel {}
