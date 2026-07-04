use axum::{
    extract::{Path, Query, State},
    Json,
};
use chrono::NaiveDate;
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use serde_json::Value as JsonValue;
use tracing::info;

use crate::middleware::auth_context::AuthContext;
use crate::services::ar::{
    ArReconciliationService, AutoMatchRequest, CreateReconciliationRequest,
    GenerateReconciliationRequest, ReconciliationQuery, UpdateReconciliationRequest,
};
use crate::utils::app_state::AppState;
use crate::utils::error::AppError;
use crate::utils::response::{ApiResponse, PaginatedResponse};

#[derive(Debug, Deserialize)]
pub struct CreateReconciliationApiRequest {
    pub reconciliation_no: String,
    pub customer_id: i32,
    pub customer_name: Option<String>,
    pub period_start: NaiveDate,
    pub period_end: NaiveDate,
    pub opening_balance: Decimal,
    pub total_invoices: Decimal,
    pub total_collections: Decimal,
}

#[derive(Debug, Clone, Serialize)]
pub struct ReconciliationResponse {
    pub id: i32,
    pub reconciliation_no: String,
    pub customer_id: i32,
    pub customer_name: Option<String>,
    pub period_start: String,
    pub period_end: String,
    pub opening_balance: String,
    pub total_invoices: String,
    pub total_collections: String,
    pub closing_balance: String,
    pub reconciliation_status: Option<String>,
    pub created_at: String,
}

impl From<crate::models::ar_reconciliation::Model> for ReconciliationResponse {
    fn from(model: crate::models::ar_reconciliation::Model) -> Self {
        Self {
            id: model.id,
            reconciliation_no: model.reconciliation_no,
            customer_id: model.customer_id,
            customer_name: model.customer_name,
            period_start: model.period_start.to_string(),
            period_end: model.period_end.to_string(),
            opening_balance: model.opening_balance.to_string(),
            total_invoices: model.total_invoices.to_string(),
            total_collections: model.total_collections.to_string(),
            closing_balance: model.closing_balance.to_string(),
            reconciliation_status: model.reconciliation_status,
            created_at: model.created_at.to_rfc3339(),
        }
    }
}

pub async fn create_reconciliation(
    State(state): State<AppState>,
    _auth: AuthContext,
    Json(req): Json<CreateReconciliationApiRequest>,
) -> Result<Json<ApiResponse<ReconciliationResponse>>, AppError> {
    let service = ArReconciliationService::new(state.db);
    let create_req = CreateReconciliationRequest {
        reconciliation_no: req.reconciliation_no,
        customer_id: req.customer_id,
        customer_name: req.customer_name,
        period_start: req.period_start,
        period_end: req.period_end,
        opening_balance: req.opening_balance,
        total_invoices: req.total_invoices,
        total_collections: req.total_collections,
        notes: None,
    };

    service
        .create(create_req)
        .await
        .map(|model| Json(ApiResponse::success(ReconciliationResponse::from(model))))
        .map_err(|e| {
            tracing::error!("创建对账单失败: {}", e);
            AppError::internal(format!("创建对账单失败: {}", e))
        })
}

#[derive(Debug, Deserialize)]
pub struct ListReconciliationsQuery {
    pub status: Option<String>,
    pub customer_id: Option<i32>,
    pub page: Option<u64>,
    pub page_size: Option<u64>,
}

pub async fn list_reconciliations(
    State(state): State<AppState>,
    _auth: AuthContext,
    Query(query): Query<ListReconciliationsQuery>,
) -> Result<Json<ApiResponse<PaginatedResponse<ReconciliationResponse>>>, AppError> {
    let service = ArReconciliationService::new(state.db);
    let page = query.page.unwrap_or(1).clamp(1, 1000); // 批次 95 P3-3~8：分页 clamp 防 DoS
    let page_size = query.page_size.unwrap_or(20).clamp(1, 100);
    let req = ReconciliationQuery {
        status: query.status,
        customer_id: query.customer_id,
        page,
        page_size,
    };

    service
        .list(req)
        .await
        .map(|(models, total)| {
            let responses: Vec<ReconciliationResponse> = models
                .into_iter()
                .map(ReconciliationResponse::from)
                .collect();
            Json(ApiResponse::success(PaginatedResponse::new(
                responses, total, page, page_size,
            )))
        })
        .map_err(|e| {
            tracing::error!("获取对账单列表失败: {}", e);
            AppError::internal(format!("获取对账单列表失败: {}", e))
        })
}

