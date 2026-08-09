//! P2-4 AI 分析深化 Handler
//!
//! 实现 16 个 HTTP 端点：工艺优化 5 + 质量预测 5 + 看板/健康/概览 3 + 批量 2 + 历史查询 2
//!
//! 设计依据：doto.md P2-4 任务清单
//! 创建时间: 2026-06-17
//!
//! V15 P0-S26：AI 端点权限码注册（对应 PERMISSION_RESOURCES 中 ai-* 资源）
//! 权限映射：工艺优化端点 → ai-process-opt:read/write，
//! 质量预测端点 → ai-quality-pred:read/write，
//! /ai/summary → ai-summary:read，
//! /ai/health → 无权限码（健康检查，公开）
//! V15 P0-S27：所有查询/写操作端点接入 data_scope 过滤 + 资源归属校验（IDOR 防护）。
//! 销售员仅看自己创建的 AI 推理记录，部门经理看本部门（AI 表无 department_id，Dept 退化为 Self），管理员看全部。

use axum::{
    extract::{Path, Query, State},
    Json,
};
use serde::Deserialize;

use crate::container::AppState;
use crate::middleware::auth_context::AuthContext;
use crate::services::ai_extend_service::{
    AcknowledgeQualityPredDto, AiExtendService, ApplyProcessOptDto, CreateProcessOptDto,
    CreateQualityPredDto, ListProcessOptQuery, ListQualityPredQuery,
};
use crate::services::ai::recipe_opt::RecipeOptResponse;
use crate::utils::error::AppError;
use crate::utils::response::ApiResponse;

// =====================================================
// V15 P1 1.3+8.1 / 8.2 / 2.1+8.3 集成端点请求体
// =====================================================

/// V15 P1 1.3+8.1：推送工艺优化到化验室打样的请求体
#[derive(Debug, Deserialize, Default)]
pub struct PushToLabDipDto {
    /// 可选：覆盖默认对色光源（D65）
    pub light_source: Option<String>,
    /// 可选：覆盖默认打样版数（4）
    pub sample_versions: Option<i32>,
}

/// V15 P1 8.2：关联生产配方 ID 的请求体
#[derive(Debug, Deserialize)]
pub struct LinkToProductionDto {
    pub production_recipe_id: i32,
}

/// V15 P1 2.1+8.3：回填质量预测实际结果的请求体
#[derive(Debug, Deserialize)]
pub struct RecordActualResultDto {
    pub actual_risk_level: String,
    pub actual_avg_qualification_rate: rust_decimal::Decimal,
}

/// V15 P2 14.2.3：回填实际结果和索赔金额（误判成本追踪）的请求体
#[derive(Debug, Deserialize)]
pub struct RecordActualGradeDto {
    pub actual_grade: String,
    pub claim_amount: Option<rust_decimal::Decimal>,
}

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
    // V15 P2 14-4.5：输入校验（color_no / fabric_type 长度 + dye_type 枚举）
    if body.request.color_no.trim().is_empty() || body.request.color_no.len() > 64 {
        return Err(AppError::validation("color_no 长度须在 1-64 之间"));
    }
    if body.request.fabric_type.trim().is_empty() || body.request.fabric_type.len() > 64 {
        return Err(AppError::validation("fabric_type 长度须在 1-64 之间"));
    }
    if let Some(ref dye) = body.request.dye_type {
        let valid_dyes = [
            "reactive", "活性", "disperse", "分散", "acid", "酸性",
            "vat", "还原", "direct", "直接", "cationic", "阳离子", "sulfur", "硫化",
        ];
        if !dye.trim().is_empty() && !valid_dyes.contains(&dye.as_str()) {
            return Err(AppError::validation(format!(
                "dye_type 不合法，允许值：{}", valid_dyes.join("/")
            )));
        }
    }

    let mut dto = body;
    dto.operator_id = Some(auth.user_id as i64);

    let svc = AiExtendService::new(state.db);
    let (resp, id) = svc.create_process_optimization(dto).await?;

    // batch-14 P3: AI 操作审计 - 区分敏感操作
    tracing::info!(
        target: "ai_audit",
        user_id = auth.user_id,
        operation = "create_process_optimization",
        sensitivity = "medium",
        resource_id = id,
        "AI 工艺优化请求"
    );

    Ok(Json(ApiResponse::success(serde_json::json!({
        "id": id,
        "response": resp,
    }))))
}

