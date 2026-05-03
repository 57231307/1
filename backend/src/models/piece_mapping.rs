#![allow(dead_code)]

//! 匹号映射 Model
//!
//! 匹号映射模块

use chrono::{DateTime, Utc};
use rust_decimal::Decimal;
use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

/// 匹号映射 Entity
#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Serialize, Deserialize)]
#[sea_orm(table_name = "piece_mapping")]
pub struct Model {
    /// 映射 ID（主键）
    #[sea_orm(primary_key)]
    pub id: i32,

    /// 批次号
    pub batch_no: String,

    /// 产品 ID（外键）
    pub product_id: i32,

    /// 匹号
    pub piece_no: String,

    /// 长度（米）
    pub length: Decimal,

    /// 重量（千克）
    pub weight: Option<Decimal>,

    /// 状态：ACTIVE=活跃，USED=已使用
    pub status: String,

    /// 备注
    pub remarks: Option<String>,

    /// 创建时间
    pub is_deleted: bool,
    pub created_at: DateTime<Utc>,

    /// 更新时间
    pub updated_at: DateTime<Utc>,
}

/// 匹号映射关联关系
#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {
    /// 映射 - 产品（多对一）
    #[sea_orm(
        belongs_to = "super::product::Entity",
        from = "Column::ProductId",
        to = "super::product::Column::Id"
    )]
    Product,
}

impl Related<super::product::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::Product.def()
    }
}

impl ActiveModelBehavior for ActiveModel {}
