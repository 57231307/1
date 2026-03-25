//! 库存匹数 Model
//!
//! 库存匹数模块

use chrono::{DateTime, Utc};
use rust_decimal::Decimal;
use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

/// 库存匹数 Entity
#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Serialize, Deserialize)]
#[sea_orm(table_name = "inventory_pieces")]
pub struct Model {
    /// 匹数 ID（主键）
    #[sea_orm(primary_key)]
    pub id: i32,

    /// 批次号
    pub batch_no: String,

    /// 产品 ID（外键）
    pub product_id: i32,

    /// 仓库 ID（外键）
    pub warehouse_id: i32,

    /// 库位 ID（外键）
    pub location_id: Option<i32>,

    /// 匹号
    pub piece_no: String,

    /// 长度（米）
    pub length: Decimal,

    /// 重量（千克）
    pub weight: Option<Decimal>,

    /// 状态：AVAILABLE=可用，RESERVED=预留， defect=次品
    pub status: String,

    /// 备注
    pub remarks: Option<String>,

    /// 创建时间
    pub created_at: DateTime<Utc>,

    /// 更新时间
    pub updated_at: DateTime<Utc>,
}

/// 库存匹数关联关系
#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {
    /// 匹数 - 产品（多对一）
    #[sea_orm(
        belongs_to = "super::product::Entity",
        from = "Column::ProductId",
        to = "super::product::Column::Id"
    )]
    Product,

    /// 匹数 - 仓库（多对一）
    #[sea_orm(
        belongs_to = "super::warehouse::Entity",
        from = "Column::WarehouseId",
        to = "super::warehouse::Column::Id"
    )]
    Warehouse,
}

impl Related<super::product::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::Product.def()
    }
}

impl Related<super::warehouse::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::Warehouse.def()
    }
}

impl ActiveModelBehavior for ActiveModel {}
