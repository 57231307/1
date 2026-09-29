#![allow(dead_code)]
//! 客户-标签关联表 Entity（customer_tag）
//!
//! 多对多关联：customers(N) <-> crm_tag(N)，通过本中间表实现。
//! UNIQUE(customer_id, tag_id) 保证同一客户不重复挂载同一标签。

use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

/// 客户-标签关联 Entity
#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Eq, Serialize, Deserialize)]
#[sea_orm(table_name = "customer_tag")]
pub struct Model {
    #[sea_orm(primary_key)]
    pub id: i32,
    /// 客户 ID（关联 customers.id，INTEGER）
    pub customer_id: i32,
    /// 标签 ID（关联 crm_tag.id，INTEGER）
    pub tag_id: i32,
    /// 创建者用户 ID
    pub created_by: Option<i32>,
    /// 创建时间
    pub created_at: DateTimeWithTimeZone,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {
    #[sea_orm(
        belongs_to = "super::customer::Entity",
        from = "Column::CustomerId",
        to = "super::customer::Column::Id"
    )]
    Customer,
    #[sea_orm(
        belongs_to = "super::crm_tag::Entity",
        from = "Column::TagId",
        to = "super::crm_tag::Column::Id"
    )]
    CrmTag,
}

impl Related<super::customer::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::Customer.def()
    }
}

impl Related<super::crm_tag::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::CrmTag.def()
    }
}

impl ActiveModelBehavior for ActiveModel {}
