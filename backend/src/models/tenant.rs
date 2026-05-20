#![allow(dead_code)]

use chrono::{DateTime, Utc};
use sea_orm::entity::prelude::*;
use sea_orm::prelude::DateTimeUtc;
use serde::{Serialize, Deserialize};

#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Eq, Serialize, Deserialize)]
#[sea_orm(table_name = "tenants")]
pub struct Model {
    #[sea_orm(primary_key)]
    pub id: i32,
    #[sea_orm(unique)]
    pub code: String,
    pub name: String,
    pub description: Option<String>,
    pub status: String,
    pub plan_id: Option<i32>,
    pub admin_user_id: Option<i32>,
    pub db_schema: Option<String>,
    pub custom_domain: Option<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub expired_at: Option<DateTime<Utc>>,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {
    #[sea_orm(
        belongs_to = "super::tenant_plan::Entity",
        from = "Column::PlanId",
        to = "super::tenant_plan::Column::Id"
    )]
    Plan,
}

impl ActiveModelBehavior for ActiveModel {}

#[derive(Clone, Debug, PartialEq, Eq, EnumIter, DeriveActiveEnum)]
#[sea_orm(rs_type = "String", db_type = "String(StringLen::N(20))")]
pub enum TenantStatus {
    #[sea_orm(string_value = "ACTIVE")]
    Active,
    #[sea_orm(string_value = "SUSPENDED")]
    Suspended,
    #[sea_orm(string_value = "EXPIRED")]
    Expired,
    #[sea_orm(string_value = "PENDING")]
    Pending,
}
