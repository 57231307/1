#![allow(dead_code)]

//! 业务追溯 Model
//!
//! 业务追溯模块

use chrono::{DateTime, Utc};
use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

/// 业务追溯 Entity
#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Serialize, Deserialize)]
#[sea_orm(table_name = "business_traces")]
pub struct Model {
    /// 追溯 ID（主键）
    #[sea_orm(primary_key)]
    pub id: i32,

    /// 批次号
    pub batch_no: String,

    /// 产品 ID（外键）
    pub product_id: i32,

    /// 仓库 ID（外键）
    pub warehouse_id: Option<i32>,

    /// 当前环节
    pub current_stage: String,

    /// 数量
    pub quantity: rust_decimal::Decimal,

    /// 单位
    pub unit: String,

    /// 备注
    pub remarks: Option<String>,

    /// 创建时间
    pub is_deleted: bool,
    pub created_at: DateTime<Utc>,

    /// 更新时间
    pub updated_at: DateTime<Utc>,
}

/// 业务追溯关联关系
#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}

impl ActiveModelBehavior for ActiveModel {}
