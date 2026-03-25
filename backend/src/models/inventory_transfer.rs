use chrono::{DateTime, Utc};
use rust_decimal::Decimal;
use sea_orm::entity::prelude::*;

#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Eq)]
#[sea_orm(table_name = "inventory_transfers")]
pub struct Model {
    #[sea_orm(primary_key)]
    pub id: i32,
    pub transfer_no: String,
    pub from_warehouse_id: i32,
    pub to_warehouse_id: i32,
    pub transfer_date: DateTime<Utc>,
    pub status: String,
    pub total_quantity: Decimal,
    pub notes: Option<String>,
    pub created_by: Option<i32>,
    pub approved_by: Option<i32>,
    pub approved_at: Option<DateTime<Utc>>,
    pub shipped_at: Option<DateTime<Utc>>,
    pub received_at: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {
    #[sea_orm(
        belongs_to = "super::warehouse::Entity",
        from = "Column::FromWarehouseId",
        to = "super::warehouse::Column::Id"
    )]
    FromWarehouse,
    #[sea_orm(
        belongs_to = "super::warehouse::Entity",
        from = "Column::ToWarehouseId",
        to = "super::warehouse::Column::Id"
    )]
    ToWarehouse,
    #[sea_orm(has_many = "super::inventory_transfer_item::Entity")]
    Items,
}

impl Related<super::warehouse::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::FromWarehouse.def()
    }
}

impl Related<super::inventory_transfer_item::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::Items.def()
    }
}

impl ActiveModelBehavior for ActiveModel {}
