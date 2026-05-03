#![allow(dead_code)]

use chrono::{DateTime, NaiveDate, Utc};
use rust_decimal::Decimal;
use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

/// 财务分析结果模型
/// 用于存储每次财务分析的具体结果数据
#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Serialize, Deserialize)]
#[sea_orm(table_name = "financial_analysis_results")]
pub struct Model {
    #[sea_orm(primary_key)]
    pub id: i32,
    /// 分析类型
    #[sea_orm(column_name = "analysis_type")]
    pub analysis_type: String,
    /// 周期（如：2024-01）
    pub period: String,
    /// 指标ID
    #[sea_orm(column_name = "indicator_id")]
    pub indicator_id: i32,
    /// 实际值
    #[sea_orm(column_name = "indicator_value")]
    pub indicator_value: Decimal,
    /// 目标值
    #[sea_orm(column_name = "target_value")]
    pub target_value: Option<Decimal>,
    /// 差异
    pub variance: Option<Decimal>,
    /// 差异率
    #[sea_orm(column_name = "variance_rate")]
    pub variance_rate: Option<Decimal>,
    /// 趋势方向
    pub trend: Option<String>,
    /// 分析日期
    #[sea_orm(column_name = "analysis_date")]
    pub analysis_date: Option<NaiveDate>,
    /// 创建人
    #[sea_orm(column_name = "created_by")]
    pub created_by: Option<i32>,
    /// 创建时间
    #[sea_orm(column_type = "Timestamp")]
    #[sea_orm(column_name = "created_at")]
    pub is_deleted: bool,
    pub created_at: DateTime<Utc>,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}

impl ActiveModelBehavior for ActiveModel {}
