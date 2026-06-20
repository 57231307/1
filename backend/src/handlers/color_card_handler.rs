//! 色卡仓储管理 Handler
//!
//! 实现 13 个 HTTP 端点：色卡 CRUD + 色号 CRUD + 借出/归还/遗失/扫码/导入/导出
//! 设计依据：docs/superpowers/specs/2026-06-16-color-card-design.md §4.2
//! 创建时间: 2026-06-17

use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    response::IntoResponse,
    Json,
};
use serde_json::json;

use crate::middleware::auth_context::AuthContext;
use crate::middleware::tenant::extract_tenant_id;
use crate::models::color_card_borrow_dto::{
    BorrowColorCardDto, ListBorrowRecordsQuery, ListColorCardsQuery,
    MarkDamagedColorCardDto, MarkLostColorCardDto, ReturnColorCardDto,
};
use crate::models::color_card_borrow_record;
use crate::models::color_card_create_dto::{
    ArchiveColorCardDto, CreateColorCardDto, UpdateColorCardDto,
};
use crate::models::color_card_item;
use crate::models::color_card_item_dto::{BatchImportItemsDto, ColorItemDto};
use crate::models::color_card_response_dto::{
    BorrowRecordInfo, ColorCardDetail, ColorCardListItem, ColorItemInfo, PagedResponse,
};
use crate::services::color_card_borrow_service::{
    BorrowError, ColorCardBorrowService,
};
use crate::services::color_card_crud_service::{ColorCardCrudService, CrudError};
use crate::services::color_card_item_service::{ColorCardItemService, ItemError};
use crate::services::color_card_scan_service::ColorCardScanService;
use crate::utils::app_state::AppState;
use crate::utils::error::AppError;
use crate::utils::response::ApiResponse;

// ----------------------------------------------------------------------
// 错误转换辅助
// ----------------------------------------------------------------------

fn crud_err(e: CrudError) -> AppError {
    match e {
        CrudError::NotFound => AppError::not_found("色卡不存在"),
        CrudError::InvalidState => AppError::business("当前状态不允许此操作"),
        CrudError::Validation(msg) => AppError::validation(msg),
        CrudError::Database(e) => AppError::database(e.to_string()),
    }
}

fn item_err(e: ItemError) -> AppError {
    match e {
        ItemError::ColorCardNotFound => AppError::not_found("色卡不存在"),
        ItemError::ItemNotFound => AppError::not_found("色号不存在"),
        ItemError::InvalidState => AppError::business("当前色卡状态不允许此操作"),
        ItemError::Validation(msg) => AppError::validation(msg),
        ItemError::Database(e) => AppError::database(e.to_string()),
    }
}

fn borrow_err(e: BorrowError) -> AppError {
    match e {
        BorrowError::ColorCardNotFound => AppError::not_found("色卡不存在"),
        BorrowError::RecordNotFound => AppError::not_found("借出记录不存在"),
        BorrowError::InvalidState(msg) => AppError::business(msg),
        BorrowError::Validation(msg) => AppError::validation(msg),
        BorrowError::Database(e) => AppError::database(e.to_string()),
    }
}

// ----------------------------------------------------------------------
// 色卡 CRUD（5 端点）
// ----------------------------------------------------------------------

/// GET /api/v1/erp/color-cards - 色卡列表
pub async fn list_color_cards(
    auth: AuthContext,
    State(state): State<AppState>,
    Query(query): Query<ListColorCardsQuery>,
) -> Result<Json<ApiResponse<PagedResponse<ColorCardListItem>>>, AppError> {
    let tenant_id = extract_tenant_id(&auth)? as i64;
    let service = ColorCardCrudService::from_state(&state);
    let page = query.page.unwrap_or(1);
    let page_size = query.page_size.unwrap_or(20);

    let (items, total) = service
        .list(
            tenant_id,
            page,
            page_size,
            query.card_type,
            query.season,
            query.status,
            query.keyword,
        )
        .await
        .map_err(crud_err)?;

    let list: Vec<ColorCardListItem> = items
        .into_iter()
        .map(|m| ColorCardListItem {
            id: m.id,
            card_no: m.card_no,
            card_name: m.card_name,
            card_type: m.card_type,
            season: m.season,
            brand: m.brand,
            total_colors: m.total_colors,
            status: m.status,
            cover_image_url: m.cover_image_url,
            created_at: m.created_at,
        })
        .collect();

    Ok(Json(ApiResponse::success(PagedResponse {
        items: list,
        total,
        page,
        page_size,
    })))
}

