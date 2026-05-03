#![allow(dead_code)]

//! 辅助核算汇总 Model
//!
//! 按维度汇总数据

use chrono::{DateTime, Utc};
use rust_decimal::Decimal;
use sea_orm::entity::prelude::*;

/// 辅助核算汇总 Entity
use serde::{Serialize, Deserialize};
#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Eq, Serialize, Deserialize)]
#[sea_orm(table_name = "assist_accounting_summary")]
pub struct Model {
    #[sea_orm(primary_key)]
    pub id: i32,

    /// 会计期间（格式：YYYY-MM）
    pub accounting_period: String,

    /// 维度编码
    pub dimension_code: String,

    /// 维度值 ID（如批次 ID、色号 ID 等）
    pub dimension_value_id: i32,

    /// 维度值名称
    pub dimension_value_name: String,

    /// 会计科目 ID
    pub account_subject_id: i32,

    /// 借方金额合计
    #[sea_orm(column_type = "Decimal(Some((12, 2)))")]
    pub total_debit: Decimal,

    /// 贷方金额合计
    #[sea_orm(column_type = "Decimal(Some((12, 2)))")]
    pub total_credit: Decimal,

    /// 数量（米）合计
    #[sea_orm(column_type = "Decimal(Some((12, 2)))")]
    pub total_quantity_meters: Decimal,

    /// 数量（公斤）合计
    #[sea_orm(column_type = "Decimal(Some((12, 2)))")]
    pub total_quantity_kg: Decimal,

    /// 记录数
    pub record_count: i64,

    /// 创建时间
    pub is_deleted: bool,
    pub created_at: DateTime<Utc>,

    /// 更新时间
    pub updated_at: DateTime<Utc>,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}

impl ActiveModelBehavior for ActiveModel {}
