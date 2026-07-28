use crate::container::AppState;
use crate::middleware::auth_context::AuthContext;
use crate::models::inventory_stock::Model as InventoryStock;
use crate::models::product;
// 批次 213 P2-5 修复（v12 复审）：硬编码 "active" 替换为 master_data 常量
use crate::models::status::master_data;
use crate::services::inventory_stock_service::{CreateStockArgs, InventoryStockService};
use crate::utils::error::AppError;
use crate::utils::response::{ApiResponse, PaginatedResponse};
use axum::{
    extract::{Path, Query, State},
    Json,
};
use chrono::Utc;
use rust_decimal::Decimal;
use sea_orm::{ColumnTrait, DatabaseConnection, EntityTrait, QueryFilter};
// 批次 357 v13 复审 baseline 清零：移除 unused import（Deserialize, Serialize 编译器报未使用）
use validator::Validate as _; // 仅用于 `ListStockParams::validate()` 方法解析

use super::inventory_stock_handler_dto::{
    CreateStockFabricRequest, ListStockParams, LowStockParams, StockResponse,
    UpdateStockWithVersionRequest,
};
// V15 P0-S12/P0-S15 修复（Batch 475c）：导出端点使用水印版 xlsx 工具
use crate::utils::xlsx_export::{build_xlsx_response_with_watermark, WatermarkConfig, XlsxTable};
// V15 P0-S11：导出审计日志写入所需依赖
use crate::models::audit_log::{OperationType, Severity};
use crate::services::audit_log_service::{AuditEvent, AuditLogService};
use std::sync::Arc;

pub async fn get_stock(
    State(state): State<AppState>,
    auth: AuthContext,
    Path(id): Path<i32>,
) -> Result<Json<ApiResponse<serde_json::Value>>, AppError> {
    let service = InventoryStockService::new(state.db.clone());

    // P2-1 修复（批次 388 v13 复审）：原 map_err 将所有错误映射为 not_found，
    // 导致 DB 错误也返回 404。service 返回 AppError，直接用 ? 透传保留原始错误分类
    let stock = service.find_by_id(id).await?;

    let response = StockResponse {
        id: stock.id,
        warehouse_id: stock.warehouse_id,
        product_id: stock.product_id,
        quantity_on_hand: stock.quantity_on_hand,
        quantity_available: stock.quantity_available,
        quantity_reserved: stock.quantity_reserved,
        reorder_point: stock.reorder_point,
        max_stock_point: stock.max_stock_point,
        bin_location: stock.bin_location,
        created_at: stock.created_at,
        updated_at: stock.updated_at,
    };

    let mut response_json = serde_json::to_value(response)?;

    // 数据权限控制：获取角色数据权限并应用字段过滤
    // P2-1 修复（批次 388 v13 复审）：原 if let Ok(Some(...)) 合并 Err 与 Ok(None) 语义，
    // 改为 match 精确区分三种情况：有配置 / 无配置 / 查询出错
    if let Some(role_id) = auth.role_id {
        match state
            .data_permission_service
            .get_role_data_permission(role_id, "inventory_stock")
            .await
        {
            Ok(Some(permission)) => {
                state.data_permission_service.filter_fields(
                    &mut response_json,
                    &permission.allowed_fields,
                    &permission.hidden_fields,
                );
            }
            Ok(None) => {
                // 没有配置数据权限且不是管理员，使用默认字段隐藏
                if role_id != 1 {
                    if let Some(obj) = response_json.as_object_mut() {
                        obj.remove("quantity_on_hand");
                        obj.remove("quantity_available");
                        obj.remove("quantity_reserved");
                        obj.remove("reorder_point");
                        obj.remove("reorder_quantity");
                    }
                }
            }
            Err(e) => {
                // P2-1 修复：DB 查询出错时仅记录警告，不应用默认隐藏（避免误隐藏字段）
                tracing::warn!(
                    role_id,
                    error = %e,
                    "批次 388 P2-1: 查询库存数据权限失败，跳过字段过滤"
                );
            }
        }
    }

    Ok(Json(ApiResponse::success(response_json)))
}

