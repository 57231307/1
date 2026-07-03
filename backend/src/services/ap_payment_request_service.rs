//! 付款申请 Service
//!
//! 付款申请服务层，负责付款申请的核心业务逻辑
//! 包含付款申请创建、提交、审批、拒绝等全流程管理

use crate::models::{ap_invoice, ap_payment_request, ap_payment_request_item};
use crate::utils::admin_checker::{ADMIN_ROLE_CODE, MANAGER_ROLE_CODE};
use crate::utils::error::AppError;
use chrono::{NaiveDate, Utc};
use rust_decimal::Decimal;
use sea_orm::{
    ActiveModelTrait, ColumnTrait, DatabaseConnection, EntityTrait, Order, PaginatorTrait,
    QueryFilter, QueryOrder, QuerySelect, Set, TransactionTrait,
};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use validator::Validate;

/// 付款审批金额阶梯常量（v18 批次 48：消除硬编码，与 quotation_approval_service 阈值一致）
const PAYMENT_APPROVAL_THRESHOLD_MANAGER: i64 = 100_000; // 10 万以下 manager 可审批
const PAYMENT_APPROVAL_THRESHOLD_ADMIN: i64 = 500_000; // 10-50 万仅 admin 可审批

/// 付款申请服务
pub struct ApPaymentRequestService {
    db: Arc<DatabaseConnection>,
}

impl ApPaymentRequestService {
    /// 创建服务实例
    pub fn new(db: Arc<DatabaseConnection>) -> Self {
        Self { db }
    }

    // 生成付款申请单号
    // 格式：PR + 年月日 + 三位序号（PR20260315001）
    crate::impl_generate_no!(
        generate_request_no,
        "PRQ",
        ap_payment_request::Entity,
        ap_payment_request::Column::RequestNo
    );

    /// 创建付款申请
    pub async fn create(
        &self,
        req: CreateApPaymentRequest,
        user_id: i32,
    ) -> Result<ap_payment_request::Model, AppError> {
        let txn = (*self.db).begin().await?;

        // 1. 生成付款申请单号
        let request_no = self.generate_request_no().await?;

        // 2. 验证应付单是否存在且有效
        // 批次 31 v7 P1-1 修复：原循环内逐条 find_by_id（N 次查询），
        // 改为批量查询所有 invoice_id（1 次查询），消除 N+1 问题。
        let invoice_ids: Vec<i32> = req.items.iter().map(|i| i.invoice_id).collect();
        let invoices = ap_invoice::Entity::find()
            .filter(ap_invoice::Column::Id.is_in(invoice_ids))
            .all(&txn)
            .await?;
        // 构建 invoice_id -> Model 的映射，便于循环内 O(1) 查找
        let invoice_map: std::collections::HashMap<i32, ap_invoice::Model> =
            invoices.into_iter().map(|inv| (inv.id, inv)).collect();

        for item in &req.items {
            let invoice = invoice_map
                .get(&item.invoice_id)
                .ok_or_else(|| AppError::not_found(format!("应付单 ID: {}", item.invoice_id)))?;

            // 检查应付单状态
            if invoice.invoice_status == "DRAFT" || invoice.invoice_status == "CANCELLED" {
                return Err(AppError::business(format!(
                    "应付单{}状态为{}，不可申请付款",
                    invoice.invoice_no, invoice.invoice_status
                )));
            }

            // 检查申请金额是否超过未付金额
            let unpaid = invoice.unpaid_amount;
            if item.apply_amount > unpaid {
                return Err(AppError::business(format!(
                    "应付单{}未付金额为{}，申请金额{}超过未付金额",
                    invoice.invoice_no, unpaid, item.apply_amount
                )));
            }
        }

        // 3. 创建付款申请主表
        let request = ap_payment_request::ActiveModel {
            request_no: Set(request_no),
            request_date: Set(req.request_date),
            supplier_id: Set(req.supplier_id),
            payment_type: Set(req.payment_type),
            payment_method: Set(req.payment_method),
            request_amount: Set(req.request_amount),
            approval_status: Set("DRAFT".to_string()),
            currency: Set(req.currency.unwrap_or_else(|| crate::constants::DEFAULT_CURRENCY.to_string())),
            exchange_rate: Set(req.exchange_rate.unwrap_or(Decimal::new(1, 0))),
            expected_payment_date: Set(req.expected_payment_date),
            bank_name: Set(req.bank_name),
            bank_account: Set(req.bank_account),
            bank_account_name: Set(req.bank_account_name),
            notes: Set(req.notes),
            attachment_urls: Set(req.attachment_urls),
            created_by: Set(user_id),
            ..Default::default()
        }
        .insert(&txn)
        .await?;

        // 4. 创建付款申请明细
        for item_req in req.items {
            let _item = ap_payment_request_item::ActiveModel {
                request_id: Set(request.id),
                invoice_id: Set(item_req.invoice_id),
                apply_amount: Set(item_req.apply_amount),
                notes: Set(item_req.notes),
                ..Default::default()
            }
            .insert(&txn)
            .await?;
        }

        txn.commit().await?;

        Ok(request)
    }

