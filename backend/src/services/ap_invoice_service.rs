//! 应付单 Service
//!
//! 应付单服务层，负责应付单的核心业务逻辑
//! 包含应付单自动生成、手工创建、审核、核销等全流程管理

use crate::models::{ap_invoice, purchase_receipt, purchase_return};
use crate::utils::number_generator::DocumentNumberGenerator;
use crate::utils::error::AppError;
use chrono::{Duration, NaiveDate, Utc};
use rust_decimal::Decimal;
use sea_orm::{
    ActiveModelTrait, ColumnTrait, DatabaseConnection, EntityTrait, Order, PaginatorTrait,
    QueryFilter, QueryOrder, Set, TransactionTrait,
};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use validator::Validate;

/// 应付单服务
pub struct ApInvoiceService {
    db: Arc<DatabaseConnection>,
}

impl ApInvoiceService {
    /// 创建服务实例
    pub fn new(db: Arc<DatabaseConnection>) -> Self {
        Self { db }
    }

    /// 生成应付单号
    /// 格式：AP + 年月日 + 三位序号（AP20260315001）
    pub async fn generate_invoice_no(&self) -> Result<String, AppError> {
        DocumentNumberGenerator::generate_no(
            &*self.db,
            "API",
            ap_invoice::Entity,
            ap_invoice::Column::InvoiceNo,
        )
        .await
    }

    /// 从采购入库单自动生成应付单
    pub async fn auto_generate_from_receipt(
        &self,
        receipt_id: i32,
        user_id: i32,
    ) -> Result<ap_invoice::Model, AppError> {
        let txn = (*self.db).begin().await?;

        // 1. 查询采购入库单
        let receipt = purchase_receipt::Entity::find_by_id(receipt_id)
            .one(&txn)
            .await?
            .ok_or(AppError::ResourceNotFound(format!(
                "采购入库单 {}",
                receipt_id
            )))?;

        // 2. 检查是否已生成应付
        let exists = ap_invoice::Entity::find()
            .filter(ap_invoice::Column::SourceType.eq("PURCHASE_RECEIPT"))
            .filter(ap_invoice::Column::SourceId.eq(receipt_id))
            .one(&txn)
            .await?;

        if exists.is_some() {
            return Err(AppError::BusinessError("该入库单已生成应付单".to_string()));
        }

        // 3. 获取供应商信息
        let _supplier = crate::models::supplier::Entity::find_by_id(receipt.supplier_id)
            .one(&txn)
            .await?
            .ok_or(AppError::ResourceNotFound(format!(
                "供应商 {}",
                receipt.supplier_id
            )))?;

        // 使用默认账期 30 天
        let payment_terms = 30;

        // 4. 生成应付单
        let invoice_no = self.generate_invoice_no().await?;
        let invoice_date = receipt.receipt_date;
        let due_date = invoice_date + Duration::days(payment_terms as i64);

        let invoice = ap_invoice::ActiveModel {
            invoice_no: Set(invoice_no),
            supplier_id: Set(receipt.supplier_id),
            invoice_type: Set("PURCHASE".to_string()),
            source_type: Set(Some("PURCHASE_RECEIPT".to_string())),
            source_id: Set(Some(receipt_id)),
            invoice_date: Set(invoice_date),
            due_date: Set(due_date),
            payment_terms: Set(payment_terms),
            amount: Set(receipt.total_amount),
            paid_amount: Set(Decimal::new(0, 2)),
            unpaid_amount: Set(receipt.total_amount),
            invoice_status: Set("AUDITED".to_string()), // 自动生成直接审核
            currency: Set("CNY".to_string()),
            exchange_rate: Set(Decimal::new(1, 2)),
            tax_amount: Set(Decimal::new(0, 2)),
            created_by: Set(user_id),
            ..Default::default()
        }
        .insert(&txn)
        .await?;

        txn.commit().await?;

        Ok(invoice)
    }

