#![allow(dead_code)]
//! 疵点明细模型（fabric_defect_record 表）
//!
//! v14 批次 426：验布打卷流程贯通
//! 依据：面料行业真实业务调研文档 §12.4 验布打卷与成品入库
//! 真实业务：验布机采集疵点 → 系统按评分制式自动计算扣分 → 汇总到验布记录
//! 疵点类型：断经/油污/色花/破洞/纬斜/横档/色差/窄封/折痕/染色不均匀等

use rust_decimal::Decimal;
use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

/// 疵点明细模型（12 种疵点类型标准编码；四分制 ≤3寸=1/3-6寸=2/6-9寸=3/>9寸=4 破洞连续=4；十分制经纬向分档 1-10 破洞=10；同码内扣分不超制式上限）
#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Serialize, Deserialize, Default)]
#[sea_orm(table_name = "fabric_defect_record")]
pub struct Model {
    #[sea_orm(primary_key)]
    pub id: i32,

    /// 关联验布记录 ID
    pub inspection_id: i32,

    /// 疵点类型（标准编码）：broken_end断经/oil_stain油污/color_spot色花/hole破洞/skew_lane纬斜/streak横档/color_diff色差/narrow_width窄封/crease折痕/uneven_dye染色不均匀/lint飞花/other其他
    pub defect_type: String,

    /// 疵点位置（码数，验布机码表读数）
    #[sea_orm(column_type = "Decimal(Some((10, 2)))")]
    pub position_yards: Decimal,

    /// 疵点长度（英寸，评分计算依据）
    #[sea_orm(column_type = "Decimal(Some((8, 2)))")]
    pub defect_length_inches: Decimal,

    /// 方向：warp(经向) / weft(纬向) / other(其他)
    pub direction: String,

    /// 是否破洞（破洞不论大小一律最高扣分：四分制4分/十分制10分）
    pub is_hole: bool,

    /// 是否连续性疵点（横档/色差/窄封/折痕/染色不均匀等，每码最高扣分）
    pub is_continuous: bool,

    /// 是否超过半门幅（十分制纬向评分依据：半门幅以上=10分，以下=5分）
    pub is_half_width: bool,

    /// 扣分（四分制 1/2/3/4，十分制 1/3/5/10）
    pub points: i32,

    /// 疵点描述
    pub description: Option<String>,

    /// 创建时间
    pub created_at: DateTimeWithTimeZone,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {
    /// 多对一：关联验布记录
    #[sea_orm(
        belongs_to = "super::fabric_inspection_record::Entity",
        from = "Column::InspectionId",
        to = "super::fabric_inspection_record::Column::Id"
    )]
    Inspection,
}

impl Related<super::fabric_inspection_record::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::Inspection.def()
    }
}

impl ActiveModelBehavior for ActiveModel {}
