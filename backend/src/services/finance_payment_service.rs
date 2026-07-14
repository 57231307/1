
use crate::models::finance_payment;
use crate::models::status::finance_payment as payment_status;
use crate::utils::error::AppError;
// 批次 260 修复：接入 paginate_with_total 统一分页逻辑
use crate::utils::pagination::paginate_with_total;
use chrono::{DateTime, Utc};
use rust_decimal::Decimal;
use sea_orm::DatabaseConnection;
use sea_orm::{ActiveModelTrait, ColumnTrait, EntityTrait, PaginatorTrait, QueryFilter, Set};
use std::sync::Arc;

#[derive(Debug, Clone)]
pub struct FinancePaymentService {
    db: Arc<DatabaseConnection>,
}

/// 创建付款输入参数（service 层，payment_no/payment_date 已由 handler 解析为非 Option）
#[derive(Debug, Clone)]
pub struct CreatePaymentInput {
    pub payment_no: String,
    pub invoice_id: Option<i32>,
    pub amount: Decimal,
    pub payment_date: DateTime<Utc>,
    pub payment_method: Option<String>,
    pub notes: Option<String>,
    pub created_by: Option<i32>,
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

    pub async fn create_payment(
        &self,
        input: CreatePaymentInput,
    ) -> Result<finance_payment::Model, AppError> {
        if input.amount <= Decimal::ZERO {
            return Err(AppError::business("付款金额必须大于零"));
        }
        // P3 3-30 修复：金额精度校验，最多 2 位小数（货币精度）
        if input.amount.round_dp(2) != input.amount {
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
            .check_date_locked(input.payment_date.date_naive())
            .await
            .map_err(|e| AppError::business(e.to_string()))?;

        // 验证关联单据是否存在（事务内，与付款插入原子化）
        if let Some(inv_id) = input.invoice_id {
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
            payment_no: Set(input.payment_no),
            invoice_id: Set(input.invoice_id),
            amount: Set(input.amount),
            payment_date: Set(input.payment_date),
            payment_method: Set(input.payment_method),
            notes: Set(input.notes),
            status: Set(payment_status::PENDING.to_string()),
            created_by: Set(input.created_by),
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

        // 批次 260 修复：接入 paginate_with_total 统一分页逻辑（内部已处理 saturating_sub(1) 偏移）
        let paginator = query.paginate(&*self.db, page_size);
        let (payments, total) = paginate_with_total(paginator, page.clamp(1, 1000)).await?;

        Ok((payments, total))
    }

}