    /// 更新付款申请（仅草稿状态）
    ///
    /// 批次 86 v2 复审 P2-6 修复：find_by_id 后追加 lock_exclusive 串行化并发状态变更
    pub async fn update(
        &self,
        id: i32,
        req: UpdateApPaymentRequest,
        user_id: i32,
    ) -> Result<ap_payment_request::Model, AppError> {
        let txn = (*self.db).begin().await?;

        // 1. 查询付款申请（加 lock_exclusive 串行化）
        let request = ap_payment_request::Entity::find_by_id(id)
            .lock_exclusive()
            .one(&txn)
            .await?
            .ok_or_else(|| AppError::not_found(format!("付款申请 {}", id)))?;

        // 2. 检查状态（仅草稿可修改）
        if request.approval_status != "DRAFT" {
            return Err(AppError::business(format!(
                "付款申请状态为{}，不可修改",
                request.approval_status
            )));
        }

        // 3. 更新付款申请主表
        let mut request_active: ap_payment_request::ActiveModel = request.into();

        if let Some(request_date) = req.request_date {
            request_active.request_date = Set(request_date);
        }
        if let Some(payment_type) = req.payment_type {
            request_active.payment_type = Set(payment_type);
        }
        if let Some(payment_method) = req.payment_method {
            request_active.payment_method = Set(payment_method);
        }
        if let Some(request_amount) = req.request_amount {
            request_active.request_amount = Set(request_amount);
        }
        if let Some(expected_payment_date) = req.expected_payment_date {
            request_active.expected_payment_date = Set(Some(expected_payment_date));
        }
        if let Some(bank_name) = req.bank_name {
            request_active.bank_name = Set(Some(bank_name));
        }
        if let Some(bank_account) = req.bank_account {
            request_active.bank_account = Set(Some(bank_account));
        }
        if let Some(bank_account_name) = req.bank_account_name {
            request_active.bank_account_name = Set(Some(bank_account_name));
        }
        if let Some(notes) = req.notes {
            request_active.notes = Set(Some(notes));
        }
        if let Some(attachment_urls) = req.attachment_urls {
            request_active.attachment_urls = Set(Some(attachment_urls));
        }

        request_active.updated_by = Set(Some(user_id));

        let request = crate::services::audit_log_service::AuditLogService::update_with_audit(
            &txn,
            "auto_audit",
            request_active,
            // P1 1-1 修复（批次 59b）：原 Some(0) 占位符改为真实操作人 user_id
            Some(user_id),
        )
        .await?;

        txn.commit().await?;

        Ok(request)
    }

    /// 删除付款申请（仅草稿/被拒状态）
    ///
    /// 批次 86 v2 复审 P2-7 修复：find_by_id 后追加 lock_exclusive 串行化并发状态变更
    pub async fn delete(&self, id: i32, user_id: i32) -> Result<(), AppError> {
        let txn = (*self.db).begin().await?;

        // 1. 查询付款申请（加 lock_exclusive 串行化）
        let request = ap_payment_request::Entity::find_by_id(id)
            .lock_exclusive()
            .one(&txn)
            .await?
            .ok_or_else(|| AppError::not_found(format!("付款申请 {}", id)))?;

        // 2. 检查状态（仅草稿或被拒可删除）
        if !["DRAFT", "REJECTED"].contains(&request.approval_status.as_str()) {
            return Err(AppError::business(format!(
                "付款申请状态为{}，不可删除",
                request.approval_status
            )));
        }

        // 3. 删除付款申请（级联删除明细）（P0 8-3 修复：补审计日志）
        // 批次 94 P2-10：原 Some(0) 占位改为真实操作人 user_id，便于审计追踪
        crate::services::audit_log_service::AuditLogService::delete_with_audit::<
            ap_payment_request::Entity,
            _,
        >(&txn, "ap_payment_request", request.id, Some(user_id))
        .await?;

        txn.commit().await?;

        Ok(())
    }

