//! 采购入库明细 Model
//! 用于记录采购入库单的明细项

use chrono::{DateTime, Utc};
use rust_decimal::Decimal;
use sea_orm::entity::prelude::*;
use sea_orm::prelude::StringLen::N;
use serde::{Deserialize, Serialize};

/// 采购入库明细 Model
#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Eq, Serialize, Deserialize)]
#[sea_orm(table_name = "purchase_receipt_items")]
pub struct Model {
    #[sea_orm(primary_key)]
    pub id: i32,

    /// 采购入库单 ID
    pub receipt_id: i32,

    /// 采购订单明细 ID
    pub order_item_id: Option<i32>,

    /// 产品 ID
    pub product_id: i32,

    /// 入库数量（主单位）
    #[sea_orm(column_type = "Decimal(Some((18, 4)))")]
    pub quantity: Decimal,

    /// 入库数量（辅助单位）
    #[sea_orm(column_type = "Decimal(Some((18, 4)))")]
    pub quantity_alt: Decimal,

    /// 单价（本位币）
    #[sea_orm(column_type = "Decimal(Some((18, 6)))")]
    pub unit_price: Decimal,

    /// 单价（外币）
    #[sea_orm(column_type = "Decimal(Some((18, 6)))")]
    pub unit_price_foreign: Decimal,

    /// 小计（本位币）
    #[sea_orm(column_type = "Decimal(Some((18, 2)))")]
    pub subtotal: Decimal,

    /// 税额（本位币）
    #[sea_orm(column_type = "Decimal(Some((18, 2)))")]
    pub tax_amount: Decimal,

    /// 总金额（本位币）
    #[sea_orm(column_type = "Decimal(Some((18, 2)))")]
    pub total_amount: Decimal,

    /// 质检状态：PENDING=待质检，INSPECTING=质检中，PASSED=合格，FAILED=不合格
    #[sea_orm(column_type = "String(N(20))")]
    pub inspection_status: String,

    /// 合格数量
    #[sea_orm(column_type = "Decimal(Some((18, 4)))")]
    pub passed_quantity: Decimal,

    /// 不合格数量
    #[sea_orm(column_type = "Decimal(Some((18, 4)))")]
    pub rejected_quantity: Decimal,

    /// 备注
    pub notes: Option<String>,

    /// 创建时间
    pub created_at: DateTime<Utc>,

    /// 更新时间
    pub updated_at: DateTime<Utc>,
}

/// 采购入库明细 Relation
#[derive(Copy, Clone, Debug, DeriveRelation, EnumIter)]
pub enum Relation {
    #[sea_orm(
        belongs_to = "super::purchase_receipt::Entity",
        from = "Column::ReceiptId",
        to = "super::purchase_receipt::Column::Id"
    )]
    Receipt,
    #[sea_orm(
        belongs_to = "super::purchase_order_item::Entity",
        from = "Column::OrderItemId",
        to = "super::purchase_order_item::Column::Id"
    )]
    OrderItem,
    #[sea_orm(
        belongs_to = "super::product::Entity",
        from = "Column::ProductId",
        to = "super::product::Column::Id"
    )]
    Product,
}

impl Related<super::purchase_receipt::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::Receipt.def()
    }
}

impl Related<super::purchase_order_item::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::OrderItem.def()
    }
}

impl Related<super::product::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::Product.def()
    }
}

impl ActiveModelBehavior for ActiveModel {}