    /// 从采购退货单自动生成应付单（红字）
    pub async fn auto_generate_from_return(
        &self,
        return_id: i32,
        user_id: i32,
    ) -> Result<ap_invoice::Model, AppError> {
        let txn = (*self.db).begin().await?;

        // 1. 查询采购退货单
        let return_doc = purchase_return::Entity::find_by_id(return_id)
            .one(&txn)
            .await?
            .ok_or(AppError::ResourceNotFound(format!(
                "采购退货单 {}",
                return_id
            )))?;

        // 2. 检查是否已生成应付
        let exists = ap_invoice::Entity::find()
            .filter(ap_invoice::Column::SourceType.eq("PURCHASE_RETURN"))
            .filter(ap_invoice::Column::SourceId.eq(return_id))
            .one(&txn)
            .await?;

        if exists.is_some() {
            return Err(AppError::BusinessError(
                "该退货单已生成应付单（红字）".to_string(),
            ));
        }

        // 3. 生成红字应付单（负数）
        let invoice_no = self.generate_invoice_no().await?;
        let invoice_date = return_doc.return_date;
        let payment_terms = 0; // 退货立即冲销
        let due_date = invoice_date;

        // 退货金额为负数
        let amount = -return_doc.total_amount;

        let invoice = ap_invoice::ActiveModel {
            invoice_no: Set(invoice_no),
            supplier_id: Set(return_doc.supplier_id),
            invoice_type: Set("PURCHASE".to_string()),
            source_type: Set(Some("PURCHASE_RETURN".to_string())),
            source_id: Set(Some(return_id)),
            invoice_date: Set(invoice_date),
            due_date: Set(due_date),
            payment_terms: Set(payment_terms),
            amount: Set(amount),
            paid_amount: Set(Decimal::new(0, 2)),
            unpaid_amount: Set(amount),
            invoice_status: Set("AUDITED".to_string()), // 自动生成直接审核
            currency: Set("CNY".to_string()),
            exchange_rate: Set(Decimal::new(1, 2)),
            tax_amount: Set(Decimal::new(0, 2)),
            created_by: Set(user_id),
            ..Default::default()
        }
        .insert(&txn)
        .await?;

        txn.commit().await?;

        Ok(invoice)
    }

    /// 手工创建应付单
    pub async fn create_manual(
        &self,
        req: CreateApInvoiceRequest,
        user_id: i32,
    ) -> Result<ap_invoice::Model, AppError> {
        let txn = (*self.db).begin().await?;

        // 1. 生成应付单号
        let invoice_no = self.generate_invoice_no().await?;

        // 2. 创建应付单
        let invoice = ap_invoice::ActiveModel {
            invoice_no: Set(invoice_no),
            supplier_id: Set(req.supplier_id),
            invoice_type: Set(req.invoice_type),
            source_type: Set(Some("MANUAL".to_string())),
            source_id: Set(None),
            invoice_date: Set(req.invoice_date),
            due_date: Set(req.due_date),
            payment_terms: Set(req.payment_terms),
            amount: Set(req.amount),
            paid_amount: Set(Decimal::new(0, 2)),
            unpaid_amount: Set(req.amount),
            invoice_status: Set("DRAFT".to_string()),
            currency: Set(req.currency.unwrap_or_else(|| "CNY".to_string())),
            exchange_rate: Set(req.exchange_rate.unwrap_or(Decimal::new(1, 0))),
            tax_amount: Set(req.tax_amount.unwrap_or(Decimal::new(0, 2))),
            notes: Set(req.notes),
            attachment_urls: Set(req.attachment_urls),
            created_by: Set(user_id),
            ..Default::default()
        }
        .insert(&txn)
        .await?;

        txn.commit().await?;

        Ok(invoice)
    }

