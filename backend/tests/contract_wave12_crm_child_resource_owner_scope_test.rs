//! CRM 子资源面行级归属门契约锁（客户/商机/线索多端点）。
//!
//! 钉死"每个子资源端点在落库/出参前必须经 data_scope 判定源，
//! 不可见即 403+FORBIDDEN、绝不降级成空列表或静默 None"。
//!
//! 真 PostgreSQL（`test_common::setup_test_db()`），禁 sqlite/mock/ignore。
//!
//! 覆盖的端点面：商机跟进记录读、客户 RFM 读、客户操作日志读、客户 CLV 计算与读、
//! 商机阶段时长分析、线索培育计划读与执行、客户联系人删除、客户标签读与解除、
//! 商机阶段变更记录。

mod test_common;

use axum::{
    Router,
    body::Body,
    extract::State,
    http::{Method, Request, StatusCode},
    middleware::{Next, from_fn_with_state},
    response::Response,
    routing::{delete, get, post},
};
use bingxi_backend::container::AppState;
use bingxi_backend::handlers::crm_customer_handler::{
    delete_contact, detach_customer_tag, list_customer_tags,
};
use bingxi_backend::handlers::crm_handler::{
    calculate_customer_clv, execute_nurture_plan, get_customer_clv, get_rfm_score,
    get_stage_duration_analysis, list_customer_audit_logs, list_nurture_plans,
    list_opportunity_follow_ups, record_opportunity_stage_change,
};
use bingxi_backend::middleware::auth_context::AuthContext;
use bingxi_backend::models::{
    crm_lead, crm_opportunity, crm_tag, customer, customer_audit_log, customer_contact,
    customer_lifetime_value, customer_tag, lead_nurture_plan, opportunity_follow_up,
    opportunity_stage_history, user,
};
use chrono::Utc;
use rust_decimal::Decimal;
use sea_orm::{
    ActiveModelTrait, ActiveValue::Set, ColumnTrait, EntityTrait, QueryFilter, QueryOrder,
};
use serde_json::{Value, json};
use std::sync::Arc;
use tower::ServiceExt;

const OWNER_A: i32 = 9001;
const OWNER_B: i32 = 9002;
const CUST_OWN: i32 = 9100;
const CUST_CROSS: i32 = 9101;
const OPP_OWN: i32 = 9200;
const OPP_CROSS: i32 = 9201;
const LEAD_OWN: i32 = 9300;
const LEAD_CROSS: i32 = 9301;
const FOLLOW_UP_ID: i32 = 9400;
const AUDIT_LOG_ID: i32 = 9401;
const STAGE_HIST_OWN: i32 = 9402;
const STAGE_HIST_CROSS: i32 = 9403;
const NURTURE_PLAN_OWN: i32 = 9500;
const NURTURE_PLAN_CROSS: i32 = 9501;
const CONTACT_OWN: i32 = 9600;
const CONTACT_CROSS: i32 = 9601;
const TAG_ID: i32 = 9700;
const CUST_TAG_LINK_OWN: i32 = 9701;
const CUST_TAG_LINK_CROSS: i32 = 9702;

fn make_auth(user_id: i32) -> AuthContext {
    AuthContext {
        user_id,
        username: format!("owner_scope_{user_id}"),
        role_id: Some(2),
        department_id: Some(1),
        data_scope: Some("self".to_string()),
        dept_ids: None,
        dept_member_user_ids: None,
    }
}

async fn inject_auth(
    State(auth): State<AuthContext>,
    mut req: Request<Body>,
    next: Next,
) -> Response {
    req.extensions_mut().insert(auth.clone());
    next.run(req).await
}

async fn call(app: &Router, method: Method, uri: &str, body: Option<Value>) -> (StatusCode, Value) {
    let builder = Request::builder().method(method).uri(uri);
    let req = match body {
        Some(v) => {
            let bytes = serde_json::to_vec(&v).unwrap();
            builder
                .header("content-type", "application/json")
                .body(Body::from(bytes))
                .unwrap()
        }
        None => builder.body(Body::empty()).unwrap(),
    };
    let resp = app.clone().oneshot(req).await.unwrap();
    let status = resp.status();
    let bytes = axum::body::to_bytes(resp.into_body(), 4 * 1024 * 1024)
        .await
        .unwrap();
    (
        status,
        serde_json::from_slice(&bytes).unwrap_or_else(|e| {
            panic!(
                "响应非 JSON（{uri}）: {e}; body={}",
                String::from_utf8_lossy(&bytes)
            )
        }),
    )
}