/// POST /api/v1/erp/color-cards - 创建色卡
pub async fn create_color_card(
    auth: AuthContext,
    State(state): State<AppState>,
    Json(dto): Json<CreateColorCardDto>,
) -> Result<Json<ApiResponse<ColorCardListItem>>, AppError> {
    let tenant_id = extract_tenant_id(&auth)? as i64;
    let service = ColorCardCrudService::from_state(&state);

    let created = service.create(dto, tenant_id).await.map_err(crud_err)?;

    Ok(Json(ApiResponse::success(ColorCardListItem {
        id: created.id,
        card_no: created.card_no,
        card_name: created.card_name,
        card_type: created.card_type,
        season: created.season,
        brand: created.brand,
        total_colors: created.total_colors,
        status: created.status,
        cover_image_url: created.cover_image_url,
        created_at: created.created_at,
    })))
}

/// GET /api/v1/erp/color-cards/:id - 色卡详情（含色号列表）
pub async fn get_color_card(
    auth: AuthContext,
    State(state): State<AppState>,
    Path(id): Path<i64>,
) -> Result<Json<ApiResponse<ColorCardDetail>>, AppError> {
    let tenant_id = extract_tenant_id(&auth)? as i64;
    let crud_svc = ColorCardCrudService::from_state(&state);
    let item_svc = ColorCardItemService::from_state(&state);

    let card = crud_svc.get_by_id(id, tenant_id).await.map_err(crud_err)?;
    let (items, _) = item_svc
        .list(id, tenant_id, 1, 1000)
        .await
        .map_err(item_err)?;

    let item_infos: Vec<ColorItemInfo> = items
        .into_iter()
        .map(item_to_info)
        .collect();

    Ok(Json(ApiResponse::success(ColorCardDetail {
        id: card.id,
        card_no: card.card_no,
        card_name: card.card_name,
        card_type: card.card_type,
        season: card.season,
        brand: card.brand,
        total_colors: card.total_colors,
        status: card.status,
        description: card.description,
        cover_image_url: card.cover_image_url,
        items: item_infos,
        created_at: card.created_at,
        updated_at: card.updated_at,
    })))
}

/// PUT /api/v1/erp/color-cards/:id - 更新色卡
pub async fn update_color_card(
    auth: AuthContext,
    State(state): State<AppState>,
    Path(id): Path<i64>,
    Json(dto): Json<UpdateColorCardDto>,
) -> Result<Json<ApiResponse<ColorCardListItem>>, AppError> {
    let tenant_id = extract_tenant_id(&auth)? as i64;
    let service = ColorCardCrudService::from_state(&state);

    let updated = service
        .update(id, tenant_id, dto)
        .await
        .map_err(crud_err)?;

    Ok(Json(ApiResponse::success(ColorCardListItem {
        id: updated.id,
        card_no: updated.card_no,
        card_name: updated.card_name,
        card_type: updated.card_type,
        season: updated.season,
        brand: updated.brand,
        total_colors: updated.total_colors,
        status: updated.status,
        cover_image_url: updated.cover_image_url,
        created_at: updated.created_at,
    })))
}

/// DELETE /api/v1/erp/color-cards/:id - 归档色卡
pub async fn archive_color_card(
    auth: AuthContext,
    State(state): State<AppState>,
    Path(id): Path<i64>,
    Json(dto): Json<ArchiveColorCardDto>,
) -> Result<Json<ApiResponse<ColorCardListItem>>, AppError> {
    let tenant_id = extract_tenant_id(&auth)? as i64;
    let service = ColorCardCrudService::from_state(&state);

    let updated = service
        .archive(id, tenant_id, dto)
        .await
        .map_err(crud_err)?;

    Ok(Json(ApiResponse::success(ColorCardListItem {
        id: updated.id,
        card_no: updated.card_no,
        card_name: updated.card_name,
        card_type: updated.card_type,
        season: updated.season,
        brand: updated.brand,
        total_colors: updated.total_colors,
        status: updated.status,
        cover_image_url: updated.cover_image_url,
        created_at: updated.created_at,
    })))
}

// ----------------------------------------------------------------------
// 色号 CRUD（4 端点）
// ----------------------------------------------------------------------

