#![allow(dead_code)]
//! 缸号主表：管理染色生产流程（染缸批次状态/计划产量/起止时间）。
//! 与 batch_dye_lot 互补：本表聚焦"缸"维度的生产执行主表，batch_dye_lot 聚焦"批次-染料配方"维度明细。

use rust_decimal::Decimal;
use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Serialize, Deserialize, Default)]
#[sea_orm(table_name = "dye_batch")]
pub struct Model {
    #[sea_orm(primary_key)]
    pub id: i32,
    pub batch_no: String,
    pub greige_fabric_id: Option<i32>,
    pub color_no: Option<String>,
    // V15 P0-F01：染色批号（dye_lot_no），面料四维标识之一
    // 术语：与 batch_no（缸号=染色批次号，同一概念不同叫法）不同，
    //       dye_lot_no 是面料行业 lot 概念，指同产品同颜色不同时间/染缸的批次标识
    // 历史：dye_batch 主表缺失此字段，导致四层级联断裂、成本归集不完整
    // 迁移：048_add_dye_lot_no_to_dye_batch.sql 新增字段，历史数据回填 'DEFAULT'
    pub dye_lot_no: String,
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