    /// 更新应付单（仅草稿状态）
    pub async fn update(
        &self,
        id: i32,
        req: UpdateApInvoiceRequest,
        user_id: i32,
    ) -> Result<ap_invoice::Model, AppError> {
        let txn = (*self.db).begin().await?;

        // 1. 查询应付单
        let invoice = ap_invoice::Entity::find_by_id(id)
            .one(&txn)
            .await?
            .ok_or(AppError::ResourceNotFound(format!("应付单 {}", id)))?;

        // 2. 检查状态（仅草稿可修改）
        if invoice.invoice_status != "DRAFT" {
            return Err(AppError::BusinessError(format!(
                "应付单状态为{}，不可修改",
                invoice.invoice_status
            )));
        }

        // 3. 更新应付单
        let mut invoice_active: ap_invoice::ActiveModel = invoice.into();

        if let Some(invoice_type) = req.invoice_type {
            invoice_active.invoice_type = Set(invoice_type);
        }
        if let Some(invoice_date) = req.invoice_date {
            invoice_active.invoice_date = Set(invoice_date);
        }
        if let Some(due_date) = req.due_date {
            invoice_active.due_date = Set(due_date);
        }
        if let Some(payment_terms) = req.payment_terms {
            invoice_active.payment_terms = Set(payment_terms);
        }
        if let Some(amount) = req.amount {
            invoice_active.amount = Set(amount);
            invoice_active.unpaid_amount = Set(amount);
        }
        if let Some(notes) = req.notes {
            invoice_active.notes = Set(Some(notes));
        }
        if let Some(attachment_urls) = req.attachment_urls {
            invoice_active.attachment_urls = Set(Some(attachment_urls));
        }

        invoice_active.updated_by = Set(Some(user_id));

        let invoice = invoice_active.update(&txn).await?;

        txn.commit().await?;

        Ok(invoice)
    }

    /// 删除应付单（仅草稿状态）
    pub async fn delete(&self, id: i32) -> Result<(), AppError> {
        let txn = (*self.db).begin().await?;

        // 1. 查询应付单
        let invoice = ap_invoice::Entity::find_by_id(id)
            .one(&txn)
            .await?
            .ok_or(AppError::ResourceNotFound(format!("应付单 {}", id)))?;

        // 2. 检查状态（仅草稿可删除）
        if invoice.invoice_status != "DRAFT" {
            return Err(AppError::BusinessError(format!(
                "应付单状态为{}，不可删除",
                invoice.invoice_status
            )));
        }

        // 3. 删除应付单
        ap_invoice::Entity::delete_by_id(invoice.id)
            .exec(&txn)
            .await?;

        txn.commit().await?;

        Ok(())
    }

    /// 审核应付单
    pub async fn approve(&self, id: i32, user_id: i32) -> Result<ap_invoice::Model, AppError> {
        let txn = (*self.db).begin().await?;

        // 1. 查询应付单
        let invoice = ap_invoice::Entity::find_by_id(id)
            .one(&txn)
            .await?
            .ok_or(AppError::ResourceNotFound(format!("应付单 {}", id)))?;

        // 2. 检查状态
        if invoice.invoice_status != "DRAFT" {
            return Err(AppError::BusinessError(format!(
                "应付单状态为{}，不可审核",
                invoice.invoice_status
            )));
        }

        // 3. 审核应付单
        let now = Utc::now();
        let mut invoice_active: ap_invoice::ActiveModel = invoice.into();
        invoice_active.invoice_status = Set("AUDITED".to_string());
        invoice_active.approved_by = Set(Some(user_id));
        invoice_active.approved_at = Set(Some(now));
        invoice_active.updated_at = Set(now);

        let invoice = invoice_active.update(&txn).await?;

        txn.commit().await?;

        Ok(invoice)
    }

