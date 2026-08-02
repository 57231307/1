//! 出口商检服务
//! V15 P2 B08-12：出口商检记录 CRUD + 到期预警
use crate::models::export_inspection::{ActiveModel, Column, Entity as Ei, Model};
use crate::utils::error::AppError;
use sea_orm::*;
use std::sync::Arc;

#[allow(dead_code)]
pub struct ExportInspectionService {
    db: Arc<DatabaseConnection>,
}

#[allow(dead_code)]
impl ExportInspectionService {
    pub fn new(db: Arc<DatabaseConnection>) -> Self {
        Self { db }
    }

    pub async fn list(&self, params: ListParams) -> Result<(Vec<Model>, u64), AppError> {
        let mut query = Ei::find();
        if let Some(sales_order_id) = params.sales_order_id {
            query = query.filter(Column::SalesOrderId.eq(sales_order_id));
        }
        if let Some(inspection_no) = params.inspection_no {
            query = query.filter(Column::InspectionNo.contains(inspection_no));
        }
        if let Some(result) = params.result {
            query = query.filter(Column::Result.eq(result));
        }
        let paginator = query
            .order_by_desc(Column::CreatedAt)
            .paginate(&*self.db, params.page_size.unwrap_or(20));
        let total = paginator.num_items().await?;
        let items = paginator.fetch_page(params.page.unwrap_or(0)).await?;
        Ok((items, total))
    }

    pub async fn get_by_id(&self, id: i32) -> Result<Model, AppError> {
        Ei::find_by_id(id)
            .one(&*self.db)
            .await?
            .ok_or_else(|| AppError::not_found(format!("出口商检记录 {} 不存在", id)))
    }

    pub async fn create(&self, data: CreateInspectionReq) -> Result<Model, AppError> {
        let active = ActiveModel {
            inspection_no: Set(data.inspection_no),
            sales_order_id: Set(data.sales_order_id),
            delivery_id: Set(data.delivery_id),
            product_name: Set(data.product_name),
            hs_code: Set(data.hs_code),
            inspection_type: Set(data.inspection_type),
            inspection_agency: Set(data.inspection_agency),
            inspection_date: Set(data.inspection_date),
            result: Set("pending".to_string()),
            report_url: Set(None),
            certificate_no: Set(None),
            certificate_expiry: Set(None),
            remarks: Set(data.remarks),
            created_by: Set(data.created_by),
            ..Default::default()
        };
        let model = active.insert(&*self.db).await?;
        Ok(model)
    }

    pub async fn update_result(
        &self,
        id: i32,
        result: String,
        report_url: Option<String>,
        certificate_no: Option<String>,
        certificate_expiry: Option<chrono::NaiveDate>,
    ) -> Result<Model, AppError> {
        let model = self.get_by_id(id).await?;
        let mut active: ActiveModel = model.into();
        active.result = Set(result);
        active.report_url = Set(report_url);
        active.certificate_no = Set(certificate_no);
        active.certificate_expiry = Set(certificate_expiry);
        let model = active.update(&*self.db).await?;
        Ok(model)
    }

    pub async fn delete(&self, id: i32) -> Result<(), AppError> {
        let model = self.get_by_id(id).await?;
        model.delete(&*self.db).await?;
        Ok(())
    }
}

#[allow(dead_code)]
pub struct ListParams {
    pub sales_order_id: Option<i32>,
    pub inspection_no: Option<String>,
    pub result: Option<String>,
    pub page: Option<u64>,
    pub page_size: Option<u64>,
}

#[allow(dead_code)]
pub struct CreateInspectionReq {
    pub inspection_no: String,
    pub sales_order_id: i32,
    pub delivery_id: Option<i32>,
    pub product_name: String,
    pub hs_code: String,
    pub inspection_type: String,
    pub inspection_agency: String,
    pub inspection_date: chrono::NaiveDate,
    pub remarks: Option<String>,
    pub created_by: i32,
}
