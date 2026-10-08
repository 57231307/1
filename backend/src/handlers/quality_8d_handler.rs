//! 8D 质量管理流程 Handler
//!
//! 实现 7 个 HTTP 端点：
//!   - 列表/详情/按质量异常查询（GET）
//!   - 启动 8D 流程（POST /）— not_started → d0_plan
//!   - 推进下一 D 阶段（POST /:id/advance）— 8 条合法边
//!   - 关闭 8D 流程（POST /:id/close）— d8_recognize → closed

use axum::{
    Json,
    extract::{Path, Query, State},
};
use chrono::{DateTime, NaiveDate, Utc};
use sea_orm::EntityTrait;
use serde::{Deserialize, Serialize};

use crate::container::AppState;
use crate::middleware::auth_context::AuthContext;
use crate::models::custom_order;
use crate::models::quality_8d_dto::{
    AdvanceStepPayload, CloseEightDRequest, ListEightDQuery, StartEightDRequest,
};
use crate::models::quality_8d_report;
use crate::models::quality_issue;
use crate::services::quality_8d_service::{EightDError, QualityEightDService};
use crate::utils::data_scope::check_resource_owner_by_member_scope;
use crate::utils::error::AppError;
use crate::utils::response::ApiResponse;

// ==================== DTO 定义 ====================

/// 启动 8D 流程请求 DTO
#[allow(dead_code, reason = "序列化/反序列化字段")]
#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct StartEightDDto {
    pub quality_issue_id: i64,
    /// D0 准备阶段计划说明（可选）
    pub plan: Option<String>,
}

/// 推进下一 D 阶段请求 DTO（直接复用 AdvanceStepPayload 枚举）
pub type AdvanceDto = AdvanceStepPayload;

