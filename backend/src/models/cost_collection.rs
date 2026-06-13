#![allow(dead_code)]
// TODO(tech-debt): 业务接入或重评估后逐项移除；rustc 1.94+ 编译时由编译器报告具体死代码位置。
// TODO(tech-debt): 业务接入后逐项移除此标注；rustc 1.94+ 编译时由编译器报告具体死代码位置。

use chrono::{DateTime, NaiveDate, Utc};
use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Serialize, Deserialize)]
#[sea_orm(table_name = "cost_collections")]
pub struct Model {
    #[sea_orm(primary_key)]
    pub id: i32,
    pub collection_no: String,
    pub collection_date: NaiveDate,

    pub cost_object_type: Option<String>,
    pub cost_object_id: Option<i32>,
    pub cost_object_no: Option<String>,

    pub batch_no: Option<String>,
    pub color_no: Option<String>,
    pub dye_lot_no: Option<String>,
    pub workshop: Option<String>,
    pub production_order_no: Option<String>,

    pub direct_material: Decimal,
    pub direct_labor: Decimal,
    pub manufacturing_overhead: Decimal,
    pub processing_fee: Decimal,
    pub dyeing_fee: Decimal,

    pub total_cost: Decimal,

    pub output_quantity_meters: Option<Decimal>,
    pub output_quantity_kg: Option<Decimal>,
    pub unit_cost_meters: Option<Decimal>,
    pub unit_cost_kg: Option<Decimal>,

    pub status: String,

    pub created_by: Option<i32>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}

impl ActiveModelBehavior for ActiveModel {}
