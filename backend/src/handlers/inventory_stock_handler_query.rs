//! 库存处理器：事务/汇总查询业务（list_transactions + get_inventory_summary + get_stock_by_product + get_stock_alerts）
//!
//! 拆分自 inventory_stock_handler.rs：原 4 个查询 fn + tests 独立成文件。

use crate::middleware::auth_context::AuthContext;
use crate::models::dto::PageRequest;
use crate::services::inventory_stock_service::InventoryStockService;
use crate::utils::app_state::AppState;
use crate::utils::error::AppError;
use crate::utils::response::{ApiResponse, PaginatedResponse};
use axum::{
    extract::{Path, Query, State},
    Json,
};
use rust_decimal::Decimal;

use super::inventory_stock_handler_dto::{
    InventorySummaryItem, ListStockFabricParams, ListTransactionParams, TransactionResponse,
};

pub async fn list_transactions(
    State(state): State<AppState>,
    _auth: AuthContext,
    Query(params): Query<ListTransactionParams>,
) -> Result<Json<ApiResponse<PaginatedResponse<TransactionResponse>>>, AppError> {
    let service = InventoryStockService::new(state.db.clone());

    // 页码采用 1-based 约定，由 service 内部转换为 0-based
    let page = params.page.unwrap_or(1).clamp(1, 1000); // 批次 95 P3-3~8：分页 clamp 防 DoS
    let page_size = params.page_size.unwrap_or(20).clamp(1, 100);

    let (transactions, total) = service
        .list_transactions(crate::services::inventory_stock_query::ListTransactionsQuery {
            page,
            page_size,
            batch_no: params.batch_no,
            color_no: params.color_no,
            product_id: params.product_id,
            warehouse_id: params.warehouse_id,
            transaction_type: params.transaction_type,
            start_date: params.start_date,
            end_date: params.end_date,
        })
        .await?;

    let transaction_responses: Vec<TransactionResponse> = transactions
        .into_iter()
        .map(|txn| TransactionResponse {
            id: txn.id,
            transaction_type: txn.transaction_type,
            product_id: txn.product_id,
            warehouse_id: txn.warehouse_id,
            batch_no: txn.batch_no,
            color_no: txn.color_no,
            quantity_meters: txn.quantity_meters,
            quantity_kg: txn.quantity_kg,
            quantity_before_meters: txn.quantity_before_meters.unwrap_or(Decimal::ZERO),
            quantity_before_kg: txn.quantity_before_kg.unwrap_or(Decimal::ZERO),
            quantity_after_meters: txn.quantity_after_meters.unwrap_or(Decimal::ZERO),
            quantity_after_kg: txn.quantity_after_kg.unwrap_or(Decimal::ZERO),
            source_bill_type: txn.source_bill_type,
            source_bill_no: txn.source_bill_no,
            remarks: txn.notes,
            created_at: txn.created_at,
        })
        .collect();

    Ok(Json(crate::utils::response::ApiResponse::success(
        PaginatedResponse::new(transaction_responses, total, page, page_size),
    )))
}

/// 获取库存汇总（按批次 + 色号）
///
/// # 查询参数
/// - `page`: 页码，默认为1
/// - `page_size`: 每页大小，默认为20
/// - `warehouse_id`: 仓库ID筛选
/// - `product_id`: 产品ID筛选
/// - `batch_no`: 批次号筛选
/// - `color_no`: 色号筛选
/// - `grade`: 等级筛选
pub async fn get_inventory_summary(
    State(state): State<AppState>,
    _auth: AuthContext,
    Query(params): Query<ListStockFabricParams>,
) -> Result<
    Json<
        crate::utils::response::ApiResponse<
            crate::utils::response::PaginatedResponse<InventorySummaryItem>,
        >,
    >,
    AppError,
