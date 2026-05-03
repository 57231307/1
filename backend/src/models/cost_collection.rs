#![allow(dead_code)]

//! 成本归集 Entity
//!
//! 对应数据库表：cost_collections

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

    // 成本对象
    pub cost_object_type: Option<String>,
    pub cost_object_id: Option<i32>,
    pub cost_object_no: Option<String>,

    // 面料行业字段
    pub batch_no: Option<String>,
    pub color_no: Option<String>,
    pub dye_lot_no: Option<String>,
    pub workshop: Option<String>,
    pub production_order_no: Option<String>,

    // 成本构成
    pub direct_material: Decimal,
    pub direct_labor: Decimal,
    pub manufacturing_overhead: Decimal,
    pub processing_fee: Decimal,
    pub dyeing_fee: Decimal,

    // 总金额
    pub total_cost: Decimal,

    // 双计量单位
    pub output_quantity_meters: Option<Decimal>,
    pub output_quantity_kg: Option<Decimal>,
    pub unit_cost_meters: Option<Decimal>,
    pub unit_cost_kg: Option<Decimal>,

    // 状态
    pub status: String,

    pub created_by: i32,
    pub is_deleted: bool,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}

impl ActiveModelBehavior for ActiveModel {}
