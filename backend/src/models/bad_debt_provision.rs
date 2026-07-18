//! 坏账准备计提 Model（V15 P0-B01 Batch 481 创建）
//!
//! 表 bad_debt_provisions：按客户+期间+账龄桶记录坏账准备计提与转回
//! 账龄法：1 年内 5% / 1-2 年 20% / 2-3 年 50% / 3 年以上 100%
//! 状态机：draft → confirmed → reversed（可回转）

use chrono::{DateTime, Utc};
use rust_decimal::Decimal;
use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Eq, Serialize, Deserialize)]
#[sea_orm(table_name = "bad_debt_provisions")]
pub struct Model {
    #[sea_orm(primary_key)]
    pub id: i64,
    pub customer_id: i64,
    pub customer_name: Option<String>,
    pub period_year: i32,
    pub period_month: i32,
    /// 账龄桶：within_1y / 1_to_2y / 2_to_3y / over_3y
    pub aging_bucket: String,
    pub base_amount: Decimal,
    pub provision_rate: Decimal,
    pub provision_amount: Decimal,
    pub voucher_id: Option<i64>,
    /// 状态：draft / confirmed / reversed
    pub status: String,
    pub created_by: i32,
    pub confirmed_at: Option<DateTime<Utc>>,
    pub reversed_at: Option<DateTime<Utc>>,
    pub reverse_voucher_id: Option<i64>,
    pub remark: Option<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {
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
    CreatedBy,
}

impl Related<super::customer::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::Customer.def()
    }
}

impl Related<super::user::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::CreatedBy.def()
    }
}

impl ActiveModelBehavior for ActiveModel {}
