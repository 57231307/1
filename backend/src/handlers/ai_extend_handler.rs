//! P2-4 AI 分析深化 Handler
//!
//! 实现 16 个 HTTP 端点：
//! - 工艺优化（5）：创建 / 列表 / 详情 / 按色号布类 / 应用反馈 / 删除
//! - 质量预测（5）：创建 / 列表 / 详情 / 按产品 / 确认 / 删除
//! - 看板 / 健康检查 / 概览（3）
//! - 批量（2）
//! - 按色号+布类 / 按产品 历史（2）
//!
//! 设计依据：doto.md P2-4 任务清单
//! 创建时间: 2026-06-17

use axum::{
    extract::{Path, Query, State},
    Json,
};
use serde::Deserialize;

use crate::middleware::auth_context::AuthContext;
use crate::services::ai_extend_service::{
    AcknowledgeQualityPredDto, AiExtendService, ApplyProcessOptDto, CreateProcessOptDto,
    CreateQualityPredDto, ListProcessOptQuery, ListQualityPredQuery,
};
use crate::utils::app_state::AppState;
use crate::utils::error::AppError;
use crate::utils::response::ApiResponse;

// =====================================================
// 工艺优化端点（5）
// =====================================================

/// POST /api/v1/erp/ai/process-optimizations
/// 触发工艺优化（算法 + 落库）
pub async fn create_process_optimization(
    State(state): State<AppState>,
    auth: AuthContext,
    Json(body): Json<CreateProcessOptDto>,
) -> Result<Json<ApiResponse<serde_json::Value>>, AppError> {
    let mut dto = body;
    dto.operator_id = Some(auth.user_id as i64);

    let svc = AiExtendService::new(state.db);
    let (resp, id) = svc.create_process_optimization(dto).await?;
    Ok(Json(ApiResponse::success(serde_json::json!({
        "id": id,
        "response": resp,
    }))))
}

/// GET /api/v1/erp/ai/process-optimizations
/// 工艺优化列表
pub async fn list_process_optimizations(
    State(state): State<AppState>,
    _auth: AuthContext,
    Query(q): Query<ListProcessOptQuery>,
) -> Result<Json<ApiResponse<serde_json::Value>>, AppError> {
    let svc = AiExtendService::new(state.db);
    let vo = svc.list_process_optimizations(q).await?;
    Ok(Json(ApiResponse::success(serde_json::json!({
        "items": vo.items,
        "total": vo.total,
        "page": vo.page,
        "page_size": vo.page_size,
    }))))
}

/// GET /api/v1/erp/ai/process-optimizations/:id
/// 工艺优化详情
pub async fn get_process_optimization(
    State(state): State<AppState>,
    _auth: AuthContext,
    Path(id): Path<i64>,
) -> Result<Json<ApiResponse<serde_json::Value>>, AppError> {
    let svc = AiExtendService::new(state.db);
    let model = svc.get_process_optimization(id).await?;
    Ok(Json(ApiResponse::success(serde_json::to_value(model)?)))
}

/// POST /api/v1/erp/ai/process-optimizations/:id/apply
/// 标记工艺优化已应用 + 反馈打分
pub async fn apply_process_optimization(
    State(state): State<AppState>,
    auth: AuthContext,
    Path(id): Path<i64>,
    Json(mut body): Json<ApplyProcessOptDto>,
) -> Result<Json<ApiResponse<serde_json::Value>>, AppError> {
    body.operator_id = Some(auth.user_id as i64);
    let svc = AiExtendService::new(state.db);
    let model = svc.apply_process_optimization(id, body).await?;
    Ok(Json(ApiResponse::success(serde_json::to_value(model)?)))
}

/// DELETE /api/v1/erp/ai/process-optimizations/:id
/// 删除工艺优化记录
pub async fn delete_process_optimization(
    State(state): State<AppState>,
    _auth: AuthContext,
    Path(id): Path<i64>,
) -> Result<Json<ApiResponse<serde_json::Value>>, AppError> {
    let svc = AiExtendService::new(state.db);
    svc.delete_process_optimization(id).await?;
    Ok(Json(ApiResponse::success(serde_json::json!({
        "deleted": true,
        "id": id,
    }))))
}

// =====================================================
// 质量预测端点（5）
// =====================================================

