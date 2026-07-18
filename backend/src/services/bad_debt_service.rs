//! 坏账管理服务（V15 P0-B01/B02 Batch 481 创建）
//!
//! 包含两部分业务：
//!
//! **B01 坏账准备计提**（账龄分析法）
//! - 期末按账龄桶扫描未收 ar_invoice，按客户+桶聚合计提
//! - 账龄桶比例：1 年内 5% / 1-2 年 20% / 2-3 年 50% / 3 年以上 100%
//! - 状态机：draft → confirmed → reversed
//!
//! **B02 坏账核销审批**（二级审批流）
//! - 申请人 → 财务经理（一级）→ 总经理（二级）→ 核销执行
//! - 状态机：pending → finance_approved → approved（终态）
//!                 → rejected（任一级拒绝，终态）
//!                 → cancelled（申请人取消，终态）
//!
//! 关联任务：P0-B01（§17.3-D1）/ P0-B02（§17.3-D2）
//! 关联文件：models/bad_debt_provision.rs / models/bad_debt_writeoff.rs /
//!          models/bad_debt_dto.rs / handlers/bad_debt_handler.rs / routes/bad_debt.rs

use chrono::Utc;
use rust_decimal::prelude::*;
use rust_decimal::Decimal;
use rust_decimal_macros::dec;
use sea_orm::{
    ActiveModelTrait, ColumnTrait, DatabaseConnection, EntityTrait, PaginatorTrait, QueryFilter,
    QueryOrder, QuerySelect, Set, TransactionTrait,
};
use std::sync::Arc;
use thiserror::Error;

use crate::models::ar_invoice;
use crate::models::bad_debt_dto::{
    ApproveWriteoffRequest, CancelWriteoffRequest, CreateWriteoffRequest, ListProvisionQuery,
    ListWriteoffQuery, RejectWriteoffRequest, ReverseProvisionRequest, RunProvisionRequest,
};
use crate::models::bad_debt_provision::{self, ActiveModel as ProvisionActiveModel, Entity as ProvisionEntity};
use crate::models::bad_debt_writeoff::{self, ActiveModel as WriteoffActiveModel, Entity as WriteoffEntity};
use crate::utils::app_state::AppState;
use crate::utils::error::AppError;
use crate::utils::pagination::paginate_with_total;

/// 业务错误（B01 + B02 共用）
#[derive(Debug, Error)]
pub enum BadDebtError {
    #[error("坏账准备记录不存在")]
    ProvisionNotFound,
    #[error("坏账核销申请不存在")]
    WriteoffNotFound,
    #[error("应收单不存在")]
    ArInvoiceNotFound,
    #[error("当前状态 {current} 不允许此操作（期望 {expected}）")]
    InvalidState {
        current: String,
        expected: &'static str,
    },
    #[error("核销金额超过应收单未收金额：申请 {requested}，未收 {unpaid}")]
    WriteoffAmountExceeds { requested: Decimal, unpaid: Decimal },
    #[error("不能审批自己提交的核销申请")]
    SelfApprovalForbidden,
    #[error("只有申请人可以取消核销申请")]
    NotApplicant,
    #[error("参数校验失败: {0}")]
    Validation(String),
    #[error("数据库错误: {0}")]
    Database(#[from] sea_orm::DbErr),
    /// paginate_with_total 返回 AppError，透传所需
    #[error("应用错误: {0}")]
    App(#[from] AppError),
}

// ==================== B01 坏账准备计提 ====================

/// 账龄桶枚举
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AgingBucket {
    Within1Y,
    OneTo2Y,
    TwoTo3Y,
    Over3Y,
}

impl AgingBucket {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Within1Y => "within_1y",
            Self::OneTo2Y => "1_to_2y",
            Self::TwoTo3Y => "2_to_3y",
            Self::Over3Y => "over_3y",
        }
    }