    /// 取消应付单
    pub async fn cancel(
        &self,
        id: i32,
        reason: String,
        user_id: i32,
    ) -> Result<ap_invoice::Model, AppError> {
        let txn = (*self.db).begin().await?;

        // 1. 查询应付单
        let invoice = ap_invoice::Entity::find_by_id(id)
            .one(&txn)
            .await?
            .ok_or(AppError::ResourceNotFound(format!("应付单 {}", id)))?;

        // 2. 检查状态（已审核或部分付款可取消）
        if !["AUDITED", "PARTIAL_PAID"].contains(&invoice.invoice_status.as_str()) {
            return Err(AppError::BusinessError(format!(
                "应付单状态为{}，不可取消",
                invoice.invoice_status
            )));
        }

        // 3. 检查是否已付款
        if invoice.paid_amount > Decimal::new(0, 2) {
            return Err(AppError::BusinessError(
                "应付单已有付款记录，不可取消".to_string(),
            ));
        }

        // 4. 取消应付单
        let now = Utc::now();
        let mut invoice_active: ap_invoice::ActiveModel = invoice.into();
        invoice_active.invoice_status = Set("CANCELLED".to_string());
        invoice_active.cancelled_by = Set(Some(user_id));
        invoice_active.cancelled_at = Set(Some(now));
        invoice_active.cancelled_reason = Set(Some(reason));
        invoice_active.updated_at = Set(now);

        let invoice = invoice_active.update(&txn).await?;

        txn.commit().await?;

        Ok(invoice)
    }

    /// 获取应付单详情
    pub async fn get_by_id(&self, id: i32) -> Result<ap_invoice::Model, AppError> {
        let invoice = ap_invoice::Entity::find_by_id(id)
            .one(&*self.db)
            .await?
            .ok_or(AppError::ResourceNotFound(format!("应付单 {}", id)))?;

        Ok(invoice)
    }

    /// 获取应付单列表（含分页、筛选）
    pub async fn get_list(
        &self,
        supplier_id: Option<i32>,
        invoice_status: Option<String>,
        invoice_type: Option<String>,
        start_date: Option<NaiveDate>,
        end_date: Option<NaiveDate>,
        page: u64,
        page_size: u64,
    ) -> Result<(Vec<ap_invoice::Model>, u64), AppError> {
        let mut query = ap_invoice::Entity::find();

        // 筛选条件
        if let Some(sid) = supplier_id {
            query = query.filter(ap_invoice::Column::SupplierId.eq(sid));
        }
        if let Some(status) = invoice_status {
            query = query.filter(ap_invoice::Column::InvoiceStatus.eq(status));
        }
        if let Some(itype) = invoice_type {
            query = query.filter(ap_invoice::Column::InvoiceType.eq(itype));
        }
        if let Some(sd) = start_date {
            query = query.filter(ap_invoice::Column::InvoiceDate.gte(sd));
        }
        if let Some(ed) = end_date {
            query = query.filter(ap_invoice::Column::InvoiceDate.lte(ed));
        }

        // 分页
        let paginator = query
            .order_by(ap_invoice::Column::CreatedAt, Order::Desc)
            .paginate(&*self.db, page_size);

        let total = paginator.num_items().await?;
        let items = paginator.fetch_page(page).await?;

        Ok((items, total))
    }

    /// 获取账龄分析
    pub async fn get_aging_analysis(
        &self,
        supplier_id: Option<i32>,
    ) -> Result<Vec<AgingAnalysisItem>, AppError> {
        let mut query = ap_invoice::Entity::find();

        if let Some(sid) = supplier_id {
            query = query.filter(ap_invoice::Column::SupplierId.eq(sid));
        }

        // 查询未付清的应付单
        let invoices = query
            .filter(ap_invoice::Column::InvoiceStatus.ne("PAID"))
            .filter(ap_invoice::Column::InvoiceStatus.ne("CANCELLED"))
            .all(&*self.db)
            .await?;

        let today = Utc::now().naive_utc().date();
        let mut aging_map: std::collections::BTreeMap<String, AgingAnalysisItem> =
            std::collections::BTreeMap::new();

        for invoice in invoices {
            let unpaid = invoice.unpaid_amount;
            let days_overdue = if invoice.due_date < today {
                (today - invoice.due_date).num_days() as i32
            } else {
                -1 // 未到期
            };

            // 按账龄区间分类
            let aging_bucket = if days_overdue < 0 {
                "未到期".to_string()
            } else if days_overdue <= 30 {
                "逾期 1-30 天".to_string()
            } else if days_overdue <= 60 {
                "逾期 31-60 天".to_string()
            } else if days_overdue <= 90 {
                "逾期 61-90 天".to_string()
            } else if days_overdue <= 180 {
                "逾期 91-180 天".to_string()
            } else {
                "逾期 180 天以上".to_string()
            };

            let entry =
                aging_map
                    .entry(aging_bucket.clone())
                    .or_insert_with(|| AgingAnalysisItem {
                        aging_bucket,
                        invoice_count: 0,
                        total_amount: Decimal::new(0, 2),
                    });

            entry.invoice_count += 1;
            entry.total_amount += unpaid;
        }

        Ok(aging_map.into_values().collect())
    }