pub async fn create_stock(
    State(state): State<AppState>,
    _auth: AuthContext,
    Json(payload): Json<CreateStockFabricRequest>,
) -> Result<Json<ApiResponse<StockResponse>>, AppError> {
    let service = InventoryStockService::new(state.db.clone());

    let stock = service
        .create_stock(CreateStockArgs {
            warehouse_id: payload.warehouse_id,
            product_id: payload.product_id,
            batch_no: payload.batch_no,
            color_no: payload.color_no,
            quantity_meters: payload.quantity_meters,
            quantity_kg: payload.quantity_kg.unwrap_or(Decimal::ZERO),
            grade: payload.grade,
            dye_lot_no: payload.dye_lot_no,
            gram_weight: payload.gram_weight,
            width: payload.width,
            stock_status: master_data::ACTIVE.to_string(),
            quality_status: "qualified".to_string(),
        })
        .await?;

    Ok(Json(ApiResponse::success(StockResponse {
        id: stock.id,
        warehouse_id: stock.warehouse_id,
        product_id: stock.product_id,
        quantity_on_hand: stock.quantity_on_hand,
        quantity_available: stock.quantity_available,
        quantity_reserved: stock.quantity_reserved,
        reorder_point: stock.reorder_point,
        max_stock_point: stock.max_stock_point,
        bin_location: stock.bin_location,
        created_at: stock.created_at,
        updated_at: stock.updated_at,
    })))
}

pub async fn update_stock(
    State(state): State<AppState>,
    _auth: AuthContext,
    Path(id): Path<i32>,
    Json(payload): Json<UpdateStockWithVersionRequest>,
) -> Result<Json<ApiResponse<StockResponse>>, AppError> {
    let service = InventoryStockService::new(state.db.clone());

    // P2-1 修复（批次 388 v13 复审）：原 map_err 将所有错误映射为 not_found，改为 ? 透传
    let stock = service.find_by_id(id).await?;

    // Optimistic lock check
    if stock.version != payload.version {
        return Err(AppError::business(
            "库存记录已被其他用户修改，请刷新后重试".to_string(),
        ));
    }

    use sea_orm::{ActiveModelTrait, Set};
    let mut active_model: crate::models::inventory_stock::ActiveModel = stock.into();

    if let Some(qoh) = payload.quantity_on_hand {
        active_model.quantity_on_hand = Set(qoh);
    }
    if let Some(qavail) = payload.quantity_available {
        active_model.quantity_available = Set(qavail);
    }
    if let Some(qres) = payload.quantity_reserved {
        active_model.quantity_reserved = Set(qres);
    }
    if let Some(rop) = payload.reorder_point {
        active_model.reorder_point = Set(rop);
    }
    if let Some(msp) = payload.max_stock_point {
        active_model.max_stock_point = Set(msp);
    }
    if let Some(roq) = payload.reorder_quantity {
        active_model.reorder_quantity = Set(roq);
    }
    if let Some(bl) = payload.bin_location {
        active_model.bin_location = Set(Some(bl));
    }
    active_model.version = Set(payload.version + 1);
    active_model.updated_at = Set(Utc::now());

    // P2-1 修复（批次 388 v13 复审）：原 map_err 将 DbErr 映射为 internal，
    // 改为 ? 透传，由 From<DbErr> for AppError 自动分类（RecordNotFound→404, 其他→500）
    let updated = active_model.update(&*state.db).await?;

    Ok(Json(ApiResponse::success(StockResponse {
        id: updated.id,
        warehouse_id: updated.warehouse_id,
        product_id: updated.product_id,
        quantity_on_hand: updated.quantity_on_hand,
        quantity_available: updated.quantity_available,
        quantity_reserved: updated.quantity_reserved,
        reorder_point: updated.reorder_point,
        max_stock_point: updated.max_stock_point,
        bin_location: updated.bin_location,
        created_at: updated.created_at,
        updated_at: updated.updated_at,
    })))
}