    /// 计提比例（账龄法）
    pub fn provision_rate(&self) -> Decimal {
        match self {
            Self::Within1Y => dec!(0.05),
            Self::OneTo2Y => dec!(0.20),
            Self::TwoTo3Y => dec!(0.50),
            Self::Over3Y => Decimal::ONE,
        }
    }

    /// 根据逾期天数计算账龄桶
    pub fn from_overdue_days(days: i64) -> Self {
        if days <= 365 {
            Self::Within1Y
        } else if days <= 730 {
            Self::OneTo2Y
        } else if days <= 1095 {
            Self::TwoTo3Y
        } else {
            Self::Over3Y
        }
    }
}

/// 坏账管理服务（B01 计提 + B02 核销审批）
pub struct BadDebtService {
    db: Arc<DatabaseConnection>,
}

impl BadDebtService {
    pub fn new(db: Arc<DatabaseConnection>) -> Self {
        Self { db }
    }

    pub fn from_state(state: &AppState) -> Self {
        Self::new(state.db.clone())
    }

    /// 触发期末计提（B01）
    ///
    /// 业务规则：
    /// 1. 扫描所有 unpaid_amount > 0 的 ar_invoice
    /// 2. 按 customer_id + aging_bucket 聚合 base_amount
    /// 3. 每个聚合生成一条 draft 状态的计提记录
    /// 4. 同期同客户同桶已存在 draft/confirmed 记录则跳过（避免重复计提）
    pub async fn run_monthly_provision(
        &self,
        req: RunProvisionRequest,
        created_by: i32,
    ) -> Result<Vec<bad_debt_provision::Model>, BadDebtError> {
        // 期间校验
        if !(1..=12).contains(&req.period_month) {
            return Err(BadDebtError::Validation(format!(
                "period_month {} 不合法（1-12）",
                req.period_month
            )));
        }
        if !(2000..=2100).contains(&req.period_year) {
            return Err(BadDebtError::Validation(format!(
                "period_year {} 不合法（2000-2100）",
                req.period_year
            )));
        }

        let txn = (*self.db).begin().await?;

        // 扫描未收 ar_invoice
        let invoices = ar_invoice::Entity::find()
            .filter(ar_invoice::Column::UnpaidAmount.gt(Decimal::ZERO))
            .filter(ar_invoice::Column::ApprovalStatus.eq("approved"))
            .all(&txn)
            .await?;

        let today = Utc::now().date_naive();

        // 按 (customer_id, aging_bucket) 聚合
        let mut buckets: std::collections::HashMap<(i64, AgingBucket), Decimal> =
            std::collections::HashMap::new();
        let mut customer_names: std::collections::HashMap<i64, Option<String>> =
            std::collections::HashMap::new();

        for inv in invoices {
            // 计算逾期天数：today - due_date（负数表示未到期，按 0 桶处理）
            let overdue_days = (today - inv.due_date).num_days().max(0);
            let bucket = AgingBucket::from_overdue_days(overdue_days);

            let customer_id = inv.customer_id as i64;
            *buckets.entry((customer_id, bucket)).or_default() += inv.unpaid_amount;
            customer_names
                .entry(customer_id)
                .or_insert_with(|| inv.customer_name.clone());
        }

        let now = Utc::now();
        let mut created: Vec<bad_debt_provision::Model> = Vec::new();

        for ((customer_id, bucket), base_amount) in buckets {
            // 幂等检查：同期同客户同桶已存在 draft/confirmed 记录则跳过
            let existing = ProvisionEntity::find()
                .filter(bad_debt_provision::Column::CustomerId.eq(customer_id))
                .filter(bad_debt_provision::Column::PeriodYear.eq(req.period_year))
                .filter(bad_debt_provision::Column::PeriodMonth.eq(req.period_month))
                .filter(bad_debt_provision::Column::AgingBucket.eq(bucket.as_str()))
                .filter(bad_debt_provision::Column::Status.is_in(["draft", "confirmed"]))
                .one(&txn)
                .await?;
            if existing.is_some() {
                continue;
            }

            let rate = bucket.provision_rate();
            let provision_amount = base_amount * rate;
            let active = ProvisionActiveModel {
                id: Default::default(),
                customer_id: Set(customer_id),
                customer_name: Set(customer_names.get(&customer_id).cloned().flatten()),
                period_year: Set(req.period_year),
                period_month: Set(req.period_month),
                aging_bucket: Set(bucket.as_str().to_string()),
                base_amount: Set(base_amount),
                provision_rate: Set(rate),
                provision_amount: Set(provision_amount),
                voucher_id: Set(None),
                status: Set("draft".to_string()),
                created_by: Set(created_by),
                confirmed_at: Set(None),
                reversed_at: Set(None),
                reverse_voucher_id: Set(None),
                remark: Set(None),
                created_at: Set(now),
                updated_at: Set(now),
            };
            let model = active.insert(&txn).await?;
            created.push(model);
        }

        txn.commit().await?;
        Ok(created)
    }

