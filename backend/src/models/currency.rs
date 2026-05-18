#![allow(dead_code)]

//! 币种/汇率 Model
//!
//! 多币种支持

use chrono::{DateTime, Utc};
use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

/// 币种 Entity
#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Serialize, Deserialize)]
#[sea_orm(table_name = "currencies")]
pub struct Model {
    /// 币种 ID（主键）
    #[sea_orm(primary_key)]
    pub id: i32,

    /// 币种代码（CNY/USD/EUR）
    #[sea_orm(unique)]
    pub code: String,

    /// 币种名称
    pub name: String,

    /// 币种符号
    pub symbol: Option<String>,

    /// 是否本位币
    pub is_base: bool,

    /// 精度（小数位）
    pub precision: i32,

    /// 是否启用
    pub is_active: bool,

    /// 是否删除

    /// 创建时间
    pub created_at: DateTime<Utc>,

    /// 更新时间
    pub updated_at: DateTime<Utc>,
}

/// 币种关联关系
#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}

impl ActiveModelBehavior for ActiveModel {}
