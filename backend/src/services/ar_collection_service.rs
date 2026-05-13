use sea_orm::{ActiveModelTrait, DatabaseConnection, EntityTrait, Set};
use std::sync::Arc;

use crate::models::ar_collection;
use crate::utils::error::AppError;
use crate::utils::number_generator::DocumentNumberGenerator;
use rust_decimal::Decimal;
use chrono::Utc;

pub struct ArCollectionService {
    db: Arc<DatabaseConnection>,
}

impl ArCollectionService {
    pub fn new(db: Arc<DatabaseConnection>) -> Self {
        Self { db }
    }

    pub async fn create_collection(
        &self,
        customer_id: i32,
        amount: Decimal,
        invoice_id: Option<i32>,
        user_id: i32,
    ) -> Result<ar_collection::Model, AppError> {
        let collection_no = DocumentNumberGenerator::generate_no(
            &self.db,
            "COL",
            ar_collection::Entity,
            ar_collection::Column::CollectionNo,
        )
        .await?;

        let collection = ar_collection::ActiveModel {
            collection_no: Set(collection_no),
            collection_date: Set(Utc::now().date_naive()),
            customer_id: Set(customer_id),
            collection_amount: Set(amount),
            status: Set("CONFIRMED".to_string()),
            created_by: Set(user_id),
            created_at: Set(Utc::now()),
            updated_at: Set(Utc::now()),
            ..Default::default()
        };

        let collection_model = collection.insert(&*self.db).await?;

        use crate::services::event_bus::{BusinessEvent, EVENT_BUS};
        EVENT_BUS.publish(BusinessEvent::CollectionCompleted {
            collection_id: collection_model.id,
            invoice_id,
            amount,
        });

        Ok(collection_model)
    }
}
