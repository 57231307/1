#![allow(dead_code)]

//! 固定资产 Entity
use chrono::{DateTime, NaiveDate, Utc};
use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Serialize, Deserialize)]
#[sea_orm(table_name = "fixed_assets")]
pub struct Model {
    #[sea_orm(primary_key)]
    pub id: i32,
    pub asset_no: String,
    pub asset_name: String,
    pub asset_category: Option<String>,
    pub specification: Option<String>,
    pub model: Option<String>,
    pub use_department_id: Option<i32>,
    pub use_location: Option<String>,
    pub responsible_person_id: Option<i32>,
    pub original_value: Decimal,
    pub salvage_value: Option<Decimal>,
    pub salvage_rate: Option<Decimal>,
    pub depreciable_value: Option<Decimal>,
    pub depreciation_method: Option<String>,
    pub useful_life: Option<i32>,
    pub monthly_depreciation: Option<Decimal>,
    pub accumulated_depreciation: Decimal,
    pub net_value: Option<Decimal>,
    pub status: String,
    pub purchase_date: Option<NaiveDate>,
    pub in_service_date: Option<NaiveDate>,
    pub disposal_date: Option<NaiveDate>,
    pub supplier_id: Option<i32>,
    pub supplier_name: Option<String>,
    pub created_by: i32,
    pub is_deleted: bool,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}
impl ActiveModelBehavior for ActiveModel {}