    /// 确认计提（draft → confirmed）
    pub async fn confirm_provision(
        &self,
        provision_id: i64,
    ) -> Result<bad_debt_provision::Model, BadDebtError> {
        let txn = (*self.db).begin().await?;
        let existing = ProvisionEntity::find_by_id(provision_id)
            .one(&txn)
            .await?
            .ok_or(BadDebtError::ProvisionNotFound)?;

        if existing.status != "draft" {
            return Err(BadDebtError::InvalidState {
                current: existing.status,
                expected: "draft",
            });
        }

        let now = Utc::now();
        let mut active: ProvisionActiveModel = existing.into();
        active.status = Set("confirmed".to_string());
        active.confirmed_at = Set(Some(now));
        active.updated_at = Set(now);
        let updated = active.update(&txn).await?;
        txn.commit().await?;
        Ok(updated)
    }

    /// 转回计提（confirmed → reversed）
    pub async fn reverse_provision(
        &self,
        provision_id: i64,
        req: ReverseProvisionRequest,
    ) -> Result<bad_debt_provision::Model, BadDebtError> {
        let txn = (*self.db).begin().await?;
        let existing = ProvisionEntity::find_by_id(provision_id)
            .one(&txn)
            .await?
            .ok_or(BadDebtError::ProvisionNotFound)?;

        if existing.status != "confirmed" {
            return Err(BadDebtError::InvalidState {
                current: existing.status,
                expected: "confirmed",
            });
        }

        let now = Utc::now();
        let mut active: ProvisionActiveModel = existing.into();
        active.status = Set("reversed".to_string());
        active.reversed_at = Set(Some(now));
        active.reverse_voucher_id = Set(req.reverse_voucher_id);
        if let Some(remark) = req.remark {
            active.remark = Set(Some(remark));
        }
        active.updated_at = Set(now);
        let updated = active.update(&txn).await?;
        txn.commit().await?;
        Ok(updated)
    }

    /// 按 ID 查询计提记录
    pub async fn get_provision(
        &self,
        provision_id: i64,
    ) -> Result<bad_debt_provision::Model, BadDebtError> {
        ProvisionEntity::find_by_id(provision_id)
            .one(&*self.db)
            .await?
            .ok_or(BadDebtError::ProvisionNotFound)
    }

