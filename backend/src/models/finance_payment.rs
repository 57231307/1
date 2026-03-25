use chrono::{DateTime, Utc};
use rust_decimal::Decimal;
use sea_orm::entity::prelude::*;

#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Eq)]
#[sea_orm(table_name = "finance_payments")]
pub struct Model {
    #[sea_orm(primary_key)]
    pub id: i32,
    pub payment_no: String,
    pub payment_type: String,
    pub order_type: String,
    pub order_id: Option<i32>,
    pub customer_id: Option<i32>,
    pub supplier_id: Option<i32>,
    pub amount: Decimal,
    pub paid_amount: Decimal,
    pub balance_amount: Decimal,
    pub payment_date: DateTime<Utc>,
    pub payment_method: Option<String>,
    pub reference_no: Option<String>,
    pub notes: Option<String>,
    pub status: String,
    pub created_by: Option<i32>,
    pub approved_by: Option<i32>,
    pub approved_at: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}

impl ActiveModelBehavior for ActiveModel {}
