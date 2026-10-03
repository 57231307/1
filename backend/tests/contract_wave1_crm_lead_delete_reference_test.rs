//! CRM 线索删除引用校验锁（本波修复：`fk_crm_opportunity_lead` 裸 500 → 400 BUSINESS_ERROR）
//!
//! 锁定的 file:line 契约（`backend/src/services/crm/lead.rs:470-512`）：
//! - L475-486 引用预校验：存在任意 `crm_opportunity.lead_id` 引用 →
//!   `AppError::business_displayable("该线索已转商机，不可删除，请先处理关联商机")`
//!   → HTTP 400 + code=BUSINESS_ERROR + **真实文案外显**（displayable 族）
//! - L488-511 竞态兜底：预校验通过后仍被并发引用命中 FK（DatabaseError 且 msg==DB_RELATION）
//!   → 同样降级为上述业务错；其余 DB 错误原样传播
//! - 无引用 → `AuditLogService::delete_with_audit` 硬删 + 审计落库（audit_logs action=DELETE）
//! - `backend/src/handlers/crm_handler.rs:328-341`（HTTP 层：先 get_lead(Some(&ctx)) 归属门禁，
//!   再 service.delete_lead；出参 data=="删除成功" biz_msg::DELETE_OK）
//!
//! 覆盖策略（路线一，#4669 判责；表结构唯一来源 = backend/migration，不再自建 DDL）：
//! - 全部用例经 `test_common::setup_test_db()` 打已迁移 PostgreSQL；service 层与
//!   HTTP handler 层各锁一遍 400/200/403。`crm_opportunity` 的 customer/lead 双 FK
//!   为真实约束（`fk_crm_opportunity_customer`/`fk_crm_opportunity_lead`），引用行
//!   按裁定 R1 自种子合法父行（customers 行本身无 FK 前置，owner_id 列在迁移中
//!   无外键约束，直接写归属人 100 与线索 owner 对齐）。
//! - `#[ignore]` 活库用例：真实业务链 create_lead（LD 单号 advisory_xact_lock，仅 PG
//!   支持）→ create_opportunity（含 customers FK 的完整 DTO 链）→ delete_lead 400。
//!
//! 已知不可判定分支（诚实记录）：竞态兜底分支（预校验与 DELETE 之间插入引用）无法在
//! 单连接测试上确定性触发，未在断言范围，见交付报告风险点。

mod test_common;

use axum::{
    Router,
    body::Body,
    extract::State,
    http::{Request, StatusCode},
    middleware::{Next, from_fn_with_state},
    response::{IntoResponse, Response},
    routing::delete,
};
use bingxi_backend::container::AppState;
use bingxi_backend::handlers::crm_handler;
use bingxi_backend::middleware::auth_context::AuthContext;
use bingxi_backend::models::{audit_log, crm_lead, crm_opportunity, customer};
use bingxi_backend::services::crm::cust::CrmService;
use bingxi_backend::utils::error::AppError;
use bingxi_backend::utils::messages::biz_msg;
use sea_orm::{ActiveModelTrait, ActiveValue::Set, ColumnTrait, EntityTrait, QueryFilter};
use serde_json::Value;
use std::sync::Arc;
use tower::ServiceExt;

const REFUSED_MSG: &str = "该线索已转商机，不可删除，请先处理关联商机";

async fn live_db() -> sea_orm::DatabaseConnection {
    test_common::setup_test_db().await
}

async fn seed_lead(db: &sea_orm::DatabaseConnection, id: i32, owner_id: i32) {
    crm_lead::ActiveModel {
        id: Set(id),
        lead_no: Set(format!("LD-FIX-{id:03}")),
        lead_source: Set("OTHER".to_string()),
        lead_status: Set(Some(
            bingxi_backend::models::status::crm_lead::NEW.to_string(),
        )),
        company_name: Set(Some(format!("测试公司{id}"))),
        contact_name: Set("张三".to_string()),
        owner_id: Set(owner_id),
        owner_name: Set(format!("用户{owner_id}")),
        ..Default::default()
    }
    .insert(db)
    .await
    .unwrap();
}

/// 引用行：真表 `crm_opportunity` 带 customer/lead 双 FK（迁移
/// `fk_crm_opportunity_customer`/`fk_crm_opportunity_lead`），按裁定 R1 先自种子
/// 合法父客户行（owner_id=100 与线索归属人口径一致），再插引用本线索的商机行。
async fn link_opportunity(db: &sea_orm::DatabaseConnection, opp_id: i32, lead_id: i32) {
    customer::ActiveModel {
        id: Set(1),
        customer_code: Set("C-LEADREF-0001".to_string()),
        customer_name: Set("线索引用校验客户".to_string()),
        credit_limit: Set(rust_decimal::Decimal::ZERO),
        payment_terms: Set(30),
        status: Set(bingxi_backend::models::status::master_data::ACTIVE.to_string()),
        customer_type: Set("retail".to_string()),
        owner_id: Set(100),
        created_at: Set(chrono::Utc::now()),
        updated_at: Set(chrono::Utc::now()),
        ..Default::default()
    }
    .insert(db)
    .await
    .expect("种子客户行插入失败");
    crm_opportunity::ActiveModel {
        id: Set(opp_id),
        opportunity_no: Set(format!("OPP-LEADREF-{opp_id:03}")),
        opportunity_name: Set("引用线索的商机".to_string()),
        customer_id: Set(1),
        lead_id: Set(Some(lead_id)),
        owner_id: Set(100),
        owner_name: Set("用户100".to_string()),
        ..Default::default()
    }
    .insert(db)
    .await
    .expect("种子商机引用行插入失败");
}