pub async fn delete_stock(
    State(state): State<AppState>,
    auth: AuthContext,
    Path(id): Path<i32>,
) -> Result<Json<ApiResponse<()>>, AppError> {
    let service = InventoryStockService::new(state.db.clone());

    // P2-1 修复（批次 388 v13 复审）：原 map_err 将所有错误映射为 not_found，改为 ? 透传
    service.find_by_id(id).await?;

    // P2-1 修复：原 map_err 将所有错误映射为 internal，改为 ? 透传保留原始错误分类
    service.delete_stock(id, Some(auth.user_id)).await?;

    Ok(Json(ApiResponse::success(())))
}

pub async fn list_stock(
    State(state): State<AppState>,
    auth: AuthContext,
    Query(params): Query<ListStockParams>,
) -> Result<Json<ApiResponse<PaginatedResponse<serde_json::Value>>>, AppError> {
    params
        .validate()
        .map_err(|e| AppError::validation(e.to_string()))?;

    let service = InventoryStockService::new(state.db.clone());
    let page = params.page.unwrap_or(1).clamp(1, 1000);
    let page_size = params.page_size.unwrap_or(20).clamp(1, 100);

    let (stock_list, total) = service
        .list_stock(page, page_size, params.warehouse_id, params.product_id)
        .await?;

    let stock_responses: Vec<_> = stock_list.into_iter().map(to_stock_response).collect();

    send_inventory_alerts(&state, &stock_responses).await;

    let mut stock_json = serialize_stock_responses(stock_responses)?;

    apply_data_permission_filter(&state, &auth, &mut stock_json).await;

    Ok(Json(crate::utils::response::ApiResponse::success(
        PaginatedResponse::new(stock_json, total, page, page_size),
    )))
}

fn to_stock_response(stock: InventoryStock) -> StockResponse {
    StockResponse {
        id: stock.id,
        warehouse_id: stock.warehouse_id,
        product_id: stock.product_id,
        quantity_on_hand: stock.quantity_on_hand,
        quantity_available: stock.quantity_available,
        quantity_reserved: stock.quantity_reserved,
        reorder_point: stock.reorder_point,
        max_stock_point: stock.max_stock_point,
        bin_location: stock.bin_location,
        created_at: stock.created_at,
        updated_at: stock.updated_at,
    }
}

async fn send_inventory_alerts(state: &AppState, stock_responses: &[StockResponse]) {
    let Some(ref event_service) = state.event_notification_service else {
        return;
    };

    let alert_product_ids: Vec<i32> = stock_responses
        .iter()
        .filter(|s| s.quantity_on_hand < s.reorder_point)
        .map(|s| s.product_id)
        .collect();

    if alert_product_ids.is_empty() {
        return;
    }

    let product_map = load_alert_products(&*state.db, &alert_product_ids).await;
    let setting = event_service.get_setting_for_user(0).await.ok();

    for stock in stock_responses {
        if stock.quantity_on_hand < stock.reorder_point {
            if let Some(product) = product_map.get(&stock.product_id) {
                send_alert_for_stock(event_service, &setting, product, stock).await;
            }
        }
    }
}

async fn load_alert_products(
    db: &DatabaseConnection,
    product_ids: &[i32],
) -> std::collections::HashMap<i32, product::Model> {
    match product::Entity::find()
        .filter(product::Column::Id.is_in(product_ids.iter().copied()))
        .all(db)
        .await
    {
        Ok(p) => p.into_iter().map(|p| (p.id, p)).collect(),
        Err(e) => {
            tracing::warn!(error = %e, "批次 388 P2-1: 查询库存预警产品信息失败，本轮预警通知将跳过");
            std::collections::HashMap::new()
        }
    }
}

