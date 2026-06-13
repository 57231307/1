#![allow(dead_code)]
// TODO(tech-debt): 业务接入或重评估后逐项移除；rustc 1.94+ 编译时由编译器报告具体死代码位置。
// TODO(tech-debt): 业务接入后逐项移除此标注；rustc 1.94+ 编译时由编译器报告具体死代码位置。

use chrono::{DateTime, Utc};
use rust_decimal::Decimal;
use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Eq, Serialize, Deserialize)]
#[sea_orm(table_name = "sales_orders")]
pub struct Model {
    #[sea_orm(primary_key)]
    pub id: i32,
    pub order_no: String,
    pub customer_id: i32,
    pub opportunity_id: Option<i32>,
    pub order_date: DateTime<Utc>,
    pub required_date: DateTime<Utc>,
    pub ship_date: Option<DateTime<Utc>>,
    pub status: String,
    pub subtotal: Decimal,
    pub tax_amount: Decimal,
    pub discount_amount: Decimal,
    pub shipping_cost: Decimal,
    pub total_amount: Decimal,
    pub paid_amount: Decimal,
    pub balance_amount: Decimal,
    pub shipping_address: Option<String>,
    pub billing_address: Option<String>,
    pub notes: Option<String>,
    pub created_by: Option<i32>,
    pub approved_by: Option<i32>,
    pub approved_at: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {
    #[sea_orm(has_many = "super::sales_order_item::Entity")]
    Items,
    #[sea_orm(
        belongs_to = "super::customer::Entity",
        from = "Column::CustomerId",
        to = "super::customer::Column::Id"
    )]
    Customer,
    #[sea_orm(
        belongs_to = "super::user::Entity",
        from = "Column::CreatedBy",
        to = "super::user::Column::Id"
    )]
    Creator,
    #[sea_orm(
        belongs_to = "super::user::Entity",
        from = "Column::ApprovedBy",
        to = "super::user::Column::Id"
    )]
    Approver,
    #[sea_orm(
        belongs_to = "super::crm_opportunity::Entity",
        from = "Column::OpportunityId",
        to = "super::crm_opportunity::Column::Id"
    )]
    Opportunity,
}

impl Related<super::customer::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::Customer.def()
    }
}

impl Related<super::sales_order_item::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::Items.def()
    }
}

impl Related<super::crm_opportunity::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::Opportunity.def()
    }
}

impl ActiveModelBehavior for ActiveModel {}
