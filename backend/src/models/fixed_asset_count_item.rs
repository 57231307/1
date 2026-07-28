#![allow(dead_code)]
//! 固定资产盘点明细 Model
//!
//! V15 P1 17.8-D4：资产盘点明细（每项资产的账实对比记录）

use chrono::{DateTime, Utc};
use rust_decimal::Decimal;
use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

/// 固定资产盘点明细 Entity
#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Serialize, Deserialize)]
#[sea_orm(table_name = "fixed_asset_count_items")]
pub struct Model {
    /// 明细 ID（主键）
    #[sea_orm(primary_key)]
    pub id: i32,

    /// 盘点单 ID（外键）
    pub count_id: i32,

    /// 固定资产 ID（外键）
    pub asset_id: i32,

    /// 资产编号（冗余，便于查询）
    pub asset_no: String,

    /// 资产名称（冗余）
    pub asset_name: String,

    /// 账面原值
    pub book_original_value: Decimal,

    /// 账面净值
    pub book_net_value: Option<Decimal>,

    /// 账面存放地点
    pub book_use_location: Option<String>,

    /// 实际原值（盘点录入）
    pub actual_original_value: Option<Decimal>,

    /// 实际净值（盘点录入）
    pub actual_net_value: Option<Decimal>,

    /// 实际存放地点（盘点录入）
    pub actual_use_location: Option<String>,

    /// 盘点结果：consistent=一致，surplus=盘盈，shortage=盘亏，damaged=毁损
    pub count_result: Option<String>,

    /// 差异类型：surplus/shortage/damaged
    pub variance_type: Option<String>,

    /// 差异金额
    pub variance_amount: Option<Decimal>,

    /// 备注
    pub remarks: Option<String>,

    /// 盘点人 ID
    pub counted_by: Option<i32>,

    /// 盘点时间
    pub counted_at: Option<DateTime<Utc>>,

    /// 创建时间
    pub created_at: DateTime<Utc>,

    /// 更新时间
    pub updated_at: DateTime<Utc>,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {
    #[sea_orm(
        belongs_to = "super::fixed_asset_count::Entity",
        from = "Column::CountId",
        to = "super::fixed_asset_count::Column::Id"
    )]
    Count,
    #[sea_orm(
        belongs_to = "super::fixed_asset::Entity",
        from = "Column::AssetId",
        to = "super::fixed_asset::Column::Id"
    )]
    FixedAsset,
}

impl Related<super::fixed_asset_count::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::Count.def()
    }
}

impl Related<super::fixed_asset::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::FixedAsset.def()
    }
}

impl ActiveModelBehavior for ActiveModel {}
