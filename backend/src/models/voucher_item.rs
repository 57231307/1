#![allow(dead_code)]

//! 凭证分录 Entity
//!
//! 对应数据库表：voucher_items

use chrono::{DateTime, Utc};
use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Serialize, Deserialize)]
#[sea_orm(table_name = "voucher_items")]
pub struct Model {
    #[sea_orm(primary_key)]
    pub id: i32,
    pub voucher_id: i32,
    pub line_no: i32,

    // 科目
    pub subject_code: String,
    pub subject_name: String,

    // 金额
    pub debit: Decimal,
    pub credit: Decimal,

    // 摘要
    pub summary: Option<String>,

    // 辅助核算
    pub assist_customer_id: Option<i32>,
    pub assist_supplier_id: Option<i32>,
    pub assist_department_id: Option<i32>,
    pub assist_employee_id: Option<i32>,
    pub assist_project_id: Option<i32>,
    pub assist_batch_id: Option<i32>,
    pub assist_color_no_id: Option<i32>,
    pub assist_dye_lot_id: Option<i32>,
    pub assist_grade: Option<String>,
    pub assist_workshop_id: Option<i32>,

    // 双计量单位
    pub quantity_meters: Option<Decimal>,
    pub quantity_kg: Option<Decimal>,
    pub unit_price: Option<Decimal>,

    pub is_deleted: bool,

    pub created_at: DateTime<Utc>,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {
    #[sea_orm(
        belongs_to = "super::voucher::Entity",
        from = "Column::VoucherId",
        to = "super::voucher::Column::Id"
    )]
    Voucher,
}

impl Related<super::voucher::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::Voucher.def()
    }
}

impl ActiveModelBehavior for ActiveModel {}
