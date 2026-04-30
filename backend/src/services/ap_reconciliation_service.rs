//! 供应商对账 Service
//!
//! 供应商对账服务层，负责对账的核心业务逻辑
//! 包含生成对账单、确认对账、争议处理等管理

use crate::models::{ap_invoice, ap_payment, ap_reconciliation};
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

/// 供应商对账服务
pub struct ApReconciliationService {
    db: Arc<DatabaseConnection>,
}

impl ApReconciliationService {
    /// 创建服务实例
    pub fn new(db: Arc<DatabaseConnection>) -> Self {
        Self { db }
    }

    /// 生成对账单号
    /// 格式：REC + 年月日 + 三位序号（REC20260315001）
    pub async fn generate_reconciliation_no(&self) -> Result<String, AppError> {
        let today = Utc::now().format("%Y%m%d").to_string();
        let prefix = format!("REC{}", today);

        // 查询今日对账单数量
        let count = ap_reconciliation::Entity::find()
            .filter(ap_reconciliation::Column::ReconciliationNo.starts_with(&prefix))
            .count(&*self.db)
            .await?;

        Ok(format!("{}{:03}", prefix, count + 1))
    }

    /// 生成供应商对账单
    pub async fn generate_reconciliation(
        &self,
        req: GenerateReconciliationRequest,
        user_id: i32,
    ) -> Result<ap_reconciliation::Model, AppError> {
        let txn = (*self.db).begin().await?;

        // 1. 生成对账单号
        let reconciliation_no = self.generate_reconciliation_no().await?;

        // 2. 查询该供应商在对账期间内的应付单
        let invoices = ap_invoice::Entity::find()
            .filter(ap_invoice::Column::SupplierId.eq(req.supplier_id))
            .filter(ap_invoice::Column::InvoiceStatus.ne("CANCELLED"))
            .filter(ap_invoice::Column::InvoiceDate.gte(req.start_date))
            .filter(ap_invoice::Column::InvoiceDate.lte(req.end_date))
            .all(&txn)
            .await?;

        // 3. 查询该供应商在对账期间内的付款单
        let payments = ap_payment::Entity::find()
            .filter(ap_payment::Column::SupplierId.eq(req.supplier_id))
            .filter(ap_payment::Column::PaymentStatus.eq("CONFIRMED"))
            .filter(ap_payment::Column::PaymentDate.gte(req.start_date))
            .filter(ap_payment::Column::PaymentDate.lte(req.end_date))
            .all(&txn)
            .await?;

        // 4. 计算期初余额（对账开始日期前的未付金额）
        let opening_balance = ap_invoice::Entity::find()
            .filter(ap_invoice::Column::SupplierId.eq(req.supplier_id))
            .filter(ap_invoice::Column::InvoiceStatus.ne("CANCELLED"))
            .filter(ap_invoice::Column::InvoiceDate.lt(req.start_date))
            .all(&txn)
            .await?
            .iter()
            .map(|inv| inv.unpaid_amount)
            .sum::<Decimal>();

        // 5. 计算本期应付合计
        let total_invoice: Decimal = invoices.iter().map(|inv| inv.amount).sum();

        // 6. 计算本期付款合计
        let total_payment: Decimal = payments.iter().map(|pay| pay.payment_amount).sum();

        // 7. 计算期末余额
        let closing_balance = opening_balance + total_invoice - total_payment;

        // 8. 创建对账单
        let reconciliation = ap_reconciliation::ActiveModel {
            reconciliation_no: Set(reconciliation_no),
            supplier_id: Set(req.supplier_id),
            start_date: Set(req.start_date),
            end_date: Set(req.end_date),
            opening_balance: Set(opening_balance),
            total_invoice: Set(total_invoice),
            total_payment: Set(total_payment),
            closing_balance: Set(closing_balance),
            reconciliation_status: Set("PENDING".to_string()),
            notes: Set(req.notes),
            created_by: Set(user_id),
            ..Default::default()
        }
        .insert(&txn)
        .await?;

        txn.commit().await?;

        Ok(reconciliation)
    }

    /// 确认对账单
    pub async fn confirm_reconciliation(
        &self,
        id: i32,
        user_id: i32,
    ) -> Result<ap_reconciliation::Model, AppError> {
        let txn = (*self.db).begin().await?;

        // 1. 查询对账单
        let reconciliation = ap_reconciliation::Entity::find_by_id(id)
            .one(&txn)
            .await?
            .ok_or(AppError::ResourceNotFound(format!("对账单 {}", id)))?;

        // 2. 检查状态
        if reconciliation.reconciliation_status != "PENDING" {
            return Err(AppError::BusinessError(format!(
                "对账单状态为{}，不可确认",
                reconciliation.reconciliation_status
            )));
        }

        // 3. 确认对账单
        let now = Utc::now();
        let mut reconciliation_active: ap_reconciliation::ActiveModel = reconciliation.into();
        reconciliation_active.reconciliation_status = Set("CONFIRMED".to_string());
        reconciliation_active.confirmed_by = Set(Some(user_id));
        reconciliation_active.confirmed_at = Set(Some(now));
        reconciliation_active.updated_at = Set(now);

        let reconciliation = reconciliation_active.update(&txn).await?;

        txn.commit().await?;

        Ok(reconciliation)
    }

