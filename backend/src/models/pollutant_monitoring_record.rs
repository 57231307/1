#![allow(dead_code)]
//! 污染物监测记录模型（pollutant_monitoring_records 表）
//!
//! V15 P1 batch-08 缺陷 19：废水/废气/固废排放监测
//! 依据：《水污染防治法》《大气污染防治法》《固废污染防治法》

use rust_decimal::Decimal;
use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

/// 污染物监测记录模型
///
/// 真实业务：定期/实时监测污染物排放浓度，超标立即预警
#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Serialize, Deserialize, Default)]
#[sea_orm(table_name = "pollutant_monitoring_records")]
pub struct Model {
    #[sea_orm(primary_key)]
    pub id: i32,
    /// 监测类型：wastewater(废水) / exhaust(废气) / noise(噪声) / solid_waste(固废)
    pub monitoring_type: String,
    /// 监测点（如"总排口"/"定型机排气筒"/"厂界东"）
    pub monitoring_point: String,
    /// 污染物名称：COD/氨氮/色度/VOCs/噪声/污泥
    pub pollutant_name: String,
    /// 实测值
    #[sea_orm(column_type = "Decimal(Some((14, 4)))")]
    pub measured_value: Decimal,
    /// 单位：mg/L, mg/m³, dB, 吨
    pub unit: String,
    /// 排放限值
    #[sea_orm(column_type = "Decimal(Some((14, 4)))")]
    pub limit_value: Decimal,
    /// 是否超标
    pub is_exceeding: bool,
    /// 超标倍数
    #[sea_orm(column_type = "Decimal(Some((10, 4)))")]
    pub exceeding_ratio: Option<Decimal>,
    /// 监测时间
    pub monitoring_time: DateTimeWithTimeZone,
    /// 监测方法
    pub monitoring_method: Option<String>,
    /// 监测设备ID
    pub equipment_id: Option<i32>,
    /// 操作员ID
    pub operator_id: Option<i32>,
    pub remarks: Option<String>,
    pub created_at: DateTimeWithTimeZone,
    pub updated_at: DateTimeWithTimeZone,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}

impl ActiveModelBehavior for ActiveModel {}