async fn seed(db: &sea_orm::DatabaseConnection) {
    for uid in [OWNER_A, OWNER_B] {
        user::ActiveModel {
            id: Set(uid),
            username: Set(format!("owner_scope_{uid}")),
            password_hash: Set("test-only-not-a-real-hash".to_string()),
            is_active: Set(true),
            is_totp_enabled: Set(false),
            department_id: Set(Some(1)),
            created_at: Set(Utc::now()),
            updated_at: Set(Utc::now()),
            ..Default::default()
        }
        .insert(db)
        .await
        .unwrap();
    }

    for (cid, owner) in [(CUST_OWN, OWNER_A), (CUST_CROSS, OWNER_B)] {
        customer::ActiveModel {
            id: Set(cid),
            customer_code: Set(format!("CUS-{cid}")),
            customer_name: Set(format!("归属门测试客户-{cid}")),
            credit_limit: Set(Decimal::ZERO),
            payment_terms: Set(30),
            status: Set("active".to_string()),
            customer_type: Set("retail".to_string()),
            owner_id: Set(owner),
            department_id: Set(Some(1)),
            created_at: Set(Utc::now()),
            updated_at: Set(Utc::now()),
            ..Default::default()
        }
        .insert(db)
        .await
        .unwrap();
    }

    for (oid, owner, cust) in [
        (OPP_OWN, OWNER_A, CUST_OWN),
        (OPP_CROSS, OWNER_B, CUST_CROSS),
    ] {
        crm_opportunity::ActiveModel {
            id: Set(oid),
            opportunity_no: Set(format!("OPP-{oid}")),
            opportunity_name: Set(format!("归属门商机-{oid}")),
            customer_id: Set(cust),
            owner_id: Set(owner),
            department_id: Set(Some(1)),
            owner_name: Set(format!("owner_scope_{owner}")),
            opportunity_stage: Set(Some("QUALIFICATION".to_string())),
            created_at: Set(Some(Utc::now())),
            updated_at: Set(Some(Utc::now())),
            ..Default::default()
        }
        .insert(db)
        .await
        .unwrap();
    }

    opportunity_follow_up::ActiveModel {
        id: Set(FOLLOW_UP_ID),
        opportunity_id: Set(OPP_OWN),
        follow_up_type: Set("phone".to_string()),
        content: Set("own 跟进".to_string()),
        follow_up_time: Set(Utc::now()),
        user_id: Set(OWNER_A),
        user_name: Set("owner_scope_9001".to_string()),
        ..Default::default()
    }
    .insert(db)
    .await
    .unwrap();

    customer_audit_log::ActiveModel {
        id: Set(AUDIT_LOG_ID),
        customer_id: Set(CUST_OWN),
        operation: Set("update".to_string()),
        user_id: Set(OWNER_A),
        user_name: Set("owner_scope_9001".to_string()),
        ..Default::default()
    }
    .insert(db)
    .await
    .unwrap();

    for (sid, opp) in [(STAGE_HIST_OWN, OPP_OWN), (STAGE_HIST_CROSS, OPP_CROSS)] {
        opportunity_stage_history::ActiveModel {
            id: Set(sid),
            opportunity_id: Set(opp),
            from_stage: Set(Some("QUALIFICATION".to_string())),
            to_stage: Set("NEEDS_ANALYSIS".to_string()),
            changed_at: Set(Utc::now()),
            changed_by: Set(Some(OWNER_A)),
            duration_days: Set(Some(5)),
        }
        .insert(db)
        .await
        .unwrap();
    }

    for (lid, owner) in [(LEAD_OWN, OWNER_A), (LEAD_CROSS, OWNER_B)] {
        crm_lead::ActiveModel {
            id: Set(lid),
            lead_no: Set(format!("LEAD-{lid}")),
            lead_source: Set("web".to_string()),
            contact_name: Set(format!("联系人-{lid}")),
            owner_id: Set(owner),
            // owner_name 为 m0013 crm_lead NOT NULL 列，缺省经 `..Default::default()` 落
            // ActiveValue::NotSet 会被 Postgres 判为 NULL 触发 23502；按本仓硬规则取真实
            // 种子用户名（= 上面 users 种入的 username），与同文件 crm_opportunity 种法同源。
            owner_name: Set(format!("owner_scope_{owner}")),
            department_id: Set(Some(1)),
            ..Default::default()
        }
        .insert(db)
        .await
        .unwrap();
    }

    for (pid, lid, status) in [
        (NURTURE_PLAN_OWN, LEAD_OWN, "pending"),
        (NURTURE_PLAN_CROSS, LEAD_CROSS, "pending"),
    ] {
        lead_nurture_plan::ActiveModel {
            id: Set(pid),
            lead_id: Set(lid),
            plan_name: Set(format!("培育计划-{pid}")),
            nurture_type: Set("email".to_string()),
            status: Set(Some(status.to_string())),
            created_at: Set(Some(Utc::now())),
            ..Default::default()
        }
        .insert(db)
        .await
        .unwrap();
    }

    for (ct_id, cid) in [(CONTACT_OWN, CUST_OWN), (CONTACT_CROSS, CUST_CROSS)] {
        customer_contact::ActiveModel {
            id: Set(ct_id),
            customer_id: Set(cid),
            name: Set(format!("联系人-{ct_id}")),
            phone: Set("13800000000".to_string()),
            is_primary: Set(true),
            created_at: Set(Utc::now().fixed_offset()),
            updated_at: Set(Utc::now().fixed_offset()),
            ..Default::default()
        }
        .insert(db)
        .await
        .unwrap();
    }

    // 标签字典行：`crm_tag` 属迁移种子参照表（见 src/services/test_common.rs 的
    // SEALED_REFERENCE_TABLES），逐用例**不** TRUNCATE，故本文件每个 #[tokio::test] 再调
    // seed() 时，若直插固定主键 TAG_ID=9700 会在第二个用例撞 `crm_tag_pkey` 23505
    // （name 亦 UNIQUE）。而 TAG_ID 常量还被 detach URL 与 customer_tag.tag_id 引用，必须
    // 保持稳定，故取"按主键查、已存在即跳过"的幂等口径（CI 以 --test-threads=1 串行跑真库，
    // 查后即插无竞态），与同仓 departments 种子既有的幂等写法同源。
    if crm_tag::Entity::find_by_id(TAG_ID)
        .one(db)
        .await
        .unwrap()
        .is_none()
    {
        crm_tag::ActiveModel {
            id: Set(TAG_ID),
            name: Set("高价值".to_string()),
            color: Set("#1890ff".to_string()),
            created_at: Set(Utc::now().fixed_offset()),
            updated_at: Set(Utc::now().fixed_offset()),
            ..Default::default()
        }
        .insert(db)
        .await
        .unwrap();
    }

    for (link_id, cid) in [
        (CUST_TAG_LINK_OWN, CUST_OWN),
        (CUST_TAG_LINK_CROSS, CUST_CROSS),
    ] {
        customer_tag::ActiveModel {
            id: Set(link_id),
            customer_id: Set(cid),
            tag_id: Set(TAG_ID),
            created_at: Set(Utc::now().fixed_offset()),
            ..Default::default()
        }
        .insert(db)
        .await
        .unwrap();
    }
}