pub async fn get_reconciliation(
    State(state): State<AppState>,
    _auth: AuthContext,
    Path(id): Path<i32>,
) -> Result<Json<ApiResponse<ReconciliationResponse>>, AppError> {
    let service = ArReconciliationService::new(state.db);

    service
        .get_by_id(id)
        .await
        .map_err(|e| {
            tracing::error!("获取对账单失败: {}", e);
            AppError::internal(format!("获取对账单失败: {}", e))
        })?
        .map(|model| Json(ApiResponse::success(ReconciliationResponse::from(model))))
        .ok_or_else(|| AppError::not_found(format!("对账单 {} 不存在", id)))
}

#[derive(Debug, Deserialize)]
pub struct UpdateStatusRequest {
    pub status: String,
}

pub async fn update_reconciliation_status(
    State(state): State<AppState>,
    auth: AuthContext,
    Path(id): Path<i32>,
    Json(req): Json<UpdateStatusRequest>,
) -> Result<Json<ApiResponse<ReconciliationResponse>>, AppError> {
    let service = ArReconciliationService::new(state.db);

    service
        .update_status(id, &req.status, auth.user_id)
        .await
        .map(|model| Json(ApiResponse::success(ReconciliationResponse::from(model))))
        .map_err(|e| {
            tracing::error!("更新对账单状态失败: {}", e);
            AppError::internal(format!("更新对账单状态失败: {}", e))
        })
}

// ============================================================================
// 批次 108 P1-6 修复：ar/recon 路由接入（update/delete/confirm/dispute/close/send）
//
// 原状态：service::ar::recon.rs 的 update/delete/confirm/dispute/close/send 方法
// 已完整实现（含事务 + lock_exclusive + 状态门 + 审计日志），但 /ar-reconciliations
// 路由仅挂载 list/create/get/update_status 4 端点，导致 6 个 service 方法无业务入口
// （#[allow(dead_code)] + TODO 标注）。
//
// 修复：新增 6 个 handler 并挂载到 /ar-reconciliations 路由，移除 service 的 dead_code 标注。
// ============================================================================

/// 更新对账单请求体（API 层）
#[derive(Debug, Deserialize)]
pub struct UpdateReconciliationApiRequest {
    pub opening_balance: Option<Decimal>,
    pub total_invoices: Option<Decimal>,
    pub total_collections: Option<Decimal>,
    pub notes: Option<String>,
}

/// 更新对账单（PUT /ar-reconciliations/:id）
///
/// 仅草稿状态可更新，closing_balance 由 service 自动重算
pub async fn update_reconciliation(
    State(state): State<AppState>,
    auth: AuthContext,
    Path(id): Path<i32>,
    Json(req): Json<UpdateReconciliationApiRequest>,
) -> Result<Json<ApiResponse<ReconciliationResponse>>, AppError> {
    info!("用户 {} 更新对账单 ID: {}", auth.username, id);

    let service = ArReconciliationService::new(state.db.clone());
    let update_req = UpdateReconciliationRequest {
        opening_balance: req.opening_balance,
        total_invoices: req.total_invoices,
        total_collections: req.total_collections,
        notes: req.notes,
    };

    service
        .update(id, update_req, auth.user_id)
        .await
        .map(|model| {
            Json(ApiResponse::success_with_message(
                ReconciliationResponse::from(model),
                "对账单更新成功",
            ))
        })
        .map_err(|e| {
            tracing::error!("更新对账单失败: {}", e);
            AppError::internal(format!("更新对账单失败: {}", e))
        })
}

/// 删除对账单（DELETE /ar-reconciliations/:id）
///
/// 仅草稿状态可删除（service 内部状态门）
pub async fn delete_reconciliation(
    State(state): State<AppState>,
    auth: AuthContext,
    Path(id): Path<i32>,
) -> Result<Json<ApiResponse<()>>, AppError> {
    info!("用户 {} 删除对账单 ID: {}", auth.username, id);

    let service = ArReconciliationService::new(state.db.clone());

    service
        .delete(id, auth.user_id)
        .await
        .map(|_| Json(ApiResponse::success_with_message((), "对账单删除成功")))
        .map_err(|e| {
            tracing::error!("删除对账单失败: {}", e);
            AppError::internal(format!("删除对账单失败: {}", e))
        })
}