/// 8D 报告响应 DTO
#[allow(dead_code, reason = "序列化输出字段")]
#[derive(Debug, Serialize, Clone)]
pub struct EightDReportInfo {
    pub id: i64,
    pub quality_issue_id: i64,
    pub status: String,
    pub d0_date: Option<DateTime<Utc>>,
    pub d0_prepared_by: Option<i32>,
    pub d0_plan: Option<String>,
    pub d1_date: Option<DateTime<Utc>>,
    pub d1_team_members: Option<String>,
    pub d2_date: Option<DateTime<Utc>>,
    pub d2_problem_description: Option<String>,
    pub d3_date: Option<DateTime<Utc>>,
    pub d3_interim_action: Option<String>,
    pub d4_date: Option<DateTime<Utc>>,
    pub d4_root_cause_method: Option<String>,
    pub d4_root_cause_detail: Option<String>,
    pub d4_root_cause_summary: Option<String>,
    pub d5_date: Option<DateTime<Utc>>,
    pub d5_permanent_action: Option<String>,
    pub d5_action_owner: Option<String>,
    pub d5_due_date: Option<NaiveDate>,
    pub d5_completed_at: Option<DateTime<Utc>>,
    pub d6_date: Option<DateTime<Utc>>,
    pub d6_verification_result: Option<String>,
    pub d7_date: Option<DateTime<Utc>>,
    pub d7_prevention_action: Option<String>,
    pub d8_date: Option<DateTime<Utc>>,
    pub d8_closure_summary: Option<String>,
    pub closed_at: Option<DateTime<Utc>>,
    pub closed_by: Option<i32>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl From<quality_8d_report::Model> for EightDReportInfo {
    fn from(m: quality_8d_report::Model) -> Self {
        Self {
            id: m.id,
            quality_issue_id: m.quality_issue_id,
            status: m.status,
            d0_date: m.d0_date,
            d0_prepared_by: m.d0_prepared_by,
            d0_plan: m.d0_plan,
            d1_date: m.d1_date,
            d1_team_members: m.d1_team_members,
            d2_date: m.d2_date,
            d2_problem_description: m.d2_problem_description,
            d3_date: m.d3_date,
            d3_interim_action: m.d3_interim_action,
            d4_date: m.d4_date,
            d4_root_cause_method: m.d4_root_cause_method,
            d4_root_cause_detail: m.d4_root_cause_detail,
            d4_root_cause_summary: m.d4_root_cause_summary,
            d5_date: m.d5_date,
            d5_permanent_action: m.d5_permanent_action,
            d5_action_owner: m.d5_action_owner,
            d5_due_date: m.d5_due_date,
            d5_completed_at: m.d5_completed_at,
            d6_date: m.d6_date,
            d6_verification_result: m.d6_verification_result,
            d7_date: m.d7_date,
            d7_prevention_action: m.d7_prevention_action,
            d8_date: m.d8_date,
            d8_closure_summary: m.d8_closure_summary,
            closed_at: m.closed_at,
            closed_by: m.closed_by,
            created_at: m.created_at,
            updated_at: m.updated_at,
        }
    }
}

/// 分页响应
#[allow(dead_code, reason = "序列化输出字段")]
#[derive(Debug, Serialize, Clone)]
pub struct EightDPagedResponse<T> {
    pub items: Vec<T>,
    pub total: u64,
    pub page: u64,
    pub page_size: u64,
}

/// EightDError → AppError
pub fn eight_d_err(e: EightDError) -> AppError {
    match e {
        EightDError::NotFound => AppError::not_found("8D 报告不存在"),
        EightDError::QualityIssueNotFound => AppError::not_found("质量异常不存在"),
        // 状态门装配点：AlreadyExists 属「已处于某态不可重复动作」，与本 handler 的
        // InvalidState 同为业务族；文案不含内部 ID/状态 token，可外显。
        EightDError::AlreadyExists => {
            AppError::business_displayable("该质量异常已存在 8D 报告，不能重复启动")
        }
        EightDError::InvalidState { current, expected } => AppError::business(format!(
            "当前状态 {} 不允许此操作（期望 {}）",
            current, expected
        )),
        EightDError::Validation(msg) => AppError::validation(msg),
        EightDError::Database(e) => AppError::database(e.to_string()),
        // paginate_with_total 返回的 AppError 直接透传
        EightDError::App(e) => e,
    }
}

// ==================== 归属门辅助 ====================

/// 8D 报告归属经父链继承：report → quality_issue → custom_order.created_by。
/// 加载 custom_order 行并提取归属人 ID，供单行门判定。
async fn resolve_owner_via_parent_chain(
    state: &AppState,
    quality_issue_id: i64,
) -> Result<Option<i32>, AppError> {
    let issue = quality_issue::Entity::find_by_id(quality_issue_id)
        .one(&*state.db)
        .await?
        .ok_or_else(|| AppError::not_found("质量异常不存在"))?;
    let order = custom_order::Entity::find_by_id(issue.custom_order_id)
        .one(&*state.db)
        .await?
        .ok_or_else(|| AppError::not_found("定制订单不存在"))?;
    Ok(order.created_by.map(|v| v as i32))
}

// ==================== Handler 端点 ====================

/// POST /api/v1/erp/quality-8d-reports - 启动 8D 流程（not_started → d0_plan）
pub async fn start_8d(
    auth: AuthContext,
    State(state): State<AppState>,
    Json(dto): Json<StartEightDDto>,
) -> Result<Json<ApiResponse<EightDReportInfo>>, AppError> {
    let ctx = auth.to_data_scope_context();
    // 行级归属门（写口）：8D 报告归属经父链 quality_issue → custom_order 继承，
    // 启动前须证明操作人对父定制订单有归属权
    let owner = resolve_owner_via_parent_chain(&state, dto.quality_issue_id).await?;
    if !check_resource_owner_by_member_scope(&ctx, owner) {
        return Err(AppError::permission_denied(
            "无权操作该质量异常关联的 8D 流程（数据范围限制）",
        ));
    }
    let service = QualityEightDService::from_state(&state);
    let req = StartEightDRequest {
        quality_issue_id: dto.quality_issue_id,
        prepared_by: auth.user_id,
        plan: dto.plan,
    };
    let record = service.start_8d(req).await.map_err(eight_d_err)?;
    Ok(Json(ApiResponse::success(record.into())))
}

/// GET /api/v1/erp/quality-8d-reports - 8D 报告列表
pub async fn list_8d(
    auth: AuthContext,
    State(state): State<AppState>,
    Query(query): Query<ListEightDQuery>,
) -> Result<Json<ApiResponse<EightDPagedResponse<EightDReportInfo>>>, AppError> {
    let ctx = auth.to_data_scope_context();
    let service = QualityEightDService::from_state(&state);
    let page = query.page.unwrap_or(1).clamp(1, 1000);
    let page_size = query.page_size.unwrap_or(20).clamp(1, 200);

    let (items, total) = service.list(query, Some(&ctx)).await.map_err(eight_d_err)?;
    let infos: Vec<EightDReportInfo> = items.into_iter().map(Into::into).collect();

    Ok(Json(ApiResponse::success(EightDPagedResponse {
        items: infos,
        total,
        page,
        page_size,
    })))
}

/// GET /api/v1/erp/quality-8d-reports/:id - 8D 报告详情
pub async fn get_8d(
    auth: AuthContext,
    State(state): State<AppState>,
    Path(id): Path<i64>,
) -> Result<Json<ApiResponse<EightDReportInfo>>, AppError> {
    let ctx = auth.to_data_scope_context();
    let service = QualityEightDService::from_state(&state);
    let record = service.get_by_id(id).await.map_err(eight_d_err)?;
    // 行级归属门（读口）：归属经父链继承
    let owner = resolve_owner_via_parent_chain(&state, record.quality_issue_id).await?;
    if !check_resource_owner_by_member_scope(&ctx, owner) {
        return Err(AppError::permission_denied(
            "无权访问该 8D 报告（数据范围限制）",
        ));
    }
    Ok(Json(ApiResponse::success(record.into())))
}

/// GET /api/v1/erp/quality-8d-reports/by-issue/:quality_issue_id - 按质量异常查询 8D 报告
pub async fn get_by_issue(
    auth: AuthContext,
    State(state): State<AppState>,
    Path(quality_issue_id): Path<i64>,
) -> Result<Json<ApiResponse<Option<EightDReportInfo>>>, AppError> {
    let ctx = auth.to_data_scope_context();
    // 行级归属门（读口）：先校验质量异常父链归属，再返回关联 8D 报告
    let owner = resolve_owner_via_parent_chain(&state, quality_issue_id).await?;
    if !check_resource_owner_by_member_scope(&ctx, owner) {
        return Err(AppError::permission_denied(
            "无权访问该质量异常关联的 8D 报告（数据范围限制）",
        ));
    }
    let service = QualityEightDService::from_state(&state);
    let record = service
        .get_by_quality_issue(quality_issue_id)
        .await
        .map_err(eight_d_err)?;
    Ok(Json(ApiResponse::success(record.map(Into::into))))
}

/// POST /api/v1/erp/quality-8d-reports/:id/advance - 推进下一 D 阶段
pub async fn advance(
    auth: AuthContext,
    State(state): State<AppState>,
    Path(id): Path<i64>,
    Json(payload): Json<AdvanceDto>,
) -> Result<Json<ApiResponse<EightDReportInfo>>, AppError> {
    let ctx = auth.to_data_scope_context();
    let service = QualityEightDService::from_state(&state);
    // 行级归属门（写口）：推进前取报告行 → 父链归属判定，越权零写入
    let record = service.get_by_id(id).await.map_err(eight_d_err)?;
    let owner = resolve_owner_via_parent_chain(&state, record.quality_issue_id).await?;
    if !check_resource_owner_by_member_scope(&ctx, owner) {
        return Err(AppError::permission_denied(
            "无权操作该 8D 报告（数据范围限制）",
        ));
    }
    let updated = service.advance(id, payload).await.map_err(eight_d_err)?;
    Ok(Json(ApiResponse::success(updated.into())))
}

/// POST /api/v1/erp/quality-8d-reports/:id/close - 关闭 8D 流程（d8_recognize → closed）
pub async fn close_8d(
    auth: AuthContext,
    State(state): State<AppState>,
    Path(id): Path<i64>,
) -> Result<Json<ApiResponse<EightDReportInfo>>, AppError> {
    let ctx = auth.to_data_scope_context();
    let service = QualityEightDService::from_state(&state);
    // 行级归属门（写口）：关闭前取报告行 → 父链归属判定，越权零写入
    let record = service.get_by_id(id).await.map_err(eight_d_err)?;
    let owner = resolve_owner_via_parent_chain(&state, record.quality_issue_id).await?;
    if !check_resource_owner_by_member_scope(&ctx, owner) {
        return Err(AppError::permission_denied(
            "无权操作该 8D 报告（数据范围限制）",
        ));
    }
    let req = CloseEightDRequest {
        closed_by: auth.user_id,
    };
    let record = service.close_8d(id, req).await.map_err(eight_d_err)?;
    Ok(Json(ApiResponse::success(record.into())))
}