async fn seeded_app(auth: AuthContext) -> (Router, Arc<sea_orm::DatabaseConnection>) {
    let db = Arc::new(test_common::setup_test_db().await);
    seed(&db).await;
    let state = AppState {
        db: db.clone(),
        ..Default::default()
    };
    let app = Router::new()
        .route(
            "/crm/opportunities/{id}/follow-ups",
            get(list_opportunity_follow_ups),
        )
        .route("/crm/customers/{id}/rfm", get(get_rfm_score))
        .route(
            "/crm/customers/{id}/audit-logs",
            get(list_customer_audit_logs),
        )
        .route(
            "/crm/customers/{id}/clv/calculate",
            post(calculate_customer_clv),
        )
        .route("/crm/customers/{id}/clv", get(get_customer_clv))
        .route(
            "/crm/opportunities/stage-duration",
            get(get_stage_duration_analysis),
        )
        .route("/crm/leads/nurture-plans", get(list_nurture_plans))
        .route(
            "/crm/leads/nurture-plans/{id}/execute",
            post(execute_nurture_plan),
        )
        .route(
            "/crm/customers/{id}/contacts/{contact_id}",
            delete(delete_contact),
        )
        .route("/crm/customers/{id}/tags", get(list_customer_tags))
        .route(
            "/crm/customers/{id}/tags/{tag_id}",
            delete(detach_customer_tag),
        )
        .route(
            "/crm/opportunities/{id}/stage-change",
            post(record_opportunity_stage_change),
        )
        .with_state(state)
        .layer(from_fn_with_state(auth, inject_auth));
    (app, db)
}

