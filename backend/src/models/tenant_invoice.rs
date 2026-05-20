#![allow(dead_code)]

use chrono::{DateTime, Utc};
use sea_orm::entity::prelude::*;
use serde::{Serialize, Deserialize};
use rust_decimal::Decimal;

#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Eq, Serialize, Deserialize)]
#[sea_orm(table_name = "tenant_invoices")]
pub struct Model {
    #[sea_orm(primary_key)]
    pub id: i32,
    pub tenant_id: i32,
    pub subscription_id: i32,
    pub invoice_number: String,
    pub billing_period_start: DateTime<Utc>,
    pub billing_period_end: DateTime<Utc>,
    pub amount: Decimal,
    pub status: String,
    pub paid_at: Option<DateTime<Utc>>,
    pub due_date: DateTime<Utc>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {
    #[sea_orm(
        belongs_to = "super::tenant::Entity",
        from = "Column::TenantId",
        to = "super::tenant::Column::Id"
    )]
    Tenant,
    #[sea_orm(
        belongs_to = "super::tenant_subscription::Entity",
        from = "Column::SubscriptionId",
        to = "super::tenant_subscription::Column::Id"
    )]
    Subscription,
}

impl ActiveModelBehavior for ActiveModel {}
