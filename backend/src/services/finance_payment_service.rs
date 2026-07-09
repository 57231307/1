
use crate::models::finance_payment;
use crate::models::status::finance_payment as payment_status;
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
        // P3 3-30 修复：金额精度校验，最多 2 位小数（货币精度）
        if amount.round_dp(2) != amount {
            return Err(AppError::business("付款金额精度不能超过 2 位小数"));
        }

        // P1 5-4 修复（批次 63）：整体包裹 txn，发票存在性检查与付款插入原子化
        // 原实现发票检查用 &*self.db，付款插入也用 &*self.db，无事务包裹，
        // 并发可创建引用已删除发票的付款（检查与插入之间存在 TOCTOU 窗口）。
        use sea_orm::TransactionTrait;
        let txn = (*self.db).begin().await?;

        let period_svc = crate::services::accounting_period_service::AccountingPeriodService::new(
            self.db.clone(),
        );
        period_svc
            .check_date_locked(payment_date.date_naive())
            .await
            .map_err(|e| AppError::business(e.to_string()))?;

        // 验证关联单据是否存在（事务内，与付款插入原子化）
        if let Some(inv_id) = invoice_id {
            let invoice_exists = crate::models::finance_invoice::Entity::find_by_id(inv_id)
                .one(&txn)
                .await?
                .is_some();
            if !invoice_exists {
                return Err(AppError::business(format!("关联发票 ID {} 不存在", inv_id)));
            }
        }

        let active_payment = finance_payment::ActiveModel {
            id: Default::default(),
            payment_no: Set(payment_no),
            invoice_id: Set(invoice_id),
            amount: Set(amount),
            payment_date: Set(payment_date),
            payment_method: Set(payment_method),
            notes: Set(notes),
            status: Set(payment_status::PENDING.to_string()),
            created_by: Set(created_by),
            created_at: Set(Utc::now()),
            updated_at: Set(Utc::now()),
        };

        let result = active_payment
            .insert(&txn)
            .await
            .map_err(AppError::from)?;

        txn.commit().await?;

        Ok(result)
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
        // SeaORM fetch_page 为 0-indexed，HTTP 层 page 为 1-indexed，需减 1 对齐
        let payments = paginator.fetch_page(page.saturating_sub(1)).await?;

        Ok((payments, total))
    }

}
