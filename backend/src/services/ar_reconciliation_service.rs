//! 应收对账 Service
//!
//! 提供客户应收对账单的生成、发送、确认和争议处理

#![allow(dead_code)]

use chrono::{NaiveDate, Utc};
use rust_decimal::Decimal;
use sea_orm::{
    ActiveModelTrait, ColumnTrait, EntityTrait, PaginatorTrait, QueryFilter, QueryOrder, Set,
};
use std::sync::Arc;
use sea_orm::DatabaseConnection;

use crate::models::ar_reconciliation::{
    ActiveModel, Entity as ReconciliationEntity, Model as ReconciliationModel,
};
use crate::utils::error::AppError;

/// 创建对账单请求
#[derive(Debug, Clone)]
pub struct CreateReconciliationRequest {
    pub reconciliation_no: String,
    pub customer_id: i32,
    pub customer_name: Option<String>,
    pub period_start: NaiveDate,
    pub period_end: NaiveDate,
    pub opening_balance: Decimal,
    pub total_invoices: Decimal,
    pub total_collections: Decimal,
    pub notes: Option<String>,
}

/// 更新对账单请求
#[derive(Debug, Clone)]
pub struct UpdateReconciliationRequest {
    pub opening_balance: Option<Decimal>,
    pub total_invoices: Option<Decimal>,
    pub total_collections: Option<Decimal>,
    pub notes: Option<String>,
}

/// 对账单查询参数
#[derive(Debug, Clone)]
pub struct ReconciliationQuery {
    pub status: Option<String>,
    pub customer_id: Option<i32>,
    pub page: u64,
    pub page_size: u64,
}

/// 应收对账 Service
pub struct ArReconciliationService {
    db: Arc<DatabaseConnection>,
}

impl ArReconciliationService {
    pub fn new(db: Arc<DatabaseConnection>) -> Self {
        Self { db }
    }

    /// 创建对账单
    pub async fn create(
        &self,
        req: CreateReconciliationRequest,
    ) -> Result<ReconciliationModel, AppError> {
        let closing_balance = req.opening_balance + req.total_invoices - req.total_collections;

        let active_model = ActiveModel {
            id: Set(0),
            reconciliation_no: Set(req.reconciliation_no),
            reconciliation_date: Set(Utc::now().date_naive()),
            period_start: Set(req.period_start),
            period_end: Set(req.period_end),
            customer_id: Set(req.customer_id),
            customer_name: Set(req.customer_name),
            opening_balance: Set(req.opening_balance),
            total_invoices: Set(req.total_invoices),
            total_collections: Set(req.total_collections),
            closing_balance: Set(closing_balance),
            reconciliation_status: Set(Some("draft".to_string())),
            confirmed_by_customer: Set(None),
            dispute_reason: Set(None),
            confirmed_by: Set(None),
            confirmed_at: Set(None),
            created_by: Set(None),
            created_at: Set(Utc::now()),
            updated_at: Set(Utc::now()),
        };

        let model = active_model
            .insert(&*self.db)
            .await
            .map_err(|e| AppError::DatabaseError(e.to_string()))?;

        Ok(model)
    }

    /// 根据ID获取对账单
    pub async fn get_by_id(&self, id: i32) -> Result<Option<ReconciliationModel>, AppError> {
        let model = ReconciliationEntity::find_by_id(id)
            .one(&*self.db)
            .await
            .map_err(|e| AppError::DatabaseError(e.to_string()))?;

        Ok(model)
    }

    /// 获取对账单列表
    pub async fn list(
        &self,
        query: ReconciliationQuery,
    ) -> Result<(Vec<ReconciliationModel>, u64), AppError> {
        let mut select = ReconciliationEntity::find();

        if let Some(status) = query.status {
            select = select.filter(crate::models::ar_reconciliation::Column::ReconciliationStatus.eq(status));
        }

        if let Some(customer_id) = query.customer_id {
            select = select.filter(crate::models::ar_reconciliation::Column::CustomerId.eq(customer_id));
        }

        let total = select
            .clone()
            .count(&*self.db)
            .await
            .map_err(|e| AppError::DatabaseError(e.to_string()))?;

        let paginator = select
            .order_by_desc(crate::models::ar_reconciliation::Column::CreatedAt)
            .paginate(&*self.db, query.page_size);

        let models = paginator
            .fetch_page(query.page - 1)
            .await
            .map_err(|e| AppError::DatabaseError(e.to_string()))?;

        Ok((models, total))
    }

