//! 颜色代码映射 Model
//!
//! 颜色代码映射模块

use chrono::{DateTime, Utc};
use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

/// 颜色代码映射 Entity
#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Serialize, Deserialize)]
#[sea_orm(table_name = "color_code_mapping")]
pub struct Model {
    /// 映射 ID（主键）
    #[sea_orm(primary_key)]
    pub id: i32,

    /// 产品颜色 ID（外键）
    pub product_color_id: i32,

    /// 客户编码
    pub customer_code: String,

    /// 客户名称
    pub customer_name: Option<String>,

    /// 客户颜色代码
    pub customer_color_code: Option<String>,

    /// 客户颜色名称
    pub customer_color_name: Option<String>,

    /// 是否启用
    pub is_active: bool,

    /// 备注
    pub remarks: Option<String>,

    /// 创建时间
    pub created_at: DateTime<Utc>,

    /// 更新时间
    pub updated_at: DateTime<Utc>,
}

/// 颜色代码映射关联关系
#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {
    /// 映射 - 产品颜色（多对一）
    #[sea_orm(
        belongs_to = "super::product_color::Entity",
        from = "Column::ProductColorId",
        to = "super::product_color::Column::Id"
    )]
    ProductColor,
}

impl Related<super::product_color::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::ProductColor.def()
    }
}

impl ActiveModelBehavior for ActiveModel {}
