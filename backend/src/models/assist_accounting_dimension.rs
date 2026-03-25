//! 辅助核算维度 Model
//!
//! 8 个辅助核算维度：批次、色号、缸号、等级、车间、仓库、客户、供应商

use sea_orm::entity::prelude::*;
use chrono::{DateTime, Utc};

/// 辅助核算维度 Entity
#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Eq)]
#[sea_orm(table_name = "assist_accounting_dimensions")]
pub struct Model {
    #[sea_orm(primary_key)]
    pub id: i32,

    /// 维度编码：BATCH, COLOR, DYE_LOT, GRADE, WORKSHOP, WAREHOUSE, CUSTOMER, SUPPLIER
    pub dimension_code: String,

    /// 维度名称
    pub dimension_name: String,

    /// 维度描述
    pub description: Option<String>,

    /// 是否启用
    pub is_active: bool,

    /// 排序顺序
    pub sort_order: i32,

    /// 创建时间
    pub created_at: DateTime<Utc>,

    /// 更新时间
    pub updated_at: DateTime<Utc>,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}

impl ActiveModelBehavior for ActiveModel {}
