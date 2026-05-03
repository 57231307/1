#![allow(dead_code)]

//! 染色配方管理模型

use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Serialize, Deserialize)]
#[sea_orm(table_name = "dye_recipe")]
pub struct Model {
    #[sea_orm(primary_key)]
    pub id: i32,
    pub recipe_no: String,
    pub color_code: String,
    pub color_name: String,
    pub fabric_type: Option<String>,
    pub dye_type: Option<String>,
    pub chemical_formula: Option<String>,
    pub temperature: Option<Decimal>,
    pub time_minutes: Option<i32>,
    pub ph_value: Option<Decimal>,
    pub liquor_ratio: Option<Decimal>,
    pub auxiliaries: Option<serde_json::Value>,
    pub status: String,
    pub version: Option<i32>,
    pub parent_recipe_id: Option<i32>,
    pub approved_by: Option<i32>,
    pub approved_at: Option<DateTimeUtc>,
    pub remarks: Option<String>,
    pub created_by: Option<i32>,
    pub is_deleted: bool,
    pub created_at: DateTimeUtc,
    pub updated_at: DateTimeUtc,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {
    #[sea_orm(
        belongs_to = "super::user::Entity",
        from = "Column::ApprovedBy",
        to = "super::user::Column::Id",
        on_update = "Cascade",
        on_delete = "SetNull"
    )]
    ApprovedBy,
    #[sea_orm(
        belongs_to = "super::user::Entity",
        from = "Column::CreatedBy",
        to = "super::user::Column::Id",
        on_update = "Cascade",
        on_delete = "SetNull"
    )]
    CreatedBy,
}

impl ActiveModelBehavior for ActiveModel {}