    /// 提交付款申请（进入审批流程）
    pub async fn submit(
        &self,
        id: i32,
        user_id: i32,
    ) -> Result<ap_payment_request::Model, AppError> {
        let txn = (*self.db).begin().await?;

        // 1. 查询付款申请（加 lock_exclusive 串行化并发提交）
        let request = ap_payment_request::Entity::find_by_id(id)
            .lock_exclusive()
            .one(&txn)
            .await?
            .ok_or_else(|| AppError::not_found(format!("付款申请 ID: {}", id)))?;

        // 2. 检查状态（仅草稿可提交）
        if request.approval_status != "DRAFT" {
            return Err(AppError::business(format!(
                "付款申请状态为{}，不可提交",
                request.approval_status
            )));
        }

        // 3. 检查关联的应付单
        let items = ap_payment_request_item::Entity::find()
            .filter(ap_payment_request_item::Column::RequestId.eq(id))
            .all(&txn)
            .await?;

        if items.is_empty() {
            return Err(AppError::business("付款申请没有明细，不可提交".to_string()));
        }

        // 4. 提交付款申请
        let now = Utc::now();
        let mut request_active: ap_payment_request::ActiveModel = request.into();
        request_active.approval_status = Set("APPROVING".to_string());
        request_active.submitted_by = Set(Some(user_id));
        request_active.submitted_at = Set(Some(now));
        request_active.updated_at = Set(now);

        let request = crate::services::audit_log_service::AuditLogService::update_with_audit(
            &txn,
            "auto_audit",
            request_active,
            // P1 1-1 修复（批次 59b）：原 Some(0) 占位符改为真实操作人 user_id
            Some(user_id),
        )
        .await?;

        txn.commit().await?;

        Ok(request)
    }

    /// 审批付款申请（按金额分级审批）
    pub async fn approve(
        &self,
        id: i32,
        user_id: i32,
    ) -> Result<ap_payment_request::Model, AppError> {
        let txn = (*self.db).begin().await?;

        // 1. 查询付款申请（加 lock_exclusive 串行化并发审批）
        let request = ap_payment_request::Entity::find_by_id(id)
            .lock_exclusive()
            .one(&txn)
            .await?
            .ok_or_else(|| AppError::not_found(format!("付款申请 {}", id)))?;

        // 2. 检查状态
        if request.approval_status != "APPROVING" {
            return Err(AppError::business(format!(
                "付款申请状态为{}，不可审批",
                request.approval_status
            )));
        }

        // 3. 检查审批权限
        self.check_approval_permission(&request.request_amount, user_id)
            .await?;

        // 4. 审批通过
        let now = Utc::now();
        let mut request_active: ap_payment_request::ActiveModel = request.into();
        request_active.approval_status = Set("APPROVED".to_string());
        request_active.approved_by = Set(Some(user_id));
        request_active.approved_at = Set(Some(now));
        request_active.updated_at = Set(now);

        let request = crate::services::audit_log_service::AuditLogService::update_with_audit(
            &txn,
            "auto_audit",
            request_active,
            // P1 1-1 修复（批次 59b）：原 Some(0) 占位符改为真实操作人 user_id
            Some(user_id),
        )
        .await?;

        txn.commit().await?;

        Ok(request)
    }

    /// 拒绝付款申请
    pub async fn reject(
        &self,
        id: i32,
        reason: String,
        user_id: i32,
    ) -> Result<ap_payment_request::Model, AppError> {
        let txn = (*self.db).begin().await?;

        // 1. 查询付款申请（加 lock_exclusive 串行化并发拒绝）
        let request = ap_payment_request::Entity::find_by_id(id)
            .lock_exclusive()
            .one(&txn)
            .await?
            .ok_or_else(|| AppError::not_found(format!("付款申请 {}", id)))?;

        // 2. 检查状态
        if request.approval_status != "APPROVING" {
            return Err(AppError::business(format!(
                "付款申请状态为{}，不可拒绝",
                request.approval_status
            )));
        }

        // 3. 拒绝付款申请
        let now = Utc::now();
        let mut request_active: ap_payment_request::ActiveModel = request.into();
        request_active.approval_status = Set("REJECTED".to_string());
        request_active.rejected_by = Set(Some(user_id));
        request_active.rejected_at = Set(Some(now));
        request_active.rejected_reason = Set(Some(reason));
        request_active.updated_at = Set(now);

        let request = crate::services::audit_log_service::AuditLogService::update_with_audit(
            &txn,
            "auto_audit",
            request_active,
            // P1 1-1 修复（批次 59b）：原 Some(0) 占位符改为真实操作人 user_id
            Some(user_id),
        )
        .await?;

        txn.commit().await?;

        Ok(request)
    }