/// POST /api/v1/erp/ai/quality-predictions
/// 触发质量预测（算法 + 落库）
pub async fn create_quality_prediction(
    State(state): State<AppState>,
    auth: AuthContext,
    Json(body): Json<CreateQualityPredDto>,
) -> Result<Json<ApiResponse<serde_json::Value>>, AppError> {
    let mut dto = body;
    dto.operator_id = Some(auth.user_id as i64);

    let svc = AiExtendService::new(state.db);
    let (resp, id) = svc.create_quality_prediction(dto).await?;
    Ok(Json(ApiResponse::success(serde_json::json!({
        "id": id,
        "response": resp,
    }))))
}

/// GET /api/v1/erp/ai/quality-predictions
/// 质量预测列表
pub async fn list_quality_predictions(
    State(state): State<AppState>,
    _auth: AuthContext,
    Query(q): Query<ListQualityPredQuery>,
) -> Result<Json<ApiResponse<serde_json::Value>>, AppError> {
    let svc = AiExtendService::new(state.db);
    let vo = svc.list_quality_predictions(q).await?;
    Ok(Json(ApiResponse::success(serde_json::json!({
        "items": vo.items,
        "total": vo.total,
        "page": vo.page,
        "page_size": vo.page_size,
    }))))
}

/// GET /api/v1/erp/ai/quality-predictions/:id
/// 质量预测详情
pub async fn get_quality_prediction(
    State(state): State<AppState>,
    _auth: AuthContext,
    Path(id): Path<i64>,
) -> Result<Json<ApiResponse<serde_json::Value>>, AppError> {
    let svc = AiExtendService::new(state.db);
    let model = svc.get_quality_prediction(id).await?;
    Ok(Json(ApiResponse::success(serde_json::to_value(model)?)))
}

/// POST /api/v1/erp/ai/quality-predictions/:id/acknowledge
/// 质量预测确认
pub async fn acknowledge_quality_prediction(
    State(state): State<AppState>,
    auth: AuthContext,
    Path(id): Path<i64>,
    Json(mut body): Json<AcknowledgeQualityPredDto>,
) -> Result<Json<ApiResponse<serde_json::Value>>, AppError> {
    body.operator_id = Some(auth.user_id as i64);
    let svc = AiExtendService::new(state.db);
    let model = svc.acknowledge_quality_prediction(id, body).await?;
    Ok(Json(ApiResponse::success(serde_json::to_value(model)?)))
}

/// DELETE /api/v1/erp/ai/quality-predictions/:id
/// 删除质量预测记录
pub async fn delete_quality_prediction(
    State(state): State<AppState>,
    _auth: AuthContext,
    Path(id): Path<i64>,
) -> Result<Json<ApiResponse<serde_json::Value>>, AppError> {
    let svc = AiExtendService::new(state.db);
    svc.delete_quality_prediction(id).await?;
    Ok(Json(ApiResponse::success(serde_json::json!({
        "deleted": true,
        "id": id,
    }))))
}

// =====================================================
// 看板 / 概览 / 历史（4）
// =====================================================

/// GET /api/v1/erp/ai/summary
/// AI 概览（应用率 / 风险等级分布 / 最新 5 条）
pub async fn ai_summary(
    State(state): State<AppState>,
    _auth: AuthContext,
) -> Result<Json<ApiResponse<serde_json::Value>>, AppError> {
    let svc = AiExtendService::new(state.db);
    let summary = svc.ai_summary().await?;
    Ok(Json(ApiResponse::success(summary)))
}

/// GET /api/v1/erp/ai/health
/// AI 服务健康检查（v11 批次 155 P2-C：算法元信息下沉到 AiExtendService::algorithm_metadata）
pub async fn ai_health() -> Result<Json<ApiResponse<serde_json::Value>>, AppError> {
    use crate::services::ai_extend_service::AiExtendService;
    Ok(Json(ApiResponse::success(serde_json::json!({
        "status": "ok",
        "version": "P2-4",
        "modules": AiExtendService::algorithm_metadata(),
    }))))
}

#[derive(Debug, Deserialize)]
pub struct ByColorQuery {
    pub color_no: String,
    pub fabric_type: String,
    pub limit: Option<u64>,
}