fn assert_403_forbidden(status: StatusCode, v: &Value, label: &str) {
    assert_eq!(status, StatusCode::FORBIDDEN, "{label} 必须 403: {v}");
    assert_eq!(v["code"], "FORBIDDEN", "{label} 信封 code 契约: {v}");
    assert!(
        v["message"].is_string() && v["trace_id"].is_string(),
        "{label} 失败信封键齐全: {v}"
    );
}

// =============================================================
// D01：follow-ups
// =============================================================

#[tokio::test]
async fn d01_own_opportunity_follow_ups_is_2xx() {
    let (app, _db) = seeded_app(make_auth(OWNER_A)).await;
    let (status, v) = call(
        &app,
        Method::GET,
        &format!("/crm/opportunities/{OPP_OWN}/follow-ups"),
        None,
    )
    .await;
    assert_eq!(status, StatusCode::OK, "D01 own 读应 2xx: {v}");
    let rows = v["data"].as_array().unwrap();
    assert!(!rows.is_empty(), "D01 应返回预置的跟进记录");
}

#[tokio::test]
async fn d01_cross_opportunity_follow_ups_is_403() {
    let (app, _db) = seeded_app(make_auth(OWNER_A)).await;
    let (status, v) = call(
        &app,
        Method::GET,
        &format!("/crm/opportunities/{OPP_CROSS}/follow-ups"),
        None,
    )
    .await;
    assert_403_forbidden(status, &v, "D01");
}

// =============================================================
// D02：RFM
// =============================================================

#[tokio::test]
async fn d02_own_customer_rfm_is_2xx() {
    let (app, _db) = seeded_app(make_auth(OWNER_A)).await;
    let (status, v) = call(
        &app,
        Method::GET,
        &format!("/crm/customers/{CUST_OWN}/rfm"),
        None,
    )
    .await;
    assert_eq!(status, StatusCode::OK, "D02 own RFM 应 2xx: {v}");
}

#[tokio::test]
async fn d02_cross_customer_rfm_is_403() {
    let (app, _db) = seeded_app(make_auth(OWNER_A)).await;
    let (status, v) = call(
        &app,
        Method::GET,
        &format!("/crm/customers/{CUST_CROSS}/rfm"),
        None,
    )
    .await;
    assert_403_forbidden(status, &v, "D02");
}

// =============================================================
// D03：audit-logs
// =============================================================

#[tokio::test]
async fn d03_own_customer_audit_logs_is_2xx() {
    let (app, _db) = seeded_app(make_auth(OWNER_A)).await;
    let (status, v) = call(
        &app,
        Method::GET,
        &format!("/crm/customers/{CUST_OWN}/audit-logs"),
        None,
    )
    .await;
    assert_eq!(status, StatusCode::OK, "D03 own 审计日志应 2xx: {v}");
    let rows = v["data"].as_array().unwrap();
    assert!(!rows.is_empty(), "D03 应返回预置的审计日志行");
}