/// 发送对账单给客户（POST /ar-reconciliations/:id/send）
///
/// 状态：draft → sent
pub async fn send_reconciliation(
    State(state): State<AppState>,
    auth: AuthContext,
    Path(id): Path<i32>,
) -> Result<Json<ApiResponse<ReconciliationResponse>>, AppError> {
    info!("用户 {} 发送对账单 ID: {}", auth.username, id);

    let service = ArReconciliationService::new(state.db.clone());

    service
        .send(id, auth.user_id)
        .await
        .map(|model| {
            Json(ApiResponse::success_with_message(
                ReconciliationResponse::from(model),
                "对账单已发送给客户",
            ))
        })
        .map_err(|e| {
            tracing::error!("发送对账单失败: {}", e);
            AppError::internal(format!("发送对账单失败: {}", e))
        })
}

// 客户确认对账单（POST /ar-reconciliations/:id/confirm）
//
// 状态：sent → confirmed
//
// 注：confirm/dispute 的业务实现复用现有 enhanced 版本（customer_confirm/customer_dispute，
// 在 service::ar::vfy 中实现，包含自动对账逻辑），不在此处重复定义。
// 路由层通过 `ar_reconciliation_handler::confirm_reconciliation` / `dispute_reconciliation` 引用。

/// 关闭对账单（POST /ar-reconciliations/:id/close）
///
/// 状态：confirmed/disputed → closed
pub async fn close_reconciliation(
    State(state): State<AppState>,
    auth: AuthContext,
    Path(id): Path<i32>,
) -> Result<Json<ApiResponse<ReconciliationResponse>>, AppError> {
    info!("用户 {} 关闭对账单 ID: {}", auth.username, id);

    let service = ArReconciliationService::new(state.db.clone());

    service
        .close(id, auth.user_id)
        .await
        .map(|model| {
            Json(ApiResponse::success_with_message(
                ReconciliationResponse::from(model),
                "对账单已关闭",
            ))
        })
        .map_err(|e| {
            tracing::error!("关闭对账单失败: {}", e);
            AppError::internal(format!("关闭对账单失败: {}", e))
        })
}

// ============================================================================
// 增强版合并自 ar_reconciliation_enhanced_handler.rs
// 包含自动对账、账龄分析、对账明细、客户确认/争议、PDF 导出等高级能力
// ============================================================================

/// 自动对账请求参数
#[derive(Debug, Deserialize)]
pub struct AutoMatchQueryParams {
    pub customer_id: Option<i32>,
    pub start_date: NaiveDate,
    pub end_date: NaiveDate,
    pub match_strategy: Option<String>,
}

/// 账龄分析查询参数
#[derive(Debug, Deserialize)]
pub struct AgingReportQueryParams {
    pub customer_id: Option<i32>,
}

/// 确认对账单请求
#[derive(Debug, Deserialize)]
pub struct ConfirmRequest {}

/// 争议处理请求
#[derive(Debug, Deserialize)]
pub struct DisputeRequest {
    pub reason: String,
}

/// 生成对账单 API 请求体
#[derive(Debug, Deserialize)]
pub struct GenerateReconciliationApiRequest {
    pub customer_id: i32,
    pub start_date: NaiveDate,
    pub end_date: NaiveDate,
    pub notes: Option<String>,
}

/// 自动对账结果列表查询参数
#[derive(Debug, Deserialize)]
pub struct ListResultsQuery {
    pub page: Option<u64>,
    pub page_size: Option<u64>,
    pub customer_id: Option<i32>,
    #[allow(dead_code)] // TODO(tech-debt): 对账模块接入业务后移除
    pub start_date: Option<NaiveDate>,
    #[allow(dead_code)] // TODO(tech-debt): 对账模块接入业务后移除
    pub end_date: Option<NaiveDate>,
}

/// 客户确认请求体
#[derive(Debug, Deserialize)]
pub struct CreateConfirmationRequest {
    pub reconciliation_id: i32,
}

/// 更新确认状态请求
#[derive(Debug, Deserialize)]
pub struct UpdateConfirmationStatusRequest {
    pub status: String,
    #[allow(dead_code)] // TODO(tech-debt): 对账模块接入业务后移除
    pub remark: Option<String>,
}

/// 创建争议请求体
#[derive(Debug, Deserialize)]
pub struct CreateDisputeApiRequest {
    pub reconciliation_id: Option<i32>,
    #[allow(dead_code)] // TODO(tech-debt): 对账模块接入业务后移除
    pub customer_id: Option<i32>,
    pub reason: Option<String>,
    pub description: Option<String>,
}

/// 解决争议请求体
#[derive(Debug, Deserialize)]
pub struct ResolveDisputeRequest {
    pub resolution: String,
    pub status: Option<String>,
}

