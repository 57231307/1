use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    Json,
};
use rust_decimal::Decimal;
use crate::utils::app_state::AppState;
use serde::{Deserialize, Serialize};
use tracing::info;

use crate::services::assist_accounting_service::AssistAccountingService;
use crate::utils::error::AppError;
use crate::utils::ApiResponse;

/// 辅助核算维度响应
#[derive(Debug, Serialize)]
pub struct AssistDimensionResponse {
    pub id: i32,
    pub dimension_code: String,
    pub dimension_name: String,
    pub description: Option<String>,
    pub is_active: bool,
    pub sort_order: i32,
}

/// 辅助核算记录响应
#[derive(Debug, Serialize)]
pub struct AssistRecordResponse {
    pub id: i32,
    pub business_type: String,
    pub business_no: String,
    pub business_id: i32,
    pub account_subject_id: i32,
    pub debit_amount: Decimal,
    pub credit_amount: Decimal,
    pub five_dimension_id: String,
    pub product_id: i32,
    pub batch_no: String,
    pub color_no: String,
    pub dye_lot_no: Option<String>,
    pub grade: String,
    pub warehouse_id: i32,
    pub quantity_meters: Decimal,
    pub quantity_kg: Decimal,
    pub workshop_id: Option<i32>,
    pub customer_id: Option<i32>,
    pub supplier_id: Option<i32>,
    pub remarks: Option<String>,
    pub created_at: chrono::DateTime<chrono::Utc>,
}

/// 辅助核算汇总响应
#[derive(Debug, Serialize)]
pub struct AssistSummaryResponse {
    pub id: i32,
    pub accounting_period: String,
    pub dimension_code: String,
    pub dimension_value_id: i32,
    pub dimension_value_name: String,
    pub account_subject_id: i32,
    pub total_debit: Decimal,
    pub total_credit: Decimal,
    pub total_quantity_meters: Decimal,
    pub total_quantity_kg: Decimal,
    pub record_count: i64,
}

/// 辅助核算列表响应
#[derive(Debug, Serialize)]
pub struct AssistRecordListResponse {
    pub records: Vec<AssistRecordResponse>,
    pub total: u64,
    pub page: u64,
    pub page_size: u64,
}

/// 辅助核算查询参数
#[derive(Debug, Deserialize)]
pub struct AssistRecordQueryParams {
    pub accounting_period: Option<String>,
    pub dimension_code: Option<String>,
    pub business_type: Option<String>,
    pub warehouse_id: Option<i32>,
    pub page: Option<u64>,
    pub page_size: Option<u64>,
}

/// 获取所有辅助核算维度
pub async fn list_assist_dimensions(
    State(state): State<AppState>,
) -> Result<Json<ApiResponse<Vec<AssistDimensionResponse>>>, AppError> {
    info!("正在从数据库查询辅助核算维度列表");

    let service = AssistAccountingService::new(state.db.clone());
    let dimensions = service.list_dimensions().await?;

    let response: Vec<AssistDimensionResponse> = dimensions
        .into_iter()
        .map(|d| AssistDimensionResponse {
            id: d.id,
            dimension_code: d.dimension_code,
            dimension_name: d.dimension_name,
            description: d.description,
            is_active: d.is_active,
            sort_order: d.sort_order,
        })
        .collect();

    info!("辅助核算维度列表查询成功，共 {} 条记录", response.len());
    Ok(Json(ApiResponse::success(response)))
}

/// 查询辅助核算记录
pub async fn query_assist_records(
    State(state): State<AppState>,
    Query(params): Query<AssistRecordQueryParams>,
) -> Result<Json<AssistRecordListResponse>, (StatusCode, String)> {
    let service = AssistAccountingService::new(state.db.clone());

    let page = params.page.unwrap_or(0);
    let page_size = params.page_size.unwrap_or(20);

    match service
        .query_assist_records(
            params.accounting_period.as_deref(),
            params.dimension_code.as_deref(),
            params.business_type.as_deref(),
            params.warehouse_id,
            page,
            page_size,
        )
        .await
    {
        Ok((records, total)) => {
            let record_responses: Vec<AssistRecordResponse> = records
                .into_iter()
                .map(|r| AssistRecordResponse {
                    id: r.id,
                    business_type: r.business_type,
                    business_no: r.business_no,
                    business_id: r.business_id,
                    account_subject_id: r.account_subject_id,
                    debit_amount: r.debit_amount,
                    credit_amount: r.credit_amount,
                    five_dimension_id: r.five_dimension_id,
                    product_id: r.product_id,
                    batch_no: r.batch_no,
                    color_no: r.color_no,
                    dye_lot_no: r.dye_lot_no,
                    grade: r.grade,
                    warehouse_id: r.warehouse_id,
                    quantity_meters: r.quantity_meters,
                    quantity_kg: r.quantity_kg,
                    workshop_id: r.workshop_id,
                    customer_id: r.customer_id,
                    supplier_id: r.supplier_id,
                    remarks: r.remarks,
                    created_at: r.created_at,
                })
                .collect();

            Ok(Json(AssistRecordListResponse {
                records: record_responses,
                total,
                page,
                page_size,
            }))
        }
        Err(e) => Err((StatusCode::INTERNAL_SERVER_ERROR, e.to_string())),
    }
}

