//! 应收对账 Service
//!
//! 提供客户应收对账单的生成、发送、确认和争议处理
//!
//! # 主要功能
//! - 对账单自动生成（基于销售发票和收款记录）
//! - 对账单状态流转（草稿→已发送→已确认→有争议→已关闭）
//! - 自动对账匹配（发票与收款匹配）
//! - 账龄分析计算

#![allow(dead_code)]

use chrono::{NaiveDate, Utc};
use rust_decimal::Decimal;
use rust_decimal::prelude::Zero;
use sea_orm::{
    ActiveModelTrait, ColumnTrait, EntityTrait, PaginatorTrait, QueryFilter, QueryOrder, Set,
    TransactionTrait, Condition,
};
use std::sync::Arc;
use sea_orm::DatabaseConnection;

use crate::models::ar_reconciliation::{
    ActiveModel, Entity as ReconciliationEntity, Model as ReconciliationModel,
};
use crate::models::ar_reconciliation_item::{
    ActiveModel as ItemActiveModel, Entity as ItemEntity, Model as ItemModel,
};
use crate::models::ar_aging_analysis::{
    ActiveModel as AgingActiveModel, Entity as AgingEntity, Model as AgingModel,
};
use crate::utils::error::AppError;

/// 创建对账单请求
#[derive(Debug, Clone)]
pub struct CreateReconciliationRequest {
    pub reconciliation_no: String,
    pub customer_id: i32,
    pub start_date: NaiveDate,
    pub end_date: NaiveDate,
    pub opening_balance: Decimal,
    pub current_receivable: Decimal,
    pub current_received: Decimal,
    pub remarks: Option<String>,
}

/// 更新对账单请求
#[derive(Debug, Clone)]
pub struct UpdateReconciliationRequest {
    pub opening_balance: Option<Decimal>,
    pub current_receivable: Option<Decimal>,
    pub current_received: Option<Decimal>,
    pub remarks: Option<String>,
}

/// 对账单查询参数
#[derive(Debug, Clone)]
pub struct ReconciliationQuery {
    pub status: Option<String>,
    pub customer_id: Option<i32>,
    pub page: u64,
    pub page_size: u64,
}

/// 应收对账 Service
pub struct ArReconciliationService {
    db: Arc<DatabaseConnection>,
}

impl ArReconciliationService {
    pub fn new(db: Arc<DatabaseConnection>) -> Self {
        Self { db }
    }

    /// 创建对账单
    pub async fn create(
        &self,
        req: CreateReconciliationRequest,
    ) -> Result<ReconciliationModel, AppError> {
        let closing_balance = req.opening_balance + req.current_receivable - req.current_received;

        let active_model = ActiveModel {
            reconciliation_no: Set(req.reconciliation_no),
            customer_id: Set(req.customer_id),
            start_date: Set(req.start_date),
            end_date: Set(req.end_date),
            opening_balance: Set(req.opening_balance),
            current_receivable: Set(req.current_receivable),
            current_received: Set(req.current_received),
            closing_balance: Set(closing_balance),
            status: Set("DRAFT".to_string()),
            confirmed_date: Set(None),
            dispute_reason: Set(None),
            remarks: Set(req.remarks),
            is_deleted: Set(false),
            created_at: Set(Utc::now()),
            updated_at: Set(Utc::now()),
            ..Default::default()
        };

        let model = active_model
            .insert(&*self.db)
            .await
            .map_err(|e| AppError::DatabaseError(e.to_string()))?;

        Ok(model)
    }

    /// 根据ID获取对账单
    pub async fn get_by_id(&self, id: i32) -> Result<Option<ReconciliationModel>, AppError> {
        let model = ReconciliationEntity::find_by_id(id)
            .one(&*self.db)
            .await
            .map_err(|e| AppError::DatabaseError(e.to_string()))?;

        Ok(model)
    }

