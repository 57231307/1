#![allow(dead_code, unused_imports, unused_variables)]
//! 固定资产处置 Model
//!
//! 固定资产处置模块

use chrono::{DateTime, NaiveDate, Utc};
use rust_decimal::Decimal;
use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

/// 固定资产处置 Entity
#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Serialize, Deserialize)]
#[sea_orm(table_name = "fixed_asset_disposals")]
pub struct Model {
    /// 处置单 ID（主键）
    #[sea_orm(primary_key)]
    pub id: i32,

    /// 处置单号
    #[sea_orm(unique)]
    pub disposal_no: String,

    /// 固定资产 ID（外键）
    pub asset_id: i32,

    /// 处置日期
    pub disposal_date: NaiveDate,

    /// 处置方式：SALE=出售，SCRAP=报废，TRANSFER=转让
    pub disposal_type: String,

    /// 处置数量
    pub quantity: i32,

    /// 处置金额
    pub disposal_amount: Decimal,

    /// 处置原因
    pub disposal_reason: String,

    /// 处置状态：DRAFT=草稿，APPROVED=已审批，COMPLETED=已完成
    pub status: String,

    /// 备注
    pub remarks: Option<String>,

    /// 创建人 ID
    pub created_by: i32,

    /// 创建时间
    pub created_at: DateTime<Utc>,

    /// 更新时间
    pub updated_at: DateTime<Utc>,
}

/// 固定资产处置关联关系
#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {
    /// 处置 - 固定资产（多对一）
    #[sea_orm(
        belongs_to = "super::fixed_asset::Entity",
        from = "Column::AssetId",
        to = "super::fixed_asset::Column::Id"
    )]
    FixedAsset,

    /// 处置 - 用户（创建人，多对一）
    #[sea_orm(
        belongs_to = "super::user::Entity",
        from = "Column::CreatedBy",
        to = "super::user::Column::Id"
    )]
    Creator,
}

impl Related<super::fixed_asset::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::FixedAsset.def()
    }
}

impl Related<super::user::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::Creator.def()
    }
}

impl ActiveModelBehavior for ActiveModel {}
