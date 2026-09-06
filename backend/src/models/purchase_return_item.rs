#![allow(dead_code)]
use rust_decimal::Decimal;
use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Serialize, Deserialize)]
#[sea_orm(table_name = "purchase_return_item")]
pub struct Model {
    #[sea_orm(primary_key)]
    pub id: i32,
    pub return_id: i32,
    pub line_no: i32,
    pub product_id: i32,
    #[sea_orm(column_type = "Decimal(Some((18, 4)))")]
    pub quantity: Decimal,
    #[sea_orm(column_type = "Decimal(Some((18, 4)))")]
    pub quantity_alt: Decimal,
    #[sea_orm(column_type = "Decimal(Some((18, 6)))")]
    pub unit_price: Decimal,
    #[sea_orm(column_type = "Decimal(Some((18, 6)))")]
    pub unit_price_foreign: Decimal,
    #[sea_orm(column_type = "Decimal(Some((5, 2)))")]
    pub discount_percent: Decimal,
    #[sea_orm(column_type = "Decimal(Some((5, 2)))")]
    pub tax_percent: Decimal,
    #[sea_orm(column_type = "Decimal(Some((18, 2)))")]
    pub subtotal: Decimal,
    #[sea_orm(column_type = "Decimal(Some((18, 2)))")]
    pub tax_amount: Decimal,
    #[sea_orm(column_type = "Decimal(Some((18, 2)))")]
    pub discount_amount: Decimal,
    #[sea_orm(column_type = "Decimal(Some((18, 2)))")]
    pub total_amount: Decimal,
    pub notes: Option<String>,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub updated_at: chrono::DateTime<chrono::Utc>,

    // ========== v14 批次 417：面料行业追溯字段（D-P1-4） ==========
    // 设计规范：追溯字段全部不可空。后补列经迁移补 DEFAULT '' + NOT NULL
    // （见 migration business mod.ts 对应 ALTER），Model 保持 String。
    // 背景：列初建时无 DEFAULT，存量行/NotSet 插入均为 NULL，sqlx 解码
    // String 遇 NULL 报 "Missing value for column"（run 34019751699 shard-11）。
    /// 色号（面料行业追溯字段）
    pub color_no: String,
    /// 缸号（面料行业追溯字段，白坯布退货时为空串）
    pub dye_lot_no: String,
    /// 批号（面料行业追溯字段）
    pub batch_no: String,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {
    #[sea_orm(
        belongs_to = "super::purchase_return::Entity",
        from = "Column::ReturnId",
        to = "super::purchase_return::Column::Id"
    )]
    Return,
    #[sea_orm(
        belongs_to = "super::product::Entity",
        from = "Column::ProductId",
        to = "super::product::Column::Id"
    )]
    Product,
}

impl Related<super::purchase_return::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::Return.def()
    }
}

impl Related<super::product::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::Product.def()
    }
}

impl ActiveModelBehavior for ActiveModel {}