/// 按业务单查询辅助核算记录
pub async fn get_assist_records_by_business(
    State(state): State<AppState>,
    Query(params): Query<BusinessQueryParams>,
) -> Result<Json<Vec<AssistRecordResponse>>, (StatusCode, String)> {
    let service = AssistAccountingService::new(state.db.clone());

    match service
        .find_by_business(&params.business_type, &params.business_no)
        .await
    {
        Ok(records) => {
            let record_responses: Vec<AssistRecordResponse> = records
                .into_iter()
                .map(|r| AssistRecordResponse {
                    id: r.id,
                    business_type: r.business_type,
                    business_no: r.business_no,
                    business_id: r.business_id,
                    account_subject_id: r.account_subject_id,
                    debit_amount: r.debit_amount,
                    credit_amount: r.credit_amount,
                    five_dimension_id: r.five_dimension_id,
                    product_id: r.product_id,
                    batch_no: r.batch_no,
                    color_no: r.color_no,
                    dye_lot_no: r.dye_lot_no,
                    grade: r.grade,
                    warehouse_id: r.warehouse_id,
                    quantity_meters: r.quantity_meters,
                    quantity_kg: r.quantity_kg,
                    workshop_id: r.workshop_id,
                    customer_id: r.customer_id,
                    supplier_id: r.supplier_id,
                    remarks: r.remarks,
                    created_at: r.created_at,
                })
                .collect();

            Ok(Json(record_responses))
        }
        Err(e) => Err((StatusCode::INTERNAL_SERVER_ERROR, e.to_string())),
    }
}

/// 业务单查询参数
#[derive(Debug, Deserialize)]
pub struct BusinessQueryParams {
    pub business_type: String,
    pub business_no: String,
}

/// 按五维 ID 查询辅助核算记录
pub async fn get_assist_records_by_five_dimension(
    State(state): State<AppState>,
    Path(five_dimension_id): Path<String>,
) -> Result<Json<Vec<AssistRecordResponse>>, (StatusCode, String)> {
    let service = AssistAccountingService::new(state.db.clone());

    match service.find_by_five_dimension(&five_dimension_id).await {
        Ok(records) => {
            let record_responses: Vec<AssistRecordResponse> = records
                .into_iter()
                .map(|r| AssistRecordResponse {
                    id: r.id,
                    business_type: r.business_type,
                    business_no: r.business_no,
                    business_id: r.business_id,
                    account_subject_id: r.account_subject_id,
                    debit_amount: r.debit_amount,
                    credit_amount: r.credit_amount,
                    five_dimension_id: r.five_dimension_id,
                    product_id: r.product_id,
                    batch_no: r.batch_no,
                    color_no: r.color_no,
                    dye_lot_no: r.dye_lot_no,
                    grade: r.grade,
                    warehouse_id: r.warehouse_id,
                    quantity_meters: r.quantity_meters,
                    quantity_kg: r.quantity_kg,
                    workshop_id: r.workshop_id,
                    customer_id: r.customer_id,
                    supplier_id: r.supplier_id,
                    remarks: r.remarks,
                    created_at: r.created_at,
                })
                .collect();

            Ok(Json(record_responses))
        }
        Err(e) => Err((StatusCode::INTERNAL_SERVER_ERROR, e.to_string())),
    }
}

/// 获取辅助核算汇总
pub async fn get_assist_summary(
    State(state): State<AppState>,
    Query(params): Query<AssistSummaryQueryParams>,
) -> Result<Json<ApiResponse<Vec<AssistSummaryResponse>>>, AppError> {
    info!(
        "正在查询辅助核算汇总，期间：{}，维度：{:?}",
        params.accounting_period, params.dimension_code
    );

    let service = AssistAccountingService::new(state.db.clone());

    let dimension_code = params.dimension_code.as_deref().unwrap_or("");

    let summaries = if dimension_code.is_empty() {
        vec![]
    } else {
        service
            .find_summary_by_period_and_dimension(&params.accounting_period, dimension_code)
            .await
            .map_err(|e| AppError::InternalError(e.to_string()))?
            .into_iter()
            .map(|s| AssistSummaryResponse {
                id: s.id,
                accounting_period: s.accounting_period,
                dimension_code: s.dimension_code,
                dimension_value_id: s.dimension_value_id,
                dimension_value_name: s.dimension_value_name,
                account_subject_id: s.account_subject_id,
                total_debit: s.total_debit,
                total_credit: s.total_credit,
                total_quantity_meters: s.total_quantity_meters,
                total_quantity_kg: s.total_quantity_kg,
                record_count: s.record_count,
            })
            .collect()
    };

    info!("辅助核算汇总查询成功，共 {} 条记录", summaries.len());
    Ok(Json(ApiResponse::success(summaries)))
}

/// 辅助核算汇总查询参数
#[derive(Debug, Deserialize)]
pub struct AssistSummaryQueryParams {
    pub accounting_period: String,
    pub dimension_code: Option<String>,
}
