#![allow(dead_code)]
// TODO(tech-debt): 业务接入或重评估后逐项移除；rustc 1.94+ 编译时由编译器报告具体死代码位置。
// TODO(tech-debt): 业务接入后逐项移除此标注；rustc 1.94+ 编译时由编译器报告具体死代码位置。

//! 坯布管理模型（原料布匹管理）

use chrono::NaiveDate;
use rust_decimal::Decimal;
use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Serialize, Deserialize)]
#[sea_orm(table_name = "greige_fabric")]
pub struct Model {
    #[sea_orm(primary_key)]
    pub id: i32,
    pub fabric_no: String,
    pub fabric_name: String,
    pub product_id: Option<i32>,
    pub supplier_id: Option<i32>,
    pub composition: Option<String>,
    pub yarn_count: Option<String>,
    pub density: Option<String>,
    #[sea_orm(column_type = "Decimal(Some((10, 2)))")]
    pub width: Option<Decimal>,
    #[sea_orm(column_type = "Decimal(Some((10, 2)))")]
    pub gram_weight: Option<Decimal>,
    pub structure: Option<String>,
    pub production_date: Option<NaiveDate>,
    pub batch_no: Option<String>,
    #[sea_orm(column_type = "Decimal(Some((12, 2)))")]
    pub quantity_meters: Option<Decimal>,
    #[sea_orm(column_type = "Decimal(Some((12, 2)))")]
    pub quantity_kg: Option<Decimal>,
    pub warehouse_id: Option<i32>,
    pub status: Option<String>,
    pub is_deleted: Option<bool>,
    pub created_at: DateTimeWithTimeZone,
    pub updated_at: DateTimeWithTimeZone,
    pub fabric_type: Option<String>,
    pub color_code: Option<String>,
    #[sea_orm(column_type = "Decimal(Some((10, 2)))")]
    pub width_cm: Option<Decimal>,
    #[sea_orm(column_type = "Decimal(Some((10, 2)))")]
    pub weight_kg: Option<Decimal>,
    #[sea_orm(column_type = "Decimal(Some((12, 2)))")]
    pub length_m: Option<Decimal>,
    pub location: Option<String>,
    pub quality_grade: Option<String>,
    pub purchase_date: Option<NaiveDate>,
    pub remarks: Option<String>,
    pub created_by: Option<i32>,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {
    #[sea_orm(
        belongs_to = "super::product::Entity",
        from = "Column::ProductId",
        to = "super::product::Column::Id",
        on_update = "Cascade",
        on_delete = "SetNull"
    )]
    Product,
    #[sea_orm(
        belongs_to = "super::supplier::Entity",
        from = "Column::SupplierId",
        to = "super::supplier::Column::Id",
        on_update = "Cascade",
        on_delete = "SetNull"
    )]
    Supplier,
    #[sea_orm(
        belongs_to = "super::warehouse::Entity",
        from = "Column::WarehouseId",
        to = "super::warehouse::Column::Id",
        on_update = "Cascade",
        on_delete = "SetNull"
    )]
    Warehouse,
}

impl ActiveModelBehavior for ActiveModel {}