/// GET /api/v1/erp/color-cards/:id/items - 色号列表
pub async fn list_color_items(
    auth: AuthContext,
    State(state): State<AppState>,
    Path(id): Path<i64>,
    Query(query): Query<ListItemsQuery>,
) -> Result<Json<ApiResponse<PagedResponse<ColorItemInfo>>>, AppError> {
    let tenant_id = extract_tenant_id(&auth)? as i64;
    let service = ColorCardItemService::from_state(&state);
    let page = query.page.unwrap_or(1);
    let page_size = query.page_size.unwrap_or(100);

    let (items, total) = service
        .list(id, tenant_id, page, page_size)
        .await
        .map_err(item_err)?;

    let infos: Vec<ColorItemInfo> = items.into_iter().map(item_to_info).collect();

    Ok(Json(ApiResponse::success(PagedResponse {
        items: infos,
        total,
        page,
        page_size,
    })))
}

/// POST /api/v1/erp/color-cards/:id/items - 新增色号
pub async fn create_color_item(
    auth: AuthContext,
    State(state): State<AppState>,
    Path(id): Path<i64>,
    Json(dto): Json<ColorItemDto>,
) -> Result<Json<ApiResponse<ColorItemInfo>>, AppError> {
    let tenant_id = extract_tenant_id(&auth)? as i64;
    let service = ColorCardItemService::from_state(&state);

    let created = service.create(id, tenant_id, dto).await.map_err(item_err)?;
    Ok(Json(ApiResponse::success(item_to_info(created))))
}

/// PUT /api/v1/erp/color-cards/:id/items/:item_id - 更新色号
pub async fn update_color_item(
    auth: AuthContext,
    State(state): State<AppState>,
    Path((id, item_id)): Path<(i64, i64)>,
    Json(dto): Json<ColorItemDto>,
) -> Result<Json<ApiResponse<ColorItemInfo>>, AppError> {
    let tenant_id = extract_tenant_id(&auth)? as i64;
    let service = ColorCardItemService::from_state(&state);

    let updated = service
        .update(id, item_id, tenant_id, dto)
        .await
        .map_err(item_err)?;
    Ok(Json(ApiResponse::success(item_to_info(updated))))
}

/// DELETE /api/v1/erp/color-cards/:id/items/:item_id - 删除色号
pub async fn delete_color_item(
    auth: AuthContext,
    State(state): State<AppState>,
    Path((id, item_id)): Path<(i64, i64)>,
) -> Result<Json<ApiResponse<()>>, AppError> {
    let tenant_id = extract_tenant_id(&auth)? as i64;
    let service = ColorCardItemService::from_state(&state);

    service
        .delete(id, item_id, tenant_id)
        .await
        .map_err(item_err)?;
    Ok(Json(ApiResponse::success(())))
}

// ----------------------------------------------------------------------
// 借出 / 归还 / 遗失 / 历史（5 端点）
// ----------------------------------------------------------------------

/// POST /api/v1/erp/color-cards/borrow - 借出色卡
pub async fn borrow_color_card(
    auth: AuthContext,
    State(state): State<AppState>,
    Json(dto): Json<BorrowColorCardDto>,
) -> Result<Json<ApiResponse<BorrowRecordInfo>>, AppError> {
    let tenant_id = extract_tenant_id(&auth)? as i64;
    let user_id = auth.user_id as i64;
    let service = ColorCardBorrowService::from_state(&state);

    let borrowed_by = dto.borrowed_by.unwrap_or(user_id);

    let record = service
        .borrow(
            dto.color_card_id,
            dto.customer_id,
            borrowed_by,
            dto.expected_return_at,
            dto.purpose,
            dto.notes,
            tenant_id,
        )
        .await
        .map_err(borrow_err)?;

    Ok(Json(ApiResponse::success(record_to_info(record))))
}

/// POST /api/v1/erp/color-cards/return/:record_id - 归还色卡
pub async fn return_color_card(
    auth: AuthContext,
    State(state): State<AppState>,
    Path(record_id): Path<i64>,
    Json(dto): Json<ReturnColorCardDto>,
) -> Result<Json<ApiResponse<BorrowRecordInfo>>, AppError> {
    let tenant_id = extract_tenant_id(&auth)? as i64;
    let service = ColorCardBorrowService::from_state(&state);

    let record = service
        .return_card(record_id, dto.actual_return_at, dto.notes, tenant_id)
        .await
        .map_err(borrow_err)?;

    Ok(Json(ApiResponse::success(record_to_info(record))))
}

