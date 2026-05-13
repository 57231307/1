#![allow(dead_code)]
#![allow(dead_code, unused_imports, unused_variables)]
//! 批次追溯日志 Model
//!
//! 批次追溯日志模块

use chrono::{DateTime, Utc};
use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

/// 批次追溯日志 Entity
#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Serialize, Deserialize)]
#[sea_orm(table_name = "batch_trace_log")]
pub struct Model {
    /// 日志 ID（主键）
    #[sea_orm(primary_key)]
    pub id: i32,

    /// 批次号
    pub batch_no: String,

    /// 操作类型：CREATE=创建，TRANSFER=转移，ADJUST=调整
    pub operation_type: String,

    /// 源单据类型
    pub source_type: Option<String>,

    /// 源单据 ID
    pub source_id: Option<i32>,

    /// 源单据号
    pub source_no: Option<String>,

    /// 操作数量
    pub quantity: Option<rust_decimal::Decimal>,

    /// 操作前库存
    pub quantity_before: Option<rust_decimal::Decimal>,

    /// 操作后库存
    pub quantity_after: Option<rust_decimal::Decimal>,

    /// 备注
    pub remarks: Option<String>,

    /// 操作人 ID
    pub operated_by: Option<i32>,

    /// 操作时间
    pub operated_at: DateTime<Utc>,
    pub is_deleted: bool,
}

/// 批次追溯日志关联关系
#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}

impl ActiveModelBehavior for ActiveModel {}
