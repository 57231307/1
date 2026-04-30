use crate::utils::app_state::AppState;
use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    Json,
};
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};

use crate::services::business_trace_service::BusinessTraceService;

/// 追溯链响应
#[derive(Debug, Serialize)]
pub struct TraceChainResponse {
    pub id: i32,
    pub trace_chain_id: String,
    pub five_dimension_id: String,
    pub product_id: i32,
    pub batch_no: String,
    pub color_no: String,
    pub dye_lot_no: Option<String>,
    pub grade: String,
    pub current_stage: String,
    pub current_bill_type: String,
    pub current_bill_no: String,
    pub quantity_meters: Decimal,
    pub quantity_kg: Decimal,
    pub warehouse_id: i32,
    pub supplier_id: Option<i32>,
    pub customer_id: Option<i32>,
    pub trace_status: String,
    pub created_at: chrono::DateTime<chrono::Utc>,
}

/// 完整追溯链响应
#[derive(Debug, Serialize)]
pub struct FullTraceChainResponse {
    pub trace_chain_id: String,
    pub five_dimension_id: String,
    pub product_id: i32,
    pub batch_no: String,
    pub color_no: String,
    pub grade: String,
    pub stages: Vec<TraceStageDetail>,
    pub total_stages: usize,
    pub start_time: chrono::DateTime<chrono::Utc>,
    pub end_time: Option<chrono::DateTime<chrono::Utc>>,
}

/// 追溯环节详情
#[derive(Debug, Serialize)]
pub struct TraceStageDetail {
    pub stage_id: i32,
    pub stage_name: String,
    pub stage_type: String,
    pub bill_type: String,
    pub bill_no: String,
    pub quantity_meters: Decimal,
    pub quantity_kg: Decimal,
    pub warehouse_id: i32,
    pub warehouse_name: Option<String>,
    pub supplier_name: Option<String>,
    pub customer_name: Option<String>,
    pub created_at: chrono::DateTime<chrono::Utc>,
}

/// 正向追溯参数
#[derive(Debug, Deserialize)]
pub struct ForwardTraceParams {
    pub supplier_id: i32,
    pub batch_no: String,
}

/// 反向追溯参数
#[derive(Debug, Deserialize)]
pub struct BackwardTraceParams {
    pub customer_id: i32,
    pub batch_no: String,
}

/// 追溯列表响应
#[derive(Debug, Serialize)]
pub struct TraceListResponse {
    pub traces: Vec<TraceChainResponse>,
    pub total: u64,
}

/// 按五维 ID 查询追溯链
pub async fn get_trace_by_five_dimension(
    State(state): State<AppState>,
    Path(five_dimension_id): Path<String>,
) -> Result<Json<FullTraceChainResponse>, (StatusCode, String)> {
    let service = BusinessTraceService::new(state.db.clone());

    match service
        .find_trace_chain_by_five_dimension(&five_dimension_id)
        .await
    {
        Ok(traces) => {
            if traces.is_empty() {
                return Err((StatusCode::NOT_FOUND, "未找到追溯链".to_string()));
            }

            let first_trace = traces.first().unwrap();
            let last_trace = traces.last().unwrap();
            let total_stages = traces.len();

            // 克隆需要的字段，避免借用冲突
            let trace_chain_id = first_trace.trace_chain_id.clone();
            let five_dimension_id_val = first_trace.five_dimension_id.clone();
            let product_id = first_trace.product_id;
            let batch_no = first_trace.batch_no.clone();
            let color_no = first_trace.color_no.clone();
            let grade = first_trace.grade.clone();
            let start_time = first_trace.created_at;
            let end_time = Some(last_trace.created_at);

            let stages: Vec<TraceStageDetail> = traces
                .into_iter()
                .map(|t| TraceStageDetail {
                    stage_id: t.id,
                    stage_name: get_stage_name(&t.current_stage),
                    stage_type: t.current_stage,
                    bill_type: t.current_bill_type,
                    bill_no: t.current_bill_no,
                    quantity_meters: t.quantity_meters,
                    quantity_kg: t.quantity_kg,
                    warehouse_id: t.warehouse_id,
                    warehouse_name: None,
                    supplier_name: None,
                    customer_name: None,
                    created_at: t.created_at,
                })
                .collect();

            let response = FullTraceChainResponse {
                trace_chain_id,
                five_dimension_id: five_dimension_id_val,
                product_id,
                batch_no,
                color_no,
                grade,
                stages,
                total_stages,
                start_time,
                end_time,
            };

            Ok(Json(response))
        }
        Err(e) => Err((StatusCode::INTERNAL_SERVER_ERROR, e.to_string())),
    }
}

