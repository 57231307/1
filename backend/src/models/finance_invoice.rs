#![allow(dead_code)]

use chrono::{DateTime, Utc};
use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Eq, Serialize, Deserialize)]
#[sea_orm(table_name = "finance_invoices")]
pub struct Model {
    #[sea_orm(primary_key)]
    pub id: i32,
    pub invoice_no: String,
    pub order_id: Option<i32>,
    pub customer_id: Option<i32>,
    pub customer_name: String,
    pub invoice_type: String,
    pub amount: rust_decimal::Decimal,
    pub tax_amount: rust_decimal::Decimal,
    pub total_amount: rust_decimal::Decimal,
    pub status: String,
    pub invoice_date: Option<DateTime<Utc>>,
    pub due_date: Option<DateTime<Utc>>,
    pub paid_date: Option<DateTime<Utc>>,
    pub payment_method: Option<String>,
    pub notes: Option<String>,
    pub is_deleted: bool,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {
    #[sea_orm(
        belongs_to = "super::customer::Entity",
        from = "Column::CustomerId",
        to = "super::customer::Column::Id"
    )]
    Customer,
}

impl Related<super::customer::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::Customer.def()
    }
}


use sea_orm::ActiveValue;
use serde_json::Value;

#[async_trait::async_trait]
impl ActiveModelBehavior for ActiveModel {
    async fn before_save<C>(self, db: &C, insert: bool) -> Result<Self, DbErr>
    where
        C: ConnectionTrait,
    {
        if !insert {
            if let ActiveValue::Set(id) | ActiveValue::Unchanged(id) = self.id.clone() {
                if let Ok(Some(old_data)) = Entity::find_by_id(id.clone()).one(db).await {
                    if let Ok(old_json) = serde_json::to_value(&old_data) {
                        // To get new data, we apply ActiveModel over old_data
                        let mut new_data = old_data.clone();
                        // This requires applying the active model, but ActiveModel doesn't have a simple apply method without moving.
                        // Actually we can just serialize the ActiveModel directly if we want, but it serializes differently.
                    }
                }
            }
        }
        Ok(self)
    }
}

