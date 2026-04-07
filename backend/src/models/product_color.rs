#![allow(dead_code, unused_imports, unused_variables)]
use chrono::{DateTime, Utc};
use rust_decimal::Decimal;
use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

/// 产品色号实体模型（面料行业）
/// 存储每个产品的可用色号信息
#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Eq, Serialize, Deserialize)]
#[sea_orm(table_name = "product_colors")]
pub struct Model {
    /// 色号 ID（主键）
    #[sea_orm(primary_key)]
    pub id: i32,
    /// 产品 ID（外键）
    pub product_id: i32,
    /// 色号编码
    pub color_no: String,
    /// 颜色名称
    pub color_name: String,
    /// 潘通色号
    pub pantone_code: Option<String>,
    /// 色号类型：常规色/定制色
    pub color_type: String,
    /// 染色配方（保密）
    pub dye_formula: Option<String>,
    /// 特殊色号加价（元/米）
    #[sea_orm(column_type = "Decimal(Some((10, 2)))")]
    pub extra_cost: Decimal,
    /// 是否激活
    pub is_active: bool,
    /// 创建时间
    pub created_at: DateTime<Utc>,
    /// 更新时间
    pub updated_at: DateTime<Utc>,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {
    #[sea_orm(
        belongs_to = "super::product::Entity",
        from = "Column::ProductId",
        to = "super::product::Column::Id"
    )]
    Product,
}

impl Related<super::product::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::Product.def()
    }
}

impl ActiveModelBehavior for ActiveModel {}
