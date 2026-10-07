//! 两处后端挂载/白名单契约锁：报废审批端点接出 + /privacy 白名单与挂载对齐
//!
//! 1. **报废 GM 级审批端点**（services/quality_inspection_service.rs:661/:700 的 service 死码
//!    接出）：真实挂载 = `/api/v1/erp/production/quality-inspection/defects/{id}/scrap-approval/{financial,gm}`
//!    （routes/production.rs quality_inspection() 区块；`{id}` = unqualified_products.id）。
//!    钉死：①路由已注册（不再出现 403「未知的资源路径」）；②状态真实两级流转（回读）；
//!    ③跳级/非报废/越权被拒且零写残留（回读）；④拒绝族恒 BUSINESS_ERROR 且出参脱敏，
//!    权限拒绝出参永久脱敏；⑤审批人身份服务端派生（body 伪造无效）。
//! 2. **/privacy/consents 白名单与挂载对齐**：真实挂载 `/api/v1/erp/privacy/*`（routes/analytics.rs
//!    `.nest("/privacy", privacy())`），白名单必须登记 seg3 名 `privacy` 而非仅 seg4 名
//!    `consents`，否则 admin 也会被权限中间件白名单层 403。本文件钉死两处对齐 + 端点对 admin 200 + 信封形状。
//!
//! 测试路线：真 PostgreSQL（缺 TEST_DATABASE_URL 即 panic，禁 sqlite::memory）、
//! 表结构唯一来源 backend/migration、FK/父行自种子（夹具 TRUNCATE 后不重播）、真实路由函数
//! （production::quality_inspection / analytics::privacy）+ tower oneshot 端到端，无 mock。

mod test_common;

use axum::{
    body::Body,
    extract::State,
    http::{Request, StatusCode},
    middleware::{Next, from_fn_with_state},
    response::Response,
};
use bingxi_backend::container::AppState;
use bingxi_backend::middleware::auth_context::AuthContext;
use bingxi_backend::models::unqualified_product;
use bingxi_backend::routes::{analytics, production};
use bingxi_backend::utils::messages::err_msg;
use bingxi_backend::utils::path_utils::is_known_resource_segment;
use rust_decimal::Decimal;
use sea_orm::{ConnectionTrait, DbBackend, EntityTrait, Statement};
use serde_json::{Value, json};
use std::sync::Arc;
use tower::ServiceExt;

const SCRAP_FIN_URL: &str = "/quality-inspection/defects/601/scrap-approval/financial";
const SCRAP_GM_URL: &str = "/quality-inspection/defects/601/scrap-approval/gm";

/// 构造注入的认证上下文（role_id=2 非 admin；data_scope 逐用例指定）
fn make_auth(user_id: i32, scope: &str) -> AuthContext {
    AuthContext {
        user_id,
        username: format!("wave7_user_{user_id}"),
        role_id: Some(2),
        department_id: Some(1),
        data_scope: Some(scope.to_string()),
        dept_ids: None,
        dept_member_user_ids: None,
    }
}

async fn inject_auth(
    State(auth): State<AuthContext>,
    mut request: Request<Body>,
    next: Next,
) -> Response {
    request.extensions_mut().insert(auth);
    next.run(request).await
}

/// 质检域真实子路由（挂载前缀 /production 由 routes/mod.rs nest 提供；子路由内 path 即注册形态）
async fn scrap_app(user_id: i32, scope: &str) -> (axum::Router, sea_orm::DatabaseConnection) {
    let db = test_common::setup_test_db().await;
    let state = AppState {
        db: Arc::new(db.clone()),
        ..Default::default()
    };
    let app = production::quality_inspection()
        .with_state(state)
        .layer(from_fn_with_state(make_auth(user_id, scope), inject_auth));
    (app, db)
}

/// 隐私同意域真实子路由（routes/analytics.rs privacy()，挂载前缀 /privacy 在 analytics::routes()）
async fn privacy_app(user_id: i32) -> (axum::Router, sea_orm::DatabaseConnection) {
    let db = test_common::setup_test_db().await;
    let state = AppState {
        db: Arc::new(db.clone()),
        ..Default::default()
    };
    let app = analytics::privacy()
        .with_state(state)
        .layer(from_fn_with_state(make_auth(user_id, "self"), inject_auth));
    (app, db)
}

