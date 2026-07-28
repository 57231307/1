#![allow(dead_code)]
// v11 批次 143 P1-1：inventory_count 模型已接入业务（盘点单 CRUD + 审批流），
// 保留文件级 dead_code 抑制以符合 models/ 目录例外规范（SeaORM 派生宏字段）

use chrono::{DateTime, Utc};
use sea_orm::entity::prelude::*;

use serde::{Deserialize, Serialize};
#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Eq, Serialize, Deserialize)]
#[sea_orm(table_name = "inventory_counts")]
pub struct Model {
    #[sea_orm(primary_key)]
    pub id: i32,
    pub count_no: String,
    pub warehouse_id: i32,
    pub count_date: DateTime<Utc>,
    pub status: String,
    pub total_items: i32,
    pub counted_items: i32,
    pub variance_items: i32,
    pub notes: Option<String>,
    pub created_by: Option<i32>,
    pub approved_by: Option<i32>,
    pub approved_at: Option<DateTime<Utc>>,
    pub completed_at: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {
    #[sea_orm(
        belongs_to = "super::warehouse::Entity",
        from = "Column::WarehouseId",
        to = "super::warehouse::Column::Id"
    )]
    Warehouse,
    #[sea_orm(has_many = "super::inventory_count_item::Entity")]
    Items,
}

impl Related<super::warehouse::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::Warehouse.def()
    }
}

impl Related<super::inventory_count_item::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::Items.def()
    }
}

impl ActiveModelBehavior for ActiveModel {}
