use crate::container::AppState;
//! 用户隐私同意 HTTP 端点（V15 P1 batch-16 缺陷 7.3）
//!
//! 提供端点：
//! - `GET  /api/v1/erp/privacy/consents` — 查询当前用户的所有/指定 consent_type 状态
//! - `POST /api/v1/erp/privacy/consents` — 记录单类型同意/退出决定
//! - `POST /api/v1/erp/privacy/opt-in-all` — 一键同意全部追踪（首次登录确认后调用）
//! - `POST /api/v1/erp/privacy/opt-out-all` — 一键退出全部追踪（行使撤回权）
//!
//! 合规依据：《个人信息保护法》第 14 条（同意原则）+ 第 16 条（撤回权）+ GDPR 第 7 条

use axum::{
    extract::{Query, State},
    http::HeaderMap,
    Json,
};
use serde::{Deserialize, Serialize};
use validator::Validate;

use crate::middleware::auth_context::AuthContext;
use crate::services::user_consent_service::{
    ConsentStatus, RecordConsentRequest, UserConsentService,
};
use crate::utils::error::AppError;
use crate::utils::response::ApiResponse;

/// consent_type 查询参数
#[derive(Debug, Deserialize)]
pub struct ConsentQuery {
    /// 不传则返回所有类型的最新状态
    pub consent_type: Option<String>,
}

/// 记录单条 consent 响应
#[derive(Debug, Serialize)]
pub struct RecordConsentResponse {
    pub consent_type: String,
    pub consent_given: bool,
    pub consented_at: chrono::DateTime<chrono::Utc>,
}

/// 查询当前用户的同意状态
pub async fn get_consent_status(
    auth: AuthContext,
    State(state): State<AppState>,
    Query(params): Query<ConsentQuery>,
) -> Result<Json<ApiResponse<Vec<ConsentStatus>>>, AppError> {
    let svc = UserConsentService::new(state.db.clone());
    // get_current_consent 内部会校验 consent_type 是否为预定义类型
    let items = match params.consent_type.as_deref() {
        Some(ct) => {
            let one = svc.get_current_consent(auth.user_id, ct).await?;
            match one {
                Some(m) => vec![crate::services::user_consent_service::to_status(&m)],
                None => vec![],
            }
        }
        None => {
            let all = svc.list_current_consents(auth.user_id).await?;
            all.iter()
                .map(crate::services::user_consent_service::to_status)
                .collect()
        }
    };
    Ok(Json(ApiResponse::success(items)))
}

/// 记录用户对单个 consent_type 的同意/退出决定
pub async fn record_consent(
    auth: AuthContext,
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(req): Json<RecordConsentRequest>,
) -> Result<Json<ApiResponse<RecordConsentResponse>>, AppError> {
    req.validate()
        .map_err(|e| AppError::validation(e.to_string()))?;

    let ip_address = extract_client_ip(&headers);
    let user_agent = headers
        .get(axum::http::header::USER_AGENT)
        .and_then(|v| v.to_str().ok())
        .map(|s| s.to_string());

    let svc = UserConsentService::new(state.db.clone());
    let model = svc
        .record_consent(auth.user_id, req, Some(ip_address), user_agent)
        .await?;
    Ok(Json(ApiResponse::success(RecordConsentResponse {
        consent_type: model.consent_type,
        consent_given: model.consent_given,
        consented_at: model.consented_at,
    })))
}

/// 一键同意全部追踪（首次登录隐私政策确认后调用）
pub async fn opt_in_all(
    auth: AuthContext,
    State(state): State<AppState>,
    headers: HeaderMap,
) -> Result<Json<ApiResponse<Vec<ConsentStatus>>>, AppError> {
    let ip_address = extract_client_ip(&headers);
    let user_agent = headers
        .get(axum::http::header::USER_AGENT)
        .and_then(|v| v.to_str().ok())
        .map(|s| s.to_string());

    let svc = UserConsentService::new(state.db.clone());
    let models = svc
        .opt_in_all(auth.user_id, Some(ip_address), user_agent)
        .await?;
    let resp = models
        .iter()
        .map(crate::services::user_consent_service::to_status)
        .collect();
    Ok(Json(ApiResponse::success(resp)))
}

/// 一键退出全部追踪（用户行使撤回权）
pub async fn opt_out_all(
    auth: AuthContext,
    State(state): State<AppState>,
    headers: HeaderMap,
) -> Result<Json<ApiResponse<Vec<ConsentStatus>>>, AppError> {
    let ip_address = extract_client_ip(&headers);
    let user_agent = headers
        .get(axum::http::header::USER_AGENT)
        .and_then(|v| v.to_str().ok())
        .map(|s| s.to_string());

    let svc = UserConsentService::new(state.db.clone());
    let models = svc
        .opt_out_all(auth.user_id, Some(ip_address), user_agent)
        .await?;
    let resp = models
        .iter()
        .map(crate::services::user_consent_service::to_status)
        .collect();
    Ok(Json(ApiResponse::success(resp)))
}

/// 从请求头提取客户端 IP（X-Real-IP 优先，回退 X-Forwarded-For 首段）
fn extract_client_ip(headers: &HeaderMap) -> String {
    headers
        .get("x-real-ip")
        .and_then(|v| v.to_str().ok())
        .filter(|s| !s.is_empty())
        .map(|s| s.to_string())
        .or_else(|| {
            headers
                .get("x-forwarded-for")
                .and_then(|v| v.to_str().ok())
                .and_then(|s| s.split(',').next().map(|s| s.trim().to_string()))
                .filter(|s| !s.is_empty())
        })
        .unwrap_or_else(|| "unknown".to_string())
}