    /// 列表查询计提记录
    pub async fn list_provisions(
        &self,
        query: ListProvisionQuery,
    ) -> Result<(Vec<bad_debt_provision::Model>, u64), BadDebtError> {
        let page = query.page.unwrap_or(1).max(1);
        let page_size = query.page_size.unwrap_or(20).clamp(1, 200);

        let mut select = ProvisionEntity::find();
        if let Some(v) = query.customer_id {
            select = select.filter(bad_debt_provision::Column::CustomerId.eq(v));
        }
        if let Some(v) = query.period_year {
            select = select.filter(bad_debt_provision::Column::PeriodYear.eq(v));
        }
        if let Some(v) = query.period_month {
            select = select.filter(bad_debt_provision::Column::PeriodMonth.eq(v));
        }
        if let Some(v) = query.aging_bucket {
            // 校验 aging_bucket 合法性
            if !["within_1y", "1_to_2y", "2_to_3y", "over_3y"].contains(&v.as_str()) {
                return Err(BadDebtError::Validation(format!(
                    "非法 aging_bucket: {}，合法值：within_1y/1_to_2y/2_to_3y/over_3y",
                    v
                )));
            }
            select = select.filter(bad_debt_provision::Column::AgingBucket.eq(v));
        }
        if let Some(v) = query.status {
            if !["draft", "confirmed", "reversed"].contains(&v.as_str()) {
                return Err(BadDebtError::Validation(format!(
                    "非法 status: {}，合法值：draft/confirmed/reversed",
                    v
                )));
            }
            select = select.filter(bad_debt_provision::Column::Status.eq(v));
        }

        let paginator = select
            .order_by_desc(bad_debt_provision::Column::CreatedAt)
            .paginate(&*self.db, page_size);

        let (items, total) = paginate_with_total(paginator, page.clamp(1, 1000)).await?;
        Ok((items, total))
    }

    // ==================== B02 坏账核销审批 ====================

    /// 申请核销
    ///
    /// 业务规则：
    /// 1. 校验 ar_invoice 存在且 approval_status='approved'
    /// 2. 校验 writeoff_amount > 0 且 <= ar_invoice.unpaid_amount
    /// 3. 创建 pending 状态核销申请，approval_level=1
    pub async fn create_writeoff(
        &self,
        req: CreateWriteoffRequest,
        applicant_user_id: i32,
        applicant_username: String,
    ) -> Result<bad_debt_writeoff::Model, BadDebtError> {
        if req.writeoff_amount <= Decimal::ZERO {
            return Err(BadDebtError::Validation(
                "writeoff_amount 必须 > 0".to_string(),
            ));
        }
        if req.reason.trim().is_empty() {
            return Err(BadDebtError::Validation("reason 不能为空".to_string()));
        }

        let txn = (*self.db).begin().await?;

        // 校验 ar_invoice
        let invoice = ar_invoice::Entity::find_by_id(req.ar_invoice_id)
            .one(&txn)
            .await?
            .ok_or(BadDebtError::ArInvoiceNotFound)?;

        if invoice.approval_status != "approved" {
            return Err(BadDebtError::Validation(format!(
                "应收单 {} 未审核通过（当前 approval_status={}）",
                req.ar_invoice_id, invoice.approval_status
            )));
        }

        if req.writeoff_amount > invoice.unpaid_amount {
            return Err(BadDebtError::WriteoffAmountExceeds {
                requested: req.writeoff_amount,
                unpaid: invoice.unpaid_amount,
            });
        }

        let now = Utc::now();
        let active = WriteoffActiveModel {
            id: Default::default(),
            customer_id: Set(req.customer_id),
            ar_invoice_id: Set(req.ar_invoice_id),
            writeoff_amount: Set(req.writeoff_amount),
            reason: Set(req.reason),
            applicant_user_id: Set(applicant_user_id),
            applicant_username: Set(applicant_username),
            applicant_at: Set(now),
            approval_level: Set(1),
            approval_status: Set("pending".to_string()),
            finance_manager_id: Set(None),
            finance_manager_at: Set(None),
            finance_manager_comment: Set(None),
            general_manager_id: Set(None),
            general_manager_at: Set(None),
            general_manager_comment: Set(None),
            voucher_id: Set(None),
            completed_at: Set(None),
            cancelled_at: Set(None),
            cancel_reason: Set(None),
            remark: Set(req.remark),
            created_at: Set(now),
            updated_at: Set(now),
        };
        let model = active.insert(&txn).await?;
        txn.commit().await?;
        Ok(model)
    }

