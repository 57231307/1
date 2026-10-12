//! 出口商检处理器
//! V15 P2 B08-12
use crate::container::AppState;
use crate::middleware::auth_context::AuthContext;
use crate::services::export_inspection_service::{
    CreateInspectionReq, ExportInspectionService, ListParams,
};
use crate::utils::error::AppError;
use crate::utils::response::{ApiResponse, PaginatedResponse};
use axum::{
    Json,
    extract::{Path, Query, State},
};
use serde::Deserialize;

/// 出口商检单列表（分页）。
///
/// 出参遵循全站统一成功信封 `ApiResponse`，分页载荷为 `PaginatedResponse{items,total,page,page_size}`
/// （与 handlers/purchase_inspection_handler.rs 同形，本仓仅此一种分页信封）。
/// `page`/`page_size` 回显 handler 收到的分页入参（缺省 page=1/page_size=20，与 service 内部取值一致）。
pub async fn list_inspections(
    State(state): State<AppState>,
    Query(params): Query<ListQuery>,
) -> Result<Json<ApiResponse<serde_json::Value>>, AppError> {
    let service = ExportInspectionService::new(state.db.clone());
    let page = params.page.unwrap_or(1);
    let page_size = params.page_size.unwrap_or(20);
    let (items, total) = service
        .list(ListParams {
            sales_order_id: params.sales_order_id,
            inspection_no: params.inspection_no,
            result: params.result,
            page: params.page,
            page_size: params.page_size,
        })
        .await?;
    let data = serde_json::to_value(PaginatedResponse::new(items, total, page, page_size))?;
    Ok(Json(ApiResponse::success(data)))
}

/// 出口商检单详情。
///
/// 出参遵循全站统一成功信封 `ApiResponse`，data 为 `export_inspection::Model` 序列化对象。
pub async fn get_inspection(
    State(state): State<AppState>,
    Path(id): Path<i32>,
) -> Result<Json<ApiResponse<serde_json::Value>>, AppError> {
    let service = ExportInspectionService::new(state.db.clone());
    let item = service.get_by_id(id).await?;
    Ok(Json(ApiResponse::success(serde_json::to_value(item)?)))
}

/// 建单：新建一张出口商检记录（结论落待检态）。
///
/// 判据：请求体不接受 result（由 service 恒落 pending）与 created_by（建单人取会话，
/// 审计归属不可伪造）；created_by 取 `AuthContext.user_id` 注入 service。
/// 调用方：routes/export_inspection_routes.rs（POST /export-inspections）。
#[axum::debug_handler]
pub async fn create_inspection(
    State(state): State<AppState>,
    auth: AuthContext,
    Json(body): Json<CreateInspectionBody>,
) -> Result<Json<ApiResponse<serde_json::Value>>, AppError> {
    let service = ExportInspectionService::new(state.db.clone());
    let req = CreateInspectionReq {
        inspection_no: body.inspection_no,
        sales_order_id: body.sales_order_id,
        delivery_id: body.delivery_id,
        product_name: body.product_name,
        hs_code: body.hs_code,
        inspection_type: body.inspection_type,
        inspection_agency: body.inspection_agency,
        inspection_date: body.inspection_date,
        remarks: body.remarks,
        created_by: auth.user_id,
    };
    let model = service.create(req).await?;
    tracing::info!(
        inspection_id = model.id,
        operator = auth.user_id,
        "出口商检单已创建"
    );
    Ok(Json(ApiResponse::success(serde_json::to_value(model)?)))
}

/// 登记商检结论：写入 result 与证书信息。
///
/// 判据：result 取值合法性由 service 依据词表 `export_inspection_result::ALL` 强校验，
/// 词表外一律 400 VALIDATION_ERROR，不在 handler 夹紧或兜底。
/// 调用方：routes/export_inspection_routes.rs（PUT /export-inspections/{id}/result）。
#[axum::debug_handler]
pub async fn update_result(
    State(state): State<AppState>,
    auth: AuthContext,
    Path(id): Path<i32>,
    Json(body): Json<UpdateResultBody>,
) -> Result<Json<ApiResponse<serde_json::Value>>, AppError> {
    let service = ExportInspectionService::new(state.db.clone());
    let model = service
        .update_result(
            id,
            body.result,
            body.report_url,
            body.certificate_no,
            body.certificate_expiry,
        )
        .await?;
    tracing::info!(
        inspection_id = model.id,
        operator = auth.user_id,
        "出口商检结论登记请求已完成"
    );
    Ok(Json(ApiResponse::success(serde_json::to_value(model)?)))
}

#[allow(dead_code, reason = "反序列化输入字段")]
#[derive(Deserialize)]
pub struct ListQuery {
    sales_order_id: Option<i32>,
    inspection_no: Option<String>,
    result: Option<String>,
    page: Option<u64>,
    page_size: Option<u64>,
}

/// 建单请求体：不含 result（service 恒落 pending）与 created_by（取会话）。
#[allow(dead_code, reason = "反序列化输入字段")]
#[derive(Deserialize)]
pub struct CreateInspectionBody {
    inspection_no: String,
    sales_order_id: i32,
    delivery_id: Option<i32>,
    product_name: String,
    hs_code: String,
    inspection_type: String,
    inspection_agency: String,
    inspection_date: chrono::NaiveDate,
    remarks: Option<String>,
}

/// 登记结果请求体：result 必填，证书信息可选。
#[allow(dead_code, reason = "反序列化输入字段")]
#[derive(Deserialize)]
pub struct UpdateResultBody {
    result: String,
    report_url: Option<String>,
    certificate_no: Option<String>,
    certificate_expiry: Option<chrono::NaiveDate>,
}
