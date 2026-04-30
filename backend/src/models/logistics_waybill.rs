use chrono::{DateTime, Utc};
use rust_decimal::Decimal;
use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Serialize, Deserialize)]
#[sea_orm(table_name = "logistics_waybills")]
pub struct Model {
    #[sea_orm(primary_key)]
    pub id: i32,
    pub order_id: i32,
    pub logistics_company: String,
    pub tracking_number: String,
    pub driver_name: Option<String>,
    pub driver_phone: Option<String>,
    pub freight_fee: Option<Decimal>,
    pub status: Option<String>,
    pub expected_arrival: Option<DateTime<Utc>>,
    pub actual_arrival: Option<DateTime<Utc>>,
    pub notes: Option<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {
    #[sea_orm(
        belongs_to = "super::sales_order::Entity",
        from = "Column::OrderId",
        to = "super::sales_order::Column::Id"
    )]
    SalesOrder,
}

impl Related<super::sales_order::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::SalesOrder.def()
    }
}

impl ActiveModelBehavior for ActiveModel {}