/// 自动对账：按客户匹配发票和收款，生成对账单及明细
pub async fn auto_match(
    State(state): State<AppState>,
    auth: AuthContext,
    Json(req): Json<AutoMatchQueryParams>,
) -> Result<Json<ApiResponse<JsonValue>>, AppError> {
    info!(
        "用户 {} 执行自动对账，客户ID: {:?}, 期间: {} ~ {}",
        auth.username, req.customer_id, req.start_date, req.end_date
    );

    let service = ArReconciliationService::new(state.db.clone());
    let match_req = AutoMatchRequest {
        customer_id: req.customer_id,
        start_date: req.start_date,
        end_date: req.end_date,
        match_strategy: req.match_strategy,
    };

    let results = service.auto_match(match_req, auth.user_id).await?;

    let success_count = results.len();
    info!(
        "用户 {} 自动对账完成，处理 {} 个客户",
        auth.username, success_count
    );

    Ok(Json(ApiResponse::success_with_message(
        serde_json::to_value(&results)?,
        &format!("自动对账完成，共处理 {} 个客户", success_count),
    )))
}

/// 账龄分析：按 0-30天/31-60天/61-90天/90天以上 分桶统计
pub async fn aging_report(
    State(state): State<AppState>,
    auth: AuthContext,
    Query(params): Query<AgingReportQueryParams>,
) -> Result<Json<ApiResponse<JsonValue>>, AppError> {
    info!(
        "用户 {} 查询账龄分析，客户ID: {:?}",
        auth.username, params.customer_id
    );

    let service = ArReconciliationService::new(state.db.clone());
    let report = service.get_aging_report(params.customer_id).await?;

    info!(
        "用户 {} 账龄分析完成，应收总额: {}",
        auth.username, report.total_receivable
    );

    Ok(Json(ApiResponse::success(serde_json::to_value(report)?)))
}

/// 获取对账单明细：返回对账单及其所有明细行
pub async fn get_reconciliation_details(
    State(state): State<AppState>,
    auth: AuthContext,
    Path(id): Path<i32>,
) -> Result<Json<ApiResponse<JsonValue>>, AppError> {
    info!("用户 {} 查询对账单明细 ID: {}", auth.username, id);

    let service = ArReconciliationService::new(state.db.clone());
    let details = service.get_with_details(id).await?;

    info!(
        "用户 {} 查询对账单明细成功，共 {} 条明细",
        auth.username,
        details.details.len()
    );

    Ok(Json(ApiResponse::success(serde_json::to_value(details)?)))
}

/// 客户确认对账单
pub async fn confirm_reconciliation(
    State(state): State<AppState>,
    auth: AuthContext,
    Path(id): Path<i32>,
    Json(_req): Json<ConfirmRequest>,
) -> Result<Json<ApiResponse<JsonValue>>, AppError> {
    info!("用户 {} 确认对账单 ID: {}", auth.username, id);

    let service = ArReconciliationService::new(state.db.clone());
    let reconciliation = service.customer_confirm(id, auth.user_id).await?;

    info!(
        "用户 {} 确认对账单成功：{}",
        auth.username, reconciliation.reconciliation_no
    );

    Ok(Json(ApiResponse::success_with_message(
        serde_json::to_value(reconciliation)?,
        "对账单确认成功",
    )))
}

/// 客户对对账单提出争议
pub async fn dispute_reconciliation(
    State(state): State<AppState>,
    auth: AuthContext,
    Path(id): Path<i32>,
    Json(req): Json<DisputeRequest>,
) -> Result<Json<ApiResponse<JsonValue>>, AppError> {
    info!(
        "用户 {} 对账单 ID: {} 提出争议，原因：{}",
        auth.username, id, req.reason
    );

    let service = ArReconciliationService::new(state.db.clone());
    let reconciliation = service
        .customer_dispute(id, req.reason, auth.user_id)
        .await?;

    info!(
        "用户 {} 对账单争议提交成功：{}",
        auth.username, reconciliation.reconciliation_no
    );

    Ok(Json(ApiResponse::success_with_message(
        serde_json::to_value(reconciliation)?,
        "争议提交成功",
    )))
}

