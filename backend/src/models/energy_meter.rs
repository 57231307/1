//! 能源计量设备模型（energy_meter 表）
//!
//! v14 批次 428：能耗管理贯通
//! 依据：面料行业真实业务调研文档 §12.6 能耗管理
//! 真实业务：每个车间/机台安装的能源计量设备（电表/水表/汽表）
//! IoT 对接：智能电表/蒸汽流量计/水质监测仪实时采集

use rust_decimal::Decimal;
use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

/// 能源计量设备模型
///
/// 真实业务要点：
/// - 每个车间/机台安装独立的计量设备
/// - 支持 IoT 对接（PLC/智能网关）
/// - 维护当前读数与上次读数，便于计算消耗量
#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Serialize, Deserialize, Default)]
#[sea_orm(table_name = "energy_meter")]
pub struct Model {
    #[sea_orm(primary_key)]
    pub id: i32,

    /// 计量设备编号：EM-YYYYMMDDHHMMSS-NNN
    pub meter_no: String,
    /// 计量设备名称
    pub meter_name: String,
    /// 能源类型：water(水) / electricity(电) / steam(蒸汽) / gas(天然气) / compressed_air(压缩空气)
    pub meter_type: String,
    /// 所属车间
    pub workshop: Option<String>,
    /// 关联设备 ID（机台级电表时关联 equipment 表）
    pub equipment_id: Option<i32>,
    /// 设备名称（冗余）
    pub equipment_name: Option<String>,
    /// 安装位置
    pub location: Option<String>,
    /// IoT 设备 ID（对接 PLC/智能网关）
    pub iot_device_id: Option<String>,
    /// 计量单位（吨/度/立方米）
    pub unit: String,
    /// 当前读数
    #[sea_orm(column_type = "Decimal(Some((14, 2)))")]
    pub current_reading: Decimal,
    /// 上次读数
    #[sea_orm(column_type = "Decimal(Some((14, 2)))")]
    pub previous_reading: Decimal,
    /// 上次读数时间
    pub last_reading_at: Option<DateTimeWithTimeZone>,
    /// 单价（元/单位）
    #[sea_orm(column_type = "Decimal(Some((12, 4)))")]
    pub unit_price: Decimal,
    /// 状态：active(启用) / inactive(停用) / maintenance(维护中)
    pub status: String,
    /// 备注
    pub remarks: Option<String>,

    // 软删除与审计
    pub is_deleted: bool,
    pub created_by: Option<i32>,
    pub created_at: DateTimeWithTimeZone,
    pub updated_at: DateTimeWithTimeZone,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {
    /// 一对多：能耗记录
    #[sea_orm(has_many = "super::energy_consumption_record::Entity")]
    ConsumptionRecords,
}

impl Related<super::energy_consumption_record::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::ConsumptionRecords.def()
    }
}

impl ActiveModelBehavior for ActiveModel {}