    /// 获取对账单列表
    pub async fn list(
        &self,
        query: ReconciliationQuery,
    ) -> Result<(Vec<ReconciliationModel>, u64), AppError> {
        let mut select = ReconciliationEntity::find()
            .filter(crate::models::ar_reconciliation::Column::IsDeleted.eq(false));

        if let Some(status) = query.status {
            select = select.filter(crate::models::ar_reconciliation::Column::Status.eq(status));
        }

        if let Some(customer_id) = query.customer_id {
            select = select.filter(crate::models::ar_reconciliation::Column::CustomerId.eq(customer_id));
        }

        let total = select
            .clone()
            .count(&*self.db)
            .await
            .map_err(|e| AppError::DatabaseError(e.to_string()))?;

        let paginator = select
            .order_by_desc(crate::models::ar_reconciliation::Column::CreatedAt)
            .paginate(&*self.db, query.page_size);

        let models = paginator
            .fetch_page(query.page - 1)
            .await
            .map_err(|e| AppError::DatabaseError(e.to_string()))?;

        Ok((models, total))
    }

    /// 更新对账单
    pub async fn update(
        &self,
        id: i32,
        req: UpdateReconciliationRequest,
    ) -> Result<ReconciliationModel, AppError> {
        let model = ReconciliationEntity::find_by_id(id)
            .one(&*self.db)
            .await
            .map_err(|e| AppError::DatabaseError(e.to_string()))?
            .ok_or_else(|| AppError::NotFound("对账单不存在".to_string()))?;

        let mut active_model: ActiveModel = model.into();

        if let Some(opening_balance) = req.opening_balance {
            active_model.opening_balance = Set(opening_balance);
        }
        if let Some(current_receivable) = req.current_receivable {
            active_model.current_receivable = Set(current_receivable);
        }
        if let Some(current_received) = req.current_received {
            active_model.current_received = Set(current_received);
        }
        if let Some(remarks) = req.remarks {
            active_model.remarks = Set(Some(remarks));
        }

        // 重新计算期末余额
        let opening = active_model.opening_balance.as_ref().clone();
        let receivable = active_model.current_receivable.as_ref().clone();
        let received = active_model.current_received.as_ref().clone();
        active_model.closing_balance = Set(opening + receivable - received);

        active_model.updated_at = Set(Utc::now());

        let updated = active_model
            .update(&*self.db)
            .await
            .map_err(|e| AppError::DatabaseError(e.to_string()))?;

        Ok(updated)
    }

    /// 删除对账单（软删除）
    pub async fn delete(&self, id: i32) -> Result<(), AppError> {
        let model = ReconciliationEntity::find_by_id(id)
            .one(&*self.db)
            .await
            .map_err(|e| AppError::DatabaseError(e.to_string()))?
            .ok_or_else(|| AppError::NotFound("对账单不存在".to_string()))?;

        let mut active_model: ActiveModel = model.into();
        active_model.is_deleted = Set(true);
        active_model.updated_at = Set(Utc::now());

        active_model
            .update(&*self.db)
            .await
            .map_err(|e| AppError::DatabaseError(e.to_string()))?;

        Ok(())
    }

    /// 发送对账单
    pub async fn send(&self, id: i32) -> Result<ReconciliationModel, AppError> {
        let model = ReconciliationEntity::find_by_id(id)
            .one(&*self.db)
            .await
            .map_err(|e| AppError::DatabaseError(e.to_string()))?
            .ok_or_else(|| AppError::NotFound("对账单不存在".to_string()))?;

        let mut active_model: ActiveModel = model.into();
        active_model.status = Set("SENT".to_string());
        active_model.updated_at = Set(Utc::now());

        let updated = active_model
            .update(&*self.db)
            .await
            .map_err(|e| AppError::DatabaseError(e.to_string()))?;

        Ok(updated)
    }

    /// 客户确认对账单
    pub async fn confirm(&self, id: i32) -> Result<ReconciliationModel, AppError> {
        let model = ReconciliationEntity::find_by_id(id)
            .one(&*self.db)
            .await
            .map_err(|e| AppError::DatabaseError(e.to_string()))?
            .ok_or_else(|| AppError::NotFound("对账单不存在".to_string()))?;

        let mut active_model: ActiveModel = model.into();
        active_model.status = Set("CONFIRMED".to_string());
        active_model.confirmed_date = Set(Some(Utc::now().date_naive()));
        active_model.updated_at = Set(Utc::now());

        let updated = active_model
            .update(&*self.db)
            .await
            .map_err(|e| AppError::DatabaseError(e.to_string()))?;

        Ok(updated)
    }