    /// 获取付款申请详情
    pub async fn get_by_id(&self, id: i32) -> Result<ap_payment_request::Model, AppError> {
        let request = ap_payment_request::Entity::find_by_id(id)
            .one(&*self.db)
            .await?
            .ok_or_else(|| AppError::not_found(format!("付款申请 ID: {}", id)))?;

        Ok(request)
    }

    /// 获取付款申请列表
    #[allow(clippy::too_many_arguments)]
    pub async fn get_list(
        &self,
        supplier_id: Option<i32>,
        approval_status: Option<String>,
        payment_type: Option<String>,
        start_date: Option<NaiveDate>,
        end_date: Option<NaiveDate>,
        page: u64,
        page_size: u64,
    ) -> Result<(Vec<ap_payment_request::Model>, u64), AppError> {
        let mut query = ap_payment_request::Entity::find();

        // 筛选条件
        if let Some(sid) = supplier_id {
            query = query.filter(ap_payment_request::Column::SupplierId.eq(sid));
        }
        if let Some(status) = approval_status {
            query = query.filter(ap_payment_request::Column::ApprovalStatus.eq(status));
        }
        if let Some(ptype) = payment_type {
            query = query.filter(ap_payment_request::Column::PaymentType.eq(ptype));
        }
        if let Some(sd) = start_date {
            query = query.filter(ap_payment_request::Column::RequestDate.gte(sd));
        }
        if let Some(ed) = end_date {
            query = query.filter(ap_payment_request::Column::RequestDate.lte(ed));
        }

        // 分页
        let paginator = query
            .order_by(ap_payment_request::Column::CreatedAt, Order::Desc)
            .paginate(&*self.db, page_size);

        let total = paginator.num_items().await?;
        // SeaORM fetch_page 为 0-indexed，HTTP 层 page 为 1-indexed，需减 1 对齐
        let items = paginator.fetch_page(page.saturating_sub(1)).await?;

        Ok((items, total))
    }

    /// 检查审批权限（按金额分级）
    // 批次 24 v6 P1-1 修复：原实现仅判断 role_id 是否存在，所有有角色的用户都能审批任意金额，
    // 存在严重越权漏洞（普通员工可审批 50 万+ 付款）。
    // 改为查询 role 表获取 role_code，按角色 code 实现分级审批：
    //   - admin：可审批任意金额（系统管理员）
    //   - manager：可审批 10 万以下（部门经理）
    //   - operator 及其他：无审批权限
    pub async fn check_approval_permission(
        &self,
        amount: &Decimal,
        user_id: i32,
    ) -> Result<(), AppError> {
        // 查询用户
        let user = crate::models::user::Entity::find_by_id(user_id)
            .one(&*self.db)
            .await?
            .ok_or_else(|| AppError::not_found(format!("用户 {}", user_id)))?;

        // 查询角色 code
        let role_code = if let Some(role_id) = user.role_id {
            crate::models::role::Entity::find_by_id(role_id)
                .one(&*self.db)
                .await?
                .map(|r| r.code)
                .unwrap_or_default()
        } else {
            String::new()
        };

        // 按金额分级审批
        // v18 批次 48 修复：消除金额与角色编码硬编码，改用常量
        let threshold_manager = Decimal::new(PAYMENT_APPROVAL_THRESHOLD_MANAGER, 0);
        let threshold_admin = Decimal::new(PAYMENT_APPROVAL_THRESHOLD_ADMIN, 0);
        if amount <= &threshold_manager {
            // 10 万以下：admin 或 manager 可审批
            if role_code != ADMIN_ROLE_CODE && role_code != MANAGER_ROLE_CODE {
                return Err(AppError::permission_denied(
                    "仅管理员或部门经理可审批 10 万元以下的付款".to_string(),
                ));
            }
        } else if amount <= &threshold_admin {
            // 10-50 万：仅 admin 可审批
            if role_code != ADMIN_ROLE_CODE {
                return Err(AppError::permission_denied(
                    "仅管理员可审批 10-50 万元的付款".to_string(),
                ));
            }
        } else {
            // 50 万以上：仅 admin 可审批
            if role_code != ADMIN_ROLE_CODE {
                return Err(AppError::permission_denied(
                    "仅管理员可审批 50 万元以上的付款".to_string(),
                ));
            }
        }

        Ok(())
    }
}

