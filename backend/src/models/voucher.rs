#![allow(dead_code, unused_imports, unused_variables)]
//! 凭证 Entity
//!
//! 对应数据库表：vouchers

use chrono::{DateTime, NaiveDate, Utc};
use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Serialize, Deserialize)]
#[sea_orm(table_name = "vouchers")]
pub struct Model {
    #[sea_orm(primary_key)]
    pub id: i32,
    pub voucher_no: String,
    pub voucher_type: String,
    pub voucher_date: NaiveDate,

    // 凭证来源
    pub source_type: Option<String>,
    pub source_module: Option<String>,
    pub source_bill_id: Option<i32>,
    pub source_bill_no: Option<String>,

    // 面料行业字段
    pub batch_no: Option<String>,
    pub color_no: Option<String>,
    pub dye_lot_no: Option<String>,
    pub workshop: Option<String>,
    pub production_order_no: Option<String>,

    // 双计量单位
    pub quantity_meters: Option<Decimal>,
    pub quantity_kg: Option<Decimal>,
    pub gram_weight: Option<Decimal>,

    // 状态
    pub status: String,
    pub attachment_count: i32,

    // 审核
    pub created_by: i32,
    pub reviewed_by: Option<i32>,
    pub reviewed_at: Option<DateTime<Utc>>,
    pub posted_by: Option<i32>,
    pub posted_at: Option<DateTime<Utc>>,

    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {
    #[sea_orm(has_many = "super::voucher_item::Entity")]
    Items,
}

impl Related<super::voucher_item::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::Items.def()
    }
}

impl ActiveModelBehavior for ActiveModel {}
