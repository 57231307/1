//! 付款 Service
//!
//! 付款服务层，负责付款执行的核心业务逻辑
//! 包含付款单创建、确认、付款计划等管理

use crate::models::{ap_invoice, ap_payment, ap_payment_request, ap_payment_request_item};
use crate::utils::number_generator::DocumentNumberGenerator;
use crate::utils::error::AppError;
use chrono::NaiveDate;
use rust_decimal::Decimal;
use sea_orm::{
    ActiveModelTrait, ColumnTrait, DatabaseConnection, EntityTrait, Order, PaginatorTrait,
    QueryFilter, QueryOrder, Set, TransactionTrait,
};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use validator::Validate;

/// 付款服务
pub struct ApPaymentService {
    db: Arc<DatabaseConnection>,
}

impl ApPaymentService {
    /// 创建服务实例
    pub fn new(db: Arc<DatabaseConnection>) -> Self {
        Self { db }
    }

    /// 生成付款单号
    /// 格式：PAY + 年月日 + 三位序号（PAY20260315001）
    pub async fn generate_payment_no(&self) -> Result<String, AppError> {
        DocumentNumberGenerator::generate_no(
            &self.db,
            "PAY",
            ap_payment::Entity,
            ap_payment::Column::PaymentNo,
        )
        .await
    }

    /// 创建付款单（从审批通过的付款申请）
    pub async fn create(
        &self,
        req: CreateApPaymentRequest,
        user_id: i32,
    ) -> Result<ap_payment::Model, AppError> {
        let txn = (*self.db).begin().await?;

        // 1. 生成付款单号
        let payment_no = self.generate_payment_no().await?;

        // 2. 检查付款申请是否存在且已审批
        let request = ap_payment_request::Entity::find_by_id(req.request_id)
            .one(&txn)
            .await?
            .ok_or(AppError::ResourceNotFound(format!(
                "付款申请 {}",
                req.request_id
            )))?;

        if request.approval_status != "APPROVED" {
            return Err(AppError::BusinessError(format!(
                "付款申请状态为{}，未审批通过不可创建付款单",
                request.approval_status
            )));
        }

        // 3. 检查是否已创建过付款单
        let exists = ap_payment::Entity::find()
            .filter(ap_payment::Column::RequestId.eq(Some(req.request_id)))
            .one(&txn)
            .await?;

        if exists.is_some() {
            return Err(AppError::BusinessError(
                "该付款申请已创建过付款单".to_string(),
            ));
        }

        // 4. 创建付款单
        let payment = ap_payment::ActiveModel {
            payment_no: Set(payment_no),
            payment_date: Set(req.payment_date),
            supplier_id: Set(request.supplier_id),
            request_id: Set(Some(req.request_id)),
            payment_method: Set(request.payment_method.clone()),
            payment_amount: Set(request.request_amount),
            payment_status: Set("REGISTERED".to_string()),
            currency: Set(request.currency.clone()),
            exchange_rate: Set(request.exchange_rate),
            bank_name: Set(request.bank_name.clone()),
            bank_account: Set(request.bank_account.clone()),
            notes: Set(req.notes.or(request.notes)),
            attachment_urls: Set(req.attachment_urls),
            created_by: Set(user_id),
            ..Default::default()
        }
        .insert(&txn)
        .await?;

        txn.commit().await?;

        Ok(payment)
    }

    /// 更新付款单（仅已登记状态）
    pub async fn update(
        &self,
        id: i32,
        req: UpdateApPaymentRequest,
        user_id: i32,
    ) -> Result<ap_payment::Model, AppError> {
        let txn = (*self.db).begin().await?;

        // 1. 查询付款单
        let payment = ap_payment::Entity::find_by_id(id)
            .one(&txn)
            .await?
            .ok_or(AppError::ResourceNotFound(format!("付款单 {}", id)))?;

        // 2. 检查状态（仅已登记可修改）
        if payment.payment_status != "REGISTERED" {
            return Err(AppError::BusinessError(format!(
                "付款单状态为{}，不可修改",
                payment.payment_status
            )));
        }

        // 3. 更新付款单
        let mut payment_active: ap_payment::ActiveModel = payment.into();

        if let Some(payment_date) = req.payment_date {
            payment_active.payment_date = Set(payment_date);
        }
        if let Some(payment_method) = req.payment_method {
            payment_active.payment_method = Set(payment_method);
        }
        if let Some(bank_name) = req.bank_name {
            payment_active.bank_name = Set(Some(bank_name));
        }
        if let Some(bank_account) = req.bank_account {
            payment_active.bank_account = Set(Some(bank_account));
        }
        if let Some(transaction_no) = req.transaction_no {
            payment_active.transaction_no = Set(Some(transaction_no));
        }
        if let Some(notes) = req.notes {
            payment_active.notes = Set(Some(notes));
        }
        if let Some(attachment_urls) = req.attachment_urls {
            payment_active.attachment_urls = Set(Some(attachment_urls));
        }

        payment_active.updated_by = Set(Some(user_id));

        let payment = crate::services::audit_log_service::AuditLogService::update_with_audit(&txn, "auto_audit", payment_active, Some(0)).await?;

        txn.commit().await?;

        Ok(payment)
    }