#[tokio::test]
async fn d03_cross_customer_audit_logs_is_403() {
    let (app, _db) = seeded_app(make_auth(OWNER_A)).await;
    let (status, v) = call(
        &app,
        Method::GET,
        &format!("/crm/customers/{CUST_CROSS}/audit-logs"),
        None,
    )
    .await;
    assert_403_forbidden(status, &v, "D03");
}

// =============================================================
// D04：CLV calculate（写+计算）
// =============================================================

#[tokio::test]
async fn d04_own_customer_clv_calculate_is_2xx() {
    let (app, _db) = seeded_app(make_auth(OWNER_A)).await;
    let (status, v) = call(
        &app,
        Method::POST,
        &format!("/crm/customers/{CUST_OWN}/clv/calculate"),
        None,
    )
    .await;
    assert_eq!(status, StatusCode::OK, "D04 own CLV 计算应 2xx: {v}");
}

#[tokio::test]
async fn d04_cross_customer_clv_calculate_is_403_zero_write() {
    let (app, db) = seeded_app(make_auth(OWNER_A)).await;
    let before = customer_lifetime_value::Entity::find()
        .filter(customer_lifetime_value::Column::CustomerId.eq(CUST_CROSS))
        .all(&*db)
        .await
        .unwrap()
        .len();

    let (status, v) = call(
        &app,
        Method::POST,
        &format!("/crm/customers/{CUST_CROSS}/clv/calculate"),
        None,
    )
    .await;
    assert_403_forbidden(status, &v, "D04");

    let after = customer_lifetime_value::Entity::find()
        .filter(customer_lifetime_value::Column::CustomerId.eq(CUST_CROSS))
        .all(&*db)
        .await
        .unwrap()
        .len();
    assert_eq!(after, before, "D04 越权 CLV 计算被拒后应零新增");
}

// =============================================================
// D05：CLV get
// =============================================================

#[tokio::test]
async fn d05_own_customer_clv_get_is_2xx() {
    let (app, _db) = seeded_app(make_auth(OWNER_A)).await;
    let (status, v) = call(
        &app,
        Method::GET,
        &format!("/crm/customers/{CUST_OWN}/clv"),
        None,
    )
    .await;
    assert_eq!(status, StatusCode::OK, "D05 own CLV 读应 2xx: {v}");
}

#[tokio::test]
async fn d05_cross_customer_clv_get_is_403() {
    let (app, _db) = seeded_app(make_auth(OWNER_A)).await;
    let (status, v) = call(
        &app,
        Method::GET,
        &format!("/crm/customers/{CUST_CROSS}/clv"),
        None,
    )
    .await;
    assert_403_forbidden(status, &v, "D05");
}

// =============================================================
// D06：stage-duration
// =============================================================

#[tokio::test]
async fn d06_own_opp_stage_duration_is_2xx() {
    let (app, _db) = seeded_app(make_auth(OWNER_A)).await;
    let (status, v) = call(
        &app,
        Method::GET,
        &format!("/crm/opportunities/stage-duration?opportunity_id={OPP_OWN}"),
        None,
    )
    .await;
    assert_eq!(status, StatusCode::OK, "D06 own 阶段时长应 2xx: {v}");
}

#[tokio::test]
async fn d06_cross_opp_stage_duration_is_403() {
    let (app, _db) = seeded_app(make_auth(OWNER_A)).await;
    let (status, v) = call(
        &app,
        Method::GET,
        &format!("/crm/opportunities/stage-duration?opportunity_id={OPP_CROSS}"),
        None,
    )
    .await;
    assert_403_forbidden(status, &v, "D06");
}

