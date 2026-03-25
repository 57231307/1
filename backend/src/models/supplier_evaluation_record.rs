use chrono::{DateTime, NaiveDate, Utc};
use rust_decimal::Decimal;
use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

/// 供应商评估记录模型
/// 用于记录每次供应商评估的具体评分数据
#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Serialize, Deserialize)]
#[sea_orm(table_name = "supplier_evaluation_records")]
pub struct Model {
    #[sea_orm(primary_key)]
    pub id: i32,
    /// 供应商ID
    pub supplier_id: i32,
    /// 评估周期（如：2024Q1）
    #[sea_orm(column_name = "evaluation_period")]
    pub evaluation_period: String,
    /// 评估指标ID
    #[sea_orm(column_name = "indicator_id")]
    pub indicator_id: i32,
    /// 得分
    pub score: Decimal,
    /// 满分
    #[sea_orm(column_name = "max_score")]
    pub max_score: Option<i32>,
    /// 加权得分
    #[sea_orm(column_name = "weighted_score")]
    pub weighted_score: Option<Decimal>,
    /// 评估人ID
    #[sea_orm(column_name = "evaluator_id")]
    pub evaluator_id: Option<i32>,
    /// 评估日期
    #[sea_orm(column_name = "evaluation_date")]
    pub evaluation_date: Option<NaiveDate>,
    /// 备注
    pub remark: Option<String>,
    /// 创建时间
    #[sea_orm(column_type = "Timestamp")]
    #[sea_orm(column_name = "created_at")]
    pub created_at: DateTime<Utc>,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}

impl ActiveModelBehavior for ActiveModel {}
