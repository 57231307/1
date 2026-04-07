#![allow(dead_code, unused_imports, unused_variables)]
use sea_orm::entity::prelude::*;

#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Eq)]
#[sea_orm(table_name = "supplier_categories")]
pub struct Model {
    /// 分类 ID
    #[sea_orm(primary_key)]
    pub id: i32,
    /// 分类编码
    pub category_code: String,
    /// 分类名称
    pub category_name: String,
    /// 父级分类 ID
    pub parent_id: Option<i32>,
    /// 层级（1-3）
    pub level: i32,
    /// 排序
    pub sort_order: i32,
    /// 是否启用
    pub is_enabled: bool,
    /// 创建时间
    pub created_at: DateTimeWithTimeZone,
    /// 更新时间
    pub updated_at: DateTimeWithTimeZone,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {
    /// 分类 - 子分类（一对多）
    #[sea_orm(has_many = "super::supplier_category::Entity")]
    Children,
    /// 分类 - 父分类（多对一）
    #[sea_orm(
        belongs_to = "super::supplier_category::Entity",
        from = "Column::ParentId",
        to = "super::supplier_category::Column::Id"
    )]
    Parent,
}

impl Related<super::supplier_category::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::Children.def()
    }
}

impl ActiveModelBehavior for ActiveModel {}
