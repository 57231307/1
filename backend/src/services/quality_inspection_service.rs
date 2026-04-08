#![allow(dead_code, unused_variables, unused_imports, unused_mut)]
use crate::models::quality_inspection;
use crate::models::quality_inspection_record;
use crate::models::unqualified_product;
use crate::utils::error::AppError;
use chrono::NaiveDate;
use rust_decimal::Decimal;
use sea_orm::{
    ActiveModelTrait, ColumnTrait, DatabaseConnection, EntityTrait, Order, PaginatorTrait,
    QueryFilter, QueryOrder, QuerySelect, Set,
};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tracing::info;

#[derive(Debug, Clone, Default)]
pub struct QualityInspectionQueryParams {
    pub inspection_type: Option<String>,
    pub status: Option<String>,
    pub page: i64,
    pub page_size: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateQualityInspectionStandardRequest {
    pub standard_name: String,
    pub standard_code: String,
    pub inspection_type: String,
    pub product_id: Option<i32>,
    pub product_category_id: Option<i32>,
    pub inspection_items: Option<serde_json::Value>,
    pub sampling_method: Option<String>,
    pub sampling_rate: Option<Decimal>,
    pub acceptance_criteria: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateInspectionRecordRequest {
    pub inspection_no: String,
    pub inspection_type: String,
    pub related_type: Option<String>,
    pub related_id: Option<i32>,
    pub product_id: i32,
    pub batch_no: Option<String>,
    pub supplier_id: Option<i32>,
    pub customer_id: Option<i32>,
    pub inspection_date: NaiveDate,
    pub inspector_id: Option<i32>,
    pub total_qty: Decimal,
    pub inspected_qty: Decimal,
    pub qualified_qty: Option<Decimal>,
    pub unqualified_qty: Option<Decimal>,
    pub qualification_rate: Option<Decimal>,
    pub inspection_result: String,
    pub remark: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProcessUnqualifiedRequest {
    pub unqualified_qty: Decimal,
    pub unqualified_reason: String,
    pub handling_method: String,
    pub remark: Option<String>,
}

pub struct QualityInspectionService {
    db: Arc<DatabaseConnection>,
}

impl QualityInspectionService {
    pub fn new(db: Arc<DatabaseConnection>) -> Self {
        Self { db }
    }

    pub async fn get_standards_list(
        &self,
        params: QualityInspectionQueryParams,
    ) -> Result<(Vec<quality_inspection::Model>, u64), AppError> {
        let mut query = quality_inspection::Entity::find();

        if let Some(inspection_type) = &params.inspection_type {
            query = query.filter(quality_inspection::Column::InspectionType.eq(inspection_type));
        }

        if let Some(status) = &params.status {
            query = query.filter(quality_inspection::Column::Status.eq(status));
        }

        let total = query.clone().count(&*self.db).await?;

        let standards = query
            .order_by(quality_inspection::Column::Id, Order::Desc)
            .offset((params.page * params.page_size) as u64)
            .limit(params.page_size as u64)
            .all(&*self.db)
            .await?;

        Ok((standards, total))
    }

    pub async fn create_standard(
        &self,
        req: CreateQualityInspectionStandardRequest,
        user_id: i32,
    ) -> Result<quality_inspection::Model, AppError> {
        info!(
            "用户 {} 正在创建质量检验标准：{}",
            user_id, req.standard_code
        );

        let active_model = quality_inspection::ActiveModel {
            standard_name: Set(req.standard_name),
            standard_code: Set(req.standard_code),
            product_id: Set(req.product_id),
            product_category_id: Set(req.product_category_id),
            inspection_type: Set(req.inspection_type),
            inspection_items: Set(req.inspection_items),
            sampling_method: Set(req.sampling_method),
            sampling_rate: Set(req.sampling_rate),
            acceptance_criteria: Set(req.acceptance_criteria),
            status: Set("active".to_string()),
            ..Default::default()
        };

        let result = active_model.insert(&*self.db).await?;
        info!("质量检验标准创建成功：{}", result.standard_code);
        Ok(result)
    }

    pub async fn get_standard_by_id(&self, id: i32) -> Result<quality_inspection::Model, AppError> {
        let standard = quality_inspection::Entity::find_by_id(id)
            .one(&*self.db)
            .await?
            .ok_or_else(|| AppError::NotFound(format!("质量检验标准不存在：{}", id)))?;
        Ok(standard)
    }

    pub async fn get_record_by_id(
        &self,
        id: i32,
    ) -> Result<quality_inspection_record::Model, AppError> {
        let record = quality_inspection_record::Entity::find_by_id(id)
            .one(&*self.db)
            .await?
            .ok_or_else(|| AppError::NotFound(format!("质量检验记录不存在：{}", id)))?;
        Ok(record)
    }

    pub async fn get_records_list(
        &self,
        params: QualityInspectionQueryParams,
    ) -> Result<(Vec<quality_inspection_record::Model>, u64), AppError> {
        let mut query = quality_inspection_record::Entity::find();

        if let Some(inspection_result) = &params.inspection_type {
            query = query
                .filter(quality_inspection_record::Column::InspectionResult.eq(inspection_result));
        }

        let total = query.clone().count(&*self.db).await?;

        let records = query
            .order_by(quality_inspection_record::Column::Id, Order::Desc)
            .offset((params.page * params.page_size) as u64)
            .limit(params.page_size as u64)
            .all(&*self.db)
            .await?;

        Ok((records, total))
    }

    pub async fn create_record(
        &self,
        req: CreateInspectionRecordRequest,
        user_id: i32,
    ) -> Result<quality_inspection_record::Model, AppError> {
        info!(
            "用户 {} 正在创建质量检验记录：{}",
            user_id, req.inspection_no
        );

        let active_model = quality_inspection_record::ActiveModel {
            inspection_no: Set(req.inspection_no),
            inspection_type: Set(req.inspection_type),
            related_type: Set(req.related_type),
            related_id: Set(req.related_id),
            product_id: Set(req.product_id),
            batch_no: Set(req.batch_no),
            supplier_id: Set(req.supplier_id),
            customer_id: Set(req.customer_id),
            inspection_date: Set(req.inspection_date),
            inspector_id: Set(req.inspector_id),
            total_qty: Set(req.total_qty),
            inspected_qty: Set(req.inspected_qty),
            qualified_qty: Set(req.qualified_qty),
            unqualified_qty: Set(req.unqualified_qty),
            qualification_rate: Set(req.qualification_rate),
            inspection_result: Set(req.inspection_result),
            remark: Set(req.remark),
            ..Default::default()
        };

        let result = active_model.insert(&*self.db).await?;
        info!("质量检验记录创建成功：{}", result.inspection_no);

        // 如果是采购入库的质检，同步更新入库单状态
        if result.related_type.as_deref() == Some("PURCHASE_RECEIPT") {
            if let Some(receipt_id) = result.related_id {
                let receipt = crate::models::purchase_receipt::Entity::find_by_id(receipt_id)
                    .one(&*self.db)
                    .await?;

                if let Some(r) = receipt {
                    let mut receipt_active: crate::models::purchase_receipt::ActiveModel = r.into();
                    receipt_active.inspection_status = Set(result.inspection_result.clone());
                    receipt_active.update(&*self.db).await?;
                }
            }
        }

        Ok(result)
    }

    pub async fn process_unqualified(
        &self,
        record_id: i32,
        req: ProcessUnqualifiedRequest,
        user_id: i32,
    ) -> Result<unqualified_product::Model, AppError> {
        info!("用户 {} 正在处理不合格品，记录ID：{}", user_id, record_id);

        let record = self.get_record_by_id(record_id).await?;

        let unqualified_no = format!("UQ{:08}", record_id);

        let active_model = unqualified_product::ActiveModel {
            unqualified_no: Set(unqualified_no),
            inspection_id: Set(Some(record_id)),
            product_id: Set(record.product_id),
            batch_no: Set(record.batch_no),
            unqualified_qty: Set(req.unqualified_qty),
            unqualified_reason: Set(req.unqualified_reason),
            handling_method: Set(req.handling_method),
            handling_status: Set("pending".to_string()),
            handling_by: Set(None),
            handling_at: Set(None),
            remark: Set(req.remark),
            ..Default::default()
        };

        let result = active_model.insert(&*self.db).await?;
        info!("不合格品处理记录创建成功：{}", result.unqualified_no);
        Ok(result)
    }

    pub async fn get_defects_list(
        &self,
        params: QualityInspectionQueryParams,
    ) -> Result<(Vec<unqualified_product::Model>, u64), AppError> {
        let mut query = unqualified_product::Entity::find();

        if let Some(status) = &params.status {
            query = query.filter(unqualified_product::Column::HandlingStatus.eq(status));
        }

        let total = query.clone().count(&*self.db).await?;

        let defects = query
            .order_by(unqualified_product::Column::Id, Order::Desc)
            .offset((params.page * params.page_size) as u64)
            .limit(params.page_size as u64)
            .all(&*self.db)
            .await?;

        Ok((defects, total))
    }

    pub async fn update_unqualified_status(
        &self,
        id: i32,
        handling_status: &str,
        handler_id: i32,
    ) -> Result<unqualified_product::Model, AppError> {
        let mut unqualified: unqualified_product::ActiveModel =
            unqualified_product::Entity::find_by_id(id)
                .one(&*self.db)
                .await?
                .ok_or_else(|| AppError::NotFound(format!("不合格品记录不存在：{}", id)))?
                .into();

        unqualified.handling_status = Set(handling_status.to_string());
        unqualified.handling_by = Set(Some(handler_id));
        unqualified.handling_at = Set(Some(chrono::Utc::now()));

        let result = unqualified.update(&*self.db).await?;
        info!("不合格品处理状态更新成功：{}", result.unqualified_no);
        Ok(result)
    }
}
