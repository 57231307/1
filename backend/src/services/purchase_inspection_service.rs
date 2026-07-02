//! 采购质检 Service
//!
//! 采购质检服务层，负责采购质检的核心业务逻辑

use crate::models::purchase_inspection;
use crate::utils::error::AppError;
use chrono::Utc;
use rust_decimal::Decimal;
use sea_orm::{
    ActiveModelTrait, ColumnTrait, DatabaseConnection, EntityTrait, Order, QueryFilter, QueryOrder,
    QuerySelect, Set, TransactionTrait,
};
use serde::Deserialize;
use std::sync::Arc;
use validator::Validate;

/// 采购质检服务
pub struct PurchaseInspectionService {
    db: Arc<DatabaseConnection>,
}

impl PurchaseInspectionService {
    /// 创建服务实例
    pub fn new(db: Arc<DatabaseConnection>) -> Self {
        Self { db }
    }

    // 生成质检单号
    // 格式：IQ + 年月日 + 三位序号（IQ20260315001）
    crate::impl_generate_no!(
        generate_inspection_no,
        "PI",
        purchase_inspection::Entity,
        purchase_inspection::Column::InspectionNo
    );

    /// 创建采购质检单
    pub async fn create_inspection(
        &self,
        req: CreatePurchaseInspectionRequest,
        _user_id: i32,
    ) -> Result<purchase_inspection::Model, AppError> {
        let inspection_no = self.generate_inspection_no().await?;

        let inspection = purchase_inspection::ActiveModel {
            id: Set(0),
            inspection_no: Set(inspection_no),
            receipt_id: Set(req.receipt_id),
            order_id: Set(req.order_id),
            // 供应商 ID 缺失时拒绝创建，避免脏 supplier_id=0 记录
            supplier_id: Set(req
                .supplier_id
                .ok_or_else(|| AppError::validation("采购验收单缺少供应商ID"))?),
            inspection_date: Set(req
                .inspection_date
                .unwrap_or_else(|| Utc::now().date_naive())),
            inspector_id: Set(req.inspector_id),
            inspection_type: Set(req.inspection_type),
            sample_size: Set(req.sample_size),
            defect_count: Set(Some(0)),
            pass_quantity: Set(None),
            reject_quantity: Set(None),
            inspection_status: Set(Some("pending".to_string())),
            inspection_result: Set(None),
            quality_score: Set(None),
            defect_description: Set(None),
            attachment_urls: Set(None),
            notes: Set(req.notes),
            completed_at: Set(None),
            completed_by: Set(None),
            created_at: Set(Utc::now()),
            updated_at: Set(Utc::now()),
        };

        let inspection = inspection.insert(&*self.db).await?;

        Ok(inspection)
    }

    /// 更新采购质检单
    pub async fn update_inspection(
        &self,
        inspection_id: i32,
        req: UpdatePurchaseInspectionRequest,
        user_id: i32,
    ) -> Result<purchase_inspection::Model, AppError> {
        let inspection = purchase_inspection::Entity::find_by_id(inspection_id)
            .one(&*self.db)
            .await?
            .ok_or_else(|| AppError::not_found(format!("采购质检单 {}", inspection_id)))?;

        if inspection.inspection_status.as_deref() != Some("pending") {
            return Err(AppError::business(format!(
                "质检单状态不允许修改，当前状态：{:?}",
                inspection.inspection_status
            )));
        }

        let mut inspection_active: purchase_inspection::ActiveModel = inspection.into();

        if let Some(sample_size) = req.sample_size {
            inspection_active.sample_size = Set(Some(sample_size));
        }
        if let Some(defect_description) = req.defect_description {
            inspection_active.defect_description = Set(Some(defect_description));
        }
        if let Some(notes) = req.notes {
            inspection_active.notes = Set(Some(notes));
        }
        inspection_active.updated_at = Set(Utc::now());

        let inspection = crate::services::audit_log_service::AuditLogService::update_with_audit(
            &*self.db,
            "auto_audit",
            inspection_active,
            // P1 1-1 修复（批次 59b）：原 Some(0) 占位符改为真实操作人 user_id
            Some(user_id),
        )
        .await?;

        Ok(inspection)
    }