    /// 客户提出争议
    pub async fn dispute(
        &self,
        id: i32,
        reason: String,
    ) -> Result<ReconciliationModel, AppError> {
        let model = ReconciliationEntity::find_by_id(id)
            .one(&*self.db)
            .await
            .map_err(|e| AppError::DatabaseError(e.to_string()))?
            .ok_or_else(|| AppError::NotFound("对账单不存在".to_string()))?;

        let mut active_model: ActiveModel = model.into();
        active_model.status = Set("DISPUTED".to_string());
        active_model.dispute_reason = Set(Some(reason));
        active_model.updated_at = Set(Utc::now());

        let updated = active_model
            .update(&*self.db)
            .await
            .map_err(|e| AppError::DatabaseError(e.to_string()))?;

        Ok(updated)
    }

    /// 关闭对账单
    pub async fn close(&self, id: i32) -> Result<ReconciliationModel, AppError> {
        let model = ReconciliationEntity::find_by_id(id)
            .one(&*self.db)
            .await
            .map_err(|e| AppError::DatabaseError(e.to_string()))?
            .ok_or_else(|| AppError::NotFound("对账单不存在".to_string()))?;

        let mut active_model: ActiveModel = model.into();
        active_model.status = Set("CLOSED".to_string());
        active_model.updated_at = Set(Utc::now());

        let updated = active_model
            .update(&*self.db)
            .await
            .map_err(|e| AppError::DatabaseError(e.to_string()))?;

        Ok(updated)
    }

    /// 更新对账单状态（通用）
    pub async fn update_status(&self, id: i32, status: &str) -> Result<ReconciliationModel, AppError> {
        let model = ReconciliationEntity::find_by_id(id)
            .one(&*self.db)
            .await
            .map_err(|e| AppError::DatabaseError(e.to_string()))?
            .ok_or_else(|| AppError::NotFound("对账单不存在".to_string()))?;

        let mut active_model: ActiveModel = model.into();
        active_model.status = Set(status.to_string());
        active_model.updated_at = Set(Utc::now());

        let updated = active_model
            .update(&*self.db)
            .await
            .map_err(|e| AppError::DatabaseError(e.to_string()))?;

        Ok(updated)
    }

    /// 自动生成对账单
    ///
    /// 根据客户ID和对账期间，自动汇总销售发票和收款记录生成对账单
    ///
    /// # 参数
    /// - `customer_id`: 客户 ID
    /// - `start_date`: 对账开始日期
    /// - `end_date`: 对账结束日期
    /// - `created_by`: 创建人 ID
    ///
    /// # 返回
    /// - `Ok(model)`: 生成的对账单
    /// - `Err(AppError)`: 生成失败
    pub async fn generate_reconciliation(
        &self,
        customer_id: i32,
        start_date: NaiveDate,
        end_date: NaiveDate,
        created_by: Option<i32>,
    ) -> Result<ReconciliationModel, AppError> {
        let txn = self.db.begin().await
            .map_err(|e| AppError::DatabaseError(e.to_string()))?;

        // 生成对账单编号
        let reconciliation_no = format!(
            "AR{}{}{:04}",
            Utc::now().format("%Y%m%d"),
            customer_id,
            (Utc::now().timestamp_millis() % 10000) as u16
        );

        // 计算期初余额（截止日期之前的期末余额）
        let opening_balance = self.calculate_opening_balance(customer_id, start_date).await?;

        // 计算本期应收（期间内的销售发票总额）
        let current_receivable = self.calculate_current_receivable(customer_id, start_date, end_date).await?;

        // 计算本期收款（期间内的收款总额）
        let current_received = self.calculate_current_received(customer_id, start_date, end_date).await?;

        // 计算期末余额
        let closing_balance = opening_balance + current_receivable - current_received;

        // 创建对账单
        let active_model = ActiveModel {
            reconciliation_no: Set(reconciliation_no),
            customer_id: Set(customer_id),
            start_date: Set(start_date),
            end_date: Set(end_date),
            opening_balance: Set(opening_balance),
            current_receivable: Set(current_receivable),
            current_received: Set(current_received),
            closing_balance: Set(closing_balance),
            status: Set("DRAFT".to_string()),
            confirmed_date: Set(None),
            dispute_reason: Set(None),
            remarks: Set(None),
            is_deleted: Set(false),
            created_by: Set(created_by),
            confirmed_by: Set(None),
            created_at: Set(Utc::now()),
            updated_at: Set(Utc::now()),
            confirmed_at: Set(None),
            ..Default::default()
        };

        let model = active_model
            .insert(&txn)
            .await
            .map_err(|e| AppError::DatabaseError(e.to_string()))?;

        // 生成对账单明细
        self.generate_reconciliation_items(&txn, model.id, customer_id, start_date, end_date, opening_balance).await?;

        txn.commit().await
            .map_err(|e| AppError::DatabaseError(e.to_string()))?;

        Ok(model)
    }

