//! 采购退货 Service
//!
//! 采购退货服务层，负责采购退货的核心业务逻辑

use crate::models::purchase_return;
use crate::utils::error::AppError;
use chrono::Utc;
use rust_decimal::Decimal;
use sea_orm::{
    ActiveModelTrait, ColumnTrait, DatabaseConnection, EntityTrait, Order, PaginatorTrait,
    QueryFilter, QueryOrder, Set, TransactionTrait,
};
use serde::Deserialize;
use std::sync::Arc;
use validator::Validate;

/// 采购退货服务
pub struct PurchaseReturnService {
    db: Arc<DatabaseConnection>,
}

impl PurchaseReturnService {
    /// 创建服务实例
    pub fn new(db: Arc<DatabaseConnection>) -> Self {
        Self { db }
    }

    /// 生成退货单号
    /// 格式：RT + 年月日 + 三位序号（RT20260315001）
    pub async fn generate_return_no(&self) -> Result<String, AppError> {
        let today = Utc::now().format("%Y%m%d").to_string();
        let prefix = format!("RT{}", today);

        let count: u64 = purchase_return::Entity::find()
            .filter(purchase_return::Column::ReturnNo.starts_with(&prefix))
            .count(&*self.db)
            .await?;

        Ok(format!("{}{:03}", prefix, count + 1))
    }

    /// 创建采购退货单
    pub async fn create_return(
        &self,
        req: CreatePurchaseReturnRequest,
        user_id: i32,
    ) -> Result<purchase_return::Model, AppError> {
        let txn = (*self.db).begin().await?;

        let return_no = self.generate_return_no().await?;

        // 将 reason_type 和 reason_detail 组合成 reason 字段
        let reason = if let Some(detail) = &req.reason_detail {
            format!("{}: {}", req.reason_type, detail)
        } else {
            req.reason_type.clone()
        };

        let return_order = purchase_return::ActiveModel {
            return_no: Set(return_no),
            purchase_order_id: Set(req.order_id),
            supplier_id: Set(req.supplier_id),
            return_date: Set(req.return_date),
            warehouse_id: Set(req.warehouse_id),
            reason: Set(reason),
            status: Set("DRAFT".to_string()),
            total_amount: Set(Decimal::ZERO),
            remarks: Set(req.notes),
            created_by: Set(user_id),
            ..Default::default()
        }
        .insert(&txn)
        .await?;

        txn.commit().await?;

        Ok(return_order)
    }

    /// 更新采购退货单
    pub async fn update_return(
        &self,
        return_id: i32,
        req: UpdatePurchaseReturnRequest,
        _user_id: i32,
    ) -> Result<purchase_return::Model, AppError> {
        let return_order = purchase_return::Entity::find_by_id(return_id)
            .one(&*self.db)
            .await?
            .ok_or(AppError::ResourceNotFound(format!(
                "采购退货单 {}",
                return_id
            )))?;

        if return_order.status != "DRAFT" {
            return Err(AppError::BusinessError(format!(
                "退货单状态不允许修改，当前状态：{}",
                return_order.status
            )));
        }

        // 保留 reason 字段的引用，避免 moved value
        let reason_value = return_order.reason.clone();

        let mut return_active: purchase_return::ActiveModel = return_order.into();

        if let Some(reason_detail) = req.reason_detail {
            // 更新 reason 字段，保留原有的 reason_type
            let current_reason = &reason_value;
            let reason_type = current_reason.split(':').next().unwrap_or("其他");
            return_active.reason = Set(format!("{}: {}", reason_type, reason_detail));
        }
        if let Some(notes) = req.notes {
            return_active.remarks = Set(Some(notes));
        }

        let return_order = return_active.update(&*self.db).await?;

        Ok(return_order)
    }

    /// 提交采购退货单
    pub async fn submit_return(
        &self,
        return_id: i32,
        _user_id: i32,
    ) -> Result<purchase_return::Model, AppError> {
        let return_order = purchase_return::Entity::find_by_id(return_id)
            .one(&*self.db)
            .await?
            .ok_or(AppError::ResourceNotFound(format!(
                "采购退货单 {}",
                return_id
            )))?;

        if return_order.status != "DRAFT" {
            return Err(AppError::BusinessError(format!(
                "退货单状态不允许提交，当前状态：{}",
                return_order.status
            )));
        }

        let mut return_active: purchase_return::ActiveModel = return_order.into();
        return_active.status = Set("SUBMITTED".to_string());

        let return_order = return_active.update(&*self.db).await?;

        Ok(return_order)
    }

