#![allow(dead_code)]
//! 原产地证书模型（V15 P2 B08-P2-5）
//!
//! 依据：《进出口货物原产地条例》
//! 业务：一般原产地证(CO)/普惠制产地证(GSP)/区域性优惠产地证(如 RCEP)

use chrono::{DateTime, NaiveDate, Utc};
use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Serialize, Deserialize)]
#[sea_orm(table_name = "certificates_of_origin")]
pub struct Model {
    #[sea_orm(primary_key)]
    pub id: i32,
    pub certificate_no: String,
    pub sales_order_id: Option<i32>,
    pub certificate_type: String,
    pub exporter: Option<String>,
    pub importer: Option<String>,
    pub transport_method: Option<String>,
    pub issue_date: NaiveDate,
    pub issuing_authority: Option<String>,
    pub certificate_url: Option<String>,
    pub remarks: Option<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}
impl ActiveModelBehavior for ActiveModel {}
