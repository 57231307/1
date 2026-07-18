#![allow(dead_code)]
// TODO(tech-debt): 业务接入或重评估后逐项移除；rustc 1.94+ 编译时由编译器报告具体死代码位置。

use chrono::{DateTime, Utc};
use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

/// 色卡主表实体
#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Eq, Serialize, Deserialize)]
#[sea_orm(table_name = "color_cards")]
pub struct Model {
    #[sea_orm(primary_key)]
    pub id: i64,
    pub card_no: String,
    pub card_name: String,
    pub card_type: String,
    pub season: Option<String>,
    pub brand: Option<String>,
    pub total_colors: i32,
    pub status: String,
    pub description: Option<String>,
    pub cover_image_url: Option<String>,
    /// V15 P0-F10：色卡总库存数量（发放数量不能超过此值）
    pub stock_quantity: i32,
    /// V15 P0-F10：已发放数量（与发放记录聚合保持一致）
    pub issued_quantity: i32,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {
    #[sea_orm(has_many = "super::color_card_item::Entity")]
    Items,
    /// V15 P0-F04：色卡发放记录关联（替代旧 BorrowRecords）
    #[sea_orm(has_many = "super::color_card_issue::Entity")]
    Issues,
}

impl Related<super::color_card_item::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::Items.def()
    }
}

impl Related<super::color_card_issue::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::Issues.def()
    }
}

impl ActiveModelBehavior for ActiveModel {}
