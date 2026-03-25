//! 收款单 Entity
//!
//! 对应数据库表：ar_collections

use sea_orm::entity::prelude::*;
use chrono::{DateTime, NaiveDate, Utc};
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Serialize, Deserialize)]
#[sea_orm(table_name = "ar_collections")]
pub struct Model {
    #[sea_orm(primary_key)]
    pub id: i32,
    pub collection_no: String,
    pub collection_date: NaiveDate,

    // 客户信息
    pub customer_id: i32,
    pub customer_name: Option<String>,

    // 收款信息
    pub collection_amount: Decimal,
    pub collection_method: Option<String>,
    pub bank_account: Option<String>,
    pub check_no: Option<String>,

    // 关联收款申请
    pub request_id: Option<i32>,
    pub request_no: Option<String>,

    // 状态
    pub status: String,

    // 确认
    pub confirmed_by: Option<i32>,
    pub confirmed_at: Option<DateTime<Utc>>,

    pub created_by: i32,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}

impl ActiveModelBehavior for ActiveModel {}
