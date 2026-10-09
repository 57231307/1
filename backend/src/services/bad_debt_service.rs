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
//! - 状态机：pending → finance_approved → approved（终态） / rejected（任一级拒绝，终态） / cancelled（申请人取消，终态）
//!
//! 关联任务：P0-B01（§17.3-D1）/ P0-B02（§17.3-D2）
//! 关联文件：models/bad_debt_provision.rs / models/bad_debt_writeoff.rs /
//!          models/bad_debt_dto.rs / handlers/bad_debt_handler.rs / routes/bad_debt.rs

use chrono::{DateTime, NaiveDate, Utc};
use rust_decimal::Decimal;
use sea_orm::{
    ActiveModelTrait, ColumnTrait, ConnectionTrait, DatabaseConnection, EntityTrait,
    PaginatorTrait, QueryFilter, QueryOrder, Set, TransactionTrait,
};
use std::collections::HashMap;
use std::sync::Arc;
use thiserror::Error;

use crate::container::AppState;
use crate::models::ar_invoice;
use crate::models::bad_debt_dto::{
    ApproveWriteoffRequest, CancelWriteoffRequest, CreateWriteoffRequest, ListProvisionQuery,
    ListWriteoffQuery, RejectWriteoffRequest, ReverseProvisionRequest, RunProvisionRequest,
};
use crate::models::bad_debt_provision::{
    self, ActiveModel as ProvisionActiveModel, Entity as ProvisionEntity,
};
use crate::models::bad_debt_writeoff::{
    self, ActiveModel as WriteoffActiveModel, Entity as WriteoffEntity,
};
use crate::models::status::bad_debt_provision_status as provision_status;
use crate::models::status::bad_debt_writeoff_status as writeoff_status;
use crate::models::status::common;
use crate::utils::data_scope::{
    DataScopeContext, apply_data_scope, check_resource_owner_by_member_scope,
};
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
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
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
            Self::Within1Y => Decimal::new(5, 2),
            Self::OneTo2Y => Decimal::new(20, 2),
            Self::TwoTo3Y => Decimal::new(50, 2),
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

/// 计提模型构造参数集（避免 helper 参数过多）
struct ProvisionModelParams<'a> {
    customer_id: i64,
    customer_name: Option<String>,
    req: &'a RunProvisionRequest,
    bucket: AgingBucket,
    base_amount: Decimal,
    rate: Decimal,
    created_by: i32,
    now: DateTime<Utc>,
}

/// 未收应收单按客户和账龄桶的聚合结果
struct InvoiceAggregation {
    buckets: HashMap<(i64, AgingBucket), Decimal>,
    customer_names: HashMap<i64, Option<String>>,
}

impl BadDebtService {
    pub fn new(db: Arc<DatabaseConnection>) -> Self {
        Self { db }
    }

    pub fn from_state(state: &AppState) -> Self {
        Self::new(state.db.clone())
    }

    /// 触发期末计提（B01）：扫描未收 ar_invoice，按客户+账龄桶聚合并生成 draft 计提记录
    pub async fn run_monthly_provision(
        &self,
        req: RunProvisionRequest,
        created_by: i32,
    ) -> Result<Vec<bad_debt_provision::Model>, BadDebtError> {
        Self::validate_provision_period(&req)?;

        let txn = (*self.db).begin().await?;
        let invoices = Self::query_unpaid_invoices_txn(&txn).await?;
        let today = Utc::now().date_naive();
        let agg = Self::aggregate_invoices_by_bucket(invoices, today);
        let now = Utc::now();
        let mut created: Vec<bad_debt_provision::Model> = Vec::new();

        for ((customer_id, bucket), base_amount) in agg.buckets {
            let exists = Self::check_existing_provision_txn(
                &txn,
                customer_id,
                req.period_year,
                req.period_month,
                bucket,
            )
            .await?;
            if exists {
                continue;
            }

            let rate = bucket.provision_rate();
            let params = ProvisionModelParams {
                customer_id,
                customer_name: agg.customer_names.get(&customer_id).cloned().flatten(),
                req: &req,
                bucket,
                base_amount,
                rate,
                created_by,
                now,
            };
            let active = Self::build_provision_active_model(params);
            let model = active.insert(&txn).await?;
            created.push(model);
        }

        txn.commit().await?;
        Ok(created)
    }