    /// 完成采购质检单
    pub async fn complete_inspection(
        &self,
        inspection_id: i32,
        req: CompleteInspectionRequest,
        user_id: i32,
    ) -> Result<purchase_inspection::Model, AppError> {
        let txn = (*self.db).begin().await?;

        // 批次 26 v6 P1 修复：状态机 lock_exclusive 补全，串行化并发状态变更
        let inspection = purchase_inspection::Entity::find_by_id(inspection_id)
            .lock_exclusive()
            .one(&txn)
            .await?
            .ok_or_else(|| AppError::not_found(format!("采购质检单 {}", inspection_id)))?;

        if inspection.inspection_status.as_deref() != Some("pending") {
            return Err(AppError::business(format!(
                "质检单状态不允许完成，当前状态：{:?}",
                inspection.inspection_status
            )));
        }

        // 计算质量得分
        let quality_score = self
            .calculate_quality_score(req.pass_quantity, req.reject_quantity)
            .await?;

        let mut inspection_active: purchase_inspection::ActiveModel = inspection.into();
        inspection_active.pass_quantity = Set(Some(req.pass_quantity));
        inspection_active.reject_quantity = Set(Some(req.reject_quantity));
        inspection_active.inspection_result = Set(Some(req.inspection_result));
        inspection_active.inspection_status = Set(Some("completed".to_string()));
        inspection_active.quality_score = Set(Some(quality_score));
        inspection_active.updated_at = Set(Utc::now());

        let inspection = crate::services::audit_log_service::AuditLogService::update_with_audit(
            &txn,
            "auto_audit",
            inspection_active,
            // P1 1-1 修复（批次 59b）：原 Some(0) 占位符改为真实操作人 user_id
            Some(user_id),
        )
        .await?;

        txn.commit().await?;

        Ok(inspection)
    }

    /// 计算质量得分
    pub async fn calculate_quality_score(
        &self,
        pass_quantity: Decimal,
        reject_quantity: Decimal,
    ) -> Result<Decimal, AppError> {
        let total = pass_quantity + reject_quantity;

        if total == Decimal::new(0, 0) {
            return Ok(Decimal::new(0, 2));
        }

        let score = (pass_quantity / total) * Decimal::new(100, 0);

        Ok(score)
    }

    /// 获取质检单列表
    pub async fn list_inspections(
        &self,
        page: u64,
        page_size: u64,
        status: Option<String>,
        supplier_id: Option<i32>,
    ) -> Result<(Vec<purchase_inspection::Model>, u64), AppError> {
        use sea_orm::PaginatorTrait;

        let mut query = purchase_inspection::Entity::find();

        if let Some(status) = status {
            query = query.filter(purchase_inspection::Column::InspectionStatus.eq(&status));
        }
        if let Some(supplier_id) = supplier_id {
            query = query.filter(purchase_inspection::Column::SupplierId.eq(supplier_id));
        }

        let paginator = query
            .order_by(purchase_inspection::Column::CreatedAt, Order::Desc)
            .paginate(&*self.db, page_size);

        let total = paginator.num_items().await?;
        let items = paginator.fetch_page(page.saturating_sub(1)).await?;

        Ok((items, total))
    }

    /// 获取质检单详情
    pub async fn get_inspection(
        &self,
        inspection_id: i32,
    ) -> Result<purchase_inspection::Model, AppError> {
        let inspection = purchase_inspection::Entity::find_by_id(inspection_id)
            .one(&*self.db)
            .await?
            .ok_or_else(|| AppError::not_found(format!("采购质检单 {}", inspection_id)))?;

        Ok(inspection)
    }
}

// =====================================================
// 请求/响应 DTO
// =====================================================

/// 创建采购质检单请求
#[derive(Debug, Validate, Deserialize)]
pub struct CreatePurchaseInspectionRequest {
    /// 入库单 ID
    pub receipt_id: Option<i32>,

    /// 采购订单 ID
    pub order_id: Option<i32>,

    /// 供应商 ID
    pub supplier_id: Option<i32>,

    /// 质检日期
    pub inspection_date: Option<chrono::NaiveDate>,

    /// 质检员 ID
    pub inspector_id: Option<i32>,

    /// 质检类型
    pub inspection_type: Option<String>,

    /// 样品大小
    pub sample_size: Option<Decimal>,

    /// 备注
    pub notes: Option<String>,
}

/// 更新采购质检单请求
#[derive(Debug, Default, Deserialize)]
pub struct UpdatePurchaseInspectionRequest {
    pub sample_size: Option<Decimal>,
    pub defect_description: Option<String>,
    pub notes: Option<String>,
}

/// 完成质检单请求
#[derive(Debug, Validate, Deserialize)]
pub struct CompleteInspectionRequest {
    /// 合格数量
    pub pass_quantity: Decimal,

    /// 不合格数量
    pub reject_quantity: Decimal,

    /// 质检结果
    pub inspection_result: String,
}
