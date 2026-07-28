//! 职业危害因素检测记录模型（occupational_hazard_monitorings 表）
//!
//! V15 P1 batch-08 缺陷 24：职业健康合规
//! 依据：《职业病防治法》第26条 印染车间苯/甲醛/噪声/粉尘检测

use rust_decimal::Decimal;
use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

/// 职业危害因素检测记录模型
///
/// 真实业务：定期检测印染车间职业危害因素，超标立即预警
#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Serialize, Deserialize, Default)]
#[sea_orm(table_name = "occupational_hazard_monitorings")]
pub struct Model {
    #[sea_orm(primary_key)]
    pub id: i32,
    /// 危害类型：chemical(化学) / physical(物理) / dust(粉尘) / biological(生物)
    pub hazard_type: String,
    /// 危害名称：苯/甲醛/噪声/粉尘
    pub hazard_name: String,
    /// 监测点
    pub monitoring_point: String,
    /// 实测值
    #[sea_orm(column_type = "Decimal(Some((14, 4)))")]
    pub measured_value: Decimal,
    /// 单位
    pub unit: String,
    /// 限值
    #[sea_orm(column_type = "Decimal(Some((14, 4)))")]
    pub limit_value: Decimal,
    /// 是否超标
    pub is_exceeding: bool,
    /// 超标倍数
    #[sea_orm(column_type = "Decimal(Some((10, 4)))")]
    pub exceeding_ratio: Option<Decimal>,
    /// 监测日期
    pub monitoring_date: chrono::NaiveDate,
    /// 监测机构
    pub monitoring_organization: Option<String>,
    /// 监测方法
    pub monitoring_method: Option<String>,
    /// 监测报告URL
    pub report_url: Option<String>,
    pub remarks: Option<String>,
    pub created_by: Option<i32>,
    pub created_at: DateTimeWithTimeZone,
    pub updated_at: DateTimeWithTimeZone,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}

impl ActiveModelBehavior for ActiveModel {}
