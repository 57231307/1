//! 坯布管理模型（原料布匹管理）

use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Serialize, Deserialize)]
#[sea_orm(table_name = "greige_fabric")]
pub struct Model {
    #[sea_orm(primary_key)]
    pub id: i32,
    pub fabric_no: String,
    pub fabric_name: String,
    pub fabric_type: String,
    pub color_code: Option<String>,
    pub width_cm: Option<Decimal>,
    pub weight_kg: Option<Decimal>,
    pub length_m: Option<Decimal>,
    pub supplier_id: Option<i32>,
    pub batch_no: Option<String>,
    pub warehouse_id: Option<i32>,
    pub location: Option<String>,
    pub status: String,
    pub quality_grade: Option<String>,
    pub purchase_date: Option<DateTimeUtc>,
    pub remarks: Option<String>,
    pub created_by: Option<i32>,
    pub created_at: DateTimeUtc,
    pub updated_at: DateTimeUtc,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {
    #[sea_orm(
        belongs_to = "super::entities::supplier",
        from = "super::entities::greige_fabric",
        to = "super::entities::supplier",
        on_update = "Cascade",
        on_delete = "SetNull"
    )]
    Supplier,
    #[sea_orm(
        belongs_to = "super::entities::warehouse",
        from = "super::entities::greige_fabric",
        to = "super::entities::warehouse",
        on_update = "Cascade",
        on_delete = "SetNull"
    )]
    Warehouse,
}

impl ActiveModelBehavior for ActiveModel {}
