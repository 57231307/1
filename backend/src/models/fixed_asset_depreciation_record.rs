#![allow(dead_code)]
// TODO(tech-debt): 业务接入或重评估后逐项移除；rustc 1.94+ 编译时由编译器报告具体死代码位置。

//! 固定资产折旧期间记录 Entity（批次 88 PH-2 占位符实现）
//!
//! 按期间记录每笔折旧计提明细，支持审计追溯。
//! (asset_id, period) 唯一约束防止同一资产同一期间重复计提。

use chrono::{DateTime, Utc};
use rust_decimal::Decimal;
use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

/// 固定资产折旧期间记录 Entity
#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Serialize, Deserialize)]
#[sea_orm(table_name = "fixed_asset_depreciation_records")]
pub struct Model {
    /// 记录 ID（主键）
    #[sea_orm(primary_key)]
    pub id: i32,

    /// 固定资产 ID（外键）
    pub asset_id: i32,

    /// 折旧期间（YYYY-MM 格式）
    pub period: String,

    /// 本期折旧额
    pub depreciation_amount: Decimal,

    /// 本期前累计折旧
    pub accumulated_before: Decimal,

    /// 本期后累计折旧
    pub accumulated_after: Decimal,

    /// 本期前账面净值
    pub net_value_before: Option<Decimal>,

    /// 本期后账面净值
    pub net_value_after: Option<Decimal>,

    /// 折旧方法（如 straight_line）
    pub depreciation_method: Option<String>,

    /// 创建人 ID
    pub created_by: i32,

    /// 创建时间
    pub created_at: DateTime<Utc>,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {
    /// 记录 - 固定资产（多对一）
    #[sea_orm(
        belongs_to = "super::fixed_asset::Entity",
        from = "Column::AssetId",
        to = "super::fixed_asset::Column::Id"
    )]
    FixedAsset,

    /// 记录 - 用户（创建人，多对一）
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
