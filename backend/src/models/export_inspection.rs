#![allow(dead_code)]
//! 出口商检记录模型
use chrono::{DateTime, NaiveDate, Utc};
use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Serialize, Deserialize)]
#[sea_orm(table_name = "export_inspection")]
pub struct Model {
    #[sea_orm(primary_key)]
    pub id: i32,
    pub inspection_no: String,
    pub sales_order_id: i32,
    pub delivery_id: Option<i32>,
    pub product_name: String,
    pub hs_code: String,
    pub inspection_type: String,
    pub inspection_agency: String,
    pub inspection_date: NaiveDate,
    pub result: String,
    pub report_url: Option<String>,
    pub certificate_no: Option<String>,
    pub certificate_expiry: Option<NaiveDate>,
    pub remarks: Option<String>,
    pub created_by: i32,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}
impl ActiveModelBehavior for ActiveModel {}