    /// 校验计提期间参数（period_month 1-12，period_year 2000-2100）
    fn validate_provision_period(req: &RunProvisionRequest) -> Result<(), BadDebtError> {
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
        Ok(())
    }

    /// 事务内扫描未收应收单（unpaid_amount > 0 且已审批）
    async fn query_unpaid_invoices_txn(
        txn: &impl ConnectionTrait,
    ) -> Result<Vec<ar_invoice::Model>, BadDebtError> {
        let invoices = ar_invoice::Entity::find()
            .filter(ar_invoice::Column::UnpaidAmount.gt(Decimal::ZERO))
            .filter(ar_invoice::Column::ApprovalStatus.eq(common::STATUS_APPROVED))
            .all(txn)
            .await?;
        Ok(invoices)
    }

    /// 按客户和账龄桶聚合未收应收单
    fn aggregate_invoices_by_bucket(
        invoices: Vec<ar_invoice::Model>,
        today: NaiveDate,
    ) -> InvoiceAggregation {
        let mut buckets: HashMap<(i64, AgingBucket), Decimal> = HashMap::new();
        let mut customer_names: HashMap<i64, Option<String>> = HashMap::new();

        for inv in invoices {
            let overdue_days = (today - inv.due_date).num_days().max(0);
            let bucket = AgingBucket::from_overdue_days(overdue_days);
            let customer_id = inv.customer_id as i64;
            *buckets.entry((customer_id, bucket)).or_default() += inv.unpaid_amount;
            customer_names
                .entry(customer_id)
                .or_insert_with(|| inv.customer_name.clone());
        }

        InvoiceAggregation {
            buckets,
            customer_names,
        }
    }

    /// 事务内检查同期同客户同桶是否已存在 draft/confirmed 计提记录
    async fn check_existing_provision_txn(
        txn: &impl ConnectionTrait,
        customer_id: i64,
        period_year: i32,
        period_month: i32,
        bucket: AgingBucket,
    ) -> Result<bool, BadDebtError> {
        let existing = ProvisionEntity::find()
            .filter(bad_debt_provision::Column::CustomerId.eq(customer_id))
            .filter(bad_debt_provision::Column::PeriodYear.eq(period_year))
            .filter(bad_debt_provision::Column::PeriodMonth.eq(period_month))
            .filter(bad_debt_provision::Column::AgingBucket.eq(bucket.as_str()))
            .filter(
                bad_debt_provision::Column::Status
                    .is_in([provision_status::DRAFT, provision_status::CONFIRMED]),
            )
            .one(txn)
            .await?;
        Ok(existing.is_some())
    }

