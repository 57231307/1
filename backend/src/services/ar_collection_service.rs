use sea_orm::{ActiveModelTrait, ColumnTrait, DatabaseConnection, EntityTrait, QueryFilter, Set, TransactionTrait};
use std::sync::Arc;

use crate::models::ar_collection;
use crate::models::ar_invoice;
use crate::utils::error::AppError;
use crate::utils::number_generator::DocumentNumberGenerator;
use rust_decimal::Decimal;
use chrono::{Datelike, Utc};
use tracing::info;

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
        if amount <= Decimal::ZERO {
            return Err(AppError::BadRequest("收款金额必须大于零".to_string()));
        }

        let txn = (*self.db).begin().await?;

        // 查找客户名称
        let customer_name = crate::models::customer::Entity::find_by_id(customer_id)
            .one(&txn)
            .await?
            .map(|c| c.customer_name);

        let collection_no = DocumentNumberGenerator::generate_no(
            &*self.db,
            "COL",
            ar_collection::Entity,
            ar_collection::Column::CollectionNo,
        )
        .await?;

        let collection = ar_collection::ActiveModel {
            collection_no: Set(collection_no),
            collection_date: Set(Utc::now().date_naive()),
            customer_id: Set(customer_id),
            customer_name: Set(customer_name),
            collection_amount: Set(amount),
            status: Set("CONFIRMED".to_string()),
            created_by: Set(user_id),
            created_at: Set(Utc::now()),
            updated_at: Set(Utc::now()),
            ..Default::default()
        };

        let collection_model = collection.insert(&txn).await?;

        // 关联发票：更新发票的已收金额和未收金额
        if let Some(inv_id) = invoice_id {
            let invoice = ar_invoice::Entity::find_by_id(inv_id)
                .one(&txn)
                .await?
                .ok_or_else(|| AppError::NotFound(format!("应收单 {} 不存在", inv_id)))?;

            let new_received = invoice.received_amount + amount;
            let new_unpaid = (invoice.invoice_amount - new_received).max(Decimal::ZERO);
            let new_status = if new_unpaid == Decimal::ZERO { "PAID" } else { &invoice.status };

            let mut active_invoice: ar_invoice::ActiveModel = invoice.into();
            active_invoice.received_amount = Set(new_received);
            active_invoice.unpaid_amount = Set(new_unpaid);
            active_invoice.status = Set(new_status.to_string());
            active_invoice.updated_at = Set(Utc::now());
            active_invoice.update(&txn).await?;

            info!("收款关联应收单成功：invoice_id={}, 收款金额={}, 已收={}, 未收={}", inv_id, amount, new_received, new_unpaid);
        }

        txn.commit().await?;

        use crate::services::event_bus::{BusinessEvent, EVENT_BUS};
        EVENT_BUS.publish(BusinessEvent::CollectionCompleted {
            collection_id: collection_model.id,
            invoice_id,
            amount,
        });

        let now_date = Utc::now().date_naive();
        let period = format!("{:04}-{:02}", now_date.year(), now_date.month());
        EVENT_BUS.publish(BusinessEvent::FinancialIndicatorUpdate {
            period,
            trigger_source: format!("collection_completed:{}", collection_model.collection_no),
        });

        Ok(collection_model)
    }
}
