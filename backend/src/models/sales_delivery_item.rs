#![allow(dead_code)]
// TODO(tech-debt): 业务接入或重评估后逐项移除；rustc 1.94+ 编译时由编译器报告具体死代码位置。

//! 销售交货明细 Model
//!
//! 销售交货明细模块
//! v14 批次 417：添加缺失的缸号字段（D-P1-5），保持现有字段不变以最小化变更

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

    /// v14 批次 417 新增：缸号 ID（D-P1-5 修复，原 Rust 模型缺失此字段）
    pub dye_lot_id: Option<i32>,

    /// v14 批次 417 新增：缸号（D-P1-5 修复，原 Rust 模型缺失此字段）
    pub dye_lot_no: Option<String>,

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
