//! 出口商检服务
//! V15 P2 B08-12：出口商检记录 CRUD + 到期预警
use crate::models::export_inspection::{ActiveModel, Column, Entity as Ei, Model};
use crate::models::status::export_inspection_result;
use crate::utils::error::AppError;
use crate::utils::pagination::paginate_with_total;
use sea_orm::*;
use std::sync::Arc;

pub struct ExportInspectionService {
    db: Arc<DatabaseConnection>,
}

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

    /// 建单：新建一张出口商检记录，结论固定落待检态。
    ///
    /// 判据：建单动作不携带结论，result 恒为词表 `export_inspection_result::PENDING`
    /// （唯一写入方在此，杜绝业务码硬编码漂移）；created_by 由调用方（handler）从会话注入，
    /// 请求体不得决定建单人（审计归属不可伪造）。
    /// 调用方：handlers/export_inspection_handler.rs::create_inspection（POST /export-inspections）。
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
            result: Set(export_inspection_result::PENDING.to_string()),
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

    /// 登记商检结论：写入 result 与证书信息（report_url/certificate_no/certificate_expiry）。
    ///
    /// 判据：result 必须逐字符命中词表 `export_inspection_result::ALL`（pending/pass/fail），
    /// 词表外（含大小写/中英变体、空串）一律 400 VALIDATION_ERROR fail-visible 拒绝，
    /// 不夹紧、不兜底、不落库。结论列只能经本动作改写，建单后无其它写入口。
    /// 改判策略取最保守可回退口径：允许在词表值之间重复登记（改判），每次改判记录操作人与
    /// 前后值到日志（不留痕即静默，违反可观测红线）。
    /// 调用方：handlers/export_inspection_handler.rs::update_result（PUT /export-inspections/{id}/result）。
    pub async fn update_result(
        &self,
        id: i32,
        result: String,
        report_url: Option<String>,
        certificate_no: Option<String>,
        certificate_expiry: Option<chrono::NaiveDate>,
    ) -> Result<Model, AppError> {
        if !export_inspection_result::is_valid(&result) {
            let allowed = export_inspection_result::ALL.join(", ");
            return Err(AppError::validation_displayable(format!(
                "商检结论 result 取值非法，允许值：{allowed}"
            )));
        }
        let model = self.get_by_id(id).await?;
        let previous = model.result.clone();
        let mut active: ActiveModel = model.into();
        active.result = Set(result.clone());
        active.report_url = Set(report_url);
        active.certificate_no = Set(certificate_no);
        active.certificate_expiry = Set(certificate_expiry);
        let model = active.update(&*self.db).await?;
        // 结果改判留痕：记录前后值，供审计回溯（本域暂无历史表，改判轨迹以结构化日志承载）。
        tracing::info!(
            inspection_id = model.id,
            previous_result = %previous,
            new_result = %result,
            "出口商检结论已登记/改判"
        );
        Ok(model)
    }

    /// 删除出口商检记录。
    ///
    /// 现状：删除动作尚无路由挂载点，方法体保留待删除入口交付；在此之前不得因无挂载点
    /// 而删除方法本体。调用方：暂无（入口交付后由 handler 的删除端点调用）。
    #[allow(dead_code, reason = "删除动作尚未挂载路由端点，方法体保留待入口交付")]
    pub async fn delete(&self, id: i32) -> Result<(), AppError> {
        let model = self.get_by_id(id).await?;
        model.delete(&*self.db).await?;
        Ok(())
    }
}

pub struct ListParams {
    pub sales_order_id: Option<i32>,
    pub inspection_no: Option<String>,
    pub result: Option<String>,
    pub page: Option<u64>,
    pub page_size: Option<u64>,
}

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
