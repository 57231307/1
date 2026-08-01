#![allow(dead_code)]
//! 出口商品检验记录模型（V15 P2 B08-P2-5）
//!
//! 依据：《进出口商品检验法》及实施条例
//! 业务：出口纺织品法定检验/公证鉴定/免检管理

use chrono::{DateTime, Utc};
use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Serialize, Deserialize)]
#[sea_orm(table_name = "export_inspections")]
pub struct Model {
    #[sea_orm(primary_key)]
    pub id: i32,
    pub inspection_no: String,
    pub sales_order_id: Option<i32>,
    pub delivery_id: Option<i32>,
    pub product_name: String,
    pub hs_code: Option<String>,
    pub inspection_type: String,
    pub inspection_agency: Option<String>,
    pub inspection_date: Option<chrono::NaiveDate>,
    pub result: Option<String>,
    pub report_url: Option<String>,
    pub certificate_no: Option<String>,
    pub certificate_expiry: Option<chrono::NaiveDate>,
    pub remarks: Option<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}
impl ActiveModelBehavior for ActiveModel {}
