//! 库存调整明细模型
//! 用于记录库存调整单的明细项

use sea_orm::entity::prelude::*;
use chrono::{DateTime, Utc};
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};

/// 库存调整明细 Model
#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Eq, Serialize, Deserialize)]
#[sea_orm(table_name = "inventory_adjustment_items")]
pub struct Model {
    #[sea_orm(primary_key)]
    pub id: i32,
    
    /// 调整单 ID
    pub adjustment_id: i32,
    
    /// 库存 ID
    pub stock_id: i32,
    
    /// 调整数量（正数：增加，负数：减少）
    #[sea_orm(column_type = "Decimal(Some((10, 2)))")]
    pub quantity: Decimal,
    
    /// 调整前数量
    #[sea_orm(column_type = "Decimal(Some((10, 2)))")]
    pub quantity_before: Decimal,
    
    /// 调整后数量
    #[sea_orm(column_type = "Decimal(Some((10, 2)))")]
    pub quantity_after: Decimal,
    
    /// 单位成本
    #[sea_orm(column_type = "Decimal(Some((12, 2)))")]
    pub unit_cost: Option<Decimal>,
    
    /// 调整金额
    #[sea_orm(column_type = "Decimal(Some((12, 2)))")]
    pub amount: Option<Decimal>,
    
    /// 备注
    pub notes: Option<String>,
    
    /// 创建时间
    pub created_at: DateTime<Utc>,
    
    /// 更新时间
    pub updated_at: DateTime<Utc>,
}

/// 库存调整明细 Relation
#[derive(Copy, Clone, Debug, DeriveRelation, EnumIter)]
pub enum Relation {
    #[sea_orm(
        belongs_to = "super::inventory_adjustment::Entity",
        from = "Column::AdjustmentId",
        to = "super::inventory_adjustment::Column::Id"
    )]
    Adjustment,
    #[sea_orm(
        belongs_to = "super::inventory_stock::Entity",
        from = "Column::StockId",
        to = "super::inventory_stock::Column::Id"
    )]
    Stock,
}

impl Related<super::inventory_adjustment::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::Adjustment.def()
    }
}

impl Related<super::inventory_stock::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::Stock.def()
    }
}

impl ActiveModelBehavior for ActiveModel {}