/// GET /api/v1/erp/ai/process-optimizations
/// 工艺优化列表
pub async fn list_process_optimizations(
    State(state): State<AppState>,
    auth: AuthContext,
    Query(q): Query<ListProcessOptQuery>,
) -> Result<Json<ApiResponse<serde_json::Value>>, AppError> {
    let data_scope_ctx = auth.to_data_scope_context();
    let svc = AiExtendService::new(state.db);
    let vo = svc
        .list_process_optimizations(q, Some(&data_scope_ctx))
        .await?;
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
    auth: AuthContext,
    Path(id): Path<i64>,
) -> Result<Json<ApiResponse<serde_json::Value>>, AppError> {
    let data_scope_ctx = auth.to_data_scope_context();
    let svc = AiExtendService::new(state.db);
    let model = svc
        .get_process_optimization(id, Some(&data_scope_ctx))
        .await?;
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
    let data_scope_ctx = auth.to_data_scope_context();
    body.operator_id = Some(auth.user_id as i64);
    let svc = AiExtendService::new(state.db);
    let model = svc
        .apply_process_optimization(id, body, Some(&data_scope_ctx))
        .await?;
    Ok(Json(ApiResponse::success(serde_json::to_value(model)?)))
}

/// DELETE /api/v1/erp/ai/process-optimizations/:id
/// 删除工艺优化记录
pub async fn delete_process_optimization(
    State(state): State<AppState>,
    auth: AuthContext,
    Path(id): Path<i64>,
) -> Result<Json<ApiResponse<serde_json::Value>>, AppError> {
    let data_scope_ctx = auth.to_data_scope_context();
    let svc = AiExtendService::new(state.db);
    svc.delete_process_optimization(id, Some(&data_scope_ctx))
        .await?;
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
    auth: AuthContext,
    Query(q): Query<ListQualityPredQuery>,
) -> Result<Json<ApiResponse<serde_json::Value>>, AppError> {
    let data_scope_ctx = auth.to_data_scope_context();
    let svc = AiExtendService::new(state.db);
    let vo = svc
        .list_quality_predictions(q, Some(&data_scope_ctx))
        .await?;
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
    auth: AuthContext,
    Path(id): Path<i64>,
) -> Result<Json<ApiResponse<serde_json::Value>>, AppError> {
    let data_scope_ctx = auth.to_data_scope_context();
    let svc = AiExtendService::new(state.db);
    let model = svc
        .get_quality_prediction(id, Some(&data_scope_ctx))
        .await?;
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
    let data_scope_ctx = auth.to_data_scope_context();
    body.operator_id = Some(auth.user_id as i64);
    let svc = AiExtendService::new(state.db);
    let model = svc
        .acknowledge_quality_prediction(id, body, Some(&data_scope_ctx))
        .await?;
    Ok(Json(ApiResponse::success(serde_json::to_value(model)?)))
}

