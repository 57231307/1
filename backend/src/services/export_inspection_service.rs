//! 出口商检服务
//! V15 P2 B08-12：出口商检记录 CRUD + 到期预警
use crate::models::export_inspection::{ActiveModel, Column, Entity as Ei, Model};
use crate::utils::error::AppError;
use crate::utils::pagination::paginate_with_total;
use sea_orm::*;
use std::sync::Arc;

#[allow(dead_code, reason = "预留")]
pub struct ExportInspectionService {
    db: Arc<DatabaseConnection>,
}

#[allow(dead_code, reason = "预留")]
impl ExportInspectionService {
    pub fn new(db: Arc<DatabaseConnection>) -> Self {
        Self { db }
    }

    /// 查询出口商检单列表（分页）。
    ///
    /// 功能：按销售订单/商检单号/商检结果过滤，返回当前页行集与命中总数。
    /// 调用方：handlers/export_inspection_handler.rs::list_inspections（GET /export-inspections）。
    /// 入参：params.page 为 1-based 页码（缺省第 1 页）；params.page_size 为每页行数
    ///       （缺省 20，合法范围 1-100）；page=0 或 page_size 越界按 400
    ///       VALIDATION_ERROR fail-visible 拒绝，不静默夹紧。
    /// 传给谁：SeaORM 分页器交 utils::pagination::paginate_with_total（本仓分页偏移
    ///       唯一权威，内部已做 1-based→0-based 转换，调用方不得再自行减 1）。
    /// 存什么·存哪里：只读查询，不落任何数据。
    pub async fn list(&self, params: ListParams) -> Result<(Vec<Model>, u64), AppError> {
        // 页码语义=1-based，与 handler 回显 PaginatedResponse.page 的取值一致；
        // 缺省即第 1 页，禁止把缺省写成 0（0-based 偏移会让第一页跳过首行）。
        let page = params.page.unwrap_or(1);
        // 每页上限 100 是全仓统一分页边界（既有站点 clamp(1,100) 的同一取值），
        // 本站点区别仅在处置方式：越界回 400 并点名允许值，而非静默夹紧。
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
            .paginate(&*self.db, page_size);
        let (items, total) = paginate_with_total(paginator, page).await?;
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

#[allow(dead_code, reason = "预留")]
pub struct ListParams {
    pub sales_order_id: Option<i32>,
    pub inspection_no: Option<String>,
    pub result: Option<String>,
    pub page: Option<u64>,
    pub page_size: Option<u64>,
}

#[allow(dead_code, reason = "预留")]
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