    /// 确认付款（执行支付）
    pub async fn confirm(&self, id: i32, user_id: i32) -> Result<ap_payment::Model, AppError> {
        let txn = (*self.db).begin().await?;

        // 1. 查询付款单
        let payment = ap_payment::Entity::find_by_id(id)
            .one(&txn)
            .await?
            .ok_or(AppError::ResourceNotFound(format!("付款单 ID: {}", id)))?;

        // 2. 检查状态
        if payment.payment_status != "REGISTERED" {
            return Err(AppError::BusinessError(format!(
                "付款单状态为{}，不可确认",
                payment.payment_status
            )));
        }

        // 3. 检查必要字段
        if payment.transaction_no.as_deref().is_none_or(|t| t.is_empty()) {
            return Err(AppError::BusinessError(
                "付款单必须填写交易流水号才能确认".to_string(),
            ));
        }

        // 4. 确认付款
        let now = chrono::Utc::now();
        let mut payment_active: ap_payment::ActiveModel = payment.into();
        payment_active.payment_status = Set("CONFIRMED".to_string());
        payment_active.confirmed_by = Set(Some(user_id));
        payment_active.confirmed_at = Set(Some(now));
        payment_active.updated_at = Set(now);

        let payment = crate::services::audit_log_service::AuditLogService::update_with_audit(&txn, "auto_audit", payment_active, Some(0)).await?;

        // 5. 更新关联的应付单已付金额
        if let Some(request_id) = payment.request_id {
            // 查询付款申请明细
            let items = ap_payment_request_item::Entity::find()
                .filter(ap_payment_request_item::Column::RequestId.eq(request_id))
                .all(&txn)
                .await?;

            // 计算每个应付单应分摊的付款金额（按申请金额比例）
            let total_apply_amount: Decimal = items.iter().map(|item| item.apply_amount).sum();

            for item in items {
                if total_apply_amount > Decimal::new(0, 2) {
                    let ratio = item.apply_amount.checked_div(total_apply_amount).unwrap_or_default();
                    let paid_amount = payment.payment_amount.checked_mul(ratio).unwrap_or_default();

                    // 更新应付单
                    use sea_orm::QuerySelect;
                    let invoice = ap_invoice::Entity::find_by_id(item.invoice_id)
                        .lock_exclusive()
                        .one(&txn)
                        .await?;

                    if let Some(mut inv) = invoice {
                        inv.paid_amount = inv.paid_amount.checked_add(paid_amount).unwrap_or(inv.paid_amount);
                        inv.unpaid_amount = inv.amount.checked_sub(inv.paid_amount).unwrap_or(inv.amount);

                        // 更新应付状态
                        inv.invoice_status = if inv.unpaid_amount <= Decimal::new(0, 2) {
                            "PAID".to_string()
                        } else {
                            "PARTIAL_PAID".to_string()
                        };

                        let invoice_active: ap_invoice::ActiveModel = inv.into();
                        crate::services::audit_log_service::AuditLogService::update_with_audit(&txn, "auto_audit", invoice_active, Some(0)).await?;
                    }
                }
            }
        }

        txn.commit().await?;

        Ok(payment)
    }

    /// 获取付款单详情
    pub async fn get_by_id(&self, id: i32) -> Result<ap_payment::Model, AppError> {
        let payment = ap_payment::Entity::find_by_id(id)
            .one(&*self.db)
            .await?
            .ok_or(AppError::ResourceNotFound(format!("付款单 {}", id)))?;

        Ok(payment)
    }

