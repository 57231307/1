#![allow(dead_code)]
// TODO(tech-debt): 业务接入或重评估后逐项移除；rustc 1.94+ 编译时由编译器报告具体死代码位置。
// TODO(tech-debt): 业务接入后逐项移除此标注；rustc 1.94+ 编译时由编译器报告具体死代码位置。

//! 染色配方管理模型

use rust_decimal::Decimal;
use sea_orm::entity::prelude::*;
use sea_orm::FromJsonQueryResult;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, FromJsonQueryResult)]
pub struct AuxiliariesItem {
    pub name: String,
    pub amount: rust_decimal::Decimal,
    pub unit: String,
}

#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Serialize, Deserialize, Default)]
#[sea_orm(table_name = "dye_recipe")]
pub struct Model {
    #[sea_orm(primary_key)]
    pub id: i32,
    pub recipe_no: String,
    pub recipe_name: Option<String>,
    pub color_no: Option<String>,
    pub formula: Option<String>,
    #[sea_orm(column_type = "Decimal(Some((5, 2)))")]
    pub temperature: Option<Decimal>,
    pub time_minutes: Option<i32>,
    pub status: Option<String>,
    pub is_deleted: Option<bool>,
    pub created_at: DateTimeWithTimeZone,
    pub updated_at: DateTimeWithTimeZone,
    pub color_code: Option<String>,
    pub color_name: Option<String>,
    pub fabric_type: Option<String>,
    pub dye_type: Option<String>,
    pub chemical_formula: Option<String>,
    #[sea_orm(column_type = "Decimal(Some((5, 2)))")]
    pub ph_value: Option<Decimal>,
    #[sea_orm(column_type = "Decimal(Some((10, 2)))")]
    pub liquor_ratio: Option<Decimal>,
    pub auxiliaries: Option<Vec<AuxiliariesItem>>,
    pub version: Option<i32>,
    pub parent_recipe_id: Option<i32>,
    pub approved_by: Option<i32>,
    pub approved_at: Option<DateTimeWithTimeZone>,
    pub remarks: Option<String>,
    pub created_by: Option<i32>,
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