// =====================================================
// 数据传输对象（DTO）
// =====================================================

/// 创建付款申请请求
#[derive(Debug, Deserialize, Validate)]
pub struct CreateApPaymentRequest {
    /// 供应商 ID
    pub supplier_id: i32,

    /// 申请日期
    pub request_date: NaiveDate,

    /// 付款类型
    #[validate(length(min = 1, max = 20, message = "申请单号长度必须在1到20个字符之间"))]
    pub payment_type: String,

    /// 付款方式
    #[validate(length(min = 1, max = 20, message = "申请单号长度必须在1到20个字符之间"))]
    pub payment_method: String,

    /// 申请金额（必须为正数）
    #[validate(custom(function = "validate_positive_decimal_payment"))]
    pub request_amount: Decimal,

    /// 币种（ISO 4217 三字母代码）
    #[validate(length(equal = 3, message = "币种必须为 ISO 4217 三字母代码"))]
    pub currency: Option<String>,

    /// 汇率（必须大于 0，防止 P0-1 历史缺陷的 0.01 汇率）
    #[validate(custom(function = "validate_exchange_rate_payment"))]
    pub exchange_rate: Option<Decimal>,

    /// 期望付款日期
    pub expected_payment_date: Option<NaiveDate>,

    /// 收款银行
    #[validate(length(max = 100, message = "银行名称长度不能超过100个字符"))]
    pub bank_name: Option<String>,

    /// 收款账号
    #[validate(length(max = 50, message = "银行账号长度不能超过50个字符"))]
    pub bank_account: Option<String>,

    /// 收款账户名
    #[validate(length(max = 100, message = "账户名长度不能超过100个字符"))]
    pub bank_account_name: Option<String>,

    /// 备注
    pub notes: Option<String>,

    /// 附件 URL 列表
    pub attachment_urls: Option<Vec<String>>,

    /// 付款申请明细
    pub items: Vec<ApPaymentRequestItemDto>,
}

/// 付款申请明细 DTO
#[derive(Debug, Deserialize, Validate)]
pub struct ApPaymentRequestItemDto {
    /// 应付单 ID
    pub invoice_id: i32,

    /// 申请金额
    pub apply_amount: Decimal,

    /// 备注
    pub notes: Option<String>,
}

// =====================================================
// DTO 校验函数（TS-S-5 安全加固）
// =====================================================

/// 校验 Decimal 为正数
fn validate_positive_decimal_payment(value: &Decimal) -> Result<(), validator::ValidationError> {
    if *value <= Decimal::ZERO {
        return Err(validator::ValidationError::new("金额必须为正数"));
    }
    Ok(())
}

/// 校验汇率合法：必须大于 0 且不等于 P0-1 历史缺陷值 0.01
fn validate_exchange_rate_payment(value: &Decimal) -> Result<(), validator::ValidationError> {
    if *value <= Decimal::ZERO {
        return Err(validator::ValidationError::new("汇率必须大于0"));
    }
    // P0-1 防护：拒绝 0.01 汇率（历史缺陷值）
    if *value == Decimal::new(1, 2) {
        return Err(validator::ValidationError::new(
            "汇率不能为0.01（P0-1历史缺陷值，本位币汇率应为1.0）",
        ));
    }
    Ok(())
}

/// 更新付款申请请求
#[derive(Debug, Deserialize, Serialize, Validate)]
pub struct UpdateApPaymentRequest {
    /// 申请日期
    pub request_date: Option<NaiveDate>,

    /// 付款类型
    pub payment_type: Option<String>,

    /// 付款方式
    pub payment_method: Option<String>,

    /// 申请金额
    pub request_amount: Option<Decimal>,

    /// 期望付款日期
    pub expected_payment_date: Option<NaiveDate>,

    /// 收款银行
    pub bank_name: Option<String>,

    /// 收款账号
    pub bank_account: Option<String>,

    /// 收款账户名
    pub bank_account_name: Option<String>,

    /// 备注
    pub notes: Option<String>,

    /// 附件 URL 列表
    pub attachment_urls: Option<Vec<String>>,
}
