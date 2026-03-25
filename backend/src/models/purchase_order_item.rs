//! 采购订单明细 Model
//! 用于记录采购订单的明细项

use sea_orm::entity::prelude::*;
use chrono::{DateTime, Utc};
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};

/// 采购订单明细 Model
#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Eq, Serialize, Deserialize)]
#[sea_orm(table_name = "purchase_order_items")]
pub struct Model {
    #[sea_orm(primary_key)]
    pub id: i32,
    
    /// 采购订单 ID
    pub order_id: i32,
    
    /// 产品 ID
    pub product_id: i32,
    
    /// 采购数量（主单位）
    #[sea_orm(column_type = "Decimal(Some((18, 4)))")]
    pub quantity: Decimal,
    
    /// 采购数量（辅助单位）
    #[sea_orm(column_type = "Decimal(Some((18, 4)))")]
    pub quantity_alt: Decimal,
    
    /// 单价（本位币）
    #[sea_orm(column_type = "Decimal(Some((18, 6)))")]
    pub unit_price: Decimal,
    
    /// 单价（外币）
    #[sea_orm(column_type = "Decimal(Some((18, 6)))")]
    pub unit_price_foreign: Decimal,
    
    /// 折扣率
    #[sea_orm(column_type = "Decimal(Some((5, 4)))")]
    pub discount_percent: Decimal,
    
    /// 税率
    #[sea_orm(column_type = "Decimal(Some((5, 4)))")]
    pub tax_percent: Decimal,
    
    /// 小计（本位币）
    #[sea_orm(column_type = "Decimal(Some((18, 2)))")]
    pub subtotal: Decimal,
    
    /// 税额（本位币）
    #[sea_orm(column_type = "Decimal(Some((18, 2)))")]
    pub tax_amount: Decimal,
    
    /// 折扣金额（本位币）
    #[sea_orm(column_type = "Decimal(Some((18, 2)))")]
    pub discount_amount: Decimal,
    
    /// 总金额（本位币）
    #[sea_orm(column_type = "Decimal(Some((18, 2)))")]
    pub total_amount: Decimal,
    
    /// 已入库数量（主单位）
    #[sea_orm(column_type = "Decimal(Some((18, 4)))")]
    pub received_quantity: Decimal,
    
    /// 已入库数量（辅助单位）
    #[sea_orm(column_type = "Decimal(Some((18, 4)))")]
    pub received_quantity_alt: Decimal,
    
    /// 备注
    pub notes: Option<String>,
    
    /// 创建时间
    pub created_at: DateTime<Utc>,
    
    /// 更新时间
    pub updated_at: DateTime<Utc>,
}

/// 采购订单明细 Relation
#[derive(Copy, Clone, Debug, DeriveRelation, EnumIter)]
pub enum Relation {
    #[sea_orm(
        belongs_to = "super::purchase_order::Entity",
        from = "Column::OrderId",
        to = "super::purchase_order::Column::Id"
    )]
    Order,
    #[sea_orm(
        belongs_to = "super::product::Entity",
        from = "Column::ProductId",
        to = "super::product::Column::Id"
    )]
    Product,
}

impl Related<super::purchase_order::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::Order.def()
    }
}

impl ActiveModelBehavior for ActiveModel {}
