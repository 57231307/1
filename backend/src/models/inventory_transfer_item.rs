#![allow(dead_code)]
use chrono::{DateTime, Utc};
use rust_decimal::Decimal;
use sea_orm::entity::prelude::*;

use serde::{Deserialize, Serialize};
#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Eq, Serialize, Deserialize)]
#[sea_orm(table_name = "inventory_transfer_items")]
pub struct Model {
    #[sea_orm(primary_key)]
    pub id: i32,
    pub transfer_id: i32,
    pub product_id: i32,
    pub quantity: Decimal,
    pub shipped_quantity: Decimal,
    pub received_quantity: Decimal,
    pub unit_cost: Option<Decimal>,
    pub notes: Option<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,

    // ========== v14 批次 417：面料行业追溯字段（T-P0-1） ==========
    /// 色号（面料行业追溯字段）
    pub color_no: String,
    /// 缸号（面料行业追溯字段，白坯布调拨时为 NULL）
    pub dye_lot_no: Option<String>,
    /// 批号（面料行业追溯字段）
    pub batch_no: String,
    /// 匹号（出库四维=缸号/色号/批次/匹号之第四维，用户 2026-10-02 纠正口径；
    /// 染色布必填、白坯布合法为 NULL；m0066 补列）
    pub piece_no: Option<String>,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {
    #[sea_orm(
        belongs_to = "super::inventory_transfer::Entity",
        from = "Column::TransferId",
        to = "super::inventory_transfer::Column::Id"
    )]
    Transfer,
    #[sea_orm(
        belongs_to = "super::product::Entity",
        from = "Column::ProductId",
        to = "super::product::Column::Id"
    )]
    Product,
}

impl Related<super::inventory_transfer::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::Transfer.def()
    }
}

impl Related<super::product::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::Product.def()
    }
}

impl ActiveModelBehavior for ActiveModel {}
