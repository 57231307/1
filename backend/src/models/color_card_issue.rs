//! 色卡发放记录 Model（V15 P0-F04 创建）
//!
//! 替代旧 color_card_borrow_records（已重命名为 _legacy）
//! 业务：发放/归还/遗失/损坏/取消

use chrono::{DateTime, Utc};
use rust_decimal::Decimal;
use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

/// 色卡发放记录实体
#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Eq, Serialize, Deserialize)]
#[sea_orm(table_name = "color_card_issues")]
pub struct Model {
    #[sea_orm(primary_key)]
    pub id: i64,
    pub color_card_id: i64,
    pub customer_id: i64,
    pub issue_qty: i32,
    pub issued_by: i64,
    pub issued_at: DateTime<Utc>,
    pub expected_return_date: Option<chrono::NaiveDate>,
    pub actual_return_date: Option<chrono::NaiveDate>,
    pub status: String,
    pub purpose: Option<String>,
    pub remark: Option<String>,
    pub compensation_amount: Option<Decimal>,
    pub returned_by: Option<i64>,
    pub dye_lot_no: Option<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub is_deleted: bool,
}

/// 色卡发放记录关联关系
#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {
    /// 发放记录 - 色卡（多对一）
    #[sea_orm(
        belongs_to = "super::color_card::Entity",
        from = "Column::ColorCardId",
        to = "super::color_card::Column::Id"
    )]
    ColorCard,
    /// 发放记录 - 客户（多对一）
    #[sea_orm(
        belongs_to = "super::customer::Entity",
        from = "Column::CustomerId",
        to = "super::customer::Column::Id"
    )]
    Customer,
    /// 发放记录 - 发放人（多对一，user 表）
    #[sea_orm(
        belongs_to = "super::user::Entity",
        from = "Column::IssuedBy",
        to = "super::user::Column::Id"
    )]
    Issuer,
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
        Relation::Issuer.def()
    }
}

impl ActiveModelBehavior for ActiveModel {}