/// GET /api/v1/erp/ai/process-optimizations/by-color
/// 按色号 + 布类查询历史
pub async fn list_process_optimizations_by_color(
    State(state): State<AppState>,
    _auth: AuthContext,
    Query(q): Query<ByColorQuery>,
) -> Result<Json<ApiResponse<serde_json::Value>>, AppError> {
    let svc = AiExtendService::new(state.db);
    let items =
        svc.list_process_optimizations_by_color(&q.color_no, &q.fabric_type, q.limit.unwrap_or(20).clamp(1, 100)).await?;
    Ok(Json(ApiResponse::success(serde_json::json!({
        "items": items,
    }))))
}

#[derive(Debug, Deserialize)]
pub struct ByProductQuery {
    pub product_id: i64,
    pub limit: Option<u64>,
}

/// GET /api/v1/erp/ai/quality-predictions/by-product
/// 按产品查询历史
pub async fn list_quality_predictions_by_product(
    State(state): State<AppState>,
    _auth: AuthContext,
    Query(q): Query<ByProductQuery>,
) -> Result<Json<ApiResponse<serde_json::Value>>, AppError> {
    let svc = AiExtendService::new(state.db);
    let items =
        svc.list_quality_predictions_by_product(q.product_id, q.limit.unwrap_or(20).clamp(1, 100)).await?;
    Ok(Json(ApiResponse::success(serde_json::json!({
        "items": items,
    }))))
}

// =====================================================
// 批量（2）
// =====================================================

#[derive(Debug, Deserialize)]
pub struct BatchProcessOptDto {
    pub requests: Vec<CreateProcessOptDto>,
}

/// POST /api/v1/erp/ai/process-optimizations/batch
/// 批量工艺优化（最多 20 条）
pub async fn batch_create_process_optimizations(
    State(state): State<AppState>,
    auth: AuthContext,
    Json(body): Json<BatchProcessOptDto>,
) -> Result<Json<ApiResponse<serde_json::Value>>, AppError> {
    if body.requests.len() > 20 {
        return Err(AppError::validation("批量请求数不得超过 20"));
    }
    let svc = AiExtendService::new(state.db);
    let mut results = Vec::new();
    let mut failed = 0;
    let total = body.requests.len();
    for mut req in body.requests {
        req.operator_id = Some(auth.user_id as i64);
        match svc.create_process_optimization(req).await {
            Ok((resp, id)) => results.push(serde_json::json!({
                "id": id,
                "success": true,
                "recommended_params": resp.recommended_params,
                "confidence": resp.confidence,
                "source": resp.source,
            })),
            Err(e) => {
                failed += 1;
                results.push(serde_json::json!({
                    "success": false,
                    "error": format!("{}", e),
                }));
            }
        }
    }
    Ok(Json(ApiResponse::success(serde_json::json!({
        "total": total,
        "succeeded": total - failed,
        "failed": failed,
        "results": results,
    }))))
}

#[derive(Debug, Deserialize)]
pub struct BatchQualityPredDto {
    pub requests: Vec<CreateQualityPredDto>,
}

/// POST /api/v1/erp/ai/quality-predictions/batch
/// 批量质量预测（最多 20 条）
pub async fn batch_create_quality_predictions(
    State(state): State<AppState>,
    auth: AuthContext,
    Json(body): Json<BatchQualityPredDto>,
) -> Result<Json<ApiResponse<serde_json::Value>>, AppError> {
    if body.requests.len() > 20 {
        return Err(AppError::validation("批量请求数不得超过 20"));
    }
    let svc = AiExtendService::new(state.db);
    let mut results = Vec::new();
    let mut failed = 0;
    let total = body.requests.len();
    for mut req in body.requests {
        req.operator_id = Some(auth.user_id as i64);
        match svc.create_quality_prediction(req).await {
            Ok((resp, id)) => results.push(serde_json::json!({
                "id": id,
                "success": true,
                "risk_score": resp.risk_score,
                "risk_level": resp.risk_level,
                "trend": resp.trend,
                "confidence": resp.confidence,
            })),
            Err(e) => {
                failed += 1;
                results.push(serde_json::json!({
                    "success": false,
                    "error": format!("{}", e),
                }));
            }
        }
    }
    Ok(Json(ApiResponse::success(serde_json::json!({
        "total": total,
        "succeeded": total - failed,
        "failed": failed,
        "results": results,
    }))))
}