/// DELETE /api/v1/erp/ai/quality-predictions/:id
/// 删除质量预测记录
pub async fn delete_quality_prediction(
    State(state): State<AppState>,
    auth: AuthContext,
    Path(id): Path<i64>,
) -> Result<Json<ApiResponse<serde_json::Value>>, AppError> {
    let data_scope_ctx = auth.to_data_scope_context();
    let svc = AiExtendService::new(state.db);
    svc.delete_quality_prediction(id, Some(&data_scope_ctx))
        .await?;
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
    auth: AuthContext,
) -> Result<Json<ApiResponse<serde_json::Value>>, AppError> {
    let data_scope_ctx = auth.to_data_scope_context();
    let svc = AiExtendService::new(state.db);
    let summary = svc.ai_summary(Some(&data_scope_ctx)).await?;
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
    auth: AuthContext,
    Query(q): Query<ByColorQuery>,
) -> Result<Json<ApiResponse<serde_json::Value>>, AppError> {
    let data_scope_ctx = auth.to_data_scope_context();
    let svc = AiExtendService::new(state.db);
    let items = svc
        .list_process_optimizations_by_color(
            &q.color_no,
            &q.fabric_type,
            q.limit.unwrap_or(20).clamp(1, 100),
            Some(&data_scope_ctx),
        )
        .await?;
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
    auth: AuthContext,
    Query(q): Query<ByProductQuery>,
) -> Result<Json<ApiResponse<serde_json::Value>>, AppError> {
    let data_scope_ctx = auth.to_data_scope_context();
    let svc = AiExtendService::new(state.db);
    let items = svc
        .list_quality_predictions_by_product(
            q.product_id,
            q.limit.unwrap_or(20).clamp(1, 100),
            Some(&data_scope_ctx),
        )
        .await?;
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
/// 批量工艺优化（最多 20 条，单事务原子写入）
pub async fn batch_create_process_optimizations(
    State(state): State<AppState>,
    auth: AuthContext,
    Json(body): Json<BatchProcessOptDto>,
) -> Result<Json<ApiResponse<serde_json::Value>>, AppError> {
    if body.requests.is_empty() {
        return Err(AppError::validation("批量请求不能为空"));
    }
    if body.requests.len() > 20 {
        return Err(AppError::validation("批量请求数不得超过 20"));
    }
    let svc = AiExtendService::new(state.db);
    let mut results = Vec::new();
    let mut failed = 0;
    let total = body.requests.len();
    // 阶段一：逐条调用 AI 算法（不做 DB 写入）
    let mut ai_results: Vec<(RecipeOptResponse, CreateProcessOptDto)> = Vec::new();
    for mut req in body.requests {
        req.operator_id = Some(auth.user_id as i64);
        match svc.optimize_recipe_only(&req).await {
            Ok(resp) => ai_results.push((resp, req)),
            Err(e) => {
                failed += 1;
                results.push(serde_json::json!({
                    "success": false,
                    "error": format!("{}", e),
                }));
            }
        }
    }
    // 阶段二：批量落库（单事务，全部成功或全部回滚）
    if !ai_results.is_empty() {
        let ai_count = ai_results.len();
        match svc.batch_insert_optimizations(ai_results).await {
            Ok(ids) => {
                for id in ids {
                    results.push(serde_json::json!({
                        "id": id,
                        "success": true,
                    }));
                }
            }
            Err(e) => {
                failed += ai_count;
                for _ in 0..ai_count {
                    results.push(serde_json::json!({
                        "success": false,
                        "error": format!("落库失败: {}", e),
                    }));
                }
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

// =====================================================
// V15 P1 1.3+8.1 / 8.2 / 2.1+8.3 集成端点
// =====================================================

/// POST /api/v1/erp/ai/process-optimizations/:id/push-to-lab-dip
/// V15 P1 1.3+8.1：将工艺优化推荐参数推送到化验室打样系统
pub async fn push_to_lab_dip(
    State(state): State<AppState>,
    auth: AuthContext,
    Path(id): Path<i64>,
    Json(_body): Json<PushToLabDipDto>,
) -> Result<Json<ApiResponse<serde_json::Value>>, AppError> {
    let data_scope_ctx = auth.to_data_scope_context();
    let svc = AiExtendService::new(state.db);
    let lab_dip_id = svc
        .push_to_lab_dip(id, auth.user_id as i64, Some(&data_scope_ctx))
        .await?;
    Ok(Json(ApiResponse::success(serde_json::json!({
        "process_optimization_id": id,
        "lab_dip_request_id": lab_dip_id,
        "pushed": true,
    }))))
}

/// POST /api/v1/erp/ai/process-optimizations/:id/link-to-production
/// V15 P1 8.2：将工艺优化推荐参数关联到生产配方
pub async fn link_to_production(
    State(state): State<AppState>,
    auth: AuthContext,
    Path(id): Path<i64>,
    Json(body): Json<LinkToProductionDto>,
) -> Result<Json<ApiResponse<serde_json::Value>>, AppError> {
    let data_scope_ctx = auth.to_data_scope_context();
    let svc = AiExtendService::new(state.db);
    let model = svc
        .link_to_production_recipe(id, body.production_recipe_id, Some(&data_scope_ctx))
        .await?;
    Ok(Json(ApiResponse::success(serde_json::to_value(model)?)))
}

/// POST /api/v1/erp/ai/quality-predictions/:id/actual-result
/// V15 P1 2.1+8.3：回填质量预测的实际结果（来自质检记录对账）
pub async fn record_actual_quality_result(
    State(state): State<AppState>,
    auth: AuthContext,
    Path(id): Path<i64>,
    Json(body): Json<RecordActualResultDto>,
) -> Result<Json<ApiResponse<serde_json::Value>>, AppError> {
    let data_scope_ctx = auth.to_data_scope_context();
    let svc = AiExtendService::new(state.db);
    let model = svc
        .record_actual_quality_result(
            id,
            body.actual_risk_level,
            body.actual_avg_qualification_rate,
            Some(&data_scope_ctx),
        )
        .await?;
    Ok(Json(ApiResponse::success(serde_json::to_value(model)?)))
}

/// POST /api/v1/erp/ai/quality-predictions/:id/actual-grade
/// V15 P2 14.2.3：回填实际结果和索赔金额（误判成本追踪）
pub async fn record_actual_grade(
    State(state): State<AppState>,
    auth: AuthContext,
    Path(id): Path<i64>,
    Json(body): Json<RecordActualGradeDto>,
) -> Result<Json<ApiResponse<serde_json::Value>>, AppError> {
    let data_scope_ctx = auth.to_data_scope_context();
    let svc = AiExtendService::new(state.db);
    let model = svc
        .record_actual_result(
            id,
            body.actual_grade,
            body.claim_amount,
            Some(&data_scope_ctx),
        )
        .await?;
    Ok(Json(ApiResponse::success(serde_json::to_value(model)?)))
}