    /// 构造计提 ActiveModel（参数集模式避免参数过多）
    fn build_provision_active_model(params: ProvisionModelParams<'_>) -> ProvisionActiveModel {
        let ProvisionModelParams {
            customer_id,
            customer_name,
            req,
            bucket,
            base_amount,
            rate,
            created_by,
            now,
        } = params;
        let provision_amount = base_amount * rate;
        ProvisionActiveModel {
            id: Default::default(),
            customer_id: Set(customer_id),
            customer_name: Set(customer_name),
            period_year: Set(req.period_year),
            period_month: Set(req.period_month),
            aging_bucket: Set(bucket.as_str().to_string()),
            base_amount: Set(base_amount),
            provision_rate: Set(rate),
            provision_amount: Set(provision_amount),
            voucher_id: Set(None),
            status: Set(provision_status::DRAFT.to_string()),
            created_by: Set(created_by),
            confirmed_at: Set(None),
            reversed_at: Set(None),
            reverse_voucher_id: Set(None),
            remark: Set(None),
            created_at: Set(now),
            updated_at: Set(now),
        }
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

        if existing.status != provision_status::DRAFT {
            return Err(BadDebtError::InvalidState {
                current: existing.status,
                expected: provision_status::DRAFT,
            });
        }

        let now = Utc::now();
        let mut active: ProvisionActiveModel = existing.into();
        active.status = Set(provision_status::CONFIRMED.to_string());
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

        if existing.status != provision_status::CONFIRMED {
            return Err(BadDebtError::InvalidState {
                current: existing.status,
                expected: provision_status::CONFIRMED,
            });
        }

        let now = Utc::now();
        let mut active: ProvisionActiveModel = existing.into();
        active.status = Set(provision_status::REVERSED.to_string());
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

    /// 按 ID 查询计提记录（含 IDOR 行级归属校验）
    pub async fn get_provision(
        &self,
        provision_id: i64,
        data_scope: Option<&DataScopeContext>,
    ) -> Result<bad_debt_provision::Model, BadDebtError> {
        let provision = ProvisionEntity::find_by_id(provision_id)
            .one(&*self.db)
            .await?
            .ok_or(BadDebtError::ProvisionNotFound)?;
        if let Some(ctx) = data_scope {
            if !check_resource_owner_by_member_scope(ctx, Some(provision.created_by)) {
                return Err(AppError::permission_denied(
                    "无权访问该计提记录（数据范围限制）",
                ))?;
            }
        }
        Ok(provision)
    }

    /// 列表查询计提记录（行级数据权限下推：created_by 为 NOT NULL i32，无 NULL-owner 公海语义）
    pub async fn list_provisions(
        &self,
        query: ListProvisionQuery,
        data_scope: Option<&DataScopeContext>,
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
            if !provision_status::ALL.contains(&v.as_str()) {
                return Err(BadDebtError::Validation(format!(
                    "非法 status: {}，合法值：{}",
                    v,
                    provision_status::ALL.join("/")
                )));
            }
            select = select.filter(bad_debt_provision::Column::Status.eq(v));
        }

        // 行级数据权限：bad_debt_provisions 无 department_id，created_by 是唯一归属列。
        // Dept 范围按可见部门成员集合过滤 created_by；Self 仅本人；All 不过滤。
        // created_by 列类型为 i32（NOT NULL），不存在 NULL-owner 可见性问题。
        if let Some(ctx) = data_scope {
            select = apply_data_scope(
                select,
                ctx,
                bad_debt_provision::Column::CreatedBy,
                bad_debt_provision::Column::CreatedBy,
            );
        }

        let paginator = select
            .order_by_desc(bad_debt_provision::Column::CreatedAt)
            .paginate(&*self.db, page_size);

        let (items, total) = paginate_with_total(paginator, page.clamp(1, 1000)).await?;
        Ok((items, total))
    }

    // ==================== B02 坏账核销审批 ====================

    /// 申请核销
    /// 业务规则：1. 校验 ar_invoice 存在且已审批通过（ar_invoice.approval_status=APPROVED，大写词表）；2. 校验 writeoff_amount > 0 且 <= ar_invoice.unpaid_amount；3. 创建 pending 状态核销申请，approval_level=1
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

