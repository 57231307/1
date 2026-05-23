#![allow(dead_code)]

//! 缸号管理模型（染色批次管理）

use chrono::{DateTime, Utc};
use rust_decimal::Decimal;
use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Serialize, Deserialize)]
#[sea_orm(table_name = "dye_batch")]
pub struct Model {
    #[sea_orm(primary_key)]
    pub id: i32,
    pub batch_no: String,
    pub greige_fabric_id: Option<i32>,
    pub color_no: Option<String>,
    #[sea_orm(column_type = "Decimal(Some((12, 2)))")]
    pub planned_quantity: Option<Decimal>,
    pub status: Option<String>,
    pub started_at: Option<DateTimeWithTimeZone>,
    pub completed_at: Option<DateTimeWithTimeZone>,
    pub is_deleted: Option<bool>,
    pub created_at: DateTimeWithTimeZone,
    pub updated_at: DateTimeWithTimeZone,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {
    #[sea_orm(
        belongs_to = "super::greige_fabric::Entity",
        from = "Column::GreigeFabricId",
        to = "super::greige_fabric::Column::Id",
        on_update = "Cascade",
        on_delete = "SetNull"
    )]
    GreigeFabric,
}

impl ActiveModelBehavior for ActiveModel {}