#[tokio::test]
async fn d06_no_opp_id_returns_only_visible_scope() {
    let (app, _db) = seeded_app(make_auth(OWNER_A)).await;
    let (status, v) = call(&app, Method::GET, "/crm/opportunities/stage-duration", None).await;
    assert_eq!(status, StatusCode::OK, "D06 无参查询应 2xx: {v}");
    let rows = v["data"].as_array().unwrap();
    let opp_ids: Vec<i64> = rows
        .iter()
        .filter_map(|r| r["opportunity_id"].as_i64())
        .collect();
    assert!(
        !opp_ids.contains(&(OPP_CROSS as i64)),
        "D06 不传 opp_id 时绝不返回他人商机历史（修复前返回全表）: {v}"
    );
    assert!(
        opp_ids.contains(&(OPP_OWN as i64)),
        "D06 应包含 own 商机历史: {v}"
    );
}

// =============================================================
// D07：nurture-plans list
// =============================================================

#[tokio::test]
async fn d07_own_lead_nurture_plans_visible() {
    let (app, _db) = seeded_app(make_auth(OWNER_A)).await;
    let (status, v) = call(&app, Method::GET, "/crm/leads/nurture-plans", None).await;
    assert_eq!(status, StatusCode::OK, "D07 培育计划列表应 2xx: {v}");
    let rows = v["data"].as_array().unwrap();
    let plan_ids: Vec<i64> = rows.iter().filter_map(|r| r["id"].as_i64()).collect();
    assert!(
        plan_ids.contains(&(NURTURE_PLAN_OWN as i64)),
        "D07 应含 own 线索的计划: {v}"
    );
    assert!(
        !plan_ids.contains(&(NURTURE_PLAN_CROSS as i64)),
        "D07 不应含他人线索的计划（修复前可见全表）: {v}"
    );
}

// =============================================================
// D08：nurture-plan execute
// =============================================================

#[tokio::test]
async fn d08_own_nurture_plan_execute_is_2xx() {
    let (app, _db) = seeded_app(make_auth(OWNER_A)).await;
    let (status, v) = call(
        &app,
        Method::POST,
        &format!("/crm/leads/nurture-plans/{NURTURE_PLAN_OWN}/execute"),
        None,
    )
    .await;
    assert_eq!(status, StatusCode::OK, "D08 own 执行应 2xx: {v}");
    assert_eq!(v["data"]["status"], "executed");
}

#[tokio::test]
async fn d08_cross_nurture_plan_execute_is_403_zero_write() {
    let (app, db) = seeded_app(make_auth(OWNER_A)).await;
    let (status, v) = call(
        &app,
        Method::POST,
        &format!("/crm/leads/nurture-plans/{NURTURE_PLAN_CROSS}/execute"),
        None,
    )
    .await;
    assert_403_forbidden(status, &v, "D08");

    let plan = lead_nurture_plan::Entity::find_by_id(NURTURE_PLAN_CROSS)
        .one(&*db)
        .await
        .unwrap()
        .unwrap();
    assert_eq!(
        plan.status.as_deref(),
        Some("pending"),
        "D08 越权执行后计划状态不得漂移"
    );
}

// =============================================================
// D30：delete contact
// =============================================================

#[tokio::test]
async fn d30_own_contact_delete_is_2xx() {
    let (app, _db) = seeded_app(make_auth(OWNER_A)).await;
    let (status, v) = call(
        &app,
        Method::DELETE,
        &format!("/crm/customers/{CUST_OWN}/contacts/{CONTACT_OWN}"),
        None,
    )
    .await;
    assert_eq!(status, StatusCode::OK, "D30 own 删除联系人应 2xx: {v}");
}

#[tokio::test]
async fn d30_cross_contact_delete_is_403_zero_delete() {
    let (app, db) = seeded_app(make_auth(OWNER_A)).await;
    let (status, v) = call(
        &app,
        Method::DELETE,
        &format!("/crm/customers/{CUST_CROSS}/contacts/{CONTACT_CROSS}"),
        None,
    )
    .await;
    assert_403_forbidden(status, &v, "D30");

    let contact = customer_contact::Entity::find_by_id(CONTACT_CROSS)
        .one(&*db)
        .await
        .unwrap();
    assert!(contact.is_some(), "D30 越权删除后联系人必须仍存在");
}

