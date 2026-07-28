#![allow(dead_code)]
//! 污染物排放记录模型（pollutant_discharge_records 表）
//!
//! V15 P1 batch-08 缺陷 15：环保税核算
//! 依据：《环境保护税法》印染企业废水/废气/固废排放

use rust_decimal::Decimal;
use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

/// 污染物排放记录模型（按月记录污染物排放量，作为环保税核算基础）
#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Serialize, Deserialize, Default)]
#[sea_orm(table_name = "pollutant_discharge_records")]
pub struct Model {
    #[sea_orm(primary_key)]
    pub id: i32,
    /// 排放类型：wastewater(废水) / exhaust(废气) / solid_waste(固废)
    pub discharge_type: String,
    /// 污染物名称：COD/氨氮/VOCs/污泥
    pub pollutant_name: String,
    /// 排放量
    #[sea_orm(column_type = "Decimal(Some((14, 4)))")]
    pub discharge_amount: Decimal,
    /// 排放量单位
    pub discharge_unit: String,
    /// 排放浓度
    #[sea_orm(column_type = "Decimal(Some((14, 4)))")]
    pub concentration: Option<Decimal>,
    /// 浓度单位
    pub concentration_unit: Option<String>,
    /// 污染当量数（环保税计税依据）
    #[sea_orm(column_type = "Decimal(Some((14, 4)))")]
    pub tax_unit_equivalent: Option<Decimal>,
    /// 应缴环保税额
    #[sea_orm(column_type = "Decimal(Some((14, 2)))")]
    pub tax_amount: Decimal,
    /// 所属年度
    pub period_year: i32,
    /// 所属月份
    pub period_month: i32,
    /// 监测点
    pub monitoring_point: Option<String>,
    pub remarks: Option<String>,
    pub created_by: Option<i32>,
    pub created_at: DateTimeWithTimeZone,
    pub updated_at: DateTimeWithTimeZone,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}

impl ActiveModelBehavior for ActiveModel {}
