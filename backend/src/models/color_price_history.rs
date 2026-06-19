#![allow(dead_code)]
// TODO(tech-debt): 业务接入或重评估后逐项移除；rustc 1.94+ 编译时由编译器报告具体死代码位置。

use chrono::{DateTime, Utc};
use rust_decimal::Decimal;
use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

/// 色号价格历史实体（P0-5）
///
/// 记录每次调价的变更前/后价格、操作人、原因、审批信息
#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Eq, Serialize, Deserialize)]
#[sea_orm(table_name = "color_price_history")]
pub struct Model {
    #[sea_orm(primary_key)]
    pub id: i64,
    pub product_color_price_id: i64,
    pub old_price: Decimal,
    pub new_price: Decimal,
    pub currency: String,
    pub change_type: String,
    pub change_reason: Option<String>,
    pub change_percent: Option<Decimal>,
    pub quantity: Option<Decimal>,
    pub operated_by: i64,
    pub operated_at: DateTime<Utc>,
    pub approved_by: Option<i64>,
    pub approved_at: Option<DateTime<Utc>>,
    pub tenant_id: i64,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {
    #[sea_orm(
        belongs_to = "super::product_color_price::Entity",
        from = "Column::ProductColorPriceId",
        to = "super::product_color_price::Column::Id"
    )]
    ProductColorPrice,
    #[sea_orm(
        belongs_to = "super::user::Entity",
        from = "Column::OperatedBy",
        to = "super::user::Column::Id"
    )]
    Operator,
    #[sea_orm(
        belongs_to = "super::user::Entity",
        from = "Column::ApprovedBy",
        to = "super::user::Column::Id"
    )]
    Approver,
}

impl Related<super::product_color_price::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::ProductColorPrice.def()
    }
}

impl ActiveModelBehavior for ActiveModel {}
