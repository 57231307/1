//! 出口产地证服务
//! V15 P2 B08-12：产地证 CRUD + 到期预警

use crate::models::certificate_of_origin::{ActiveModel, Column, Entity as Co, Model};
use crate::utils::error::AppError;
use rust_decimal::Decimal;
use sea_orm::*;
use std::sync::Arc;

pub struct CertificateOfOriginService {
    db: Arc<DatabaseConnection>,
}

impl CertificateOfOriginService {
    pub fn new(db: Arc<DatabaseConnection>) -> Self {
        Self { db }
    }

    pub async fn list(&self, params: ListParams) -> Result<(Vec<Model>, u64), AppError> {
        let mut query = Co::find();
        if let Some(inspection_id) = params.inspection_id {
            query = query.filter(Column::InspectionId.eq(inspection_id));
        }
        if let Some(status) = params.status {
            query = query.filter(Column::Status.eq(status));
        }
        let paginator = query
            .order_by_desc(Column::IssueDate)
            .paginate(&*self.db, params.page_size.unwrap_or(20));
        let total = paginator.num_items().await?;
        let items = paginator.fetch_page(params.page.unwrap_or(0)).await?;
        Ok((items, total))
    }

    pub async fn get_by_id(&self, id: i32) -> Result<Model, AppError> {
        Co::find_by_id(id)
            .one(&*self.db)
            .await?
            .ok_or_else(|| AppError::not_found(format!("产地证 {} 不存在", id)))
    }

    pub async fn create(&self, data: CreateCertificateReq) -> Result<Model, AppError> {
        let active = ActiveModel {
            certificate_no: Set(data.certificate_no),
            inspection_id: Set(data.inspection_id),
            product_name: Set(data.product_name),
            hs_code: Set(data.hs_code),
            origin_country: Set("China".to_string()),
            destination_country: Set(data.destination_country),
            quantity: Set(data.quantity),
            unit: Set(data.unit),
            invoice_amount: Set(data.invoice_amount),
            certificate_type: Set(data.certificate_type),
            issue_date: Set(data.issue_date),
            expiry_date: Set(data.expiry_date),
            status: Set("active".to_string()),
            remarks: Set(data.remarks),
            created_by: Set(data.created_by),
            ..Default::default()
        };
        let model = active.insert(&*self.db).await?;
        Ok(model)
    }

    pub async fn revoke(&self, id: i32) -> Result<Model, AppError> {
        let model = self.get_by_id(id).await?;
        let mut active: ActiveModel = model.into();
        active.status = Set("revoked".to_string());
        let model = active.update(&*self.db).await?;
        Ok(model)
    }
}

pub struct ListParams {
    pub inspection_id: Option<i32>,
    pub status: Option<String>,
    pub page: Option<u64>,
    pub page_size: Option<u64>,
}

pub struct CreateCertificateReq {
    pub certificate_no: String,
    pub inspection_id: Option<i32>,
    pub product_name: String,
    pub hs_code: String,
    pub destination_country: String,
    pub quantity: Decimal,
    pub unit: String,
    pub invoice_amount: Option<Decimal>,
    pub certificate_type: String,
    pub issue_date: chrono::NaiveDate,
    pub expiry_date: Option<chrono::NaiveDate>,
    pub remarks: Option<String>,
    pub created_by: i32,
}