    /// 一级审批通过（财务经理审批，pending → finance_approved）
    pub async fn finance_approve(
        &self,
        writeoff_id: i64,
        approver_user_id: i32,
        req: ApproveWriteoffRequest,
    ) -> Result<bad_debt_writeoff::Model, BadDebtError> {
        let txn = (*self.db).begin().await?;
        let existing = WriteoffEntity::find_by_id(writeoff_id)
            .one(&txn)
            .await?
            .ok_or(BadDebtError::WriteoffNotFound)?;

        if existing.approval_status != "pending" {
            return Err(BadDebtError::InvalidState {
                current: existing.approval_status,
                expected: "pending",
            });
        }
        // 反自审批：审批人不能是申请人
        if existing.applicant_user_id == approver_user_id {
            return Err(BadDebtError::SelfApprovalForbidden);
        }

        let now = Utc::now();
        let mut active: WriteoffActiveModel = existing.into();
        active.approval_level = Set(2);
        active.approval_status = Set("finance_approved".to_string());
        active.finance_manager_id = Set(Some(approver_user_id));
        active.finance_manager_at = Set(Some(now));
        active.finance_manager_comment = Set(req.comment);
        active.updated_at = Set(now);
        let updated = active.update(&txn).await?;
        txn.commit().await?;
        Ok(updated)
    }

    /// 二级审批通过（总经理审批，finance_approved → approved，终态）
    pub async fn general_manager_approve(
        &self,
        writeoff_id: i64,
        approver_user_id: i32,
        req: ApproveWriteoffRequest,
    ) -> Result<bad_debt_writeoff::Model, BadDebtError> {
        let txn = (*self.db).begin().await?;
        let existing = WriteoffEntity::find_by_id(writeoff_id)
            .one(&txn)
            .await?
            .ok_or(BadDebtError::WriteoffNotFound)?;

        if existing.approval_status != "finance_approved" {
            return Err(BadDebtError::InvalidState {
                current: existing.approval_status,
                expected: "finance_approved",
            });
        }
        // 反自审批：审批人不能是申请人
        if existing.applicant_user_id == approver_user_id {
            return Err(BadDebtError::SelfApprovalForbidden);
        }

        let now = Utc::now();
        let mut active: WriteoffActiveModel = existing.into();
        active.approval_status = Set("approved".to_string());
        active.general_manager_id = Set(Some(approver_user_id));
        active.general_manager_at = Set(Some(now));
        active.general_manager_comment = Set(req.comment);
        active.completed_at = Set(Some(now));
        active.updated_at = Set(now);
        let updated = active.update(&txn).await?;
        txn.commit().await?;
        Ok(updated)
    }

    /// 拒绝核销（pending 或 finance_approved → rejected）
    pub async fn reject(
        &self,
        writeoff_id: i64,
        approver_user_id: i32,
        req: RejectWriteoffRequest,
    ) -> Result<bad_debt_writeoff::Model, BadDebtError> {
        if req.comment.trim().is_empty() {
            return Err(BadDebtError::Validation("comment 不能为空".to_string()));
        }

        let txn = (*self.db).begin().await?;
        let existing = WriteoffEntity::find_by_id(writeoff_id)
            .one(&txn)
            .await?
            .ok_or(BadDebtError::WriteoffNotFound)?;

        if !["pending", "finance_approved"].contains(&existing.approval_status.as_str()) {
            return Err(BadDebtError::InvalidState {
                current: existing.approval_status,
                expected: "pending 或 finance_approved",
            });
        }
        // 反自审批
        if existing.applicant_user_id == approver_user_id {
            return Err(BadDebtError::SelfApprovalForbidden);
        }

        let now = Utc::now();
        let mut active: WriteoffActiveModel = existing.into();
        active.approval_status = Set("rejected".to_string());

        // 根据当前层级写入对应审批人字段
        if existing.approval_status == "pending" {
            active.finance_manager_id = Set(Some(approver_user_id));
            active.finance_manager_at = Set(Some(now));
            active.finance_manager_comment = Set(Some(req.comment));
        } else {
            active.general_manager_id = Set(Some(approver_user_id));
            active.general_manager_at = Set(Some(now));
            active.general_manager_comment = Set(Some(req.comment));
        }
        active.updated_at = Set(now);
        let updated = active.update(&txn).await?;
        txn.commit().await?;
        Ok(updated)
    }