        if invoice.approval_status != common::STATUS_APPROVED {
            // 状态门：应收单当前审批状态未达「审核通过」这一前置，归业务族（InvalidState）；
            // 原走 Validation 通道出 VALIDATION_ERROR 与本域其余状态门（同为 BUSINESS_ERROR）族不一致
            return Err(BadDebtError::InvalidState {
                current: invoice.approval_status.clone(),
                expected: common::STATUS_APPROVED,
            });
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
            approval_status: Set(writeoff_status::PENDING.to_string()),
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

        if existing.approval_status != writeoff_status::PENDING {
            return Err(BadDebtError::InvalidState {
                current: existing.approval_status,
                expected: writeoff_status::PENDING,
            });
        }
        // 反自审批：审批人不能是申请人
        if existing.applicant_user_id == approver_user_id {
            return Err(BadDebtError::SelfApprovalForbidden);
        }

        let now = Utc::now();
        let mut active: WriteoffActiveModel = existing.into();
        active.approval_level = Set(2);
        active.approval_status = Set(writeoff_status::FINANCE_APPROVED.to_string());
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

        if existing.approval_status != writeoff_status::FINANCE_APPROVED {
            return Err(BadDebtError::InvalidState {
                current: existing.approval_status,
                expected: writeoff_status::FINANCE_APPROVED,
            });
        }
        // 反自审批：审批人不能是申请人
        if existing.applicant_user_id == approver_user_id {
            return Err(BadDebtError::SelfApprovalForbidden);
        }

        // 在 existing.into() 移动前保存核销目标，用于事务内回写应收未付额
        let writeoff_ar_invoice_id = existing.ar_invoice_id;
        let writeoff_amount = existing.writeoff_amount;

        let now = Utc::now();
        let mut active: WriteoffActiveModel = existing.into();
        active.approval_status = Set(writeoff_status::APPROVED.to_string());
        active.general_manager_id = Set(Some(approver_user_id));
        active.general_manager_at = Set(Some(now));
        active.general_manager_comment = Set(req.comment);
        active.completed_at = Set(Some(now));
        active.updated_at = Set(now);
        let updated = active.update(&txn).await?;

        // 核销生效：终态 approved 时按核销金额递减应收单未付额，与建单门
        // 「writeoff_amount 不得超过 unpaid_amount」同一读法（核销消耗应收未付额）。
        // 递减不低于 0，未付额归零即视为该应收单全额坏账核销完毕。
        let invoice = ar_invoice::Entity::find_by_id(writeoff_ar_invoice_id)
            .one(&txn)
            .await?
            .ok_or(BadDebtError::ArInvoiceNotFound)?;
        let new_unpaid = (invoice.unpaid_amount - writeoff_amount).max(Decimal::ZERO);
        let mut invoice_active: ar_invoice::ActiveModel = invoice.into();
        invoice_active.unpaid_amount = Set(new_unpaid);
        invoice_active.update(&txn).await?;

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

        if ![writeoff_status::PENDING, writeoff_status::FINANCE_APPROVED]
            .contains(&existing.approval_status.as_str())
        {
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
        // 在 existing.into() 移动前保存 approval_status，用于判断当前审批层级
        let prev_status = existing.approval_status.clone();
        let mut active: WriteoffActiveModel = existing.into();
        active.approval_status = Set(writeoff_status::REJECTED.to_string());

        // 根据当前层级写入对应审批人字段
        if prev_status == writeoff_status::PENDING {
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
        if existing.approval_status != writeoff_status::PENDING {
            return Err(BadDebtError::InvalidState {
                current: existing.approval_status,
                expected: writeoff_status::PENDING,
            });
        }

        let now = Utc::now();
        let mut active: WriteoffActiveModel = existing.into();
        active.approval_status = Set(writeoff_status::CANCELLED.to_string());
        active.cancelled_at = Set(Some(now));
        active.cancel_reason = Set(Some(req.cancel_reason));
        active.updated_at = Set(now);
        let updated = active.update(&txn).await?;
        txn.commit().await?;
        Ok(updated)
    }

    /// 按 ID 查询核销申请（含 IDOR 行级归属校验）
    pub async fn get_writeoff(
        &self,
        writeoff_id: i64,
        data_scope: Option<&DataScopeContext>,
    ) -> Result<bad_debt_writeoff::Model, BadDebtError> {
        let writeoff = WriteoffEntity::find_by_id(writeoff_id)
            .one(&*self.db)
            .await?
            .ok_or(BadDebtError::WriteoffNotFound)?;
        if let Some(ctx) = data_scope {
            if !check_resource_owner_by_member_scope(ctx, Some(writeoff.applicant_user_id)) {
                return Err(AppError::permission_denied(
                    "无权访问该核销申请（数据范围限制）",
                ))?;
            }
        }
        Ok(writeoff)
    }

    /// 列表查询核销申请（行级数据权限下推：applicant_user_id 为 NOT NULL i32，无 NULL-owner 公海语义）
    pub async fn list_writeoffs(
        &self,
        query: ListWriteoffQuery,
        data_scope: Option<&DataScopeContext>,
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
            if !writeoff_status::ALL.contains(&v.as_str()) {
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

        // 行级数据权限：bad_debt_writeoffs 无 department_id，applicant_user_id 是唯一归属列。
        // Dept 范围按可见部门成员集合过滤 applicant_user_id；Self 仅本人；All 不过滤。
        // applicant_user_id 列类型为 i32（NOT NULL），不存在 NULL-owner 可见性问题。
        if let Some(ctx) = data_scope {
            select = apply_data_scope(
                select,
                ctx,
                bad_debt_writeoff::Column::ApplicantUserId,
                bad_debt_writeoff::Column::ApplicantUserId,
            );
        }

        let paginator = select
            .order_by_desc(bad_debt_writeoff::Column::CreatedAt)
            .paginate(&*self.db, page_size);

        let (items, total) = paginate_with_total(paginator, page.clamp(1, 1000)).await?;
        Ok((items, total))
    }
}
