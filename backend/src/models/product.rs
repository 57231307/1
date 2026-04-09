#![allow(dead_code, unused_imports, unused_variables)]
use chrono::{DateTime, Utc};
use rust_decimal::Decimal;
use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

/// 产品实体模型（面料行业版）
/// 存储产品基础信息，包含面料行业特色字段
#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Eq, Serialize, Deserialize)]
#[sea_orm(table_name = "products")]
pub struct Model {
    /// 产品 ID（主键）
    #[sea_orm(primary_key)]
    pub id: i32,
    /// 产品名称
    pub name: String,
    /// 产品编码（唯一）
    pub code: String,
    /// 类别 ID（外键）
    pub category_id: Option<i32>,
    /// 规格型号
    pub specification: Option<String>,
    /// 计量单位
    pub unit: String,
    /// 大货价/标准售价
    pub standard_price: Option<Decimal>,
    /// 成本价格
    pub cost_price: Option<Decimal>,
    /// 剪样价/散剪价
    pub sample_price: Option<Decimal>,
    /// 产品描述
    pub description: Option<String>,
    /// 状态：active-启用，inactive-停用
    pub status: String,
    /// 创建时间
    pub created_at: DateTime<Utc>,
    /// 更新时间
    pub updated_at: DateTime<Utc>,

    // ========== 面料行业特色字段 ==========
    /// 产品类型：坯布/成品布/辅料
    pub product_type: String,
    /// 面料成分：如 65% 棉 35% 涤
    pub fabric_composition: Option<String>,
    /// 纱支：如 40S
    pub yarn_count: Option<String>,
    /// 密度：如 133x72
    pub density: Option<String>,
    /// 幅宽（cm）
    #[sea_orm(column_type = "Decimal(Some((10, 2)))")]
    pub width: Option<Decimal>,
    /// 克重（g/m²）
    #[sea_orm(column_type = "Decimal(Some((10, 2)))")]
    pub gram_weight: Option<Decimal>,
    /// 组织结构：平纹/斜纹/缎纹
    pub structure: Option<String>,
    /// 后整理：防水/防油/阻燃
    pub finish: Option<String>,
    /// 最小起订量（米）
    #[sea_orm(column_type = "Decimal(Some((12, 2)))")]
    pub min_order_quantity: Option<Decimal>,
    /// 交货期（天）
    pub lead_time: Option<i32>,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {
    #[sea_orm(
        belongs_to = "super::product_category::Entity",
        from = "Column::CategoryId",
        to = "super::product_category::Column::Id"
    )]
    Category,
    #[sea_orm(has_many = "super::sales_order_item::Entity")]
    SalesOrderItems,
    #[sea_orm(has_many = "super::inventory_stock::Entity")]
    InventoryStock,
}

impl Related<super::product_category::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::Category.def()
    }
}

impl Related<super::sales_order_item::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::SalesOrderItems.def()
    }
}

impl Related<super::inventory_stock::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::InventoryStock.def()
    }
}

impl ActiveModelBehavior for ActiveModel {}
