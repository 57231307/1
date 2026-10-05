//! 出口产地证服务
//! V15 P2 B08-12：产地证 CRUD + 到期预警
use crate::models::certificate_of_origin::{ActiveModel, Column, Entity as Co, Model};
use crate::utils::error::AppError;
use crate::utils::pagination::paginate_with_total;
use rust_decimal::Decimal;
use sea_orm::*;
use std::sync::Arc;

#[allow(dead_code, reason = "预留")]
pub struct CertificateOfOriginService {
    db: Arc<DatabaseConnection>,
}

#[allow(dead_code, reason = "预留")]
impl CertificateOfOriginService {
    pub fn new(db: Arc<DatabaseConnection>) -> Self {
        Self { db }
    }

    /// 查询产地证列表（分页）。
    ///
    /// 功能：按商检单/状态过滤，返回当前页行集与命中总数（按签发日期倒序）。
    /// 调用方：handlers/certificate_of_origin_handler.rs::list_certificates。
    /// 入参：params.page 为 1-based 页码（缺省第 1 页）；params.page_size 为每页行数
    ///       （缺省 20，合法范围 1-100）；page=0 或 page_size 越界按 400
    ///       VALIDATION_ERROR fail-visible 拒绝，不静默夹紧。
    /// 传给谁：SeaORM 分页器交 utils::pagination::paginate_with_total（本仓分页偏移
    ///       唯一权威，内部已做 1-based→0-based 转换，调用方不得再自行减 1）。
    /// 存什么·存哪里：只读查询，不落任何数据。
    pub async fn list(&self, params: ListParams) -> Result<(Vec<Model>, u64), AppError> {
        // 页码语义=1-based（与全站 PaginatedResponse.page 回显口径一致），缺省即第 1 页。
        let page = params.page.unwrap_or(1);
        // 每页上限 100 是全仓统一分页边界（既有站点 clamp(1,100) 的同一取值），
        // 越界处置按本仓口径回 400 点名允许值，不静默夹紧。
        let page_size = params.page_size.unwrap_or(20);
        if page == 0 {
            return Err(AppError::validation_displayable(
                "page 必须是从 1 开始的页码，第一页请传 page=1",
            ));
        }
        if page_size == 0 || page_size > 100 {
            return Err(AppError::validation_displayable(
                "page_size 必须在 1-100 之间（每页返回的行数）",
            ));
        }
        let mut query = Co::find();
        if let Some(inspection_id) = params.inspection_id {
            query = query.filter(Column::InspectionId.eq(inspection_id));
        }
        if let Some(status) = params.status {
            query = query.filter(Column::Status.eq(status));
        }
        let paginator = query
            .order_by_desc(Column::IssueDate)
            .paginate(&*self.db, page_size);
        let (items, total) = paginate_with_total(paginator, page).await?;
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

#[allow(dead_code, reason = "预留")]
pub struct ListParams {
    pub inspection_id: Option<i32>,
    pub status: Option<String>,
    pub page: Option<u64>,
    pub page_size: Option<u64>,
}

#[allow(dead_code, reason = "预留")]
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