    /// 计算期初余额
    async fn calculate_opening_balance(
        &self,
        customer_id: i32,
        before_date: NaiveDate,
    ) -> Result<Decimal, AppError> {
        // 查询该客户最近一次已确认的对账单作为期初余额基础
        let last_reconciliation = ReconciliationEntity::find()
            .filter(crate::models::ar_reconciliation::Column::CustomerId.eq(customer_id))
            .filter(crate::models::ar_reconciliation::Column::Status.eq("CONFIRMED"))
            .filter(crate::models::ar_reconciliation::Column::EndDate.lt(before_date))
            .filter(crate::models::ar_reconciliation::Column::IsDeleted.eq(false))
            .order_by_desc(crate::models::ar_reconciliation::Column::EndDate)
            .one(&*self.db)
            .await
            .map_err(|e| AppError::DatabaseError(e.to_string()))?;

        if let Some(rec) = last_reconciliation {
            Ok(rec.closing_balance)
        } else {
            // 没有历史对账单，期初余额为0
            Ok(Decimal::zero())
        }
    }

    /// 计算本期应收
    async fn calculate_current_receivable(
        &self,
        _customer_id: i32,
        _start_date: NaiveDate,
        _end_date: NaiveDate,
    ) -> Result<Decimal, AppError> {
        // TODO: 从销售发票表查询期间内的应收总额
        // 当前返回0，待接入实际发票数据
        Ok(Decimal::zero())
    }

    /// 计算本期收款
    async fn calculate_current_received(
        &self,
        _customer_id: i32,
        _start_date: NaiveDate,
        _end_date: NaiveDate,
    ) -> Result<Decimal, AppError> {
        // TODO: 从收款记录表查询期间内的收款总额
        // 当前返回0，待接入实际收款数据
        Ok(Decimal::zero())
    }

    /// 生成对账单明细
    async fn generate_reconciliation_items(
        &self,
        txn: &sea_orm::DatabaseTransaction,
        reconciliation_id: i32,
        _customer_id: i32,
        _start_date: NaiveDate,
        _end_date: NaiveDate,
        opening_balance: Decimal,
    ) -> Result<(), AppError> {
        // 添加期初余额明细
        let opening_item = ItemActiveModel {
            reconciliation_id: Set(reconciliation_id),
            item_type: Set("OPENING".to_string()),
            document_type: Set(None),
            document_id: Set(None),
            document_no: Set(None),
            document_date: Set(None),
            amount: Set(opening_balance),
            matched_amount: Set(None),
            match_status: Set("UNMATCHED".to_string()),
            matched_item_id: Set(None),
            remarks: Set(Some("期初余额".to_string())),
            created_at: Set(Utc::now()),
            updated_at: Set(Utc::now()),
            ..Default::default()
        };

        opening_item.insert(txn)
            .await
            .map_err(|e| AppError::DatabaseError(e.to_string()))?;

        // TODO: 添加销售发票明细和收款明细
        // 待接入实际业务数据

        Ok(())
    }

