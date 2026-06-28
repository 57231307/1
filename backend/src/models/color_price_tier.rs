#![allow(dead_code)]
// TODO(tech-debt): 业务接入或重评估后逐项移除；rustc 1.94+ 编译时由编译器报告具体死代码位置。

use chrono::{DateTime, Utc};
use rust_decimal::Decimal;
use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

/// 色号阶梯定价实体（P0-5）
///
/// 数量区间 × 客户等级 → 阶梯价
#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Eq, Serialize, Deserialize)]
#[sea_orm(table_name = "color_price_tiers")]
pub struct Model {
    #[sea_orm(primary_key)]
    pub id: i64,
    pub product_color_price_id: i64,
    pub min_quantity: Decimal,
    pub max_quantity: Option<Decimal>,
    pub tier_price: Decimal,
    pub customer_level: Option<String>,
    pub sequence: i32,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {
    #[sea_orm(
        belongs_to = "super::product_color_price::Entity",
        from = "Column::ProductColorPriceId",
        to = "super::product_color_price::Column::Id"
    )]
    ProductColorPrice,
}

impl Related<super::product_color_price::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::ProductColorPrice.def()
    }
}

impl ActiveModelBehavior for ActiveModel {}