    /// 获取付款单列表
    pub async fn get_list(
        &self,
        supplier_id: Option<i32>,
        payment_status: Option<String>,
        payment_method: Option<String>,
        start_date: Option<NaiveDate>,
        end_date: Option<NaiveDate>,
        page: u64,
        page_size: u64,
    ) -> Result<(Vec<ap_payment::Model>, u64), AppError> {
        let mut query = ap_payment::Entity::find();

        // 筛选条件
        if let Some(sid) = supplier_id {
            query = query.filter(ap_payment::Column::SupplierId.eq(sid));
        }
        if let Some(status) = payment_status {
            query = query.filter(ap_payment::Column::PaymentStatus.eq(status));
        }
        if let Some(method) = payment_method {
            query = query.filter(ap_payment::Column::PaymentMethod.eq(method));
        }
        if let Some(sd) = start_date {
            query = query.filter(ap_payment::Column::PaymentDate.gte(sd));
        }
        if let Some(ed) = end_date {
            query = query.filter(ap_payment::Column::PaymentDate.lte(ed));
        }

        // 分页
        let paginator = query
            .order_by(ap_payment::Column::CreatedAt, Order::Desc)
            .paginate(&*self.db, page_size);

        let total = paginator.num_items().await?;
        let items = paginator.fetch_page(page).await?;

        Ok((items, total))
    }

    /// 获取付款计划（按供应商和日期）
    pub async fn get_payment_schedule(
        &self,
        supplier_id: Option<i32>,
        start_date: NaiveDate,
        end_date: NaiveDate,
    ) -> Result<Vec<PaymentScheduleItem>, AppError> {
        let mut query = ap_payment_request::Entity::find();

        if let Some(sid) = supplier_id {
            query = query.filter(ap_payment_request::Column::SupplierId.eq(sid));
        }

        // 查询已审批的付款申请
        let requests = query
            .filter(ap_payment_request::Column::ApprovalStatus.eq("APPROVED"))
            .filter(ap_payment_request::Column::ExpectedPaymentDate.between(start_date, end_date))
            .order_by(ap_payment_request::Column::ExpectedPaymentDate, Order::Asc)
            .all(&*self.db)
            .await?;

        let mut schedule_map: std::collections::BTreeMap<NaiveDate, PaymentScheduleItem> =
            std::collections::BTreeMap::new();

        for request in requests {
            let date = request
                .expected_payment_date
                .unwrap_or(request.request_date);

            let entry = schedule_map
                .entry(date)
                .or_insert_with(|| PaymentScheduleItem {
                    payment_date: date,
                    total_amount: Decimal::new(0, 2),
                    payment_count: 0,
                });

            entry.total_amount += request.request_amount;
            entry.payment_count += 1;
        }

        Ok(schedule_map.into_values().collect())
    }
}

// =====================================================
// 数据传输对象（DTO）
// =====================================================

/// 创建付款单请求
#[derive(Debug, Deserialize, Serialize, Validate)]
pub struct CreateApPaymentRequest {
    /// 付款申请 ID
    pub request_id: i32,

    /// 付款日期
    pub payment_date: NaiveDate,

    /// 备注
    pub notes: Option<String>,

    /// 附件 URL 列表（付款凭证）
    pub attachment_urls: Option<Vec<String>>,
}

/// 更新付款单请求
#[derive(Debug, Deserialize, Serialize, Validate)]
pub struct UpdateApPaymentRequest {
    /// 付款日期
    pub payment_date: Option<NaiveDate>,

    /// 付款方式
    pub payment_method: Option<String>,

    /// 付款银行
    pub bank_name: Option<String>,

    /// 付款账号
    pub bank_account: Option<String>,

    /// 交易流水号
    pub transaction_no: Option<String>,

    /// 备注
    pub notes: Option<String>,

    /// 附件 URL 列表
    pub attachment_urls: Option<Vec<String>>,
}

/// 付款计划项
#[derive(Debug, Serialize, Deserialize)]
pub struct PaymentScheduleItem {
    /// 付款日期
    pub payment_date: NaiveDate,

    /// 总金额
    pub total_amount: Decimal,

    /// 付款单数量
    pub payment_count: i64,
}