    /// 获取应付余额表
    pub async fn get_balance_summary(
        &self,
        supplier_id: Option<i32>,
    ) -> Result<BalanceSummary, AppError> {
        let mut query = ap_invoice::Entity::find();

        if let Some(sid) = supplier_id {
            query = query.filter(ap_invoice::Column::SupplierId.eq(sid));
        }

        // 查询所有有效应付单
        let invoices = query
            .filter(ap_invoice::Column::InvoiceStatus.ne("CANCELLED"))
            .all(&*self.db)
            .await?;

        let mut summary = BalanceSummary {
            total_invoice_amount: Decimal::new(0, 2),
            total_paid_amount: Decimal::new(0, 2),
            total_unpaid_amount: Decimal::new(0, 2),
            invoice_count: 0,
        };

        for invoice in invoices {
            summary.total_invoice_amount += invoice.amount;
            summary.total_paid_amount += invoice.paid_amount;
            summary.total_unpaid_amount += invoice.unpaid_amount;
            summary.invoice_count += 1;
        }

        Ok(summary)
    }
}

// =====================================================
// 数据传输对象（DTO）
// =====================================================

/// 创建应付单请求
#[derive(Debug, Deserialize, Validate)]
pub struct CreateApInvoiceRequest {
    /// 供应商 ID
    pub supplier_id: i32,

    /// 应付类型
    #[validate(length(min = 1, max = 20, message = "发票号码长度必须在1到20个字符之间"))]
    pub invoice_type: String,

    /// 应付日期
    pub invoice_date: NaiveDate,

    /// 到期日期
    pub due_date: NaiveDate,

    /// 账期（天）
    #[validate(range(min = 0, max = 365, message = "账期必须在0到365天之间"))]
    pub payment_terms: i32,

    /// 应付金额
    pub amount: Decimal,

    /// 币种
    pub currency: Option<String>,

    /// 汇率
    pub exchange_rate: Option<Decimal>,

    /// 税额
    pub tax_amount: Option<Decimal>,

    /// 备注
    pub notes: Option<String>,

    /// 附件 URL 列表
    pub attachment_urls: Option<Vec<String>>,
}

/// 更新应付单请求
#[derive(Debug, Deserialize, Validate)]
pub struct UpdateApInvoiceRequest {
    /// 应付类型
    pub invoice_type: Option<String>,

    /// 应付日期
    pub invoice_date: Option<NaiveDate>,

    /// 到期日期
    pub due_date: Option<NaiveDate>,

    /// 账期（天）
    pub payment_terms: Option<i32>,

    /// 应付金额
    pub amount: Option<Decimal>,

    /// 备注
    pub notes: Option<String>,

    /// 附件 URL 列表
    pub attachment_urls: Option<Vec<String>>,
}

/// 账龄分析项
#[derive(Debug, Serialize, Deserialize)]
pub struct AgingAnalysisItem {
    /// 账龄区间
    pub aging_bucket: String,

    /// 应付单数量
    pub invoice_count: i64,

    /// 总金额
    pub total_amount: Decimal,
}

/// 应付余额汇总
#[derive(Debug, Serialize, Deserialize)]
pub struct BalanceSummary {
    /// 应付总金额
    pub total_invoice_amount: Decimal,

    /// 已付总金额
    pub total_paid_amount: Decimal,

    /// 未付总金额
    pub total_unpaid_amount: Decimal,

    /// 应付单数量
    pub invoice_count: i64,
}
