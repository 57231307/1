#![allow(dead_code, unused_imports, unused_variables)]
//! 销售交货明细 Model
//!
//! 销售交货明细模块

use chrono::{DateTime, Utc};
use rust_decimal::Decimal;
use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

/// 销售交货明细 Entity
#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Serialize, Deserialize)]
#[sea_orm(table_name = "sales_delivery_item")]
pub struct Model {
    /// 交货明细 ID（主键）
    #[sea_orm(primary_key)]
    pub id: i32,

    /// 交货单 ID（外键）
    pub delivery_id: i32,

    /// 产品 ID（外键）
    pub product_id: i32,

    /// 批次号
    pub batch_no: Option<String>,

    /// 色号
    pub color_no: Option<String>,

    /// 交货数量
    pub quantity: Decimal,

    /// 单价
    pub unit_price: Decimal,

    /// 金额
    pub amount: Decimal,

    /// 备注
    pub remarks: Option<String>,

    /// 创建时间
    pub created_at: DateTime<Utc>,
}

/// 销售交货明细关联关系
#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {
    /// 交货明细 - 交货单（多对一）
    #[sea_orm(
        belongs_to = "super::sales_delivery::Entity",
        from = "Column::DeliveryId",
        to = "super::sales_delivery::Column::Id"
    )]
    SalesDelivery,

    /// 交货明细 - 产品（多对一）
    #[sea_orm(
        belongs_to = "super::product::Entity",
        from = "Column::ProductId",
        to = "super::product::Column::Id"
    )]
    Product,
}

impl Related<super::sales_delivery::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::SalesDelivery.def()
    }
}

impl Related<super::product::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::Product.def()
    }
}

impl ActiveModelBehavior for ActiveModel {}