    /// 审批采购退货单
    pub async fn approve_return(
        &self,
        return_id: i32,
        _user_id: i32,
    ) -> Result<purchase_return::Model, AppError> {
        let return_order = purchase_return::Entity::find_by_id(return_id)
            .one(&*self.db)
            .await?
            .ok_or(AppError::ResourceNotFound(format!(
                "采购退货单 {}",
                return_id
            )))?;

        if return_order.status != "SUBMITTED" {
            return Err(AppError::BusinessError(format!(
                "退货单状态不允许审批，当前状态：{}",
                return_order.status
            )));
        }

        let mut return_active: purchase_return::ActiveModel = return_order.into();
        return_active.status = Set("APPROVED".to_string());

        let return_order = return_active.update(&*self.db).await?;

        Ok(return_order)
    }

    /// 拒绝采购退货单
    pub async fn reject_return(
        &self,
        return_id: i32,
        reason: String,
        _user_id: i32,
    ) -> Result<purchase_return::Model, AppError> {
        let return_order = purchase_return::Entity::find_by_id(return_id)
            .one(&*self.db)
            .await?
            .ok_or(AppError::ResourceNotFound(format!(
                "采购退货单 {}",
                return_id
            )))?;

        if return_order.status != "SUBMITTED" {
            return Err(AppError::BusinessError(format!(
                "退货单状态不允许拒绝，当前状态：{}",
                return_order.status
            )));
        }

        let mut return_active: purchase_return::ActiveModel = return_order.into();
        return_active.status = Set("REJECTED".to_string());
        return_active.reason = Set(reason);

        let return_order = return_active.update(&*self.db).await?;

        Ok(return_order)
    }

    /// 获取退货单列表
    pub async fn list_returns(
        &self,
        page: u64,
        page_size: u64,
        status: Option<String>,
        supplier_id: Option<i32>,
    ) -> Result<(Vec<purchase_return::Model>, u64), AppError> {
        use sea_orm::PaginatorTrait;

        let mut query = purchase_return::Entity::find();

        if let Some(status) = status {
            query = query.filter(purchase_return::Column::Status.eq(&status));
        }
        if let Some(supplier_id) = supplier_id {
            query = query.filter(purchase_return::Column::SupplierId.eq(supplier_id));
        }

        let paginator = query
            .order_by(purchase_return::Column::CreatedAt, Order::Desc)
            .paginate(&*self.db, page_size);

        let total = paginator.num_items().await?;
        let items = paginator.fetch_page(page - 1).await?;

        Ok((items, total))
    }

    /// 获取退货单详情
    pub async fn get_return(&self, return_id: i32) -> Result<purchase_return::Model, AppError> {
        let return_order = purchase_return::Entity::find_by_id(return_id)
            .one(&*self.db)
            .await?
            .ok_or(AppError::ResourceNotFound(format!(
                "采购退货单 {}",
                return_id
            )))?;

        Ok(return_order)
    }
}

// =====================================================
// 请求/响应 DTO
// =====================================================

/// 创建采购退货单请求
#[derive(Debug, Validate, Deserialize)]
pub struct CreatePurchaseReturnRequest {
    /// 采购订单 ID
    pub order_id: Option<i32>,

    /// 供应商 ID
    pub supplier_id: i32,

    /// 退货日期
    pub return_date: chrono::NaiveDate,

    /// 仓库 ID
    pub warehouse_id: i32,

    /// 退货原因类型
    pub reason_type: String,

    /// 退货原因详情
    pub reason_detail: Option<String>,

    /// 备注
    pub notes: Option<String>,
}

/// 更新采购退货单请求
#[derive(Debug, Default, Deserialize)]
pub struct UpdatePurchaseReturnRequest {
    pub reason_detail: Option<String>,
    pub notes: Option<String>,
}
