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

/// 批次 108 P1-6 修复：update/delete/send/confirm/dispute/close 方法已通过
/// /ar-reconciliations 路由接入业务（ar_reconciliation_handler.rs + routes/finance.rs），
/// 移除 dead_code 标注。
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

    /// 删除对账单
    pub async fn delete(&self, id: i32, user_id: i32) -> Result<(), AppError> {
        // 批次 93 P1-2 修复：状态门 + delete 移入同一事务，补 lock_exclusive 串行化并发
        // 原实现 find_by_id 在 self.db → 状态门 → delete_with_audit 在 self.db，
        // 状态门与 delete 跨事务边界，并发 delete + send 会竞态绕过 draft 状态门控。
        let txn = (*self.db).begin().await?;

        let model = ReconciliationEntity::find_by_id(id)
            .lock_exclusive()
            .one(&txn)
            .await?
            .ok_or_else(|| AppError::not_found("对账单不存在"))?;

        // 只有草稿状态的对账单可以删除（状态门在 txn 内，基于 lock_exclusive 读出的 model）
        if model.reconciliation_status.as_deref() != Some("draft") {
            return Err(AppError::business(
                "只有草稿状态的对账单可以删除".to_string(),
            ));
        }

        // P0 8-3 修复：delete 操作补审计日志
        // 批次 92 P3-9：user_id 从 handler AuthContext 注入
        crate::services::audit_log_service::AuditLogService::delete_with_audit::<
            ReconciliationEntity,
            _,
        >(&txn, "ar_reconciliation", id, Some(user_id))
        .await?;

        txn.commit().await?;
        Ok(())
    }

    /// 发送对账单
    pub async fn send(&self, id: i32, user_id: i32) -> Result<ReconciliationModel, AppError> {
        // P1 3-3 修复（批次 61）：状态机 lock_exclusive 补全，串行化并发发送
        // 原实现无 txn 无 lock，状态门在事务外，并发 send 会竞态绕过 draft 状态门控。
        let txn = (*self.db).begin().await?;

        let model = ReconciliationEntity::find_by_id(id)
            .lock_exclusive()
            .one(&txn)
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

    /// 关闭对账单
    pub async fn close(&self, id: i32, user_id: i32) -> Result<ReconciliationModel, AppError> {
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

        // 批次 92 P3-9：user_id 从 handler AuthContext 注入
        let result = crate::services::audit_log_service::AuditLogService::update_with_audit(
            &txn,
            "auto_audit",
            active_model,
            Some(user_id),
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
        user_id: i32,
    ) -> Result<ReconciliationModel, AppError> {
        // P1 3-2 修复（批次 61）：状态机 lock_exclusive 补全 + 状态白名单
        // 原实现无 txn 无 lock，且无状态白名单，任意字符串都能写入 reconciliation_status，
        // 可能导致状态机被非法值破坏。改为 txn + lock_exclusive + 白名单校验。
        let txn = (*self.db).begin().await?;

        let model = ReconciliationEntity::find_by_id(id)
            .lock_exclusive()
            .one(&txn)
            .await?
            .ok_or_else(|| AppError::not_found("对账单不存在"))?;

        // 状态白名单：仅允许合法的状态值
        let allowed_statuses = ["draft", "sent", "confirmed", "disputed", "closed"];
        if !allowed_statuses.contains(&status) {
            return Err(AppError::business(format!(
                "非法的对账单状态：{}，允许的状态：{:?}",
                status, allowed_statuses
            )));
        }

        let mut active_model: ActiveModel = model.into();
        active_model.reconciliation_status = Set(Some(status.to_string()));
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
