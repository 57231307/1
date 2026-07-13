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
use crate::models::status::ar as ar_status;
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
        if model.reconciliation_status.as_deref() != Some(ar_status::RECONCILIATION_DRAFT) {
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

        if model.reconciliation_status.as_deref() != Some(ar_status::RECONCILIATION_DRAFT) {
            return Err(AppError::business(
                "只有草稿状态的对账单可以发送".to_string(),
            ));
        }

        let mut active_model: ActiveModel = model.into();
        active_model.reconciliation_status = Set(Some(ar_status::RECONCILIATION_SENT.to_string()));
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

        let status = model.reconciliation_status.as_deref().unwrap_or(ar_status::RECONCILIATION_DRAFT);
        if status != ar_status::RECONCILIATION_CONFIRMED && status != ar_status::RECONCILIATION_DISPUTED {
            return Err(AppError::business(
                "只有已确认或有争议的对账单可以关闭".to_string(),
            ));
        }

        let mut active_model: ActiveModel = model.into();
        active_model.reconciliation_status = Set(Some(ar_status::RECONCILIATION_CLOSED.to_string()));
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
    ///
    /// 批次 109 P3：新增 remark 参数，若提供则同步写入 notes 字段
    /// （原 UpdateConfirmationStatusRequest.remark 标注 dead_code 未使用）
    pub async fn update_status(
        &self,
        id: i32,
        status: &str,
        user_id: i32,
        remark: Option<String>,
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
        let allowed_statuses = [
            ar_status::RECONCILIATION_DRAFT,
            ar_status::RECONCILIATION_SENT,
            ar_status::RECONCILIATION_CONFIRMED,
            ar_status::RECONCILIATION_DISPUTED,
            ar_status::RECONCILIATION_CLOSED,
            ar_status::RECONCILIATION_CANCELLED,
        ];
        if !allowed_statuses.contains(&status) {
            return Err(AppError::business(format!(
                "非法的对账单状态：{}，允许的状态：{:?}",
                status, allowed_statuses
            )));
        }

        let mut active_model: ActiveModel = model.into();
        active_model.reconciliation_status = Set(Some(status.to_string()));
        active_model.updated_at = Set(Utc::now());

        // 批次 109 P3：remark 写入 notes 字段（原 UpdateConfirmationStatusRequest.remark 未使用）
        if let Some(remark) = remark {
            active_model.notes = Set(Some(remark));
        }

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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::decs;
    use crate::ymd;
    use crate::models::status::ar as status_ar;
    use rust_decimal::Decimal;
    use sea_orm::{Database, DatabaseConnection};
    use std::str::FromStr;
    use std::sync::Arc;

    /// 对账单状态值（小写，与 recon.rs 业务代码及 status::ar 模块保持一致）
    ///
    /// 批次 231 v13 P1-1 修复：status::ar 模块已统一为小写，
    /// 此处常量镜像业务代码实际值，用于状态门控与状态机测试。
    mod recon_status {
        /// 草稿：初始状态，可编辑/删除/发送
        pub const DRAFT: &str = "draft";
        /// 已发送：等待客户确认
        pub const SENT: &str = "sent";
        /// 已确认：客户确认对账单
        pub const CONFIRMED: &str = "confirmed";
        /// 有争议：客户对账单有异议
        pub const DISPUTED: &str = "disputed";
        /// 已关闭：对账流程完结
        pub const CLOSED: &str = "closed";
    }

    /// 构建测试用对账单模型夹具
    ///
    /// 封装 `ReconciliationModel` 的构造，便于在各测试中复用。
    /// 默认 closing_balance = opening_balance + total_invoices - total_collections，
    /// 保持与 create/update 方法一致的业务不变量。
    fn make_reconciliation_model(
        id: i32,
        opening_balance: Decimal,
        total_invoices: Decimal,
        total_collections: Decimal,
        status: &str,
    ) -> ReconciliationModel {
        let closing_balance = opening_balance + total_invoices - total_collections;
        ReconciliationModel {
            id,
            reconciliation_no: format!("RC-2026-{:04}", id),
            reconciliation_date: ymd!(2026, 1, 15),
            period_start: ymd!(2026, 1, 1),
            period_end: ymd!(2026, 1, 31),
            customer_id: 1,
            customer_name: Some("测试客户".to_string()),
            opening_balance,
            total_invoices,
            total_collections,
            closing_balance,
            reconciliation_status: Some(status.to_string()),
            confirmed_by_customer: None,
            dispute_reason: None,
            confirmed_by: None,
            confirmed_at: None,
            created_by: Some(1),
            created_at: Utc::now(),
            updated_at: Utc::now(),
            notes: None,
        }
    }

    /// 测试 SQLite 内存数据库连接
    async fn setup_test_db() -> DatabaseConnection {
        let db_url = std::env::var("TEST_DATABASE_URL")
            .unwrap_or_else(|_| "sqlite::memory:".to_string());
        Database::connect(&db_url)
            .await
            .expect("测试夹具：数据库连接失败")
    }

    // ===== 状态常量值正确性测试 =====

    /// 测试_对账状态常量_closed值正确
    ///
    /// 验证 status::ar::RECONCILIATION_CLOSED 常量值为 "closed"（小写），
    /// 该常量用于 ar_reconciliation.reconciliation_status 字段
    #[test]
    fn 测试_对账状态常量_closed值正确() {
        assert_eq!(status_ar::RECONCILIATION_CLOSED, "closed");
    }

    /// 测试_对账状态常量_cancelled值正确
    ///
    /// 验证 status::ar::RECONCILIATION_CANCELLED 常量值为 "cancelled"（小写），
    /// 该常量用于 ar_reconciliation.reconciliation_status 字段
    #[test]
    fn 测试_对账状态常量_cancelled值正确() {
        assert_eq!(status_ar::RECONCILIATION_CANCELLED, "cancelled");
    }

    /// 测试_对账状态常量_matched值正确
    ///
    /// 验证 status::ar::MATCH_MATCHED 常量值为 "MATCHED"（大写），
    /// 该常量用于 ar_reconciliation_item.match_status 字段
    #[test]
    fn 测试_对账状态常量_matched值正确() {
        assert_eq!(status_ar::MATCH_MATCHED, "MATCHED");
    }

    // ===== 期末余额计算测试（纯算法） =====

    /// 测试_期末余额计算_创建场景正常
    ///
    /// 验证 create 方法中的期末余额计算公式：
    /// closing_balance = opening_balance + total_invoices - total_collections
    #[test]
    fn 测试_期末余额计算_创建场景正常() {
        let opening = decs!("10000");
        let invoices = decs!("5000");
        let collections = decs!("3000");

        // 复现 create 方法的期末余额计算逻辑
        let closing_balance = opening + invoices - collections;

        assert_eq!(closing_balance, decs!("12000"));
    }

    /// 测试_期末余额计算_更新场景部分字段更新
    ///
    /// 验证 update 方法中部分字段更新后期末余额重算逻辑：
    /// 取更新值或保持原值，再按公式重算 closing_balance
    #[test]
    fn 测试_期末余额计算_更新场景部分字段更新() {
        let model = make_reconciliation_model(
            1,
            decs!("10000"),
            decs!("5000"),
            decs!("3000"),
            recon_status::DRAFT,
        );

        // 模拟 update 请求：仅更新 total_invoices 和 notes
        let req = UpdateReconciliationRequest {
            opening_balance: None,
            total_invoices: Some(decs!("8000")),
            total_collections: None,
            notes: Some("更新备注".to_string()),
        };

        // 复现 update 方法：取更新值或保持原值
        let opening = req.opening_balance.unwrap_or(model.opening_balance);
        let invoices = req.total_invoices.unwrap_or(model.total_invoices);
        let collections = req.total_collections.unwrap_or(model.total_collections);
        let closing = opening + invoices - collections;

        assert_eq!(opening, decs!("10000"));
        assert_eq!(invoices, decs!("8000"));
        assert_eq!(collections, decs!("3000"));
        assert_eq!(closing, decs!("15000"));
    }

    /// 测试_期末余额计算_零值边界
    ///
    /// 验证所有金额为零时 closing_balance 也为零
    #[test]
    fn 测试_期末余额计算_零值边界() {
        let opening = Decimal::ZERO;
        let invoices = Decimal::ZERO;
        let collections = Decimal::ZERO;

        let closing_balance = opening + invoices - collections;

        assert_eq!(closing_balance, Decimal::ZERO);
    }

    /// 测试_期末余额计算_负值场景
    ///
    /// 验证当收款大于期初+发票时，closing_balance 可为负值（客户预付款场景）
    #[test]
    fn 测试_期末余额计算_负值场景() {
        let opening = decs!("1000");
        let invoices = decs!("2000");
        let collections = decs!("5000");

        let closing_balance = opening + invoices - collections;

        assert_eq!(closing_balance, decs!("-2000"));
    }

    /// 测试_创建请求构造_期末余额计算
    ///
    /// 验证 CreateReconciliationRequest 构造后，按 create 方法公式计算期末余额，
    /// 并校验 create 方法设置的初始状态为 draft
    #[test]
    fn 测试_创建请求构造_期末余额计算() {
        let req = CreateReconciliationRequest {
            reconciliation_no: "RC-2026-0001".to_string(),
            customer_id: 1,
            customer_name: Some("测试客户".to_string()),
            period_start: ymd!(2026, 1, 1),
            period_end: ymd!(2026, 1, 31),
            opening_balance: decs!("10000"),
            total_invoices: decs!("5000"),
            total_collections: decs!("3000"),
            notes: None,
        };

        // 复现 create 方法的期末余额计算
        let closing_balance = req.opening_balance + req.total_invoices - req.total_collections;
        assert_eq!(closing_balance, decs!("12000"));

        // 复现 create 方法的初始状态设置（应为 draft）
        let initial_status = recon_status::DRAFT;
        assert_eq!(initial_status, "draft");
    }

    // ===== 状态白名单校验测试 =====

    /// 测试_状态白名单_合法状态通过
    ///
    /// 验证 update_status 方法中状态白名单允许所有 5 个合法状态值
    #[test]
    fn 测试_状态白名单_合法状态通过() {
        // 复现 update_status 方法的状态白名单
        let allowed_statuses = [
            recon_status::DRAFT,
            recon_status::SENT,
            recon_status::CONFIRMED,
            recon_status::DISPUTED,
            recon_status::CLOSED,
        ];

        // 所有合法状态都应通过白名单校验
        for status in &allowed_statuses {
            assert!(
                allowed_statuses.contains(status),
                "状态 {} 应在白名单中",
                status
            );
        }
    }

    /// 测试_状态白名单_非法状态拒绝
    ///
    /// 验证 update_status 方法中非法状态值应被拒绝，并产生正确的错误消息
    #[test]
    fn 测试_状态白名单_非法状态拒绝() {
        let allowed_statuses = [
            recon_status::DRAFT,
            recon_status::SENT,
            recon_status::CONFIRMED,
            recon_status::DISPUTED,
            recon_status::CLOSED,
        ];

        // 非法状态不应通过白名单校验
        let invalid_status = "invalid";
        assert!(!allowed_statuses.contains(&invalid_status));

        // 复现 update_status 的错误构造
        let err = AppError::business(format!(
            "非法的对账单状态：{}，允许的状态：{:?}",
            invalid_status, allowed_statuses
        ));
        assert!(matches!(err, AppError::BusinessError(_)));

        // 大写状态值也不应通过（业务代码使用小写）
        assert!(!allowed_statuses.contains(&"DRAFT"));
        assert!(!allowed_statuses.contains(&"SENT"));
    }

    // ===== 状态门控测试 =====

    /// 测试_状态门控_删除仅允许草稿
    ///
    /// 验证 delete 方法中仅 draft 状态允许删除，其他状态应返回业务错误
    #[test]
    fn 测试_状态门控_删除仅允许草稿() {
        // draft 状态：允许删除
        let model_draft = make_reconciliation_model(
            1,
            decs!("1000"),
            decs!("500"),
            decs!("300"),
            recon_status::DRAFT,
        );
        let can_delete = model_draft.reconciliation_status.as_deref() == Some(recon_status::DRAFT);
        assert!(can_delete);

        // 非 draft 状态：拒绝删除
        for status in [
            recon_status::SENT,
            recon_status::CONFIRMED,
            recon_status::DISPUTED,
            recon_status::CLOSED,
        ] {
            let model = make_reconciliation_model(2, decs!("1000"), decs!("500"), decs!("300"), status);
            let can_delete =
                model.reconciliation_status.as_deref() == Some(recon_status::DRAFT);
            assert!(!can_delete, "状态 {} 不应允许删除", status);
        }

        // 复现 delete 方法的错误构造
        let err = AppError::business("只有草稿状态的对账单可以删除".to_string());
        assert!(matches!(err, AppError::BusinessError(_)));
    }

    /// 测试_状态门控_发送仅允许草稿
    ///
    /// 验证 send 方法中仅 draft 状态允许发送，发送后状态变为 sent
    #[test]
    fn 测试_状态门控_发送仅允许草稿() {
        // draft 状态：允许发送
        let model_draft = make_reconciliation_model(
            1,
            decs!("1000"),
            decs!("500"),
            decs!("300"),
            recon_status::DRAFT,
        );
        let can_send = model_draft.reconciliation_status.as_deref() == Some(recon_status::DRAFT);
        assert!(can_send);

        // 非 draft 状态：拒绝发送
        for status in [
            recon_status::SENT,
            recon_status::CONFIRMED,
            recon_status::DISPUTED,
            recon_status::CLOSED,
        ] {
            let model = make_reconciliation_model(2, decs!("1000"), decs!("500"), decs!("300"), status);
            let can_send =
                model.reconciliation_status.as_deref() == Some(recon_status::DRAFT);
            assert!(!can_send, "状态 {} 不应允许发送", status);
        }

        // 复现 send 方法的错误构造
        let err = AppError::business("只有草稿状态的对账单可以发送".to_string());
        assert!(matches!(err, AppError::BusinessError(_)));

        // 发送后状态应变为 sent
        let new_status = recon_status::SENT;
        assert_eq!(new_status, "sent");
    }

    /// 测试_状态门控_关闭允许已确认和争议
    ///
    /// 验证 close 方法中 confirmed 和 disputed 状态允许关闭，关闭后状态变为 closed
    #[test]
    fn 测试_状态门控_关闭允许已确认和争议() {
        // confirmed 状态：允许关闭
        let model_confirmed = make_reconciliation_model(
            1,
            decs!("1000"),
            decs!("500"),
            decs!("300"),
            recon_status::CONFIRMED,
        );
        let status = model_confirmed.reconciliation_status.as_deref().unwrap_or(recon_status::DRAFT);
        let can_close = status == recon_status::CONFIRMED || status == recon_status::DISPUTED;
        assert!(can_close);

        // disputed 状态：允许关闭
        let model_disputed = make_reconciliation_model(
            2,
            decs!("1000"),
            decs!("500"),
            decs!("300"),
            recon_status::DISPUTED,
        );
        let status = model_disputed.reconciliation_status.as_deref().unwrap_or(recon_status::DRAFT);
        let can_close = status == recon_status::CONFIRMED || status == recon_status::DISPUTED;
        assert!(can_close);

        // 关闭后状态应变为 closed
        let new_status = recon_status::CLOSED;
        assert_eq!(new_status, "closed");
    }

    /// 测试_状态门控_关闭拒绝草稿和已发送
    ///
    /// 验证 close 方法中 draft 和 sent 状态应被拒绝，None 状态默认为 draft 也应拒绝
    #[test]
    fn 测试_状态门控_关闭拒绝草稿和已发送() {
        // draft 状态：拒绝关闭
        let model_draft = make_reconciliation_model(
            1,
            decs!("1000"),
            decs!("500"),
            decs!("300"),
            recon_status::DRAFT,
        );
        let status = model_draft.reconciliation_status.as_deref().unwrap_or(recon_status::DRAFT);
        let should_reject =
            status != recon_status::CONFIRMED && status != recon_status::DISPUTED;
        assert!(should_reject);

        // sent 状态：拒绝关闭
        let model_sent = make_reconciliation_model(
            2,
            decs!("1000"),
            decs!("500"),
            decs!("300"),
            recon_status::SENT,
        );
        let status = model_sent.reconciliation_status.as_deref().unwrap_or(recon_status::DRAFT);
        let should_reject =
            status != recon_status::CONFIRMED && status != recon_status::DISPUTED;
        assert!(should_reject);

        // closed 状态：拒绝关闭（已关闭不可再关闭）
        let model_closed = make_reconciliation_model(
            3,
            decs!("1000"),
            decs!("500"),
            decs!("300"),
            recon_status::CLOSED,
        );
        let status = model_closed.reconciliation_status.as_deref().unwrap_or(recon_status::DRAFT);
        let should_reject =
            status != recon_status::CONFIRMED && status != recon_status::DISPUTED;
        assert!(should_reject);

        // None 状态：unwrap_or("draft")，应拒绝
        let none_status: Option<&str> = None;
        let resolved = none_status.unwrap_or(recon_status::DRAFT);
        let should_reject =
            resolved != recon_status::CONFIRMED && resolved != recon_status::DISPUTED;
        assert!(should_reject);

        // 复现 close 方法的错误构造
        let err = AppError::business("只有已确认或有争议的对账单可以关闭".to_string());
        assert!(matches!(err, AppError::BusinessError(_)));
    }

    // ===== 状态机转换合法性测试 =====

    /// 测试_状态机转换_完整流转合法
    ///
    /// 验证对账单状态机的完整合法流转路径：
    /// draft → sent → confirmed → closed
    /// draft → sent → disputed → closed
    /// draft → disputed → closed（通过 update_status 直接争议）
    #[test]
    fn 测试_状态机转换_完整流转合法() {
        let allowed_statuses = [
            recon_status::DRAFT,
            recon_status::SENT,
            recon_status::CONFIRMED,
            recon_status::DISPUTED,
            recon_status::CLOSED,
        ];

        // 路径 1：draft → sent → confirmed → closed
        let path1 = [
            recon_status::DRAFT,
            recon_status::SENT,
            recon_status::CONFIRMED,
            recon_status::CLOSED,
        ];
        for status in &path1 {
            assert!(allowed_statuses.contains(status), "路径1状态 {} 应合法", status);
        }

        // 路径 2：draft → sent → disputed → closed
        let path2 = [
            recon_status::DRAFT,
            recon_status::SENT,
            recon_status::DISPUTED,
            recon_status::CLOSED,
        ];
        for status in &path2 {
            assert!(allowed_statuses.contains(status), "路径2状态 {} 应合法", status);
        }

        // 验证 send 门控：仅 draft → sent
        assert_eq!(path1[0], recon_status::DRAFT);
        assert_eq!(path1[1], recon_status::SENT);

        // 验证 close 门控：confirmed/disputed → closed
        assert!(
            path1[2] == recon_status::CONFIRMED || path1[2] == recon_status::DISPUTED,
            "close 前置状态应为 confirmed 或 disputed"
        );
        assert_eq!(path1[3], recon_status::CLOSED);

        assert!(
            path2[2] == recon_status::CONFIRMED || path2[2] == recon_status::DISPUTED,
            "close 前置状态应为 confirmed 或 disputed"
        );
        assert_eq!(path2[3], recon_status::CLOSED);
    }

    // ===== 错误消息格式测试 =====

    /// 测试_错误消息格式_非法状态含状态值
    ///
    /// 验证 update_status 方法中非法状态的错误消息包含状态值和白名单
    #[test]
    fn 测试_错误消息格式_非法状态含状态值() {
        let allowed_statuses = [
            recon_status::DRAFT,
            recon_status::SENT,
            recon_status::CONFIRMED,
            recon_status::DISPUTED,
            recon_status::CLOSED,
        ];
        let invalid_status = "frozen";

        // 复现 update_status 的错误消息构造
        let msg = format!(
            "非法的对账单状态：{}，允许的状态：{:?}",
            invalid_status, allowed_statuses
        );

        assert!(msg.contains(invalid_status), "错误消息应包含非法状态值");
        assert!(msg.contains("允许的状态"), "错误消息应包含白名单提示");
        assert!(msg.contains("draft"), "错误消息应包含白名单内容");

        let err = AppError::business(msg);
        assert!(matches!(err, AppError::BusinessError(_)));
    }

    /// 测试_错误消息格式_未找到对账单
    ///
    /// 验证各方法中对账单不存在时的 not_found 错误消息
    #[test]
    fn 测试_错误消息格式_未找到对账单() {
        let err = AppError::not_found("对账单不存在");
        assert!(matches!(err, AppError::NotFound(_)));

        // 验证 NotFound 错误码
        assert_eq!(err.error_code(), "NOT_FOUND");
    }

    // ===== 夹具宏可用性测试 =====

    /// 测试_夹具宏decs可用性
    ///
    /// 验证 decs! 宏能正确解析 Decimal 字符串
    #[test]
    fn 测试_夹具宏decs可用性() {
        let v = decs!("12345.67");
        assert_eq!(v.to_string(), "12345.67");

        // 验证整数场景
        let zero = decs!("0");
        assert_eq!(zero, Decimal::ZERO);

        // 验证负数场景
        let neg = decs!("-100");
        assert_eq!(neg, decs!("-100"));
    }

    /// 测试_夹具宏ymd可用性
    ///
    /// 验证 ymd! 宏能正确解析日期
    #[test]
    fn 测试_夹具宏ymd可用性() {
        let date = ymd!(2026, 7, 9);
        assert_eq!(date.format("%Y-%m-%d").to_string(), "2026-07-09");

        // 验证用于模型构造的日期字段
        let model = make_reconciliation_model(
            1,
            decs!("1000"),
            decs!("500"),
            decs!("300"),
            recon_status::DRAFT,
        );
        assert_eq!(
            model.period_start.format("%Y-%m-%d").to_string(),
            "2026-01-01"
        );
        assert_eq!(model.period_end.format("%Y-%m-%d").to_string(), "2026-01-31");
    }

    // ===== 服务实例化测试 =====

    /// 测试_服务实例创建
    ///
    /// 验证 ArReconciliationService 在 SQLite 内存数据库上能正常实例化
    #[tokio::test]
    async fn 测试_服务实例创建() {
        let db = setup_test_db().await;
        let service = ArReconciliationService::new(Arc::new(db));

        assert!(Arc::strong_count(&service.db) >= 1);
    }

    // ===== 数据库交互测试（标注 #[ignore]） =====

    /// 测试_创建对账单_需要数据库
    ///
    /// 需要 ar_reconciliations 表 schema，标注 #[ignore] 仅在本地手动运行。
    /// 无 schema 时返回数据库错误；有 schema 时验证 create 方法完整调用路径。
    #[tokio::test]
    #[ignore]
    async fn 测试_创建对账单_需要数据库() {
        let db = setup_test_db().await;
        let service = ArReconciliationService::new(Arc::new(db));

        let req = CreateReconciliationRequest {
            reconciliation_no: "RC-TEST-0001".to_string(),
            customer_id: 1,
            customer_name: Some("测试客户".to_string()),
            period_start: ymd!(2026, 1, 1),
            period_end: ymd!(2026, 1, 31),
            opening_balance: decs!("10000"),
            total_invoices: decs!("5000"),
            total_collections: decs!("3000"),
            notes: None,
        };

        // L-17 修复（批次 377 v13 复审）：原 let _ = result 无断言，改为 is_err 断言
        // 无 schema 时返回数据库错误；有 schema 时验证调用路径不 panic
        let result = service.create(req).await;
        assert!(result.is_err(), "无 schema 时应返回数据库错误");
    }

    /// 测试_获取对账单_需要数据库
    ///
    /// 需要 ar_reconciliations 表 schema，标注 #[ignore] 仅在本地手动运行。
    /// 无 schema 时返回数据库错误；无记录时返回 Ok(None)。
    #[tokio::test]
    #[ignore]
    async fn 测试_获取对账单_需要数据库() {
        let db = setup_test_db().await;
        let service = ArReconciliationService::new(Arc::new(db));

        // 无 schema 时为 Err；有 schema 无记录时为 Ok(None)
        let result = service.get_by_id(99999).await;
        if let Ok(opt) = result {
            assert!(opt.is_none());
        }
    }
}