    /// 取消核销（pending → cancelled，仅申请人可取消）
    pub async fn cancel(
        &self,
        writeoff_id: i64,
        operator_user_id: i32,
        req: CancelWriteoffRequest,
    ) -> Result<bad_debt_writeoff::Model, BadDebtError> {
        if req.cancel_reason.trim().is_empty() {
            return Err(BadDebtError::Validation(
                "cancel_reason 不能为空".to_string(),
            ));
        }

        let txn = (*self.db).begin().await?;
        let existing = WriteoffEntity::find_by_id(writeoff_id)
            .one(&txn)
            .await?
            .ok_or(BadDebtError::WriteoffNotFound)?;

        // 只有申请人可取消
        if existing.applicant_user_id != operator_user_id {
            return Err(BadDebtError::NotApplicant);
        }
        // 仅 pending 状态可取消
        if existing.approval_status != "pending" {
            return Err(BadDebtError::InvalidState {
                current: existing.approval_status,
                expected: "pending",
            });
        }

        let now = Utc::now();
        let mut active: WriteoffActiveModel = existing.into();
        active.approval_status = Set("cancelled".to_string());
        active.cancelled_at = Set(Some(now));
        active.cancel_reason = Set(Some(req.cancel_reason));
        active.updated_at = Set(now);
        let updated = active.update(&txn).await?;
        txn.commit().await?;
        Ok(updated)
    }

    /// 按 ID 查询核销申请
    pub async fn get_writeoff(
        &self,
        writeoff_id: i64,
    ) -> Result<bad_debt_writeoff::Model, BadDebtError> {
        WriteoffEntity::find_by_id(writeoff_id)
            .one(&*self.db)
            .await?
            .ok_or(BadDebtError::WriteoffNotFound)
    }

    /// 列表查询核销申请
    pub async fn list_writeoffs(
        &self,
        query: ListWriteoffQuery,
    ) -> Result<(Vec<bad_debt_writeoff::Model>, u64), BadDebtError> {
        let page = query.page.unwrap_or(1).max(1);
        let page_size = query.page_size.unwrap_or(20).clamp(1, 200);

        let mut select = WriteoffEntity::find();
        if let Some(v) = query.customer_id {
            select = select.filter(bad_debt_writeoff::Column::CustomerId.eq(v));
        }
        if let Some(v) = query.ar_invoice_id {
            select = select.filter(bad_debt_writeoff::Column::ArInvoiceId.eq(v));
        }
        if let Some(v) = query.approval_status {
            if !["pending", "finance_approved", "approved", "rejected", "cancelled"]
                .contains(&v.as_str())
            {
                return Err(BadDebtError::Validation(format!(
                    "非法 approval_status: {}",
                    v
                )));
            }
            select = select.filter(bad_debt_writeoff::Column::ApprovalStatus.eq(v));
        }
        if let Some(v) = query.applicant_user_id {
            select = select.filter(bad_debt_writeoff::Column::ApplicantUserId.eq(v));
        }

        let paginator = select
            .order_by_desc(bad_debt_writeoff::Column::CreatedAt)
            .paginate(&*self.db, page_size);

        let (items, total) = paginate_with_total(paginator, page.clamp(1, 1000)).await?;
        Ok((items, total))
    }
}