    /// 自动对账匹配
    ///
    /// 根据对账单ID，自动匹配发票和收款记录
    ///
    /// # 参数
    /// - `reconciliation_id`: 对账单 ID
    ///
    /// # 返回
    /// - `Ok((matched_count, unmatched_count))`: 匹配成功数量和未匹配数量
    /// - `Err(AppError)`: 匹配失败
    pub async fn auto_match(
        &self,
        reconciliation_id: i32,
    ) -> Result<(usize, usize), AppError> {
        // 获取对账单的所有未匹配明细
        let items = ItemEntity::find()
            .filter(crate::models::ar_reconciliation_item::Column::ReconciliationId.eq(reconciliation_id))
            .filter(
                Condition::any()
                    .add(crate::models::ar_reconciliation_item::Column::MatchStatus.eq("UNMATCHED"))
                    .add(crate::models::ar_reconciliation_item::Column::MatchStatus.eq("PARTIAL"))
            )
            .all(&*self.db)
            .await
            .map_err(|e| AppError::DatabaseError(e.to_string()))?;

        let mut matched_count = 0;
        let mut unmatched_count = 0;

        // 分离发票和收款
        let mut invoices: Vec<ItemModel> = Vec::new();
        let mut receipts: Vec<ItemModel> = Vec::new();

        for item in items {
            match item.item_type.as_str() {
                "INVOICE" => invoices.push(item),
                "RECEIPT" => receipts.push(item),
                _ => {}
            }
        }

        // 简单匹配算法：按金额精确匹配
        for invoice in &mut invoices {
            let invoice_amount = invoice.amount;
            let mut remaining = invoice_amount;

            for receipt in &mut receipts {
                if receipt.match_status == "MATCHED" {
                    continue;
                }

                let receipt_amount = receipt.amount.abs(); // 收款金额为负，取绝对值

                if remaining == receipt_amount {
                    // 完全匹配
                    self.update_item_match(invoice.id, receipt.id, invoice_amount).await?;
                    self.update_item_match(receipt.id, invoice.id, receipt_amount).await?;
                    matched_count += 1;
                    remaining = Decimal::zero();
                    break;
                } else if remaining > receipt_amount && !receipt_amount.is_zero() {
                    // 部分匹配：发票金额大于收款金额
                    self.update_item_match(invoice.id, receipt.id, receipt_amount).await?;
                    self.update_item_match(receipt.id, invoice.id, receipt_amount).await?;
                    remaining -= receipt_amount;
                    matched_count += 1;
                }
            }

            if remaining == invoice_amount {
                // 完全没有匹配
                unmatched_count += 1;
            }
        }

        Ok((matched_count, unmatched_count))
    }

    /// 更新明细匹配状态
    async fn update_item_match(
        &self,
        item_id: i32,
        matched_item_id: i32,
        matched_amount: Decimal,
    ) -> Result<(), AppError> {
        let item = ItemEntity::find_by_id(item_id)
            .one(&*self.db)
            .await
            .map_err(|e| AppError::DatabaseError(e.to_string()))?
            .ok_or_else(|| AppError::NotFound("对账明细不存在".to_string()))?;

        let mut active_model: ItemActiveModel = item.into();
        active_model.matched_item_id = Set(Some(matched_item_id));
        active_model.matched_amount = Set(Some(matched_amount));

        // 判断是完全匹配还是部分匹配
        let total_amount = active_model.amount.as_ref().clone();
        if matched_amount == total_amount {
            active_model.match_status = Set("MATCHED".to_string());
        } else {
            active_model.match_status = Set("PARTIAL".to_string());
        }

        active_model.updated_at = Set(Utc::now());

        active_model
            .update(&*self.db)
            .await
            .map_err(|e| AppError::DatabaseError(e.to_string()))?;

        Ok(())
    }

    /// 计算账龄分析
    ///
    /// # 参数
    /// - `customer_id`: 客户 ID（可选，为None则分析所有客户）
    /// - `analysis_date`: 分析日期
    ///
    /// # 返回
    /// - `Ok(Vec<AgingModel>)`: 账龄分析结果列表
    pub async fn calculate_aging(
        &self,
        customer_id: Option<i32>,
        analysis_date: NaiveDate,
    ) -> Result<Vec<AgingModel>, AppError> {
        // TODO: 从应收账款表查询数据并计算账龄
        // 当前返回空列表，待接入实际应收数据
        Ok(Vec::new())
    }
}
