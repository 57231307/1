//! 付款申请 Service
//!
//! 付款申请服务层，负责付款申请的核心业务逻辑
//! 包含付款申请创建、提交、审批、拒绝等全流程管理

use crate::models::{ap_invoice, ap_payment_request, ap_payment_request_item};
use crate::utils::error::AppError;
use chrono::{NaiveDate, Utc};
use rust_decimal::Decimal;
use sea_orm::{
    ActiveModelTrait, ColumnTrait, DatabaseConnection, EntityTrait, Order, PaginatorTrait,
    QueryFilter, QueryOrder, Set, TransactionTrait,
};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use validator::Validate;

/// 付款申请服务
pub struct ApPaymentRequestService {
    db: Arc<DatabaseConnection>,
}

impl ApPaymentRequestService {
    /// 创建服务实例
    pub fn new(db: Arc<DatabaseConnection>) -> Self {
        Self { db }
    }

    /// 生成付款申请单号
    /// 格式：PR + 年月日 + 三位序号（PR20260315001）
    pub async fn generate_request_no(&self) -> Result<String, AppError> {
        let today = Utc::now().format("%Y%m%d").to_string();
        let prefix = format!("PR{}", today);

        // 查询今日付款申请数量
        let count = ap_payment_request::Entity::find()
            .filter(ap_payment_request::Column::RequestNo.starts_with(&prefix))
            .count(&*self.db)
            .await?;

        Ok(format!("{}{:03}", prefix, count + 1))
    }

