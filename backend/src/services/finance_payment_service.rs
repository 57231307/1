
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

    #[allow(dead_code)] // TODO(tech-debt): 业务接入后移除
    pub async fn find_by_payment_no(
        &self,
        payment_no: &str,
    ) -> Result<Option<finance_payment::Model>, AppError> {
        finance_payment::Entity::find()
            .filter(finance_payment::Column::PaymentNo.eq(payment_no))
            .one(&*self.db)
            .await
            .map_err(AppError::from)
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

    #[allow(dead_code)] // TODO(tech-debt): 业务接入后移除
    pub async fn update_payment_status(
        &self,
        id: i32,
        status: String,
    ) -> Result<finance_payment::Model, AppError> {
        let payment_model = finance_payment::Entity::find_by_id(id)
            .one(&*self.db)
            .await?
            .ok_or_else(|| AppError::not_found(format!("付款 ID {} 不存在", id)))?;

        let current_status = payment_model.status.as_str();
        let valid_transitions: std::collections::HashMap<&str, Vec<&str>> = [
            ("pending", vec!["confirmed", "cancelled"]),
            ("confirmed", vec!["completed", "cancelled"]),
            ("completed", vec![]),
            ("cancelled", vec![]),
        ]
        .iter()
        .cloned()
        .collect();

        let empty_vec = vec![];
        let allowed = valid_transitions.get(current_status).unwrap_or(&empty_vec);
        if !allowed.contains(&status.as_str()) {
            return Err(AppError::business(format!(
                "付款状态不允许从 {} 变更为 {}",
                current_status, status
            )));
        }

        let mut payment: finance_payment::ActiveModel = payment_model.into();

        payment.status = Set(status);
        payment.updated_at = Set(Utc::now());

        payment.update(&*self.db).await.map_err(AppError::from)
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

    #[allow(dead_code)] // TODO(tech-debt): 业务接入后移除
    pub async fn delete_payment(&self, id: i32) -> Result<(), AppError> {
        let payment = finance_payment::Entity::find_by_id(id)
            .one(&*self.db)
            .await?
            .ok_or_else(|| AppError::not_found(format!("付款 ID {} 不存在", id)))?;

        if payment.status != "pending" {
            return Err(AppError::business(format!(
                "付款状态为{}，只有待处理的付款可以删除",
                payment.status
            )));
        }

        finance_payment::Entity::delete_many()
            .filter(finance_payment::Column::Id.eq(id))
            .exec(&*self.db)
            .await?;
        Ok(())
    }
}
