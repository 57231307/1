use chrono::{DateTime, Utc};
use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Eq, Serialize, Deserialize)]
#[sea_orm(table_name = "product_categories")]
pub struct Model {
    #[sea_orm(primary_key)]
    pub id: i32,
    pub category_code: String,
    pub name: String,
    pub parent_id: Option<i32>,
    pub description: Option<String>,
    pub sort_order: i32,
    pub is_active: bool,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {
    #[sea_orm(
        belongs_to = "super::product_category::Entity",
        from = "Column::ParentId",
        to = "super::product_category::Column::Id"
    )]
    Parent,
    #[sea_orm(has_many = "super::product_category::Entity")]
    Children,
    #[sea_orm(has_many = "super::product::Entity")]
    Products,
}

impl Related<super::product::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::Products.def()
    }
}

impl Related<super::product_category::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::Parent.def()
    }
}

impl ActiveModelBehavior for ActiveModel {}
