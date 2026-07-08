#![allow(dead_code)]
// TODO(tech-debt): 业务接入或重评估后逐项移除；rustc 1.94+ 编译时由编译器报告具体死代码位置。

use chrono::{DateTime, Utc};
use rust_decimal::Decimal;
use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

/// 色卡明细实体
#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Eq, Serialize, Deserialize)]
#[sea_orm(table_name = "color_card_items")]
pub struct Model {
    #[sea_orm(primary_key)]
    pub id: i64,
    pub color_card_id: i64,
    pub color_code: String,
    pub color_name: String,
    pub rgb_r: i32,
    pub rgb_g: i32,
    pub rgb_b: i32,
    pub cmyk_c: Option<Decimal>,
    pub cmyk_m: Option<Decimal>,
    pub cmyk_y: Option<Decimal>,
    pub cmyk_k: Option<Decimal>,
    pub lab_l: Option<Decimal>,
    pub lab_a: Option<Decimal>,
    pub lab_b: Option<Decimal>,
    pub pantone_code: Option<String>,
    pub cncs_code: Option<String>,
    pub custom_code: Option<String>,
    pub hex_value: String,
    pub dye_recipe_id: Option<i32>,
    pub product_color_price_id: Option<i64>,
    pub swatch_image_url: Option<String>,
    pub sequence: i32,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {
    #[sea_orm(
        belongs_to = "super::color_card::Entity",
        from = "Column::ColorCardId",
        to = "super::color_card::Column::Id"
    )]
    ColorCard,
    #[sea_orm(
        belongs_to = "super::dye_recipe::Entity",
        from = "Column::DyeRecipeId",
        to = "super::dye_recipe::Column::Id"
    )]
    DyeRecipe,
    #[sea_orm(
        belongs_to = "super::product_color_price::Entity",
        from = "Column::ProductColorPriceId",
        to = "super::product_color_price::Column::Id"
    )]
    ColorPrice,
}

impl Related<super::color_card::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::ColorCard.def()
    }
}

impl Related<super::dye_recipe::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::DyeRecipe.def()
    }
}

impl Related<super::product_color_price::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::ColorPrice.def()
    }
}

impl ActiveModelBehavior for ActiveModel {}