#[tokio::test]
async fn d30_mismatched_customer_and_contact_is_404() {
    let (app, db) = seeded_app(make_auth(OWNER_A)).await;
    let (status, v) = call(
        &app,
        Method::DELETE,
        &format!("/crm/customers/{CUST_OWN}/contacts/{CONTACT_CROSS}"),
        None,
    )
    .await;
    assert_eq!(
        status,
        StatusCode::NOT_FOUND,
        "D30 联系人不属于路径客户时 404: {v}"
    );
    let contact = customer_contact::Entity::find_by_id(CONTACT_CROSS)
        .one(&*db)
        .await
        .unwrap();
    assert!(contact.is_some(), "D30 不匹配删除后联系人必须仍存在");
}

// =============================================================
// D31：list customer tags
// =============================================================

#[tokio::test]
async fn d31_own_customer_tags_is_2xx() {
    let (app, _db) = seeded_app(make_auth(OWNER_A)).await;
    let (status, v) = call(
        &app,
        Method::GET,
        &format!("/crm/customers/{CUST_OWN}/tags"),
        None,
    )
    .await;
    assert_eq!(status, StatusCode::OK, "D31 own 标签列表应 2xx: {v}");
    let rows = v["data"].as_array().unwrap();
    assert!(!rows.is_empty(), "D31 应返回预置标签");
}

#[tokio::test]
async fn d31_cross_customer_tags_is_403() {
    let (app, _db) = seeded_app(make_auth(OWNER_A)).await;
    let (status, v) = call(
        &app,
        Method::GET,
        &format!("/crm/customers/{CUST_CROSS}/tags"),
        None,
    )
    .await;
    assert_403_forbidden(status, &v, "D31");
}

// =============================================================
// D32：detach customer tag
// =============================================================

#[tokio::test]
async fn d32_own_tag_detach_is_2xx() {
    let (app, _db) = seeded_app(make_auth(OWNER_A)).await;
    let (status, v) = call(
        &app,
        Method::DELETE,
        &format!("/crm/customers/{CUST_OWN}/tags/{TAG_ID}"),
        None,
    )
    .await;
    assert_eq!(status, StatusCode::OK, "D32 own 解除标签应 2xx: {v}");
}

#[tokio::test]
async fn d32_cross_tag_detach_is_403_zero_delete() {
    let (app, db) = seeded_app(make_auth(OWNER_A)).await;
    let (status, v) = call(
        &app,
        Method::DELETE,
        &format!("/crm/customers/{CUST_CROSS}/tags/{TAG_ID}"),
        None,
    )
    .await;
    assert_403_forbidden(status, &v, "D32");

    let link = customer_tag::Entity::find()
        .filter(customer_tag::Column::CustomerId.eq(CUST_CROSS))
        .filter(customer_tag::Column::TagId.eq(TAG_ID))
        .one(&*db)
        .await
        .unwrap();
    assert!(link.is_some(), "D32 越权解除后标签关联必须仍存在");
}

// =============================================================
// S1：stage-change — 归属门 + from_stage 以库内为准
// =============================================================

#[tokio::test]
async fn s1_own_stage_change_is_2xx_and_from_stage_from_db() {
    let (app, db) = seeded_app(make_auth(OWNER_A)).await;
    let body = json!({
        "from_stage": "NEGOTIATION",
        "to_stage": "PROPOSAL"
    });
    let (status, v) = call(
        &app,
        Method::POST,
        &format!("/crm/opportunities/{OPP_OWN}/stage-change"),
        Some(body),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "S1 own 阶段变更应 2xx: {v}");

    let history = opportunity_stage_history::Entity::find()
        .filter(opportunity_stage_history::Column::OpportunityId.eq(OPP_OWN))
        .order_by(
            opportunity_stage_history::Column::ChangedAt,
            sea_orm::Order::Desc,
        )
        .one(&*db)
        .await
        .unwrap()
        .unwrap();
    assert_eq!(
        history.from_stage.as_deref(),
        Some("QUALIFICATION"),
        "S1 from_stage 必须以库内当前阶段为准（非客户端自报 NEGOTIATION）"
    );
}

