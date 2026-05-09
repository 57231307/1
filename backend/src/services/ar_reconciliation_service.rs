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
    pub start_date: NaiveDate,
    pub end_date: NaiveDate,
    pub opening_balance: Decimal,
    pub current_receivable: Decimal,
    pub current_received: Decimal,
    pub remarks: Option<String>,
}

/// 更新对账单请求
#[derive(Debug, Clone)]
pub struct UpdateReconciliationRequest {
    pub opening_balance: Option<Decimal>,
    pub current_receivable: Option<Decimal>,
    pub current_received: Option<Decimal>,
    pub remarks: Option<String>,
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
        let closing_balance = req.opening_balance + req.current_receivable - req.current_received;

        let active_model = ActiveModel {
            reconciliation_no: Set(req.reconciliation_no),
            customer_id: Set(req.customer_id),
            start_date: Set(req.start_date),
            end_date: Set(req.end_date),
            opening_balance: Set(req.opening_balance),
            current_receivable: Set(req.current_receivable),
            current_received: Set(req.current_received),
            closing_balance: Set(closing_balance),
            status: Set("DRAFT".to_string()),
            confirmed_date: Set(None),
            dispute_reason: Set(None),
            remarks: Set(req.remarks),
            is_deleted: Set(false),
            created_at: Set(Utc::now()),
            updated_at: Set(Utc::now()),
            ..Default::default()
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
        let mut select = ReconciliationEntity::find()
            .filter(crate::models::ar_reconciliation::Column::IsDeleted.eq(false));

        if let Some(status) = query.status {
            select = select.filter(crate::models::ar_reconciliation::Column::Status.eq(status));
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
        if let Some(current_receivable) = req.current_receivable {
            active_model.current_receivable = Set(current_receivable);
        }
        if let Some(current_received) = req.current_received {
            active_model.current_received = Set(current_received);
        }
        if let Some(remarks) = req.remarks {
            active_model.remarks = Set(Some(remarks));
        }

        // 重新计算期末余额
        let opening = active_model.opening_balance.as_ref().clone();
        let receivable = active_model.current_receivable.as_ref().clone();
        let received = active_model.current_received.as_ref().clone();
        active_model.closing_balance = Set(opening + receivable - received);

        active_model.updated_at = Set(Utc::now());

        let updated = active_model
            .update(&*self.db)
            .await
            .map_err(|e| AppError::DatabaseError(e.to_string()))?;

        Ok(updated)
    }

    /// 删除对账单（软删除）
    pub async fn delete(&self, id: i32) -> Result<(), AppError> {
        let model = ReconciliationEntity::find_by_id(id)
            .one(&*self.db)
            .await
            .map_err(|e| AppError::DatabaseError(e.to_string()))?
            .ok_or_else(|| AppError::NotFound("对账单不存在".to_string()))?;

        let mut active_model: ActiveModel = model.into();
        active_model.is_deleted = Set(true);
        active_model.updated_at = Set(Utc::now());

        active_model
            .update(&*self.db)
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
        active_model.status = Set("SENT".to_string());
        active_model.updated_at = Set(Utc::now());

        let updated = active_model
            .update(&*self.db)
            .await
            .map_err(|e| AppError::DatabaseError(e.to_string()))?;

        Ok(updated)
    }

    /// 客户确认对账单
    pub async fn confirm(&self, id: i32) -> Result<ReconciliationModel, AppError> {
        let model = ReconciliationEntity::find_by_id(id)
            .one(&*self.db)
            .await
            .map_err(|e| AppError::DatabaseError(e.to_string()))?
            .ok_or_else(|| AppError::NotFound("对账单不存在".to_string()))?;

        let mut active_model: ActiveModel = model.into();
        active_model.status = Set("CONFIRMED".to_string());
        active_model.confirmed_date = Set(Some(Utc::now().date_naive()));
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
        active_model.status = Set("DISPUTED".to_string());
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
        active_model.status = Set("CLOSED".to_string());
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
        active_model.status = Set(status.to_string());
        active_model.updated_at = Set(Utc::now());

        let updated = active_model
            .update(&*self.db)
            .await
            .map_err(|e| AppError::DatabaseError(e.to_string()))?;

        Ok(updated)
    }
}