/// 为指定客户自动生成对账单（从发票/收款汇总）
pub async fn generate_reconciliation(
    State(state): State<AppState>,
    auth: AuthContext,
    Json(req): Json<GenerateReconciliationApiRequest>,
) -> Result<Json<ApiResponse<JsonValue>>, AppError> {
    info!(
        "用户 {} 生成对账单，客户 ID: {}",
        auth.username, req.customer_id
    );

    let service = ArReconciliationService::new(state.db.clone());
    let gen_req = GenerateReconciliationRequest {
        customer_id: req.customer_id,
        start_date: req.start_date,
        end_date: req.end_date,
        notes: req.notes,
    };

    let reconciliation = service
        .generate_reconciliation(gen_req, auth.user_id)
        .await?;

    info!(
        "用户 {} 生成对账单成功：{}",
        auth.username, reconciliation.reconciliation_no
    );

    Ok(Json(ApiResponse::success_with_message(
        serde_json::to_value(reconciliation)?,
        "对账单生成成功",
    )))
}

/// 导出对账单PDF（base64 编码）
pub async fn export_reconciliation_pdf(
    State(state): State<AppState>,
    auth: AuthContext,
    Path(id): Path<i32>,
) -> Result<Json<ApiResponse<JsonValue>>, AppError> {
    info!("用户 {} 导出对账单PDF，ID: {}", auth.username, id);

    let service = ArReconciliationService::new(state.db.clone());
    let pdf_content = service.export_pdf(id).await?;

    use base64::Engine;
    let encoded = base64::engine::general_purpose::STANDARD.encode(&pdf_content);

    Ok(Json(ApiResponse::success_with_message(
        serde_json::json!({
            "id": id,
            "content_type": "application/pdf",
            "size": pdf_content.len(),
            "base64_content": encoded,
        }),
        "PDF导出成功",
    )))
}

/// 自动对账结果列表（与前端别名路由对齐，复用对账单列表接口）
pub async fn list_results(
    State(state): State<AppState>,
    _auth: AuthContext,
    Query(params): Query<ListResultsQuery>,
) -> Result<Json<ApiResponse<JsonValue>>, AppError> {
    let service = ArReconciliationService::new(state.db.clone());
    let page = params.page.unwrap_or(1).clamp(1, 1000);
    let page_size = params.page_size.unwrap_or(20).clamp(1, 100); // v10 P2-4 修复：移除冗余 max(1)（clamp 已保证 >=1）
    let query = ReconciliationQuery {
        status: None,
        customer_id: params.customer_id,
        page,
        page_size,
    };

    let (items, total) = service.list(query).await?;

    Ok(Json(ApiResponse::success(serde_json::json!({
        "list": serde_json::to_value(items)?,
        "total": total,
        "page": page,
        "page_size": page_size,
    }))))
}

/// 发送对账单给客户进行确认
pub async fn send_confirmation(
    State(state): State<AppState>,
    auth: AuthContext,
    Path(id): Path<i32>,
) -> Result<Json<ApiResponse<JsonValue>>, AppError> {
    info!("用户 {} 发送对账单 ID: {} 给客户确认", auth.username, id);

    let service = ArReconciliationService::new(state.db.clone());
    let reconciliation = service.send(id, auth.user_id).await?;

    info!(
        "用户 {} 发送对账单成功：{}",
        auth.username, reconciliation.reconciliation_no
    );

    Ok(Json(ApiResponse::success_with_message(
        serde_json::to_value(reconciliation)?,
        "对账单发送成功",
    )))
}

/// 客户确认记录列表
pub async fn list_confirmations(
    State(state): State<AppState>,
    _auth: AuthContext,
    Query(params): Query<ListResultsQuery>,
) -> Result<Json<ApiResponse<JsonValue>>, AppError> {
    let service = ArReconciliationService::new(state.db.clone());
    let page = params.page.unwrap_or(1).clamp(1, 1000);
    let page_size = params.page_size.unwrap_or(20).clamp(1, 100); // v10 P2-4 修复：移除冗余 max(1)（clamp 已保证 >=1）
    let query = ReconciliationQuery {
        status: Some("confirmed".to_string()),
        customer_id: params.customer_id,
        page,
        page_size,
    };

    let (items, total) = service.list(query).await?;

    Ok(Json(ApiResponse::success(serde_json::json!({
        "list": serde_json::to_value(items)?,
        "total": total,
        "page": page,
        "page_size": page_size,
    }))))
}