#[tokio::test]
async fn s1_cross_stage_change_is_403_zero_write() {
    let (app, db) = seeded_app(make_auth(OWNER_A)).await;
    let before = opportunity_stage_history::Entity::find()
        .filter(opportunity_stage_history::Column::OpportunityId.eq(OPP_CROSS))
        .all(&*db)
        .await
        .unwrap()
        .len();

    let body = json!({
        "from_stage": "QUALIFICATION",
        "to_stage": "PROPOSAL"
    });
    let (status, v) = call(
        &app,
        Method::POST,
        &format!("/crm/opportunities/{OPP_CROSS}/stage-change"),
        Some(body),
    )
    .await;
    assert_403_forbidden(status, &v, "S1");

    let after = opportunity_stage_history::Entity::find()
        .filter(opportunity_stage_history::Column::OpportunityId.eq(OPP_CROSS))
        .all(&*db)
        .await
        .unwrap()
        .len();
    assert_eq!(after, before, "S1 越权阶段变更后不得新增历史记录");
}

#[tokio::test]
async fn s1_forged_from_stage_does_not_take_effect() {
    let (app, db) = seeded_app(make_auth(OWNER_A)).await;
    let body = json!({
        "from_stage": "CLOSED_WON",
        "to_stage": "PROPOSAL"
    });
    let (status, v) = call(
        &app,
        Method::POST,
        &format!("/crm/opportunities/{OPP_OWN}/stage-change"),
        Some(body),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "S1 合法写入应 2xx: {v}");

    let history = opportunity_stage_history::Entity::find()
        .filter(opportunity_stage_history::Column::OpportunityId.eq(OPP_OWN))
        .order_by(
            opportunity_stage_history::Column::ChangedAt,
            sea_orm::Order::Desc,
        )
        .one(&*db)
        .await
        .unwrap()
        .unwrap();
    assert_ne!(
        history.from_stage.as_deref(),
        Some("CLOSED_WON"),
        "S1 伪造的 from_stage 不得落库（修复前直接透传客户端值）"
    );
    assert_eq!(
        history.from_stage.as_deref(),
        Some("QUALIFICATION"),
        "S1 落库的 from_stage 应为库内实际阶段"
    );
}

// =============================================================
// 反空操作自证（注释）：
//
// 修复前 handler 将 auth 提取器写成 `_auth`（丢弃），仅过 RBAC 键不过行级归属：
// - D01 list_opportunity_follow_ups 仅按 opportunity_id 过滤，无 get_opportunity 门。
//   断言 "cross owner → 403" 修复前拿到 2xx 直接返回他人记录行（判红分叉行：get_opportunity 的 `?`）。
// - D02/D03/D04/D05 仅按 customer_id 查，无 check_resource_owner。
//   断言 "cross → 403" 修复前拿到 2xx（判红分叉行：check_resource_owner 返回 false → permission_denied 的 `?`）。
// - D04 写门 ensure_cross_owner_write_allowed 修复前不存在，断言"零新增"通过
//   是因为修复前直接落库成功（无 403 即无零断言）——判红的是 403 断言本身。
// - D06 传任意 opp_id 无门修复前偷读，不传参返回全表修复前他人历史混入。
//   判红分叉行：get_opportunity `?` 和 is_in(visible_opps) 排除。
// - D07 无归属过滤修复前全量返回他人线索的计划。
//   判红分叉行：is_in(visible_leads) 排除。
// - D08 仅按 plan_id find_by_id 后直接改状态无门。
//   判红分叉行：get_lead `?` 的 permission_denied。
// - D30 丢弃 _customer_id，仅按 contact_id 删除。
//   判红分叉行：check_resource_owner + contact.customer_id != customer_id 的 404。
// - D31/D32 仅按 customer_id 查/删，无归属门。
//   判红分叉行：check_resource_owner → 403。
// - S1 无归属门 + from_stage 由客户端自报直灌。
//   判红分叉行：get_opportunity 返回 existing.opportunity_stage 替代 req.from_stage。
//
// 夹具防法：setup_test_db() 自带 TRUNCATE 台账双向守卫（services/test_common.rs
// 清表前守卫 + 清表后 seaql_migrations 行数自检），本文件不调 Migrator、
// 不依赖台账内容。
// =============================================================
