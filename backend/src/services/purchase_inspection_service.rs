//! 采购质检 Service
//! 
//! 采购质检服务层，负责采购质检的核心业务逻辑

use sea_orm::{
    ActiveModelTrait, ColumnTrait, DatabaseConnection, EntityTrait, QueryFilter,
    QueryOrder, TransactionTrait, Set, Order, PaginatorTrait,
};
use std::sync::Arc;
use crate::models::purchase_inspection;
use crate::utils::error::AppError;
use chrono::Utc;
use rust_decimal::Decimal;
use serde::Deserialize;
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

    /// 生质检单号
    /// 格式：IQ + 年月日 + 三位序号（IQ20260315001）
    pub async fn generate_inspection_no(&self) -> Result<String, AppError> {
        let today = Utc::now().format("%Y%m%d").to_string();
        let prefix = format!("IQ{}", today);
        
        let count = purchase_inspection::Entity::find()
            .filter(purchase_inspection::Column::InspectionNo.starts_with(&prefix))
            .count(&*self.db)
            .await?;
        
        Ok(format!("{}{:03}", prefix, count + 1))
    }

    /// 创建采购质检单
    pub async fn create_inspection(
        &self,
        req: CreatePurchaseInspectionRequest,
        user_id: i32,
    ) -> Result<purchase_inspection::Model, AppError> {
        let inspection_no = self.generate_inspection_no().await?;
        
        let inspection = purchase_inspection::ActiveModel {
            inspection_no: Set(inspection_no),
            purchase_order_id: Set(req.order_id),
            supplier_id: Set(req.supplier_id),
            inspection_date: Set(req.inspection_date),
            inspector_id: Set(req.inspector_id),
            result: Set("PENDING".to_string()),
            qualified_quantity: Set(Decimal::ZERO),
            unqualified_quantity: Set(Decimal::ZERO),
            unqualified_reason: Set(req.notes.clone()),
            remarks: Set(req.notes),
            created_by: Set(user_id),
            ..Default::default()
        }.insert(&*self.db).await?;
        
        Ok(inspection)
    }

    /// 更新采购质检单
    pub async fn update_inspection(
        &self,
        inspection_id: i32,
        req: UpdatePurchaseInspectionRequest,
        _user_id: i32,
    ) -> Result<purchase_inspection::Model, AppError> {
        let inspection = purchase_inspection::Entity::find_by_id(inspection_id)
            .one(&*self.db)
            .await?
            .ok_or(AppError::ResourceNotFound(format!("采购质检单 {}", inspection_id)))?;
        
        if inspection.result != "PENDING" {
            return Err(AppError::BusinessError(
                format!("质检单状态不允许修改，当前状态：{}", inspection.result)
            ));
        }
        
        let mut inspection_active: purchase_inspection::ActiveModel = inspection.into();
        
        if let Some(notes) = req.notes {
            // 使用 clone 避免 moved value 错误
            let notes_clone = notes.clone();
            inspection_active.remarks = Set(Some(notes));
            inspection_active.unqualified_reason = Set(Some(notes_clone));
        }
        
        let inspection = inspection_active.update(&*self.db).await?;
        
        Ok(inspection)
    }

    /// 完成采购质检单
    pub async fn complete_inspection(
        &self,
        inspection_id: i32,
        req: CompleteInspectionRequest,
        _user_id: i32,
    ) -> Result<purchase_inspection::Model, AppError> {
        let txn = (&*self.db).begin().await?;
        
        let inspection = purchase_inspection::Entity::find_by_id(inspection_id)
            .one(&txn)
            .await?
            .ok_or(AppError::ResourceNotFound(format!("采购质检单 {}", inspection_id)))?;
        
        if inspection.result != "PENDING" {
            return Err(AppError::BusinessError(
                format!("质检单状态不允许完成，当前状态：{}", inspection.result)
            ));
        }
        
        // 计算质量得分
        let _quality_score = self.calculate_quality_score(
            req.pass_quantity,
            req.reject_quantity,
        ).await?;
        
        let now = Utc::now().naive_utc();
        let mut inspection_active: purchase_inspection::ActiveModel = inspection.into();
        inspection_active.qualified_quantity = Set(req.pass_quantity);
        inspection_active.unqualified_quantity = Set(req.reject_quantity);
        if req.reject_quantity > Decimal::ZERO {
            inspection_active.unqualified_reason = Set(Some(format!("不合格数量：{}", req.reject_quantity)));
        }
        inspection_active.result = Set(req.inspection_result);
        inspection_active.inspection_date = Set(now.date());
        
        let inspection = inspection_active.update(&txn).await?;
        
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
            query = query.filter(purchase_inspection::Column::Result.eq(&status));
        }
        if let Some(supplier_id) = supplier_id {
            query = query.filter(purchase_inspection::Column::SupplierId.eq(supplier_id));
        }
        
        let paginator = query
            .order_by(purchase_inspection::Column::CreatedAt, Order::Desc)
            .paginate(&*self.db, page_size);
        
        let total = paginator.num_items().await?;
        let items = paginator.fetch_page(page - 1).await?;
        
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
            .ok_or(AppError::ResourceNotFound(format!("采购质检单 {}", inspection_id)))?;
        
        Ok(inspection)
    }
}


// =====================================================
// 请求/响应 DTO
// =====================================================

/// 创建采购质检单请求
#[derive(Debug, Validate, Deserialize)]
#[allow(dead_code)]
pub struct CreatePurchaseInspectionRequest {
    /// 入库单 ID
    pub receipt_id: i32,

    /// 采购订单 ID
    pub order_id: Option<i32>,

    /// 供应商 ID
    pub supplier_id: i32,

    /// 质检日期
    pub inspection_date: chrono::NaiveDate,

    /// 质检员 ID
    pub inspector_id: Option<i32>,

    /// 质检类型
    #[allow(dead_code)]
    pub inspection_type: Option<String>,

    /// 备注
    pub notes: Option<String>,
}

/// 更新采购质检单请求
#[derive(Debug, Default, Deserialize)]
#[allow(dead_code)]
pub struct UpdatePurchaseInspectionRequest {
    pub sample_size: Option<i32>,
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