/// POST /api/v1/erp/color-cards/lost/:record_id - 登记遗失
pub async fn mark_lost_color_card(
    auth: AuthContext,
    State(state): State<AppState>,
    Path(record_id): Path<i64>,
    Json(dto): Json<MarkLostColorCardDto>,
) -> Result<Json<ApiResponse<BorrowRecordInfo>>, AppError> {
    let tenant_id = extract_tenant_id(&auth)? as i64;
    let service = ColorCardBorrowService::from_state(&state);

    let record = service
        .mark_lost(record_id, dto.compensation_amount, dto.notes, tenant_id)
        .await
        .map_err(borrow_err)?;

    Ok(Json(ApiResponse::success(record_to_info(record))))
}

/// POST /api/v1/erp/color-cards/damaged/:record_id - 标记损坏
pub async fn mark_damaged_color_card(
    auth: AuthContext,
    State(state): State<AppState>,
    Path(record_id): Path<i64>,
    Json(dto): Json<MarkDamagedColorCardDto>,
) -> Result<Json<ApiResponse<BorrowRecordInfo>>, AppError> {
    let tenant_id = extract_tenant_id(&auth)? as i64;
    let service = ColorCardBorrowService::from_state(&state);

    let record = service
        .mark_damaged(record_id, dto.compensation_amount, dto.notes, tenant_id)
        .await
        .map_err(borrow_err)?;

    Ok(Json(ApiResponse::success(record_to_info(record))))
}

/// GET /api/v1/erp/color-cards/borrow-records - 借出历史
pub async fn list_borrow_records(
    auth: AuthContext,
    State(state): State<AppState>,
    Query(query): Query<ListBorrowRecordsQuery>,
) -> Result<Json<ApiResponse<PagedResponse<BorrowRecordInfo>>>, AppError> {
    let tenant_id = extract_tenant_id(&auth)? as i64;
    let service = ColorCardBorrowService::from_state(&state);
    let page = query.page.unwrap_or(1);
    let page_size = query.page_size.unwrap_or(20);

    let (items, total) = service
        .list_records(tenant_id, query)
        .await
        .map_err(borrow_err)?;

    let infos: Vec<BorrowRecordInfo> = items.into_iter().map(record_to_info).collect();
    Ok(Json(ApiResponse::success(PagedResponse {
        items: infos,
        total,
        page,
        page_size,
    })))
}

// ----------------------------------------------------------------------
// 扫码 / 导入 / 导出（2 端点）
// ----------------------------------------------------------------------

/// GET /api/v1/erp/color-cards/scan/:code - 扫码查询色号详情
pub async fn scan_color_code(
    auth: AuthContext,
    State(state): State<AppState>,
    Path(code): Path<String>,
) -> Result<Json<ApiResponse<crate::models::color_card_response_dto::ScanResult>>, AppError> {
    let tenant_id = extract_tenant_id(&auth)? as i64;
    let service = ColorCardScanService::from_state(&state);

    let result = service.scan_by_code(&code, tenant_id).await?;
    Ok(Json(ApiResponse::success(result)))
}

/// POST /api/v1/erp/color-cards/:id/items/batch - 批量导入色号
pub async fn batch_import_items(
    auth: AuthContext,
    State(state): State<AppState>,
    Path(id): Path<i64>,
    Json(dto): Json<BatchImportItemsDto>,
) -> Result<Json<ApiResponse<crate::models::color_card_item_dto::BatchImportResponse>>, AppError>
{
    let tenant_id = extract_tenant_id(&auth)? as i64;
    let service = ColorCardItemService::from_state(&state);

    let result = service
        .batch_import(id, tenant_id, dto.items)
        .await
        .map_err(item_err)?;
    Ok(Json(ApiResponse::success(result)))
}

