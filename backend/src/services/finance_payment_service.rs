
use crate::models::finance_payment;
use crate::utils::error::AppError;
use chrono::{DateTime, Utc};
use rust_decimal::Decimal;
use sea_orm::DatabaseConnection;
use sea_orm::{ActiveModelTrait, ColumnTrait, EntityTrait, PaginatorTrait, QueryFilter, Set};
use std::sync::Arc;

#[derive(Debug, Clone)]
pub struct FinancePaymentService {
    db: Arc<DatabaseConnection>,
}

impl FinancePaymentService {
    pub fn new(db: Arc<DatabaseConnection>) -> Self {
        Self { db }
    }

    pub async fn find_by_id(&self, id: i32) -> Result<finance_payment::Model, AppError> {
        finance_payment::Entity::find_by_id(id)
            .one(&*self.db)
            .await?
            .ok_or_else(|| AppError::not_found(format!("付款 ID {} 不存在", id)))
    }

    #[allow(clippy::too_many_arguments)]
    pub async fn create_payment(
        &self,
        payment_no: String,
        invoice_id: Option<i32>,
        amount: Decimal,
        payment_date: DateTime<Utc>,
        payment_method: Option<String>,
        notes: Option<String>,
        created_by: Option<i32>,
    ) -> Result<finance_payment::Model, AppError> {
        if amount <= Decimal::ZERO {
            return Err(AppError::business("付款金额必须大于零"));
        }

        let period_svc = crate::services::accounting_period_service::AccountingPeriodService::new(
            self.db.clone(),
        );
        period_svc
            .check_date_locked(payment_date.date_naive())
            .await
            .map_err(|e| AppError::business(e.to_string()))?;

        // 验证关联单据是否存在
        if let Some(inv_id) = invoice_id {
            let invoice_exists = crate::models::finance_invoice::Entity::find_by_id(inv_id)
                .one(&*self.db)
                .await?
                .is_some();
            if !invoice_exists {
                return Err(AppError::business(format!("关联发票 ID {} 不存在", inv_id)));
            }
        }

        let active_payment = finance_payment::ActiveModel {
            id: Set(0),
            payment_no: Set(payment_no),
            invoice_id: Set(invoice_id),
            amount: Set(amount),
            payment_date: Set(payment_date),
            payment_method: Set(payment_method),
            notes: Set(notes),
            status: Set("pending".to_string()),
            created_by: Set(created_by),
            created_at: Set(Utc::now()),
            updated_at: Set(Utc::now()),
        };

        active_payment
            .insert(&*self.db)
            .await
            .map_err(AppError::from)
    }

    pub async fn list_payments(
        &self,
        page: u64,
        page_size: u64,
        status: Option<String>,
    ) -> Result<(Vec<finance_payment::Model>, u64), AppError> {
        let mut query = finance_payment::Entity::find();

        if let Some(s) = status {
            query = query.filter(finance_payment::Column::Status.eq(s));
        }

        let paginator = query.paginate(&*self.db, page_size);
        let total = paginator.num_items().await?;
        let payments = paginator.fetch_page(page).await?;

        Ok((payments, total))
    }

}
