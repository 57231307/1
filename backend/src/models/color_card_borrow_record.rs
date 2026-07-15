#![allow(dead_code)]
// TODO(tech-debt): 业务接入或重评估后逐项移除；rustc 1.94+ 编译时由编译器报告具体死代码位置。

use chrono::{DateTime, Utc};
use rust_decimal::Decimal;
use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

/// 色卡借出记录实体
#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Eq, Serialize, Deserialize)]
#[sea_orm(table_name = "color_card_borrow_records")]
pub struct Model {
    #[sea_orm(primary_key)]
    pub id: i64,
    pub color_card_id: i64,
    pub customer_id: i64,
    pub borrowed_by: i64,
    pub borrowed_at: DateTime<Utc>,
    pub expected_return_at: Option<DateTime<Utc>>,
    pub actual_return_at: Option<DateTime<Utc>>,
    pub status: String,
    pub purpose: Option<String>,
    pub notes: Option<String>,
    pub compensation_amount: Option<Decimal>,
    /// v14 批次 419 新增：缸号（面料行业追溯字段，T-P0-3 修复，记录借出色卡对应的染缸批次）
    pub dye_lot_no: Option<String>,
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
        belongs_to = "super::customer::Entity",
        from = "Column::CustomerId",
        to = "super::customer::Column::Id"
    )]
    Customer,
    #[sea_orm(
        belongs_to = "super::user::Entity",
        from = "Column::BorrowedBy",
        to = "super::user::Column::Id"
    )]
    Borrower,
}

impl Related<super::color_card::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::ColorCard.def()
    }
}

impl Related<super::customer::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::Customer.def()
    }
}

impl Related<super::user::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::Borrower.def()
    }
}

impl ActiveModelBehavior for ActiveModel {}