async fn post_json(app: &axum::Router, uri: &str, body: Value) -> (StatusCode, Value) {
    let resp = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri(uri)
                .header(axum::http::header::CONTENT_TYPE, "application/json")
                .body(Body::from(body.to_string()))
                .unwrap(),
        )
        .await
        .unwrap();
    let status = resp.status();
    let bytes = axum::body::to_bytes(resp.into_body(), 4 * 1024 * 1024)
        .await
        .unwrap();
    let text = String::from_utf8_lossy(&bytes).to_string();
    (
        status,
        serde_json::from_str(&text).unwrap_or(Value::String(text)),
    )
}

async fn get(app: &axum::Router, uri: &str) -> (StatusCode, Value) {
    let resp = app
        .clone()
        .oneshot(
            Request::builder()
                .method("GET")
                .uri(uri)
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    let status = resp.status();
    let bytes = axum::body::to_bytes(resp.into_body(), 4 * 1024 * 1024)
        .await
        .unwrap();
    let text = String::from_utf8_lossy(&bytes).to_string();
    (
        status,
        serde_json::from_str(&text).unwrap_or(Value::String(text)),
    )
}

/// 自种子（迁移建表后夹具 TRUNCATE 不重播业务表）：质检记录（检验员固定 user 100）。
/// 逐列对照 migration：NOT NULL = inspection_no/product_id/inspection_type/inspection_date
/// （m0005:158-176；result 列 v15/mod.rs:3846 已 DROP NOT NULL）；实体非 Option 列
/// total_qty/inspected_qty/inspection_result 一并给值，保证 Model 可读。
async fn seed_inspection(db: &sea_orm::DatabaseConnection, id: i64) {
    let sql = format!(
        r#"INSERT INTO quality_inspection_records
               (id, inspection_no, product_id, inspection_type, inspection_date,
                inspector_id, total_qty, inspected_qty, inspection_result)
           VALUES ({id},'W7SC-QIR-{id}',1,'final','2026-01-01',100,100,100,'不合格')"#
    );
    db.execute_raw(Statement::from_sql_and_values(
        DbBackend::Postgres,
        &sql,
        Vec::<sea_orm::Value>::new(),
    ))
    .await
    .unwrap_or_else(|e| panic!("quality_inspection_records 种子失败: {e}"));
}

/// 自种子：不合格品（报废）记录。逐列对照 models/unqualified_product.rs +
/// m0013 建表 + business 域 v15/mod.rs ALTER；实体非 Option 列（stock_grade_synced 等）全部给值。
async fn seed_scrap(
    db: &sea_orm::DatabaseConnection,
    id: i64,
    handling_method: &str,
    approval_status: &str,
) {
    let sql = format!(
        r#"INSERT INTO unqualified_products
               (id, unqualified_no, inspection_id, product_id, unqualified_qty,
                unqualified_reason, handling_method, handling_status,
                stock_grade_synced, scrap_approval_status, created_at, updated_at)
           VALUES ({id},'W7SC-UQ-{id}',501,1,10,'严重疵点','{handling_method}','pending',
                   false,'{approval_status}','2026-01-01T00:00:00Z','2026-01-01T00:00:00Z')"#
    );
    db.execute_raw(Statement::from_sql_and_values(
        DbBackend::Postgres,
        &sql,
        Vec::<sea_orm::Value>::new(),
    ))
    .await
    .unwrap_or_else(|e| panic!("unqualified_products 种子失败: {e}"));
}

// =========================================================
// 1) 报废两级审批：已注册 + 白名单 seg3 对齐（403 未知的资源路径 不得再出现）
// =========================================================

#[tokio::test]
async fn scrap_approval_routes_registered_and_reach_handler_envelope() {
    // 白名单层（permission.rs validate_route_whitelist 的唯一判定源）：挂载 seg3=production 已知；
    // e2e 旧臆造前缀 quality 仍不在白名单——钉死「不许自创前缀」，用例须对齐真实挂载点。
    assert!(
        is_known_resource_segment("production"),
        "scrap-approval 挂载 seg3=production 必须在白名单内"
    );
    assert!(
        !is_known_resource_segment("quality"),
        "防回潮：/quality 前缀不是本仓挂载族，禁止为其加白名单来迁就臆造用例路径"
    );

    // 行为锁：对不存在记录走真实注册路由 → 必须是 AppError 信封 404 NOT_FOUND（说明请求
    // 已穿过权限白名单族段并抵达 handler；若白名单族段缺失，会在中间件层 403「未知的资源路径」）。
    let (app, _db) = scrap_app(300, "all").await;
    let miss = |u: String| u.replace("/601/", "/99999999/");
    for uri in [
        miss(SCRAP_FIN_URL.to_string()),
        miss(SCRAP_GM_URL.to_string()),
    ] {
        let (status, v) = post_json(&app, &uri, json!({ "approved": true })).await;
        assert_eq!(
            status,
            StatusCode::NOT_FOUND,
            "{uri} 应命中路由并 404，实际体: {v}"
        );
        assert_eq!(
            v["code"], "NOT_FOUND",
            "失败体必须为 AppError 统一信封，实际: {v}"
        );
        assert_eq!(v["message"], err_msg::NOT_FOUND_PUBLIC);
        assert_ne!(
            v["message"].as_str().unwrap_or_default(),
            "未知的资源路径",
            "F 族判责原缺陷（白名单层 403）不得复发"
        );
    }

    // 源码防回潮锁：路由已挂进 quality_inspection()（service 死码接出），handler 直连两个 service 方法
    let route_src = include_str!("../src/routes/production.rs");
    assert!(
        route_src.contains("/quality-inspection/defects/{id}/scrap-approval/financial")
            && route_src.contains("/quality-inspection/defects/{id}/scrap-approval/gm"),
        "scrap-approval 两级路由必须注册在 quality_inspection() 区块（挂载随 quality-inspection 族）"
    );
    assert!(
        !route_src.contains("/production/scrap-approval/gm")
            && !route_src.contains("\"/quality/inspections/{id}/scrap-approval"),
        "防回潮：不得为迁就用例另起自创前缀"
    );
    let handler_src = include_str!("../src/handlers/quality_inspection_handler.rs");
    assert!(
        handler_src.contains("service.approve_scrap_financial(")
            && handler_src.contains("service.approve_scrap_gm("),
        "handler 必须接出 services/quality_inspection_service.rs 的两个报废审批方法（禁止删 service 或让其重回死码）"
    );
    assert!(
        !handler_src.contains("approved_by") && !handler_src.contains("approver_id: "),
        "审批人不得来自请求体（只能服务端派生 auth.user_id）"
    );
}

// =========================================================
// 2) 合法两级流转（非 admin 角色、all 数据范围）——状态真实流转，回读为证
// =========================================================

#[tokio::test]
async fn scrap_two_stage_approval_transitions_states() {
    let (app, db) = scrap_app(300, "all").await;
    seed_inspection(&db, 501).await;
    seed_scrap(&db, 601, "scrap", "pending_fin").await;

    // 一级：财务审批通过（body 伪造 approver_id 属未知字段，结构性无效）
    let (status, v) = post_json(
        &app,
        SCRAP_FIN_URL,
        json!({ "approved": true, "approver_id": 999 }),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "财务审批应 200，实际体: {v}");
    assert_eq!(v["code"], 200);
    let row = unqualified_product::Entity::find_by_id(601)
        .one(&db)
        .await
        .unwrap()
        .expect("回读财务审批结果");
    assert_eq!(
        row.scrap_approval_status, "pending_gm",
        "财务通过后必须进入待总经理态"
    );
    assert_eq!(
        row.approver_id_fin,
        Some(300),
        "审批人必须取会话 user_id 而非 body 伪造值"
    );
    assert!(row.approved_at_fin.is_some(), "财务审批时间必须落库");

    // 二级：总经理审批通过并落报废损失金额
    let (status, v) = post_json(
        &app,
        SCRAP_GM_URL,
        json!({ "approved": true, "scrap_loss_amount": 1234.56 }),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "总经理审批应 200，实际体: {v}");
    let row = unqualified_product::Entity::find_by_id(601)
        .one(&db)
        .await
        .unwrap()
        .expect("回读总经理审批结果");
    assert_eq!(row.scrap_approval_status, "approved");
    assert_eq!(
        row.handling_status, "approved",
        "终审通过必须联动处理状态（quality_handling::APPROVED）"
    );
    assert_eq!(row.approver_id_gm, Some(300));
    assert!(row.approved_at_gm.is_some());
    assert_eq!(row.scrap_loss_amount, Some(Decimal::new(123456, 2)));
}

// =========================================================
// 3) 审批门控（GM 前必须财务）+ 拒绝族 BUSINESS_ERROR + 出参脱敏 + 零写残留
// =========================================================

#[tokio::test]
async fn gm_before_financial_rejected_masked_and_no_write() {
    let (app, db) = scrap_app(300, "all").await;
    seed_inspection(&db, 501).await;
    seed_scrap(&db, 601, "scrap", "pending_fin").await;

    let (status, v) = post_json(
        &app,
        SCRAP_GM_URL,
        json!({ "approved": true, "scrap_loss_amount": 100 }),
    )
    .await;
    assert_eq!(
        status,
        StatusCode::BAD_REQUEST,
        "跳级审批必须 400，实际体: {v}"
    );
    assert_eq!(
        v["code"], "BUSINESS_ERROR",
        "状态门拒绝族固定为 BUSINESS_ERROR（不得改 VALIDATION）"
    );
    assert_eq!(
        v["message"],
        err_msg::BUSINESS_PUBLIC,
        "拒绝原因含内部状态 token，出参必须默认脱敏"
    );
    assert!(
        !v["message"]
            .as_str()
            .unwrap_or_default()
            .contains("pending"),
        "真实拒绝文案（状态 token）只准进 tracing::warn，不得外泄"
    );

    let row = unqualified_product::Entity::find_by_id(601)
        .one(&db)
        .await
        .unwrap()
        .expect("回读");
    assert_eq!(
        row.scrap_approval_status, "pending_fin",
        "被拒跳级不得推进状态"
    );
    assert_eq!(row.approver_id_gm, None);
    assert_eq!(row.approved_at_gm, None);
    assert_eq!(row.scrap_loss_amount, None);
    assert_eq!(
        row.updated_at.to_rfc3339(),
        "2026-01-01T00:00:00+00:00",
        "被拒的写必须零残留（updated_at 保持种子值）"
    );
}

#[tokio::test]
async fn non_scrap_handling_record_rejected_from_scrap_flow() {
    let (app, db) = scrap_app(300, "all").await;
    seed_inspection(&db, 501).await;
    seed_scrap(&db, 601, "rework", "not_required").await;

    let (status, v) = post_json(&app, SCRAP_FIN_URL, json!({ "approved": true })).await;
    assert_eq!(
        status,
        StatusCode::BAD_REQUEST,
        "非报废记录不得进入报废审批，实际体: {v}"
    );
    assert_eq!(v["code"], "BUSINESS_ERROR");
    assert_eq!(v["message"], err_msg::BUSINESS_PUBLIC);

    let row = unqualified_product::Entity::find_by_id(601)
        .one(&db)
        .await
        .unwrap()
        .expect("回读");
    assert_eq!(
        row.scrap_approval_status, "not_required",
        "拒绝后状态零残留"
    );
    assert_eq!(row.approver_id_fin, None);
}

#[tokio::test]
async fn gm_negative_loss_amount_rejected_before_any_write() {
    let (app, db) = scrap_app(300, "all").await;
    seed_inspection(&db, 501).await;
    seed_scrap(&db, 601, "scrap", "pending_gm").await;

    let (status, v) = post_json(
        &app,
        SCRAP_GM_URL,
        json!({ "approved": true, "scrap_loss_amount": -5 }),
    )
    .await;
    assert_eq!(
        status,
        StatusCode::BAD_REQUEST,
        "负损失金额必须拒绝，实际体: {v}"
    );
    assert_eq!(v["code"], "VALIDATION_ERROR");
    // 入参值门描述用户自提字段的公开规则、不含内部数据 → 允许外显（与状态门脱敏族分离）
    assert_eq!(v["message"], "报废损失金额不得为负数");

    let row = unqualified_product::Entity::find_by_id(601)
        .one(&db)
        .await
        .unwrap()
        .expect("回读");
    assert_eq!(row.scrap_approval_status, "pending_gm");
    assert_eq!(row.scrap_loss_amount, None);
}

// =========================================================
// 4) IDOR/行级归属：self 数据范围用户审批他人检验链路的报废单 → 403 永久脱敏 + 零残留
// =========================================================

#[tokio::test]
async fn self_scope_attacker_denied_and_target_row_untouched() {
    let (app, db) = scrap_app(200, "self").await;
    seed_inspection(&db, 501).await; // 检验员固定 100，攻击者 200 非归属链上用户
    seed_scrap(&db, 601, "scrap", "pending_fin").await;

    let (status, v) = post_json(&app, SCRAP_FIN_URL, json!({ "approved": true })).await;
    assert_eq!(
        status,
        StatusCode::FORBIDDEN,
        "越权审批必须 403，实际体: {v}"
    );
    assert_eq!(v["code"], "FORBIDDEN");
    assert_eq!(
        v["message"],
        err_msg::PERMISSION_PUBLIC,
        "权限拒绝出参永久脱敏：固定文案，不含记录 ID/归属判定依据"
    );
    assert!(
        !v["message"].as_str().unwrap_or_default().contains('6')
            && !v["message"].as_str().unwrap_or_default().contains("范围"),
        "脱敏常量不得被改成含内部 ID 的文案"
    );

    let row = unqualified_product::Entity::find_by_id(601)
        .one(&db)
        .await
        .unwrap()
        .expect("回读");
    assert_eq!(
        row.scrap_approval_status, "pending_fin",
        "越权被拒不得产生任何写入"
    );
    assert_eq!(row.approver_id_fin, None);
    assert_eq!(row.approved_at_fin, None);
}

// =========================================================
// 5) /privacy/consents：白名单与挂载对齐 + admin 200 + 统一信封
// =========================================================

#[tokio::test]
async fn privacy_whitelist_aligned_with_mount() {
    // 真实挂载 seg3=privacy 必须在白名单：若只登记 seg4 名 `consents`，admin GET
    // /privacy/consents 会在白名单层 403「未知的资源路径」，根本走不到 RBAC（防回潮钉）。
    assert!(
        is_known_resource_segment("privacy"),
        "privacy 必须按 seg3（与 .nest(\"/privacy\") 挂载）登记进白名单"
    );
    // 漂移条目纠偏：曾登记的是 seg4 误名 consents（/erp/consents 并无挂载），对齐后不得复活
    assert!(
        !is_known_resource_segment("consents"),
        "防回潮：consents 是挂载漂移条目，白名单登记必须与真实挂载段对齐，而非补伪资源名"
    );
    // 挂载本身不动（注册处即事实源）：
    let analytics_src = include_str!("../src/routes/analytics.rs");
    assert!(
        analytics_src.contains(".nest(\"/privacy\", privacy())"),
        "隐私域挂载点保持 /privacy，本次是白名单向挂载对齐"
    );
    // 修复只动白名单/路由；严禁的角色种子伪键防回潮：
    let seed_src = include_str!("../src/services/init_service_ops/permission.rs");
    assert!(
        !seed_src.contains("privacy:"),
        "严禁以给角色种子添加 privacy 伪权限键的方式蒙过白名单漂移"
    );
}

#[tokio::test]
async fn privacy_consents_endpoints_200_with_envelope() {
    let (app, _db) = privacy_app(42).await;

    // 空态：信封形状 {code:200, data:[]}
    let (status, v) = get(&app, "/consents").await;
    assert_eq!(
        status,
        StatusCode::OK,
        "已登记白名单后 admin 侧可达，实际体: {v}"
    );
    assert_eq!(v["code"], 200);
    assert_eq!(v["data"], json!([]));

    // 记录同意 → 查询可见（均为调用者自身数据，无越权面：handler 体只读 auth.user_id）
    let (status, v) = post_json(
        &app,
        "/consents",
        json!({ "consent_type": "cookie_usage", "consent_given": true, "consent_text_version": "v1.0" }),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "记录同意应 200，实际体: {v}");
    assert_eq!(v["code"], 200);
    assert_eq!(v["data"]["consent_type"], "cookie_usage");
    assert_eq!(v["data"]["consent_given"], true);

    let (status, v) = get(&app, "/consents").await;
    assert_eq!(status, StatusCode::OK);
    let items = v["data"].as_array().expect("data 必须为数组");
    assert_eq!(items.len(), 1);
    assert_eq!(items[0]["consent_type"], "cookie_usage");
    assert!(
        items[0].get("consented_at").is_some(),
        "ConsentStatus 出参键必须来自后端实体（snake_case，不引入第二套键名）"
    );
}
