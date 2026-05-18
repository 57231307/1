#![allow(dead_code)]

//! MRP计算结果 Model
//!
//! 物料需求计划计算结果存储

use chrono::{DateTime, NaiveDate, Utc};
use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

/// MRP需求来源类型
#[derive(Debug, Clone, PartialEq, Eq, EnumIter, DeriveActiveEnum, Serialize, Deserialize)]
#[sea_orm(rs_type = "String", db_type = "String(StringLen::N(20))")]
pub enum MrpSourceType {
    /// 销售订单
    #[sea_orm(string_value = "SALES_ORDER")]
    SalesOrder,
    /// 预测
    #[sea_orm(string_value = "FORECAST")]
    Forecast,
    /// 安全库存
    #[sea_orm(string_value = "SAFETY_STOCK")]
    SafetyStock,
}

/// MRP计算结果状态
#[derive(Debug, Clone, PartialEq, Eq, EnumIter, DeriveActiveEnum, Serialize, Deserialize)]
#[sea_orm(rs_type = "String", db_type = "String(StringLen::N(20))")]
pub enum MrpResultStatus {
    /// 计划中
    #[sea_orm(string_value = "PLANNED")]
    Planned,
    /// 已确认
    #[sea_orm(string_value = "CONFIRMED")]
    Confirmed,
    /// 已下达
    #[sea_orm(string_value = "RELEASED")]
    Released,
    /// 已完成
    #[sea_orm(string_value = "COMPLETED")]
    Completed,
}

/// MRP计算结果 Entity
#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Serialize, Deserialize)]
#[sea_orm(table_name = "mrp_results")]
pub struct Model {
    /// MRP结果 ID（主键）
    #[sea_orm(primary_key)]
    pub id: i32,

    /// 计算编号
    #[sea_orm(unique)]
    pub calculation_no: String,

    /// 产品 ID
    pub product_id: i32,

    /// 需求数量
    pub required_quantity: Decimal,

    /// 需求日期
    pub required_date: Option<NaiveDate>,

    /// 来源类型
    pub source_type: String,

    /// 来源 ID
    pub source_id: Option<i32>,

    /// 计划订单数量
    pub planned_order_quantity: Option<Decimal>,

    /// 计划订单日期
    pub planned_order_date: Option<NaiveDate>,

    /// 状态
    pub status: String,

    /// 备注
    pub remarks: Option<String>,

    /// 是否删除

    /// 创建时间
    pub created_at: DateTime<Utc>,

    /// 更新时间
    pub updated_at: DateTime<Utc>,
}

/// MRP计算结果关联关系
#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}

impl ActiveModelBehavior for ActiveModel {}