async fn send_alert_for_stock(
    event_service: &Arc<crate::services::event_notification_service::EventNotificationService>,
    setting: &Option<crate::models::user_notification_setting::Model>,
    product: &product::Model,
    stock: &StockResponse,
) {
    if let Some(ref s) = setting {
        if let Err(e) = event_service
            .notify_inventory_alert_with_setting(
                0,
                s,
                &product.name,
                product.id,
                &stock.quantity_on_hand.to_string(),
                &stock.reorder_point.to_string(),
            )
            .await
        {
            tracing::warn!("批次 94 P2-11：库存预警通知(with_setting)发送失败: {}", e);
        }
    } else {
        if let Err(e) = event_service
            .notify_inventory_alert(
                0,
                &product.name,
                product.id,
                &stock.quantity_on_hand.to_string(),
                &stock.reorder_point.to_string(),
            )
            .await
        {
            tracing::warn!("批次 94 P2-11：库存预警通知发送失败: {}", e);
        }
    }
}

fn serialize_stock_responses(
    stock_responses: Vec<StockResponse>,
) -> Result<Vec<serde_json::Value>, AppError> {
    stock_responses
        .into_iter()
        .map(|s| serde_json::to_value(s).map_err(AppError::from))
        .collect()
}

async fn apply_data_permission_filter(
    state: &AppState,
    auth: &AuthContext,
    stock_json: &mut Vec<serde_json::Value>,
) {
    let Some(role_id) = auth.role_id else { return };

    match state
        .data_permission_service
        .get_role_data_permission(role_id, "inventory_stock")
        .await
    {
        Ok(Some(permission)) => {
            state.data_permission_service.filter_fields_batch(
                stock_json,
                &permission.allowed_fields,
                &permission.hidden_fields,
            );
        }
        Ok(None) => {
            if role_id != 1 {
                for stock in stock_json {
                    if let Some(obj) = stock.as_object_mut() {
                        obj.remove("quantity_on_hand");
                        obj.remove("quantity_available");
                        obj.remove("quantity_reserved");
                        obj.remove("reorder_point");
                        obj.remove("reorder_quantity");
                    }
                }
            }
        }
        Err(e) => {
            tracing::warn!(role_id, error = %e, "批次 388 P2-1: 查询库存数据权限失败，跳过字段过滤");
        }
    }
}

pub async fn check_low_stock(
    State(state): State<AppState>,
    _auth: AuthContext,
    Query(params): Query<LowStockParams>,
) -> Result<Json<crate::utils::response::ApiResponse<Vec<StockResponse>>>, AppError> {
    let service = InventoryStockService::new(state.db.clone());

    let stock_list = service
        .check_low_stock(params.warehouse_id, params.product_id, params.batch_no)
        .await?;

    let stock_responses = convert_to_stock_responses(stock_list);

    // 发送库存预警通知
    if let Some(ref event_service) = state.event_notification_service {
        let product_map = load_low_stock_product_map(&state.db, &stock_responses).await;
        send_low_stock_notifications(event_service, &stock_responses, &product_map).await;
    }

    Ok(Json(crate::utils::response::ApiResponse::success(
        stock_responses,
    )))
}

/// 将库存 model 列表转换为响应 DTO
fn convert_to_stock_responses(
    stock_list: Vec<crate::models::inventory_stock::Model>,
) -> Vec<StockResponse> {
    stock_list
        .into_iter()
        .map(|stock| StockResponse {
            id: stock.id,
            warehouse_id: stock.warehouse_id,
            product_id: stock.product_id,
            quantity_on_hand: stock.quantity_on_hand,
            quantity_available: stock.quantity_available,
            quantity_reserved: stock.quantity_reserved,
            reorder_point: stock.reorder_point,
            max_stock_point: stock.max_stock_point,
            bin_location: stock.bin_location,
            created_at: stock.created_at,
            updated_at: stock.updated_at,
        })
        .collect()
}