/// GET /api/v1/erp/color-cards/export/:id - 导出色卡为 CSV
pub async fn export_color_card(
    auth: AuthContext,
    State(state): State<AppState>,
    Path(id): Path<i64>,
) -> Result<impl IntoResponse, AppError> {
    let tenant_id = extract_tenant_id(&auth)? as i64;
    let item_svc = ColorCardItemService::from_state(&state);
    let crud_svc = ColorCardCrudService::from_state(&state);

    let card = crud_svc.get_by_id(id, tenant_id).await.map_err(crud_err)?;
    let (items, _) = item_svc
        .list(id, tenant_id, 1, 10000)
        .await
        .map_err(item_err)?;

    // 构造 CSV
    let mut csv = String::from("color_code,color_name,rgb_r,rgb_g,rgb_b,hex_value,cmyk_c,cmyk_m,cmyk_y,cmyk_k,lab_l,lab_a,lab_b,pantone_code,cncs_code,custom_code\n");
    for item in items {
        csv.push_str(&format!(
            "{},{},{},{},{},{},{},{},{},{},{},{},{},{},{},{}\n",
            item.color_code,
            csv_escape(&item.color_name),
            item.rgb_r,
            item.rgb_g,
            item.rgb_b,
            item.hex_value,
            item.cmyk_c.map(|d| d.to_string()).unwrap_or_default(),
            item.cmyk_m.map(|d| d.to_string()).unwrap_or_default(),
            item.cmyk_y.map(|d| d.to_string()).unwrap_or_default(),
            item.cmyk_k.map(|d| d.to_string()).unwrap_or_default(),
            item.lab_l.map(|d| d.to_string()).unwrap_or_default(),
            item.lab_a.map(|d| d.to_string()).unwrap_or_default(),
            item.lab_b.map(|d| d.to_string()).unwrap_or_default(),
            item.pantone_code.clone().unwrap_or_default(),
            item.cncs_code.clone().unwrap_or_default(),
            item.custom_code.clone().unwrap_or_default(),
        ));
    }

    let filename = format!("color-card-{}.csv", card.card_no.replace(['/', '\\', ' '], "_"));
    let headers = [
        (axum::http::header::CONTENT_TYPE, "text/csv; charset=utf-8".to_string()),
        (
            axum::http::header::CONTENT_DISPOSITION,
            format!("attachment; filename=\"{}\"", filename),
        ),
    ];

    Ok((StatusCode::OK, headers, csv))
}

// ----------------------------------------------------------------------
// 内部辅助
// ----------------------------------------------------------------------

#[derive(Debug, serde::Deserialize)]
pub struct ListItemsQuery {
    pub page: Option<u64>,
    pub page_size: Option<u64>,
}

fn item_to_info(m: color_card_item::Model) -> ColorItemInfo {
    ColorItemInfo {
        id: m.id,
        color_code: m.color_code,
        color_name: m.color_name,
        rgb_r: m.rgb_r,
        rgb_g: m.rgb_g,
        rgb_b: m.rgb_b,
        cmyk_c: m.cmyk_c,
        cmyk_m: m.cmyk_m,
        cmyk_y: m.cmyk_y,
        cmyk_k: m.cmyk_k,
        lab_l: m.lab_l,
        lab_a: m.lab_a,
        lab_b: m.lab_b,
        pantone_code: m.pantone_code,
        cncs_code: m.cncs_code,
        custom_code: m.custom_code,
        hex_value: m.hex_value,
        dye_recipe_id: m.dye_recipe_id,
        product_color_price_id: m.product_color_price_id,
        swatch_image_url: m.swatch_image_url,
        sequence: m.sequence,
    }
}

fn record_to_info(m: color_card_borrow_record::Model) -> BorrowRecordInfo {
    BorrowRecordInfo {
        id: m.id,
        color_card_id: m.color_card_id,
        color_card_no: None,
        color_card_name: None,
        customer_id: m.customer_id,
        customer_name: None,
        borrowed_by: m.borrowed_by,
        borrowed_by_name: None,
        borrowed_at: m.borrowed_at,
        expected_return_at: m.expected_return_at,
        actual_return_at: m.actual_return_at,
        status: m.status,
        purpose: m.purpose,
        notes: m.notes,
        compensation_amount: m.compensation_amount,
    }
}

fn csv_escape(s: &str) -> String {
    if s.contains(',') || s.contains('"') || s.contains('\n') {
        format!("\"{}\"", s.replace('"', "\"\""))
    } else {
        s.to_string()
    }
}

// 抑制未使用导入警告（json 是预留 API，给未来扩展用）
#[allow(dead_code)]
fn _ensure_json_used() {
    let _ = json!({});
}