/// 正向追溯
pub async fn forward_trace(
    State(state): State<AppState>,
    Query(params): Query<ForwardTraceParams>,
) -> Result<Json<TraceListResponse>, (StatusCode, String)> {
    let service = BusinessTraceService::new(state.db.clone());

    match service
        .forward_trace(params.supplier_id, &params.batch_no)
        .await
    {
        Ok(traces) => {
            let trace_responses: Vec<TraceChainResponse> = traces
                .into_iter()
                .map(|t| TraceChainResponse {
                    id: t.id,
                    trace_chain_id: t.trace_chain_id,
                    five_dimension_id: t.five_dimension_id,
                    product_id: t.product_id,
                    batch_no: t.batch_no,
                    color_no: t.color_no,
                    dye_lot_no: t.dye_lot_no,
                    grade: t.grade,
                    current_stage: t.current_stage,
                    current_bill_type: t.current_bill_type,
                    current_bill_no: t.current_bill_no,
                    quantity_meters: t.quantity_meters,
                    quantity_kg: t.quantity_kg,
                    warehouse_id: t.warehouse_id,
                    supplier_id: t.supplier_id,
                    customer_id: t.customer_id,
                    trace_status: t.trace_status,
                    created_at: t.created_at,
                })
                .collect();

            let total = trace_responses.len() as u64;

            Ok(Json(TraceListResponse {
                traces: trace_responses,
                total,
            }))
        }
        Err(e) => Err((StatusCode::INTERNAL_SERVER_ERROR, e.to_string())),
    }
}

/// 反向追溯
pub async fn backward_trace(
    State(state): State<AppState>,
    Query(params): Query<BackwardTraceParams>,
) -> Result<Json<TraceListResponse>, (StatusCode, String)> {
    let service = BusinessTraceService::new(state.db.clone());

    match service
        .backward_trace(params.customer_id, &params.batch_no)
        .await
    {
        Ok(traces) => {
            let trace_responses: Vec<TraceChainResponse> = traces
                .into_iter()
                .map(|t| TraceChainResponse {
                    id: t.id,
                    trace_chain_id: t.trace_chain_id,
                    five_dimension_id: t.five_dimension_id,
                    product_id: t.product_id,
                    batch_no: t.batch_no,
                    color_no: t.color_no,
                    dye_lot_no: t.dye_lot_no,
                    grade: t.grade,
                    current_stage: t.current_stage,
                    current_bill_type: t.current_bill_type,
                    current_bill_no: t.current_bill_no,
                    quantity_meters: t.quantity_meters,
                    quantity_kg: t.quantity_kg,
                    warehouse_id: t.warehouse_id,
                    supplier_id: t.supplier_id,
                    customer_id: t.customer_id,
                    trace_status: t.trace_status,
                    created_at: t.created_at,
                })
                .collect();

            let total = trace_responses.len() as u64;

            Ok(Json(TraceListResponse {
                traces: trace_responses,
                total,
            }))
        }
        Err(e) => Err((StatusCode::INTERNAL_SERVER_ERROR, e.to_string())),
    }
}

/// 创建追溯快照
pub async fn create_trace_snapshot(
    State(state): State<AppState>,
    Path(trace_chain_id): Path<String>,
) -> Result<Json<String>, (StatusCode, String)> {
    let service = BusinessTraceService::new(state.db.clone());

    match service.create_snapshot(&trace_chain_id).await {
        Ok(snapshot) => Ok(Json(format!("追溯快照创建成功，快照 ID: {}", snapshot.id))),
        Err(e) => Err((StatusCode::INTERNAL_SERVER_ERROR, e.to_string())),
    }
}

/// 获取环节名称
fn get_stage_name(stage: &str) -> String {
    match stage {
        "PURCHASE_RECEIPT" => "采购收货".to_string(),
        "INVENTORY_IN" => "入库".to_string(),
        "INVENTORY_OUT" => "出库".to_string(),
        "PRODUCTION_INPUT" => "生产投入".to_string(),
        "PRODUCTION_OUTPUT" => "生产产出".to_string(),
        "SALES_DELIVERY" => "销售发货".to_string(),
        _ => format!("未知环节 ({})", stage),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_get_stage_name() {
        assert_eq!(get_stage_name("PURCHASE_RECEIPT"), "采购收货");
        assert_eq!(get_stage_name("SALES_DELIVERY"), "销售发货");
        assert!(get_stage_name("UNKNOWN").contains("未知"));
    }
}