fn make_auth(user_id: i32) -> AuthContext {
    AuthContext {
        user_id,
        username: format!("e2e_user_{user_id}"),
        role_id: Some(2),
        department_id: Some(1),
        data_scope: Some("self".to_string()),
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

fn build_app(state: AppState, auth: AuthContext) -> Router {
    Router::new()
        .route("/crm/leads/{id}", delete(crm_handler::delete_lead))
        .with_state(state)
        .layer(from_fn_with_state(auth, inject_auth))
}

async fn request_json(app: &Router, req: Request<Body>) -> (StatusCode, Value) {
    let resp = app.clone().oneshot(req).await.unwrap();
    let status = resp.status();
    let bytes = axum::body::to_bytes(resp.into_body(), 1024 * 1024)
        .await
        .unwrap();
    (status, serde_json::from_slice(&bytes).unwrap())
}

// =========================================================
// service 层（真实 PostgreSQL）
// =========================================================

/// 被引用线索删除 → 预校验拒绝：BusinessErrorDisplayable 族、错误码与文案逐字锁死，
/// 且线索必须仍在（拒绝发生在 DELETE 之前，不静默删）
#[tokio::test]
async fn delete_referenced_lead_rejected_with_displayable_business_error() {
    let db = live_db().await;
    seed_lead(&db, 10, 100).await;
    link_opportunity(&db, 1, 10).await;

    let svc = CrmService::new(Arc::new(db.clone()));
    let err = svc
        .delete_lead(10, 100)
        .await
        .expect_err("被商机引用的线索不可删除（修复前此处为 FK 裸 500 DATABASE_ERROR）");

    match &err {
        AppError::BusinessErrorDisplayable(_) => {}
        other => panic!("必须用 business_displayable（文案须可外显），实际: {other:?}"),
    }
    assert_eq!(err.error_code(), "BUSINESS_ERROR");
    assert_eq!(err.to_string(), format!("业务错误：{REFUSED_MSG}"));

    let still = crm_lead::Entity::find_by_id(10).one(&db).await.unwrap();
    assert!(still.is_some(), "拒绝路径不得删掉线索");

    // HTTP 信封：400 + code + 真实文案外显（displayable 与脱敏族的关键区别）
    let resp = err.into_response();
    assert_eq!(resp.status(), StatusCode::BAD_REQUEST);
    let bytes = axum::body::to_bytes(resp.into_body(), 1024 * 1024)
        .await
        .unwrap();
    let v: Value = serde_json::from_slice(&bytes).unwrap();
    assert_eq!(v["code"], "BUSINESS_ERROR");
    assert_eq!(v["message"], REFUSED_MSG);
    assert_ne!(v["message"], "业务处理失败", "displayable 族不得被脱敏吞掉");
    assert_ne!(v["code"], "DATABASE_ERROR", "回归锁：不得回到裸 500 映射");
}

/// 无引用线索删除 → 硬删成功 + 审计行真实落库（delete_with_audit 非静默）
#[tokio::test]
async fn delete_unreferenced_lead_succeeds_and_writes_audit() {
    let db = live_db().await;
    seed_lead(&db, 11, 100).await;

    let svc = CrmService::new(Arc::new(db.clone()));
    svc.delete_lead(11, 100).await.expect("无引用必须删除成功");

    assert!(
        crm_lead::Entity::find_by_id(11)
            .one(&db)
            .await
            .unwrap()
            .is_none(),
        "线索已硬删（本表无软删列，语义见 lead.rs:463-469 注释）"
    );
    let audits = audit_log::Entity::find()
        .filter(audit_log::Column::ResourceType.eq("crm_lead"))
        .filter(audit_log::Column::ResourceId.eq("11"))
        .all(&db)
        .await
        .unwrap();
    assert_eq!(audits.len(), 1);
    assert_eq!(audits[0].action, "DELETE");
    assert_eq!(audits[0].user_id, Some(100), "审计操作人须为真实 user_id");
    assert!(audits[0].before_snapshot.is_some(), "删除前快照必须留存");
}

// =========================================================
// HTTP handler 层（真实 PostgreSQL）
// =========================================================

async fn seeded_router(viewer: i32, referenced: bool) -> Router {
    let db = live_db().await;
    seed_lead(&db, 10, 100).await;
    if referenced {
        link_opportunity(&db, 1, 10).await;
    }
    let state = AppState {
        db: Arc::new(db),
        ..Default::default()
    };
    build_app(state, make_auth(viewer))
}

fn del(uri: &str) -> Request<Body> {
    Request::builder()
        .method("DELETE")
        .uri(uri)
        .body(Body::empty())
        .unwrap()
}

/// DELETE /crm/leads/:id 被引用 → 400 BUSINESS_ERROR（非裸 500），文案外显
#[tokio::test]
async fn http_delete_referenced_lead_400_business_error() {
    let app = seeded_router(100, true).await;
    let (status, v) = request_json(&app, del("/crm/leads/10")).await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
    assert_eq!(v["code"], "BUSINESS_ERROR");
    assert_eq!(v["message"], REFUSED_MSG);
}

/// DELETE 无引用 + owner 本人 → 200，data 为 biz_msg::DELETE_OK 常量原文
#[tokio::test]
async fn http_delete_unreferenced_lead_owner_200() {
    let app = seeded_router(100, false).await;
    let (status, v) = request_json(&app, del("/crm/leads/10")).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(v["code"], 200);
    assert_eq!(v["data"], biz_msg::DELETE_OK);
}

/// DELETE 非 owner（data_scope=self，owner_id=100 vs 用户 200）→ 403 FORBIDDEN，
/// 先于引用校验（handler 的 get_lead 归属门禁，crm_handler.rs:335-337）
#[tokio::test]
async fn http_delete_lead_cross_owner_403() {
    let app = seeded_router(200, true).await;
    let (status, v) = request_json(&app, del("/crm/leads/10")).await;
    assert_eq!(status, StatusCode::FORBIDDEN);
    assert_eq!(v["code"], "FORBIDDEN");
    // 越权者不得触发引用文案（说明归属门禁在预校验之前生效）
    assert_ne!(v["message"], REFUSED_MSG);
}

// =========================================================
// 活库（PostgreSQL）真实业务链：#[ignore]
// =========================================================

/// 生产路径全链：create_lead（LD 单号走 advisory_xact_lock，sqlite 不支持）→
/// create_opportunity（客户 FK 真校验）→ delete_lead 必须 400 BUSINESS_ERROR；
/// 无引用另一条线索删除成功。
#[tokio::test]
#[ignore = "需要 TEST_DATABASE_URL 已迁移 PostgreSQL：create_lead 单号生成用 advisory_xact_lock，且 crm_opportunity 有 customers/lead 双 FK"]
async fn live_pg_lead_conversion_then_delete_rejected() {
    use bingxi_backend::models::customer;
    use bingxi_backend::models::dto::crm_dto::{CreateLeadRequest, CreateOpportunityRequest};

    let db = Arc::new(live_db().await);
    // 最小可用客户（引用商机需要真实 customer_id）
    let cust = customer::ActiveModel {
        customer_code: Set(format!(
            "C-LEADDEL-{}",
            chrono::Utc::now().timestamp_nanos_opt().expect("测试造数取当前时刻纳秒：Utc::now 必落在 chrono 纳秒可表示区间（约1678-2262 年），None 不可达；旧 timestamp_nanos 超界同样 panic，行为等价")
        )),
        customer_name: Set("引用校验客户".to_string()),
        credit_limit: Set(rust_decimal::Decimal::ZERO),
        payment_terms: Set(30),
        status: Set("active".to_string()),
        customer_type: Set("retail".to_string()),
        owner_id: Set(100),
        created_at: Set(chrono::Utc::now()),
        updated_at: Set(chrono::Utc::now()),
        ..Default::default()
    }
    .insert(&*db)
    .await
    .unwrap();

    let svc = CrmService::new(db.clone());
    let lead = svc
        .create_lead(
            serde_json::from_value::<CreateLeadRequest>(serde_json::json!({
                "company_name": "待转商机线索",
                "lead_status": bingxi_backend::models::status::crm_lead::NEW,
            }))
            .unwrap(),
            100,
            "wave1_operator",
        )
        .await
        .expect("create_lead 应成功（单号自动生成）");
    assert!(
        lead.lead_no.starts_with("LD"),
        "单号前缀契约: {}",
        lead.lead_no
    );

    let opp = svc
        .create_opportunity(
            serde_json::from_value::<CreateOpportunityRequest>(serde_json::json!({
                "opportunity_name": "引用商机",
                "customer_id": cust.id,
                "lead_id": lead.id,
            }))
            .unwrap(),
            100,
            "wave1_operator",
        )
        .await
        .expect("create_opportunity 引用线索应成功");
    assert_eq!(opp.lead_id, Some(lead.id));

    let err = svc
        .delete_lead(lead.id, 100)
        .await
        .expect_err("已转商机线索不可删除（引用校验）");
    assert!(
        matches!(err, AppError::BusinessErrorDisplayable(_)),
        "实际: {err:?}"
    );
    assert_eq!(err.error_code(), "BUSINESS_ERROR");

    // 反向对照：另一条未引用线索真实删除成功（含审计链）
    let lead2 = svc
        .create_lead(
            serde_json::from_value::<CreateLeadRequest>(serde_json::json!({
                "company_name": "无引用线索",
            }))
            .unwrap(),
            100,
            "wave1_operator",
        )
        .await
        .unwrap();
    svc.delete_lead(lead2.id, 100)
        .await
        .expect("无引用删除应成功");
}
