use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};
use chrono::NaiveDateTime;

#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Serialize, Deserialize)]
#[sea_orm(table_name = "ar_verification")]
pub struct Model {
    #[sea_orm(primary_key)]
    pub id: i32,
    pub verify_no: String,
    pub customer_id: i32,
    pub receipt_id: i32,
    pub invoice_id: i32,
    pub verify_amount: f64,
    pub status: String,
    pub verify_date: NaiveDateTime,
    pub created_at: NaiveDateTime,
    pub updated_at: NaiveDateTime,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}

impl ActiveModelBehavior for ActiveModel {}