/// 批量查询产品信息，返回以 id 为键的 map（P2-1 修复：DB 错误降级为空集合）
async fn load_low_stock_product_map(
    db: &std::sync::Arc<sea_orm::DatabaseConnection>,
    stock_responses: &[StockResponse],
) -> std::collections::HashMap<i32, product::Model> {
    let product_ids: Vec<i32> = stock_responses
        .iter()
        .map(|stock| stock.product_id)
        .collect();
    if product_ids.is_empty() {
        return std::collections::HashMap::new();
    }
    // P2-1 修复（批次 388 v13 复审）：原 unwrap_or_default() 吞 DB 错误导致预警丢失，
    // 改为 match 记录 warn 日志后降级为空集合（跳过本轮预警通知）
    // ConnectionTrait 为 DatabaseConnection 实现，需 db.as_ref() 解引用 Arc
    let products = match product::Entity::find()
        .filter(product::Column::Id.is_in(product_ids.iter().copied()))
        .all(db.as_ref())
        .await
    {
        Ok(p) => p,
        Err(e) => {
            tracing::warn!(
                error = %e,
                "批次 388 P2-1: 查询库存预警产品信息失败，本轮预警通知将跳过"
            );
            vec![]
        }
    };
    products.into_iter().map(|p| (p.id, p)).collect()
}

/// 发送库存预警通知（v17 批次 47：循环外查询 setting 避免 N+1）
async fn send_low_stock_notifications(
    event_service: &Arc<crate::services::event_notification_service::EventNotificationService>,
    stock_responses: &[StockResponse],
    product_map: &std::collections::HashMap<i32, product::Model>,
) {
    let setting = event_service.get_setting_for_user(0).await.ok();
    for stock in stock_responses {
        if let Some(product) = product_map.get(&stock.product_id) {
            notify_single_low_stock(event_service, stock, product, setting.as_ref()).await;
        }
    }
}

/// 发送单个库存预警通知（有 setting 走 with_setting，否则回退原方法）
async fn notify_single_low_stock(
    event_service: &Arc<crate::services::event_notification_service::EventNotificationService>,
    stock: &StockResponse,
    product: &product::Model,
    setting: Option<&crate::models::user_notification_setting::Model>,
) {
    // 批次 94 P2-11：原 let _ = 静默吞错，改为 warn 日志记录
    if let Some(s) = setting {
        if let Err(e) = event_service
            .notify_inventory_alert_with_setting(
                0,
                s,
                &product.name,
                product.id,
                &stock.quantity_on_hand.to_string(),
                &stock.reorder_point.to_string(),
            )
            .await
        {
            tracing::warn!("批次 94 P2-11：库存预警通知(with_setting)发送失败: {}", e);
        }
    } else {
        // 设置查询失败时回退到原方法（兼容性保留）
        if let Err(e) = event_service
            .notify_inventory_alert(
                0,
                &product.name,
                product.id,
                &stock.quantity_on_hand.to_string(),
                &stock.reorder_point.to_string(),
            )
            .await
        {
            tracing::warn!("批次 94 P2-11：库存预警通知发送失败: {}", e);
        }
    }
}

// ========== 数据导出接口 ==========