> {
    let service = InventoryStockService::new(state.db.clone());

    let page = params.page.unwrap_or(1).clamp(1, 1000); // 批次 95 P3-3~8：分页 clamp 防 DoS
    let page_size = params.page_size.unwrap_or(20).clamp(1, 100);

    let (summary_items, total) = service
        .get_inventory_summary(
            params.warehouse_id,
            params.product_id,
            params.batch_no,
            params.color_no,
            params.grade,
            page,
            page_size,
        )
        .await?;

    let summary: Vec<InventorySummaryItem> = summary_items
        .into_iter()
        .map(|item| InventorySummaryItem {
            product_id: item.product_id,
            product_name: item.product_name,
            batch_no: item.batch_no,
            color_no: item.color_no,
            grade: item.grade,
            total_quantity_meters: item.total_quantity_meters,
            total_quantity_kg: item.total_quantity_kg,
            warehouse_name: item.warehouse_name,
        })
        .collect();

    let paginated_response = crate::utils::response::PaginatedResponse {
        items: summary,
        total,
        page,
        page_size,
    };

    Ok(Json(crate::utils::response::ApiResponse::success(
        paginated_response,
    )))
}

pub async fn get_stock_by_product(
    _auth: AuthContext,
    State(state): State<AppState>,
    Path(product_id): Path<i32>,
    Query(query): Query<PageRequest>,
) -> Result<Json<ApiResponse<serde_json::Value>>, AppError> {
    let service = InventoryStockService::new(state.db.clone());

    // v11 批次 36 修复：page_size clamp 防止 DoS（PageRequest 自带 limit 方法但此处未调用，直接透传原始值）
    let page_size = query.page_size.clamp(1, 100);
    // 批次 98 P2-A 修复（v5 复审）：page clamp 防 DoS
    let page = query.page.clamp(1, 1000);
    let (stocks, total) = service
        .get_stock_by_product(product_id, page, page_size)
        .await
        .map_err(|e| AppError::internal(format!("查询产品库存失败: {}", e)))?;

    let result = serde_json::json!({
        "list": stocks,
        "total": total,
        "page": page,
        "page_size": page_size,
    });

    Ok(Json(ApiResponse::success(result)))
}

/// 获取库存告警
/// GET /api/v1/erp/inventory/stock/alerts
pub async fn get_stock_alerts(
    _auth: AuthContext,
    State(state): State<AppState>,
    Query(query): Query<serde_json::Value>,
) -> Result<Json<ApiResponse<serde_json::Value>>, AppError> {
    let service = InventoryStockService::new(state.db.clone());

    let alerts = service
        .get_stock_alerts(query)
        .await
        .map_err(|e| AppError::internal(format!("获取库存告警失败: {}", e)))?;

    Ok(Json(ApiResponse::success(alerts)))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::handlers::inventory_stock_handler_dto::CreateStockFabricRequest;
    use std::str::FromStr;

    #[test]
    fn test_create_stock_fabric_request_deserialize() {
        let json = r#"
        {
            "warehouse_id": 1,
            "product_id": 100,
            "batch_no": "B20240101",
            "color_no": "C001",
            "dye_lot_no": "D20240101001",
            "grade": "一等品",
            "quantity_meters": "100.00",
            "gram_weight": "180.00",
            "width": "180.00",
            "location_id": 1,
            "shelf_no": "A01",
            "layer_no": "01"
        }
        "#;

        // P9-1 关键路径 unwrap 清理：单元测试中的常量 JSON 序列化使用 decs! 宏统一
        let req: CreateStockFabricRequest =
            serde_json::from_str(json).expect("P9-1: 单元测试夹具 JSON 反序列化失败，需要排查 fixture");
        assert_eq!(req.warehouse_id, 1);
        assert_eq!(req.product_id, 100);
        assert_eq!(req.batch_no, "B20240101");
        assert_eq!(req.color_no, "C001");
        assert_eq!(req.quantity_meters, crate::decs!("100.00"));
        assert_eq!(req.gram_weight, Some(crate::decs!("180.00")));
        assert_eq!(req.width, Some(crate::decs!("180.00")));
    }
}