    /// 更新对账单
    pub async fn update(
        &self,
        id: i32,
        req: UpdateReconciliationRequest,
    ) -> Result<ReconciliationModel, AppError> {
        let model = ReconciliationEntity::find_by_id(id)
            .one(&*self.db)
            .await
            .map_err(|e| AppError::DatabaseError(e.to_string()))?
            .ok_or_else(|| AppError::NotFound("对账单不存在".to_string()))?;

        let mut active_model: ActiveModel = model.into();

        if let Some(opening_balance) = req.opening_balance {
            active_model.opening_balance = Set(opening_balance);
        }
        if let Some(total_invoices) = req.total_invoices {
            active_model.total_invoices = Set(total_invoices);
        }
        if let Some(total_collections) = req.total_collections {
            active_model.total_collections = Set(total_collections);
        }

        // 重新计算期末余额
        let opening = active_model.opening_balance.as_ref().clone();
        let invoices = active_model.total_invoices.as_ref().clone();
        let collections = active_model.total_collections.as_ref().clone();
        active_model.closing_balance = Set(opening + invoices - collections);

        active_model.updated_at = Set(Utc::now());

        let updated = active_model
            .update(&*self.db)
            .await
            .map_err(|e| AppError::DatabaseError(e.to_string()))?;

        Ok(updated)
    }

    /// 删除对账单
    pub async fn delete(&self, id: i32) -> Result<(), AppError> {
        ReconciliationEntity::delete_by_id(id)
            .exec(&*self.db)
            .await
            .map_err(|e| AppError::DatabaseError(e.to_string()))?;

        Ok(())
    }

    /// 发送对账单
    pub async fn send(&self, id: i32) -> Result<ReconciliationModel, AppError> {
        let model = ReconciliationEntity::find_by_id(id)
            .one(&*self.db)
            .await
            .map_err(|e| AppError::DatabaseError(e.to_string()))?
            .ok_or_else(|| AppError::NotFound("对账单不存在".to_string()))?;

        let mut active_model: ActiveModel = model.into();
        active_model.reconciliation_status = Set(Some("sent".to_string()));
        active_model.updated_at = Set(Utc::now());

        let updated = active_model
            .update(&*self.db)
            .await
            .map_err(|e| AppError::DatabaseError(e.to_string()))?;

        Ok(updated)
    }

    /// 客户确认对账单
    pub async fn confirm(&self, id: i32, confirmed_by: Option<i32>) -> Result<ReconciliationModel, AppError> {
        let model = ReconciliationEntity::find_by_id(id)
            .one(&*self.db)
            .await
            .map_err(|e| AppError::DatabaseError(e.to_string()))?
            .ok_or_else(|| AppError::NotFound("对账单不存在".to_string()))?;

        let mut active_model: ActiveModel = model.into();
        active_model.reconciliation_status = Set(Some("confirmed".to_string()));
        active_model.confirmed_by_customer = Set(Some(true));
        active_model.confirmed_by = Set(confirmed_by);
        active_model.confirmed_at = Set(Some(Utc::now()));
        active_model.updated_at = Set(Utc::now());

        let updated = active_model
            .update(&*self.db)
            .await
            .map_err(|e| AppError::DatabaseError(e.to_string()))?;

        Ok(updated)
    }

    /// 客户提出争议
    pub async fn dispute(
        &self,
        id: i32,
        reason: String,
    ) -> Result<ReconciliationModel, AppError> {
        let model = ReconciliationEntity::find_by_id(id)
            .one(&*self.db)
            .await
            .map_err(|e| AppError::DatabaseError(e.to_string()))?
            .ok_or_else(|| AppError::NotFound("对账单不存在".to_string()))?;

        let mut active_model: ActiveModel = model.into();
        active_model.reconciliation_status = Set(Some("disputed".to_string()));
        active_model.dispute_reason = Set(Some(reason));
        active_model.updated_at = Set(Utc::now());

        let updated = active_model
            .update(&*self.db)
            .await
            .map_err(|e| AppError::DatabaseError(e.to_string()))?;

        Ok(updated)
    }

    /// 关闭对账单
    pub async fn close(&self, id: i32) -> Result<ReconciliationModel, AppError> {
        let model = ReconciliationEntity::find_by_id(id)
            .one(&*self.db)
            .await
            .map_err(|e| AppError::DatabaseError(e.to_string()))?
            .ok_or_else(|| AppError::NotFound("对账单不存在".to_string()))?;

        let mut active_model: ActiveModel = model.into();
        active_model.reconciliation_status = Set(Some("closed".to_string()));
        active_model.updated_at = Set(Utc::now());

        let updated = active_model
            .update(&*self.db)
            .await
            .map_err(|e| AppError::DatabaseError(e.to_string()))?;

        Ok(updated)
    }

    /// 更新对账单状态（通用）
    pub async fn update_status(&self, id: i32, status: &str) -> Result<ReconciliationModel, AppError> {
        let model = ReconciliationEntity::find_by_id(id)
            .one(&*self.db)
            .await
            .map_err(|e| AppError::DatabaseError(e.to_string()))?
            .ok_or_else(|| AppError::NotFound("对账单不存在".to_string()))?;

        let mut active_model: ActiveModel = model.into();
        active_model.reconciliation_status = Set(Some(status.to_string()));
        active_model.updated_at = Set(Utc::now());

        let updated = active_model
            .update(&*self.db)
            .await
            .map_err(|e| AppError::DatabaseError(e.to_string()))?;

        Ok(updated)
    }
}