/// 导出库存列表（V15 P0-S12/P0-S15 修复：导出注入水印 + 异步审计日志）
/// 规则 3：导出统一使用 xlsx 格式；直接调 service.list_stock，不触发低库存预警通知副作用
pub async fn export_stock(
    State(state): State<AppState>,
    auth: AuthContext,
    Query(params): Query<ListStockParams>,
) -> Result<axum::response::Response, AppError> {
    params
        .validate()
        .map_err(|e| AppError::validation(e.to_string()))?;

    let service = InventoryStockService::new(state.db.clone());
    let (stock_list, _total) = service
        .list_stock(1, 10000, params.warehouse_id, params.product_id)
        .await?;
    let row_count = stock_list.len();

    let stock_responses: Vec<StockResponse> =
        stock_list.into_iter().map(to_stock_response).collect();
    let mut stock_json = serialize_stock_responses(stock_responses)?;
    apply_data_permission_filter(&state, &auth, &mut stock_json).await;

    let table = build_stock_xlsx_table(&stock_json)?;
    let filename = format!(
        "inventory_stock_export_{}",
        chrono::Utc::now().format("%Y%m%d_%H%M%S")
    );

    record_export_audit(&state.db, &auth, &filename, row_count, &params);
    let watermark = build_export_watermark(&auth, row_count);

    build_xlsx_response_with_watermark(&table, &filename, &watermark)
}

fn build_stock_xlsx_table(stock_json: &[serde_json::Value]) -> Result<XlsxTable, AppError> {
    let headers = vec![
        "ID".to_string(),
        "仓库ID".to_string(),
        "产品ID".to_string(),
        "在库量".to_string(),
        "可用量".to_string(),
        "预留量".to_string(),
        "库位".to_string(),
        "创建时间".to_string(),
        "更新时间".to_string(),
    ];

    let rows = stock_json
        .iter()
        .map(|stock| {
            let obj = stock
                .as_object()
                .ok_or_else(|| AppError::internal("库存序列化失败：期望 JSON 对象"))?;
            Ok(vec![
                get_json_str(obj, "id"),
                get_json_str(obj, "warehouse_id"),
                get_json_str(obj, "product_id"),
                get_json_str(obj, "quantity_on_hand"),
                get_json_str(obj, "quantity_available"),
                get_json_str(obj, "quantity_reserved"),
                get_json_str(obj, "bin_location"),
                get_json_str(obj, "created_at"),
                get_json_str(obj, "updated_at"),
            ])
        })
        .collect::<Result<Vec<_>, AppError>>()?;

    Ok(XlsxTable {
        sheet_name: "库存列表".to_string(),
        headers,
        rows,
    })
}

fn get_json_str(obj: &serde_json::Map<String, serde_json::Value>, key: &str) -> String {
    obj.get(key)
        .map(|v| {
            if v.is_null() {
                String::new()
            } else if v.is_string() {
                v.as_str().unwrap_or("").to_string()
            } else {
                v.to_string()
            }
        })
        .unwrap_or_default()
}

fn record_export_audit(
    db: &Arc<DatabaseConnection>,
    auth: &AuthContext,
    filename: &str,
    row_count: usize,
    params: &ListStockParams,
) {
    let event = AuditEvent {
        user_id: Some(auth.user_id),
        username: Some(auth.username.clone()),
        operation_type: OperationType::Export,
        severity: Severity::Info,
        resource_type: Some("inventory_stock".to_string()),
        resource_id: None,
        resource_name: Some(format!("{}.xlsx", filename)),
        description: Some(format!(
            "用户 {} 导出库存列表（共 {} 条）",
            auth.username, row_count
        )),
        request_method: Some("GET".to_string()),
        request_path: Some("/api/v1/erp/inventory/stock/export".to_string()),
        before_snapshot: None,
        after_snapshot: Some(serde_json::json!({
            "format": "xlsx",
            "total": row_count,
            "warehouse_id_filter": params.warehouse_id,
            "product_id_filter": params.product_id,
        })),
    };
    let svc = Arc::new(AuditLogService::new(db.clone()));
    svc.record_async(event, None);
}

fn build_export_watermark(auth: &AuthContext, row_count: usize) -> WatermarkConfig {
    WatermarkConfig {
        operator: Some(auth.username.clone()),
        ip_address: None,
        exported_at: Some(chrono::Utc::now().to_rfc3339()),
        extra: Some(format!("库存导出（共 {} 条）", row_count)),
    }
}
