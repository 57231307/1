use crate::models::finance_payment;
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

    pub async fn find_by_id(&self, id: i32) -> Result<finance_payment::Model, sea_orm::DbErr> {
        finance_payment::Entity::find_by_id(id)
            .one(&*self.db)
            .await?
            .ok_or_else(|| sea_orm::DbErr::RecordNotFound(format!("付款 ID {} 不存在", id)))
    }

    pub async fn find_by_payment_no(
        &self,
        payment_no: &str,
    ) -> Result<Option<finance_payment::Model>, sea_orm::DbErr> {
        finance_payment::Entity::find()
            .filter(finance_payment::Column::PaymentNo.eq(payment_no))
            .one(&*self.db)
            .await
    }

    pub async fn create_payment(
        &self,
        payment_no: String,
        payment_type: String,
        order_type: String,
        order_id: Option<i32>,
        customer_id: Option<i32>,
        supplier_id: Option<i32>,
        amount: Decimal,
        payment_date: DateTime<Utc>,
        payment_method: Option<String>,
        reference_no: Option<String>,
        notes: Option<String>,
        created_by: Option<i32>,
    ) -> Result<finance_payment::Model, sea_orm::DbErr> {
        let active_payment = finance_payment::ActiveModel {
            id: Set(0),
            payment_no: Set(payment_no),
            payment_type: Set(payment_type),
            order_type: Set(order_type),
            order_id: Set(order_id),
            customer_id: Set(customer_id),
            supplier_id: Set(supplier_id),
            amount: Set(amount),
            paid_amount: Set(Decimal::ZERO),
            balance_amount: Set(amount),
            payment_date: Set(payment_date),
            payment_method: Set(payment_method),
            reference_no: Set(reference_no),
            notes: Set(notes),
            status: Set("pending".to_string()),
            created_by: Set(created_by),
            approved_by: Set(None),
            approved_at: Set(None),
            created_at: Set(Utc::now()),
            updated_at: Set(Utc::now()),
        };

        active_payment.insert(&*self.db).await
    }

    pub async fn update_payment_status(
        &self,
        id: i32,
        status: String,
        approved_by: Option<i32>,
    ) -> Result<finance_payment::Model, sea_orm::DbErr> {
        let mut payment: finance_payment::ActiveModel = finance_payment::Entity::find_by_id(id)
            .one(&*self.db)
            .await?
            .ok_or_else(|| sea_orm::DbErr::RecordNotFound(format!("付款 ID {} 不存在", id)))?
            .into();

        payment.status = Set(status);
        payment.approved_by = Set(approved_by);
        payment.approved_at = Set(Some(Utc::now()));
        payment.updated_at = Set(Utc::now());

        payment.update(&*self.db).await
    }

    pub async fn list_payments(
        &self,
        page: u64,
        page_size: u64,
        status: Option<String>,
    ) -> Result<(Vec<finance_payment::Model>, u64), sea_orm::DbErr> {
        let mut query = finance_payment::Entity::find();

        if let Some(s) = status {
            query = query.filter(finance_payment::Column::Status.eq(s));
        }

        let paginator = query.paginate(&*self.db, page_size);
        let total = paginator.num_items().await?;
        let payments = paginator.fetch_page(page).await?;

        Ok((payments, total))
    }

    pub async fn delete_payment(&self, id: i32) -> Result<(), sea_orm::DbErr> {
        finance_payment::Entity::delete_many()
            .filter(finance_payment::Column::Id.eq(id))
            .exec(&*self.db)
            .await?;
        Ok(())
    }
}