    /// 创建付款申请
    pub async fn create(
        &self,
        req: CreateApPaymentRequest,
        user_id: i32,
    ) -> Result<ap_payment_request::Model, AppError> {
        let txn = (&*self.db).begin().await?;

        // 1. 生成付款申请单号
        let request_no = self.generate_request_no().await?;

        // 2. 验证应付单是否存在且有效
        for item in &req.items {
            let invoice = ap_invoice::Entity::find_by_id(item.invoice_id)
                .one(&txn)
                .await?
                .ok_or(AppError::ResourceNotFound(format!(
                    "应付单 ID: {}",
                    item.invoice_id
                )))?;

            // 检查应付单状态
            if invoice.invoice_status == "DRAFT" || invoice.invoice_status == "CANCELLED" {
                return Err(AppError::BusinessError(format!(
                    "应付单{}状态为{}，不可申请付款",
                    invoice.invoice_no, invoice.invoice_status
                )));
            }

            // 检查申请金额是否超过未付金额
            let unpaid = invoice.unpaid_amount;
            if item.apply_amount > unpaid {
                return Err(AppError::BusinessError(format!(
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
            currency: Set(req.currency.unwrap_or_else(|| "CNY".to_string())),
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
    pub async fn update(
        &self,
        id: i32,
        req: UpdateApPaymentRequest,
        user_id: i32,
    ) -> Result<ap_payment_request::Model, AppError> {
        let txn = (&*self.db).begin().await?;

        // 1. 查询付款申请
        let request = ap_payment_request::Entity::find_by_id(id)
            .one(&txn)
            .await?
            .ok_or(AppError::ResourceNotFound(format!("付款申请 {}", id)))?;

        // 2. 检查状态（仅草稿可修改）
        if request.approval_status != "DRAFT" {
            return Err(AppError::BusinessError(format!(
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

        let request = request_active.update(&txn).await?;

        txn.commit().await?;

        Ok(request)
    }

    /// 删除付款申请（仅草稿/被拒状态）
    pub async fn delete(&self, id: i32) -> Result<(), AppError> {
        let txn = (&*self.db).begin().await?;

        // 1. 查询付款申请
        let request = ap_payment_request::Entity::find_by_id(id)
            .one(&txn)
            .await?
            .ok_or(AppError::ResourceNotFound(format!("付款申请 {}", id)))?;

        // 2. 检查状态（仅草稿或被拒可删除）
        if !["DRAFT", "REJECTED"].contains(&request.approval_status.as_str()) {
            return Err(AppError::BusinessError(format!(
                "付款申请状态为{}，不可删除",
                request.approval_status
            )));
        }

        // 3. 删除付款申请（级联删除明细）
        ap_payment_request::Entity::delete_by_id(request.id)
            .exec(&txn)
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
        let txn = (&*self.db).begin().await?;

        // 1. 查询付款申请
        let request = ap_payment_request::Entity::find_by_id(id)
            .one(&txn)
            .await?
            .ok_or(AppError::ResourceNotFound(format!("付款申请 ID: {}", id)))?;

        // 2. 检查状态（仅草稿可提交）
        if request.approval_status != "DRAFT" {
            return Err(AppError::BusinessError(format!(
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
            return Err(AppError::BusinessError(
                "付款申请没有明细，不可提交".to_string(),
            ));
        }

        // 4. 提交付款申请
        let now = Utc::now();
        let mut request_active: ap_payment_request::ActiveModel = request.into();
        request_active.approval_status = Set("APPROVING".to_string());
        request_active.submitted_by = Set(Some(user_id));
        request_active.submitted_at = Set(Some(now));
        request_active.updated_at = Set(now);

        let request = request_active.update(&txn).await?;

        txn.commit().await?;

        Ok(request)
    }

    /// 审批付款申请（按金额分级审批）
    pub async fn approve(
        &self,
        id: i32,
        user_id: i32,
    ) -> Result<ap_payment_request::Model, AppError> {
        let txn = (&*self.db).begin().await?;

        // 1. 查询付款申请
        let request = ap_payment_request::Entity::find_by_id(id)
            .one(&txn)
            .await?
            .ok_or(AppError::ResourceNotFound(format!("付款申请 {}", id)))?;

        // 2. 检查状态
        if request.approval_status != "APPROVING" {
            return Err(AppError::BusinessError(format!(
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

        let request = request_active.update(&txn).await?;

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
        let txn = (&*self.db).begin().await?;

        // 1. 查询付款申请
        let request = ap_payment_request::Entity::find_by_id(id)
            .one(&txn)
            .await?
            .ok_or(AppError::ResourceNotFound(format!("付款申请 {}", id)))?;

        // 2. 检查状态
        if request.approval_status != "APPROVING" {
            return Err(AppError::BusinessError(format!(
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

        let request = request_active.update(&txn).await?;

        txn.commit().await?;

        Ok(request)
    }

    /// 获取付款申请详情
    pub async fn get_by_id(&self, id: i32) -> Result<ap_payment_request::Model, AppError> {
        let request = ap_payment_request::Entity::find_by_id(id)
            .one(&*self.db)
            .await?
            .ok_or(AppError::ResourceNotFound(format!("付款申请 ID: {}", id)))?;

        Ok(request)
    }

    /// 获取付款申请列表
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
        let items = paginator.fetch_page(page).await?;

        Ok((items, total))
    }

    /// 检查审批权限（按金额分级）
    pub async fn check_approval_permission(
        &self,
        amount: &Decimal,
        user_id: i32,
    ) -> Result<(), AppError> {
        // 查询用户角色
        let user = crate::models::user::Entity::find_by_id(user_id)
            .one(&*self.db)
            .await?
            .ok_or(AppError::ResourceNotFound(format!("用户 {}", user_id)))?;

        // 获取用户角色（简化处理，使用 role_id 判断）
        // 这里假设 role_id 不为空则有审批权限
        let has_role = user.role_id.is_some();

        // 按金额分级审批
        if amount <= &Decimal::new(100000, 0) {
            // 10 万以下：财务经理审批
            if !has_role {
                return Err(AppError::PermissionDenied(
                    "财务经理才能审批 10 万元以下的付款".to_string(),
                ));
            }
        } else if amount <= &Decimal::new(500000, 0) {
            // 10-50 万：总经理审批
            if !has_role {
                return Err(AppError::PermissionDenied(
                    "总经理才能审批 50 万元以下的付款".to_string(),
                ));
            }
        } else {
            // 50 万以上：董事长审批
            if !has_role {
                return Err(AppError::PermissionDenied(
                    "董事长才能审批 50 万元以上的付款".to_string(),
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
    #[validate(length(min = 1, max = 20))]
    pub payment_type: String,

    /// 付款方式
    #[validate(length(min = 1, max = 20))]
    pub payment_method: String,

    /// 申请金额
    pub request_amount: Decimal,

    /// 币种
    pub currency: Option<String>,

    /// 汇率
    pub exchange_rate: Option<Decimal>,

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
