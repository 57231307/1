use sea_orm::{
    ActiveModelTrait, ColumnTrait, DatabaseConnection, EntityTrait, QueryFilter, Set,
    TransactionTrait,
};
use std::sync::Arc;

use crate::models::ar_collection;
use crate::models::ar_invoice;
use crate::utils::error::AppError;
use crate::utils::number_generator::DocumentNumberGenerator;
use chrono::{Datelike, Utc};
use rust_decimal::Decimal;
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
            return Err(AppError::bad_request("收款金额必须大于零"));
        }

        // 检查期间锁定
        let period_svc = crate::services::accounting_period_service::AccountingPeriodService::new(
            self.db.clone(),
        );
        let now_date = Utc::now().date_naive();
        period_svc
            .check_date_locked(now_date)
            .await
            .map_err(|e| AppError::business(e.to_string()))?;

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
            collection_date: Set(now_date),
            customer_id: Set(customer_id),
            customer_name: Set(customer_name),
            collection_amount: Set(amount),
            status: Set("pending".to_string()),
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
                .ok_or_else(|| AppError::not_found(format!("应收单 {} 不存在", inv_id)))?;

            // 检查发票状态
            if invoice.status == "CANCELLED" {
                return Err(AppError::bad_request("发票已取消，无法关联收款"));
            }

            // 检查客户一致性
            if invoice.customer_id != customer_id {
                return Err(AppError::bad_request("发票客户与收款客户不一致"));
            }

            // 检查收款金额不超过未收金额
            if amount > invoice.unpaid_amount {
                return Err(AppError::bad_request(format!(
                    "收款金额 {} 超过发票未收金额 {}",
                    amount, invoice.unpaid_amount
                )));
            }

            let new_received = invoice.received_amount + amount;
            let new_unpaid = (invoice.invoice_amount - new_received).max(Decimal::ZERO);
            let new_status = if new_unpaid == Decimal::ZERO {
                "PAID".to_string()
            } else {
                invoice.status.clone()
            };

            let mut active_invoice: ar_invoice::ActiveModel = invoice.into();
            active_invoice.received_amount = Set(new_received);
            active_invoice.unpaid_amount = Set(new_unpaid);
            active_invoice.status = Set(new_status);
            active_invoice.updated_at = Set(Utc::now());
            active_invoice.update(&txn).await?;

            info!(
                "收款关联应收单成功：invoice_id={}, 收款金额={}, 已收={}, 未收={}",
                inv_id, amount, new_received, new_unpaid
            );
        }

        txn.commit().await?;

        use crate::services::event_bus::{BusinessEvent, EVENT_BUS};
        EVENT_BUS.publish(BusinessEvent::CollectionCompleted {
            collection_id: collection_model.id,
            invoice_id,
            amount,
        });

        let period = format!("{:04}-{:02}", now_date.year(), now_date.month());
        EVENT_BUS.publish(BusinessEvent::FinancialIndicatorUpdate {
            period,
            trigger_source: format!("collection_completed:{}", collection_model.collection_no),
        });

        Ok(collection_model)
    }

    /// 确认收款
    pub async fn confirm_collection(
        &self,
        id: i32,
        user_id: i32,
    ) -> Result<ar_collection::Model, AppError> {
        let collection = ar_collection::Entity::find_by_id(id)
            .one(&*self.db)
            .await?
            .ok_or_else(|| AppError::not_found(format!("收款单 {} 不存在", id)))?;

        if collection.status != "pending" {
            return Err(AppError::business(format!(
                "收款单状态为 {}，只有待确认的收款可以确认",
                collection.status
            )));
        }

        let mut active_model: ar_collection::ActiveModel = collection.into();
        active_model.status = Set("confirmed".to_string());
        active_model.confirmed_by = Set(Some(user_id));
        active_model.confirmed_at = Set(Some(Utc::now()));
        active_model.updated_at = Set(Utc::now());

        let updated = active_model.update(&*self.db).await?;
        Ok(updated)
    }

    /// 取消收款
    pub async fn cancel_collection(&self, id: i32) -> Result<ar_collection::Model, AppError> {
        let collection = ar_collection::Entity::find_by_id(id)
            .one(&*self.db)
            .await?
            .ok_or_else(|| AppError::not_found(format!("收款单 {} 不存在", id)))?;

        if collection.status != "pending" {
            return Err(AppError::business(format!(
                "收款单状态为 {}，只有待确认的收款可以取消",
                collection.status
            )));
        }

        let mut active_model: ar_collection::ActiveModel = collection.into();
        active_model.status = Set("cancelled".to_string());
        active_model.updated_at = Set(Utc::now());

        let updated = active_model.update(&*self.db).await?;
        Ok(updated)
    }
}
