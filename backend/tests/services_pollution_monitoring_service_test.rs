use bingxi_backend::services::pollution_monitoring_service::*;
use rust_decimal::Decimal;

// =========================================================
// #323c 站点活体锁（真 PostgreSQL + tower oneshot 真实 handler）：
// POST /pollution-monitoring/records 的登记人身份只准取会话
// （AuthContext.user_id）。请求体即便多带伪造的另一个用户 B，
// pollutant_monitoring_records.operator_id 也必须是 A。
// 该表 operator_id 列无 FK（v15/mod.rs:1337 INTEGER 裸列），
// 故锁用合成会话 id 即可，无需种 users 行。
// =========================================================

mod test_common;

use axum::{
    Router,
    body::Body,
    extract::State,
    http::{Method, Request, StatusCode, header::CONTENT_TYPE},
    middleware::{Next, from_fn_with_state},
    response::Response,
    routing::post,
};
use bingxi_backend::container::AppState;
use bingxi_backend::handlers::pollution_monitoring_handler;
use bingxi_backend::middleware::auth_context::AuthContext;
use bingxi_backend::models::pollutant_monitoring_record;
use sea_orm::{ColumnTrait, EntityTrait, QueryFilter};
use serde_json::json;
use std::sync::Arc;
use tower::ServiceExt;

async fn inject_auth(
    State(auth): State<AuthContext>,
    mut request: Request<Body>,
    next: Next,
) -> Response {
    request.extensions_mut().insert(auth);
    next.run(request).await
}

#[tokio::test]
async fn create_monitoring_record_operator_identity_must_come_from_session_not_body() {
    const SESSION_USER_ID: i32 = 9323;
    const FORGED_USER_ID: i32 = 9999;

    let db = Arc::new(test_common::setup_test_db().await);
    let auth = AuthContext {
        user_id: SESSION_USER_ID,
        username: "w323c_pollution_a".to_string(),
        role_id: Some(2),
        department_id: None,
        data_scope: Some("all".to_string()),
        dept_ids: None,
        dept_member_user_ids: None,
    };
    let point = "COD-323c-总排口";
    let app = Router::new()
        .route(
            "/pollution-monitoring/records",
            post(pollution_monitoring_handler::create_monitoring_record),
        )
        .with_state(AppState {
            db: db.clone(),
            ..Default::default()
        })
        .layer(from_fn_with_state(auth, inject_auth));

    // 会话是 A，请求体伪造 B：B 必须被忽略（请求结构已删除该字段，serde 默认放行未知键）
    let resp = app
        .oneshot(
            Request::builder()
                .method(Method::POST)
                .uri("/pollution-monitoring/records")
                .header(CONTENT_TYPE, "application/json")
                .body(Body::from(
                    json!({
                        "monitoring_type": "wastewater",
                        "monitoring_point": point,
                        "pollutant_name": "COD",
                        "measured_value": "50",
                        "unit": "mg/L",
                        "limit_value": "80",
                        "monitoring_time": "2026-10-01T08:00:00+00:00",
                        "operator_id": FORGED_USER_ID,
                    })
                    .to_string(),
                ))
                .unwrap(),
        )
        .await
        .unwrap();
    let status = resp.status();
    let bytes = axum::body::to_bytes(resp.into_body(), 4 * 1024 * 1024)
        .await
        .unwrap();
    assert_eq!(
        status,
        StatusCode::OK,
        "合法登记请求必须 200，实得 {status} {}",
        String::from_utf8_lossy(&bytes)
    );

    let row = pollutant_monitoring_record::Entity::find()
        .filter(pollutant_monitoring_record::Column::MonitoringPoint.eq(point))
        .one(db.as_ref())
        .await
        .expect("监测记录直读失败")
        .expect("监测记录必须存在");
    assert_eq!(
        row.operator_id,
        Some(SESSION_USER_ID),
        "operator_id 必须等于会话用户 A（{SESSION_USER_ID}），实得 {:?}",
        row.operator_id
    );
    assert_ne!(
        row.operator_id,
        Some(FORGED_USER_ID),
        "请求体伪造的用户 B（{FORGED_USER_ID}）绝不允许落操作人列"
    );
    assert_eq!(
        row.is_exceeding, false,
        "自动判超逻辑不得因身份收口而改动（实测 50 < 限值 80）"
    );
}

#[test]
fn test_check_exceedance_normal() {
    let (is_exceeding, ratio) =
        PollutionMonitoringService::check_exceedance(Decimal::new(50, 0), Decimal::new(80, 0));
    assert!(!is_exceeding);
    assert_eq!(ratio, None);
}

#[test]
fn test_check_exceedance_at_limit() {
    // 实测值等于限值不算超标
    let (is_exceeding, ratio) =
        PollutionMonitoringService::check_exceedance(Decimal::new(80, 0), Decimal::new(80, 0));
    assert!(!is_exceeding);
    assert_eq!(ratio, None);
}

#[test]
fn test_check_exceedance_exceeded() {
    // 实测 120，限值 80 → 超标 0.5 倍
    let (is_exceeding, ratio) =
        PollutionMonitoringService::check_exceedance(Decimal::new(120, 0), Decimal::new(80, 0));
    assert!(is_exceeding);
    assert_eq!(ratio, Some(Decimal::new(5, 1))); // 0.5
}

#[test]
fn test_validate_monitoring_type() {
    assert!(PollutionMonitoringService::validate_monitoring_type("wastewater").is_ok());
    assert!(PollutionMonitoringService::validate_monitoring_type("exhaust").is_ok());
    assert!(PollutionMonitoringService::validate_monitoring_type("noise").is_ok());
    assert!(PollutionMonitoringService::validate_monitoring_type("solid_waste").is_ok());
    assert!(PollutionMonitoringService::validate_monitoring_type("invalid").is_err());
}

#[test]
fn test_validate_waste_type() {
    assert!(PollutionMonitoringService::validate_waste_type("sludge").is_ok());
    assert!(PollutionMonitoringService::validate_waste_type("waste_fabric").is_ok());
    assert!(PollutionMonitoringService::validate_waste_type("chemical_waste").is_ok());
    assert!(PollutionMonitoringService::validate_waste_type("invalid").is_err());
}

#[test]
fn test_validate_disposal_method() {
    assert!(PollutionMonitoringService::validate_disposal_method("landfill").is_ok());
    assert!(PollutionMonitoringService::validate_disposal_method("incineration").is_ok());
    assert!(PollutionMonitoringService::validate_disposal_method("reuse").is_ok());
    assert!(PollutionMonitoringService::validate_disposal_method("storage").is_ok());
    assert!(PollutionMonitoringService::validate_disposal_method("invalid").is_err());
}

#[test]
fn test_pollution_limit_reference_cod() {
    let limit = PollutionLimitReference::get_limit("wastewater", "COD");
    assert_eq!(limit, Some(Decimal::new(80, 0)));
}

#[test]
fn test_pollution_limit_reference_vocs() {
    let limit = PollutionLimitReference::get_limit("exhaust", "VOCs");
    assert_eq!(limit, Some(Decimal::new(60, 0)));
}

#[test]
fn test_pollution_limit_reference_unknown() {
    let limit = PollutionLimitReference::get_limit("wastewater", "Unknown");
    assert_eq!(limit, None);
}