/// 创建客户确认记录：基于已存在的对账单触发客户确认
pub async fn create_confirmation(
    State(state): State<AppState>,
    auth: AuthContext,
    Json(req): Json<CreateConfirmationRequest>,
) -> Result<Json<ApiResponse<JsonValue>>, AppError> {
    info!(
        "用户 {} 为对账单 {} 触发客户确认",
        auth.username, req.reconciliation_id
    );

    let service = ArReconciliationService::new(state.db.clone());
    let reconciliation = service
        .customer_confirm(req.reconciliation_id, auth.user_id)
        .await?;

    info!(
        "用户 {} 创建客户确认成功：{}",
        auth.username, reconciliation.reconciliation_no
    );

    Ok(Json(ApiResponse::success_with_message(
        serde_json::to_value(reconciliation)?,
        "客户确认创建成功",
    )))
}

/// 更新客户确认状态：复用对账单通用状态更新
pub async fn update_confirmation_status(
    State(state): State<AppState>,
    auth: AuthContext,
    Path(id): Path<i32>,
    Json(req): Json<UpdateConfirmationStatusRequest>,
) -> Result<Json<ApiResponse<JsonValue>>, AppError> {
    info!(
        "用户 {} 更新确认状态，ID: {}, 新状态: {}",
        auth.username, id, req.status
    );

    let service = ArReconciliationService::new(state.db.clone());
    let reconciliation = service.update_status(id, &req.status, auth.user_id).await?;

    Ok(Json(ApiResponse::success_with_message(
        serde_json::to_value(reconciliation)?,
        "确认状态更新成功",
    )))
}

/// 争议列表
pub async fn list_disputes(
    State(state): State<AppState>,
    _auth: AuthContext,
    Query(params): Query<ListResultsQuery>,
) -> Result<Json<ApiResponse<JsonValue>>, AppError> {
    let service = ArReconciliationService::new(state.db.clone());
    let page = params.page.unwrap_or(1).clamp(1, 1000);
    let page_size = params.page_size.unwrap_or(20).clamp(1, 100); // v10 P2-4 修复：移除冗余 max(1)（clamp 已保证 >=1）
    let query = ReconciliationQuery {
        status: Some("disputed".to_string()),
        customer_id: params.customer_id,
        page,
        page_size,
    };

    let (items, total) = service.list(query).await?;

    Ok(Json(ApiResponse::success(serde_json::json!({
        "list": serde_json::to_value(items)?,
        "total": total,
        "page": page,
        "page_size": page_size,
    }))))
}

/// 客户提出争议：复用对账单 customer_dispute 业务方法
pub async fn create_dispute(
    State(state): State<AppState>,
    auth: AuthContext,
    Json(req): Json<CreateDisputeApiRequest>,
) -> Result<Json<ApiResponse<JsonValue>>, AppError> {
    let reconciliation_id = req
        .reconciliation_id
        .ok_or_else(|| AppError::bad_request("reconciliation_id 不能为空".to_string()))?;
    let reason = req
        .description
        .clone()
        .or(req.reason.clone())
        .unwrap_or_else(|| "未填写争议原因".to_string());

    info!(
        "用户 {} 为对账单 {} 提交争议：{}",
        auth.username, reconciliation_id, reason
    );

    let service = ArReconciliationService::new(state.db.clone());
    let reconciliation = service
        .customer_dispute(reconciliation_id, reason, auth.user_id)
        .await?;

    Ok(Json(ApiResponse::success_with_message(
        serde_json::to_value(reconciliation)?,
        "争议提交成功",
    )))
}

/// 获取争议详情（复用对账单详情查询）
pub async fn get_dispute(
    State(state): State<AppState>,
    _auth: AuthContext,
    Path(id): Path<i32>,
) -> Result<Json<ApiResponse<JsonValue>>, AppError> {
    let service = ArReconciliationService::new(state.db.clone());
    let model = service
        .get_by_id(id)
        .await?
        .ok_or_else(|| AppError::not_found("争议记录不存在"))?;

    Ok(Json(ApiResponse::success(serde_json::to_value(model)?)))
}

/// 解决争议：复用对账单通用状态更新
pub async fn resolve_dispute(
    State(state): State<AppState>,
    auth: AuthContext,
    Path(id): Path<i32>,
    Json(req): Json<ResolveDisputeRequest>,
) -> Result<Json<ApiResponse<JsonValue>>, AppError> {
    info!(
        "用户 {} 解决争议 ID: {}, 方案: {}",
        auth.username, id, req.resolution
    );

    let service = ArReconciliationService::new(state.db.clone());
    let target_status = req.status.as_deref().unwrap_or("resolved");
    let reconciliation = service.update_status(id, target_status, auth.user_id).await?;

    Ok(Json(ApiResponse::success_with_message(
        serde_json::to_value(reconciliation)?,
        "争议已解决",
    )))
}