    /// 提出争议
    pub async fn dispute(
        &self,
        id: i32,
        reason: String,
        user_id: i32,
    ) -> Result<ap_reconciliation::Model, AppError> {
        let txn = (*self.db).begin().await?;

        // 1. 查询对账单
        let reconciliation = ap_reconciliation::Entity::find_by_id(id)
            .one(&txn)
            .await?
            .ok_or(AppError::ResourceNotFound(format!("对账单 {}", id)))?;

        // 2. 检查状态
        if reconciliation.reconciliation_status == "CONFIRMED" {
            return Err(AppError::BusinessError(
                "对账单已确认，不可提出争议".to_string(),
            ));
        }

        // 3. 提出争议
        let now = Utc::now();
        let mut reconciliation_active: ap_reconciliation::ActiveModel = reconciliation.into();
        reconciliation_active.reconciliation_status = Set("DISPUTED".to_string());
        reconciliation_active.disputed_by = Set(Some(user_id));
        reconciliation_active.disputed_at = Set(Some(now));
        reconciliation_active.disputed_reason = Set(Some(reason));
        reconciliation_active.updated_at = Set(now);

        let reconciliation = reconciliation_active.update(&txn).await?;

        txn.commit().await?;

        Ok(reconciliation)
    }

    /// 获取对账单详情
    pub async fn get_by_id(&self, id: i32) -> Result<ap_reconciliation::Model, AppError> {
        let reconciliation = ap_reconciliation::Entity::find_by_id(id)
            .one(&*self.db)
            .await?
            .ok_or(AppError::ResourceNotFound(format!("对账单 {}", id)))?;

        Ok(reconciliation)
    }

    /// 获取对账单列表
    pub async fn get_list(
        &self,
        supplier_id: Option<i32>,
        reconciliation_status: Option<String>,
        start_date: Option<NaiveDate>,
        end_date: Option<NaiveDate>,
        page: u64,
        page_size: u64,
    ) -> Result<(Vec<ap_reconciliation::Model>, u64), AppError> {
        let mut query = ap_reconciliation::Entity::find();

        // 筛选条件
        if let Some(sid) = supplier_id {
            query = query.filter(ap_reconciliation::Column::SupplierId.eq(sid));
        }
        if let Some(status) = reconciliation_status {
            query = query.filter(ap_reconciliation::Column::ReconciliationStatus.eq(status));
        }
        if let Some(sd) = start_date {
            query = query.filter(ap_reconciliation::Column::StartDate.gte(sd));
        }
        if let Some(ed) = end_date {
            query = query.filter(ap_reconciliation::Column::EndDate.lte(ed));
        }

        // 分页
        let paginator = query
            .order_by(ap_reconciliation::Column::CreatedAt, Order::Desc)
            .paginate(&*self.db, page_size);

        let total = paginator.num_items().await?;
        let items = paginator.fetch_page(page).await?;

        Ok((items, total))
    }

    /// 获取供应商应付汇总（从物化视图）
    pub async fn get_supplier_summary(
        &self,
        supplier_id: Option<i32>,
    ) -> Result<Vec<SupplierApSummary>, AppError> {
        // 使用 SQL 查询物化视图
        let _sql = if let Some(sid) = supplier_id {
            format!(
                "SELECT * FROM mv_supplier_ap_summary WHERE supplier_id = {}",
                sid
            )
        } else {
            "SELECT * FROM mv_supplier_ap_summary".to_string()
        };

        // 这里简化处理，实际应该使用 SeaORM 的 query 方法
        // 由于物化视图查询较复杂，暂时返回空结果
        Ok(vec![])
    }
}

// =====================================================
// 数据传输对象（DTO）
// =====================================================

/// 生成对账单请求
#[derive(Debug, Deserialize, Serialize, Validate)]
pub struct GenerateReconciliationRequest {
    /// 供应商 ID
    pub supplier_id: i32,

    /// 对账开始日期
    pub start_date: NaiveDate,

    /// 对账结束日期
    pub end_date: NaiveDate,

    /// 备注
    pub notes: Option<String>,
}

/// 供应商应付汇总
#[derive(Debug, Serialize, Deserialize)]
pub struct SupplierApSummary {
    /// 供应商 ID
    pub supplier_id: i32,

    /// 供应商编码
    pub supplier_code: String,

    /// 供应商名称
    pub supplier_name: String,

    /// 应付单总数
    pub total_invoice_count: i64,

    /// 应付总金额
    pub total_invoice_amount: Decimal,

    /// 已付总金额
    pub total_paid_amount: Decimal,

    /// 未付总金额
    pub total_unpaid_amount: Decimal,

    /// 已付清应付单数量
    pub paid_invoice_count: i64,

    /// 部分付款应付单数量
    pub partial_paid_invoice_count: i64,

    /// 逾期应付单数量
    pub overdue_invoice_count: i64,

    /// 逾期金额
    pub overdue_amount: Decimal,
}
