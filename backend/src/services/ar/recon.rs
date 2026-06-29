//! 应收对账单主流程服务（ar/recon）
//!
//! 包含对账单的 CRUD、状态机（draft → sent → confirmed/disputed → closed）、
//! 通用状态更新与对账明细查询。拆分自原 `ar_reconciliation_service.rs`。
//!
//! 协作子模块：
//! - `vfy` 自动对账算法、自动生成、客户确认/争议
//! - `inv` PDF 导出

use chrono::Utc;
use sea_orm::{
    ActiveModelTrait, ColumnTrait, EntityTrait, PaginatorTrait, QueryFilter, QueryOrder,
    QuerySelect, Set, TransactionTrait,
};

use crate::models::ar_reconciliation::{
    ActiveModel, Entity as ReconciliationEntity, Model as ReconciliationModel,
};
use crate::models::ar_reconciliation_item;
use crate::utils::error::AppError;

use super::{
    ArReconciliationService, CreateReconciliationRequest, ReconciliationDetail,
    ReconciliationQuery, ReconciliationWithDetails, UpdateReconciliationRequest,
};

impl ArReconciliationService {
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

        let model = active_model.insert(&*self.db).await?;

        Ok(model)
    }

    /// 根据ID获取对账单
    pub async fn get_by_id(&self, id: i32) -> Result<Option<ReconciliationModel>, AppError> {
        let model = ReconciliationEntity::find_by_id(id).one(&*self.db).await?;

        Ok(model)
    }

    /// 获取对账单列表
    pub async fn list(
        &self,
        query: ReconciliationQuery,
    ) -> Result<(Vec<ReconciliationModel>, u64), AppError> {
        let mut select = ReconciliationEntity::find();

        if let Some(status) = query.status {
            select = select
                .filter(crate::models::ar_reconciliation::Column::ReconciliationStatus.eq(status));
        }

        if let Some(customer_id) = query.customer_id {
            select =
                select.filter(crate::models::ar_reconciliation::Column::CustomerId.eq(customer_id));
        }

        let total = select.clone().count(&*self.db).await?;

        let paginator = select
            .order_by_desc(crate::models::ar_reconciliation::Column::CreatedAt)
            .paginate(&*self.db, query.page_size);

        let models = paginator.fetch_page(query.page - 1).await?;

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
            .await?
            .ok_or_else(|| AppError::not_found("对账单不存在"))?;

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

        let opening = *active_model.opening_balance.as_ref();
        let invoices = *active_model.total_invoices.as_ref();
        let collections = *active_model.total_collections.as_ref();
        active_model.closing_balance = Set(opening + invoices - collections);

        active_model.updated_at = Set(Utc::now());

        let updated = active_model.update(&*self.db).await?;

        Ok(updated)
    }

    /// 删除对账单
    pub async fn delete(&self, id: i32) -> Result<(), AppError> {
        let model = ReconciliationEntity::find_by_id(id)
            .one(&*self.db)
            .await?
            .ok_or_else(|| AppError::not_found("对账单不存在"))?;

        // 只有草稿状态的对账单可以删除
        if model.reconciliation_status.as_deref() != Some("draft") {
            return Err(AppError::business(
                "只有草稿状态的对账单可以删除".to_string(),
            ));
        }

        ReconciliationEntity::delete_by_id(id)
            .exec(&*self.db)
            .await?;

        Ok(())
    }

    /// 发送对账单
    pub async fn send(&self, id: i32) -> Result<ReconciliationModel, AppError> {
        let model = ReconciliationEntity::find_by_id(id)
            .one(&*self.db)
            .await?
            .ok_or_else(|| AppError::not_found("对账单不存在"))?;

        if model.reconciliation_status.as_deref() != Some("draft") {
            return Err(AppError::business(
                "只有草稿状态的对账单可以发送".to_string(),
            ));
        }

        let mut active_model: ActiveModel = model.into();
        active_model.reconciliation_status = Set(Some("sent".to_string()));
        active_model.updated_at = Set(Utc::now());

        let updated = active_model.update(&*self.db).await?;

        Ok(updated)
    }

    /// 客户确认对账单
    pub async fn confirm(
        &self,
        id: i32,
        confirmed_by: Option<i32>,
    ) -> Result<ReconciliationModel, AppError> {
        // 批次 25 v6 P0 修复：状态机 lock_exclusive 补全，串行化并发状态变更
        // 原实现状态变更无 txn 无 lock，两并发 confirm 同时通过门控后基于过期状态写入，
        // 导致 confirmed_by/confirmed_at 被覆盖、状态机被并发破坏。
        let txn = (*self.db).begin().await?;

        let model = ReconciliationEntity::find_by_id(id)
            .lock_exclusive()
            .one(&txn)
            .await?
            .ok_or_else(|| AppError::not_found("对账单不存在"))?;

        let mut active_model: ActiveModel = model.into();
        active_model.reconciliation_status = Set(Some("confirmed".to_string()));
        active_model.confirmed_by_customer = Set(Some(true));
        active_model.confirmed_by = Set(confirmed_by);
        active_model.confirmed_at = Set(Some(Utc::now()));
        active_model.updated_at = Set(Utc::now());

        let result = crate::services::audit_log_service::AuditLogService::update_with_audit(
            &txn,
            "auto_audit",
            active_model,
            confirmed_by,
        )
        .await?;

        txn.commit().await?;

        Ok(result)
    }

    /// 客户提出争议
    pub async fn dispute(&self, id: i32, reason: String) -> Result<ReconciliationModel, AppError> {
        // 批次 25 v6 P0 修复：状态机 lock_exclusive 补全，串行化并发状态变更
        // 原实现状态变更无 txn 无 lock，两并发 dispute 同时通过门控后基于过期状态写入，
        // 导致 dispute_reason 被覆盖、状态机被并发破坏。
        let txn = (*self.db).begin().await?;

        let model = ReconciliationEntity::find_by_id(id)
            .lock_exclusive()
            .one(&txn)
            .await?
            .ok_or_else(|| AppError::not_found("对账单不存在"))?;

        let mut active_model: ActiveModel = model.into();
        active_model.reconciliation_status = Set(Some("disputed".to_string()));
        active_model.dispute_reason = Set(Some(reason));
        active_model.updated_at = Set(Utc::now());

        // TODO(tech-debt): dispute 方法签名暂无 user_id 参数，先用 Some(0) 占位，
        // 待认证上下文接入后改为真实 user_id。
        let result = crate::services::audit_log_service::AuditLogService::update_with_audit(
            &txn,
            "auto_audit",
            active_model,
            Some(0),
        )
        .await?;

        txn.commit().await?;

        Ok(result)
    }

    /// 关闭对账单
    pub async fn close(&self, id: i32) -> Result<ReconciliationModel, AppError> {
        // 批次 25 v6 P0 修复：状态机 lock_exclusive 补全，串行化并发状态变更
        // 原实现状态变更无 txn 无 lock，状态门控（confirmed/disputed → closed）在并发场景下
        // 会被竞态绕过：两并发 close 同时通过门控后基于过期状态写入。
        let txn = (*self.db).begin().await?;

        let model = ReconciliationEntity::find_by_id(id)
            .lock_exclusive()
            .one(&txn)
            .await?
            .ok_or_else(|| AppError::not_found("对账单不存在"))?;

        let status = model.reconciliation_status.as_deref().unwrap_or("draft");
        if status != "confirmed" && status != "disputed" {
            return Err(AppError::business(
                "只有已确认或有争议的对账单可以关闭".to_string(),
            ));
        }

        let mut active_model: ActiveModel = model.into();
        active_model.reconciliation_status = Set(Some("closed".to_string()));
        active_model.updated_at = Set(Utc::now());

        // TODO(tech-debt): close 方法签名暂无 user_id 参数，先用 Some(0) 占位，
        // 待认证上下文接入后改为真实 user_id。
        let result = crate::services::audit_log_service::AuditLogService::update_with_audit(
            &txn,
            "auto_audit",
            active_model,
            Some(0),
        )
        .await?;

        txn.commit().await?;

        Ok(result)
    }

    /// 更新对账单状态（通用）
    pub async fn update_status(
        &self,
        id: i32,
        status: &str,
    ) -> Result<ReconciliationModel, AppError> {
        let model = ReconciliationEntity::find_by_id(id)
            .one(&*self.db)
            .await?
            .ok_or_else(|| AppError::not_found("对账单不存在"))?;

        let mut active_model: ActiveModel = model.into();
        active_model.reconciliation_status = Set(Some(status.to_string()));
        active_model.updated_at = Set(Utc::now());

        let updated = active_model.update(&*self.db).await?;

        Ok(updated)
    }

    /// 获取对账单及其明细
    pub async fn get_with_details(&self, id: i32) -> Result<ReconciliationWithDetails, AppError> {
        let reconciliation = ReconciliationEntity::find_by_id(id)
            .one(&*self.db)
            .await?
            .ok_or_else(|| AppError::not_found("对账单不存在"))?;

        let items = ar_reconciliation_item::Entity::find()
            .filter(ar_reconciliation_item::Column::ReconciliationId.eq(id))
            .order_by(
                ar_reconciliation_item::Column::CreatedAt,
                crate::services::ar::SharedOrder::Asc,
            )
            .all(&*self.db)
            .await?;

        let details: Vec<ReconciliationDetail> = items
            .into_iter()
            .map(|item| ReconciliationDetail {
                id: item.id,
                reconciliation_id: item.reconciliation_id,
                item_type: item.item_type,
                document_type: item.document_type,
                document_id: item.document_id,
                document_no: item.document_no,
                document_date: item.document_date,
                amount: item.amount,
                matched_amount: item.matched_amount,
                match_status: item.match_status,
                matched_item_id: item.matched_item_id,
                remarks: item.remarks,
            })
            .collect();

        Ok(ReconciliationWithDetails {
            reconciliation,
            details,
        })
    }
}
