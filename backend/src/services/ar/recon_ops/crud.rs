//! 应收对账单 - CRUD 操作（ar/recon_ops/crud）
//!
//! 批次 D10 拆分自原 `ar/recon.rs` 的 create / get_by_id / list / update / get_with_details 方法。
//! 职责：对账单的基本增删改查与明细查询。
//! 本模块扩展 `ArReconciliationService` 的 CRUD 公开方法。
//!
//! 批次 108 P1-6 修复：update 等方法已通过 /ar-reconciliations 路由接入业务
//! （ar_reconciliation_handler.rs + routes/finance.rs），移除 dead_code 标注。

use chrono::Utc;
use sea_orm::{
    ActiveModelTrait, ColumnTrait, EntityTrait, PaginatorTrait, QueryFilter, QueryOrder,
    QuerySelect, Set, TransactionTrait,
};

use crate::models::ar_reconciliation::{
    ActiveModel, Entity as ReconciliationEntity, Model as ReconciliationModel,
};
use crate::models::ar_reconciliation_item;
use crate::models::status::ar as ar_status;
use crate::utils::error::AppError;

use super::super::{
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
            id: Default::default(),
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
            reconciliation_status: Set(Some(ar_status::RECONCILIATION_DRAFT.to_string())),
            confirmed_by_customer: Set(None),
            dispute_reason: Set(None),
            confirmed_by: Set(None),
            confirmed_at: Set(None),
            created_by: Set(None),
            created_at: Set(Utc::now()),
            updated_at: Set(Utc::now()),
            // 批次 109 P1-1：接入 notes 持久化（原 DTO 有字段但未写入 DB）
            notes: Set(req.notes),
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

        // 批次 109 P3：按对账单日期范围过滤（原 ListResultsQuery.start_date/end_date 未使用）
        if let Some(start_date) = query.start_date {
            select = select.filter(
                crate::models::ar_reconciliation::Column::ReconciliationDate.gte(start_date),
            );
        }
        if let Some(end_date) = query.end_date {
            select = select.filter(
                crate::models::ar_reconciliation::Column::ReconciliationDate.lte(end_date),
            );
        }

        let total = select.clone().count(&*self.db).await?;

        let paginator = select
            .order_by_desc(crate::models::ar_reconciliation::Column::CreatedAt)
            .paginate(&*self.db, query.page_size);

        let models = paginator.fetch_page(query.page.saturating_sub(1)).await?;

        Ok((models, total))
    }

    /// 更新对账单
    pub async fn update(
        &self,
        id: i32,
        req: UpdateReconciliationRequest,
        user_id: i32,
    ) -> Result<ReconciliationModel, AppError> {
        // P1 3-1 修复（批次 61）：状态机 lock_exclusive 补全，串行化并发更新
        // 原实现无 txn 无 lock，并发更新会导致 closing_balance 计算基于过期数据。
        let txn = (*self.db).begin().await?;

        let model = ReconciliationEntity::find_by_id(id)
            .lock_exclusive()
            .one(&txn)
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
        // 批次 109 P1-1：接入 notes 持久化（原 DTO 有字段但未写入 DB）
        if let Some(notes) = req.notes {
            active_model.notes = Set(Some(notes));
        }

        let opening = *active_model.opening_balance.as_ref();
        let invoices = *active_model.total_invoices.as_ref();
        let collections = *active_model.total_collections.as_ref();
        active_model.closing_balance = Set(opening + invoices - collections);

        active_model.updated_at = Set(Utc::now());

        // 批次 92 P3-9：user_id 从 handler AuthContext 注入
        let updated =
            crate::services::audit_log_service::AuditLogService::update_with_audit(
                &txn,
                "auto_audit",
                active_model,
                Some(user_id),
            )
            .await?;

        txn.commit().await?;

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
