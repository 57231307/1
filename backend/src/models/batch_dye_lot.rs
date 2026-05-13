#![allow(dead_code)]

//! 批次染色批次 Model
//!
//! 批次染色批次模块

use chrono::{DateTime, NaiveDate, Utc};
use rust_decimal::Decimal;
use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

/// 批次染色批次 Entity
#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Serialize, Deserialize)]
#[sea_orm(table_name = "batch_dye_lot")]
pub struct Model {
    /// 批次染色 ID（主键）
    #[sea_orm(primary_key)]
    pub id: i32,

    /// 批次号
    #[sea_orm(unique)]
    pub batch_no: String,

    /// 产品 ID（外键）
    pub product_id: i32,

    /// 颜色 ID（外键）
    pub color_id: Option<i32>,

    /// 染色批次号
    pub dye_lot_no: String,

    /// 染色日期
    pub dye_date: NaiveDate,

    /// 染色数量
    pub quantity: Decimal,

    /// 颜色代码
    pub color_code: Option<String>,

    /// 状态：ACTIVE=活跃，COMPLETED=已完成
    pub status: String,

    /// 备注
    pub remarks: Option<String>,

    /// 创建时间
    pub is_deleted: bool,
    pub created_at: DateTime<Utc>,

    /// 更新时间
    pub updated_at: DateTime<Utc>,
}

/// 批次染色批次关联关系
#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {
    /// 批次 - 产品（多对一）
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
