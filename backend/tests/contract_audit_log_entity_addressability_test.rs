//! 审计行按实体可定位 契约锁
//!
//! 钉三件事加一组写入侧真相（全部走真实 handler 与真实库，无 mock）。
//! 一 写入单点收口，update_with_audit 收到占位符 auto_audit 时按泛型实体的
//!   存储表名落 resource_type，BIGSERIAL 与 SERIAL 主键都落真实 resource_id，
//!   且 before_snapshot 对两类主键同样可查得（旧实现 i64 主键恒写 "0" 且快照缺失）。
//! 二 调用方显式传入的实体名不被换名逻辑覆盖，原样落库。
//! 三 列表端点新增的 resource_id 等值过滤的三条性质，带过滤只返回目标实体目标
//!   记录的行、不带过滤时与加参数前的行为一致（向后兼容，旧 auto_audit 池内行仍可查）、
//!   过滤值不存在时返回空集合而不是报错也不是返回全量；resource_id 等值与 keyword
//!   子串的差异同用例钉住（等值 9 不得命中 15）。
//!
//! 夹具口径 表结构唯一来源 backend/migration，setup_test_db 逐例 TRUNCATE 业务表，
//! roles 属保留种子参照表但 auditor 码不在出厂种子内，用例自插自造角色行；
//! users 与 customers 均用例自插（外键父行）。
//! 活体用例按 --test-threads=1 串行（共享真库）。

mod test_common;

use axum::{
    Router,
    body::Body,
    extract::State,
    http::{Method, Request, StatusCode},
    middleware::{Next, from_fn_with_state},
    response::Response,
    routing::get,
};
use bingxi_backend::container::AppState;
use bingxi_backend::handlers::audit_log_handler;
use bingxi_backend::middleware::auth_context::AuthContext;
use bingxi_backend::models::status::master_data;
use bingxi_backend::models::{accounting_period, audit_log, customer, role, sales_quotation, user};
use bingxi_backend::services::audit_log_service::AuditLogService;
use bingxi_backend::utils::admin_checker::AUDITOR_ROLE_CODE;
use chrono::{DateTime, Duration, Utc};
use rust_decimal::Decimal;
use sea_orm::{
    ActiveModelTrait, ActiveValue, DatabaseConnection, EntityTrait, Order, QueryOrder, Set,
};
use serde_json::Value;
use std::sync::Arc;
use test_common::setup_test_db;
use tower::ServiceExt;

/// 注入 AuthContext 与落 updated_by 反查的操作人主键（users 逐例被夹具清空）
const OPERATOR_ID: i32 = 9811;

fn now() -> DateTime<Utc> {
    Utc::now()
}

async fn seed_operator(db: &Arc<DatabaseConnection>) {
    user::ActiveModel {
        id: Set(OPERATOR_ID),
        username: Set("audit_addr_operator".to_string()),
        password_hash: Set("test-only-not-a-real-hash".to_string()),
        real_name: Set(Some("审计定位契约锁操作人".to_string())),
        is_active: Set(true),
        is_totp_enabled: Set(false),
        created_at: Set(now()),
        updated_at: Set(now()),
        ..Default::default()
    }
    .insert(db.as_ref())
    .await
    .unwrap_or_else(|e| panic!("种子操作人插入失败 {e}"));
}

/// 自插 auditor 码角色并返回主键（can_access_audit_logs 按 roles.code 判定，
/// 出厂种子无 auditor 行，迁移也不写 roles，故这里真实造行而不是塞基线逃逸口）
async fn seed_auditor_role(db: &Arc<DatabaseConnection>) -> i32 {
    let model = role::ActiveModel {
        name: Set("审计定位契约锁审计员".to_string()),
        code: Set(AUDITOR_ROLE_CODE.to_string()),
        is_system: Set(false),
        data_scope: Set("all".to_string()),
        created_at: Set(now()),
        updated_at: Set(now()),
        ..Default::default()
    }
    .insert(db.as_ref())
    .await
    .unwrap_or_else(|e| panic!("种子审计员角色插入失败 {e}"));
    model.id
}

async fn seed_customer(db: &Arc<DatabaseConnection>, tag: &str) -> i32 {
    let m = customer::ActiveModel {
        customer_code: Set(format!("CUS-AUDT-{tag}")),
        customer_name: Set(format!("审计定位契约锁客户 {tag}")),
        credit_limit: Set(Decimal::ZERO),
        payment_terms: Set(30),
        status: Set(master_data::ACTIVE.to_string()),
        customer_type: Set("retail".to_string()),
        owner_id: Set(OPERATOR_ID),
        created_at: Set(now()),
        updated_at: Set(now()),
        ..Default::default()
    }
    .insert(db.as_ref())
    .await
    .unwrap_or_else(|e| panic!("种子客户 {tag} 插入失败 {e}"));
    m.id
}

/// 直落一行草稿报价单（BIGSERIAL 主键实体，status 取 sales_quotations 的 chk_status 词表小写值）
async fn seed_quotation(db: &Arc<DatabaseConnection>, customer_id: i32) -> i64 {
    let m = sales_quotation::ActiveModel {
        quotation_no: Set(format!(
            "QT-AUDT-{}",
            Utc::now().timestamp_nanos_opt().unwrap_or_default()
        )),
        customer_id: Set(customer_id),
        sales_user_id: Set(OPERATOR_ID as i64),
        quotation_date: Set(Utc::now().date_naive()),
        valid_until: Set(Utc::now().date_naive() + Duration::days(30)),
        price_terms: Set("FOB".to_string()),
        subtotal: Set(Decimal::ZERO),
        tax_amount: Set(Decimal::ZERO),
        total_amount: Set(Decimal::ZERO),
        status: Set("draft".to_string()),
        created_by: Set(OPERATOR_ID as i64),
        created_at: Set(now()),
        updated_at: Set(now()),
        ..Default::default()
    }
    .insert(db.as_ref())
    .await
    .unwrap_or_else(|e| panic!("种子报价单插入失败 {e}"));
    m.id
}

/// 直落一行 OPEN 会计期间（SERIAL 主键实体，状态词表 OPEN/CLOSING/CLOSED）
async fn seed_period(db: &Arc<DatabaseConnection>, tag: &str) -> i32 {
    let m = accounting_period::ActiveModel {
        year: Set(2026),
        period: Set(match tag {
            "a" => 4,
            _ => 5,
        }),
        period_name: Set(format!("审计定位期间 {tag}")),
        start_date: Set(Utc::now()),
        end_date: Set(Utc::now() + Duration::days(20)),
        status: Set("OPEN".to_string()),
        created_at: Set(now()),
        ..Default::default()
    }
    .insert(db.as_ref())
    .await
    .unwrap_or_else(|e| panic!("种子会计期间 {tag} 插入失败 {e}"));
    m.id
}

/// 直落一行审计记录（读侧过滤用例的受控夹具，列值全部显式给足）
async fn seed_audit_row(
    db: &Arc<DatabaseConnection>,
    rt: &str,
    rid: &str,
    ts: DateTime<Utc>,
) -> i32 {
    let m = audit_log::ActiveModel {
        id: ActiveValue::NotSet,
        user_id: Set(Some(OPERATOR_ID)),
        username: Set(Some("audit_addr_operator".to_string())),
        action: Set("UPDATE".to_string()),
        resource_type: Set(Some(rt.to_string())),
        resource_id: Set(Some(rid.to_string())),
        operation_type: Set(Some("UPDATE".to_string())),
        severity: Set(Some("INFO".to_string())),
        created_at: Set(Some(ts)),
        ..Default::default()
    }
    .insert(db.as_ref())
    .await
    .unwrap_or_else(|e| panic!("种子审计行插入失败 rt={rt} rid={rid} {e}"));
    m.id
}

fn make_auth(role_id: i32) -> AuthContext {
    AuthContext {
        user_id: OPERATOR_ID,
        username: "audit_addr_operator".to_string(),
        role_id: Some(role_id),
        department_id: Some(1),
        data_scope: Some("all".to_string()),
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

fn build_app(db: DatabaseConnection, role_id: i32) -> Router {
    let state = AppState {
        db: Arc::new(db),
        ..Default::default()
    };
    Router::new()
        .route("/erp/audit-logs", get(audit_log_handler::list_audit_logs))
        .with_state(state)
        .layer(from_fn_with_state(make_auth(role_id), inject_auth))
}

async fn call_list(app: &Router, query: &str) -> (StatusCode, Value) {
    let req = Request::builder()
        .method(Method::GET)
        .uri(format!("/erp/audit-logs?{query}"))
        .body(Body::empty())
        .unwrap();
    let resp = app.clone().oneshot(req).await.unwrap();
    let status = resp.status();
    let bytes = axum::body::to_bytes(resp.into_body(), 4 * 1024 * 1024)
        .await
        .unwrap();
    (status, serde_json::from_slice(&bytes).unwrap())
}

/// 取最新一条审计行（写入单点用例用，落库后即查）
async fn latest_audit_row(db: &Arc<DatabaseConnection>) -> audit_log::Model {
    audit_log::Entity::find()
        .order_by(audit_log::Column::Id, Order::Desc)
        .one(db.as_ref())
        .await
        .unwrap()
        .expect("update_with_audit 必须落一条审计行")
}

#[tokio::test]
async fn bigint_pk_placeholder_stamps_table_name_and_real_resource_id() {
    let db = Arc::new(setup_test_db().await);
    seed_operator(&db).await;
    let customer_id = seed_customer(&db, "q1").await;
    let quotation_id = seed_quotation(&db, customer_id).await;

    let loaded = sales_quotation::Entity::find_by_id(quotation_id)
        .one(db.as_ref())
        .await
        .unwrap()
        .expect("种子报价单必须可回读");
    let mut active: sales_quotation::ActiveModel = loaded.into();
    active.status = Set("pending_approval".to_string());
    active.updated_at = Set(now());

    AuditLogService::update_with_audit::<sales_quotation::Entity, sales_quotation::ActiveModel, _>(
        &*db,
        "auto_audit",
        active,
        Some(OPERATOR_ID),
    )
    .await
    .unwrap_or_else(|e| panic!("BIGSERIAL 主键实体经 update_with_audit 更新必须成功 {e}"));

    let row = latest_audit_row(&db).await;
    assert_eq!(
        row.resource_type.as_deref(),
        Some("sales_quotations"),
        "占位符 auto_audit 必须被写入单点换成实体存储表名 sales_quotations"
    );
    assert_eq!(
        row.resource_id.as_deref(),
        Some(quotation_id.to_string().as_str()),
        "BIGSERIAL 主键的 resource_id 必须是真实记录号，旧实现塌成 0 的形态禁止回潮"
    );
    assert_eq!(row.username.as_deref(), Some("audit_addr_operator"));
    let before = row
        .before_snapshot
        .as_ref()
        .expect("BIGSERIAL 主键实体更新前必须查得 before_snapshot，旧实现恒 NULL 的形态禁止回潮");
    assert_eq!(
        before.0["status"].as_str(),
        Some("draft"),
        "before_snapshot 必须是更新前的真实状态 draft"
    );
    let after = row
        .after_snapshot
        .as_ref()
        .expect("after_snapshot 必须存在");
    assert_eq!(
        after.0["status"].as_str(),
        Some("pending_approval"),
        "after_snapshot 必须是提交后的真实状态 pending_approval"
    );
}

#[tokio::test]
async fn int_pk_placeholder_stamps_table_and_explicit_type_is_kept() {
    let db = Arc::new(setup_test_db().await);
    seed_operator(&db).await;
    let period_id = seed_period(&db, "a").await;

    // 第一腿 占位符入参换真表名（SERIAL 主键同样落真实 resource_id）
    let loaded = accounting_period::Entity::find_by_id(period_id)
        .one(db.as_ref())
        .await
        .unwrap()
        .expect("种子会计期间必须可回读");
    let mut active: accounting_period::ActiveModel = loaded.into();
    active.status = Set("CLOSED".to_string());
    AuditLogService::update_with_audit::<
        accounting_period::Entity,
        accounting_period::ActiveModel,
        _,
    >(&*db, "auto_audit", active, Some(OPERATOR_ID))
    .await
    .unwrap_or_else(|e| panic!("SERIAL 主键实体经 update_with_audit 更新必须成功 {e}"));

    let row = latest_audit_row(&db).await;
    assert_eq!(
        row.resource_type.as_deref(),
        Some("accounting_periods"),
        "占位符必须换成会计期间实体表名 accounting_periods"
    );
    assert_eq!(
        row.resource_id.as_deref(),
        Some(period_id.to_string().as_str()),
        "SERIAL 主键的 resource_id 必须是真实记录号"
    );
    let before = row
        .before_snapshot
        .as_ref()
        .expect("SERIAL 主键必须查得 before_snapshot");
    assert_eq!(before.0["status"].as_str(), Some("OPEN"));

    // 第二腿 显式实体名原样落库（换名逻辑不得覆盖调用方给定的值）
    let loaded = accounting_period::Entity::find_by_id(period_id)
        .one(db.as_ref())
        .await
        .unwrap()
        .expect("第一腿更新后期间必须仍可回读");
    let mut active: accounting_period::ActiveModel = loaded.into();
    active.close_remark = Set(Some("显式实体名用例".to_string()));
    AuditLogService::update_with_audit::<
        accounting_period::Entity,
        accounting_period::ActiveModel,
        _,
    >(
        &*db,
        "explicit_accounting_period",
        active,
        Some(OPERATOR_ID),
    )
    .await
    .unwrap_or_else(|e| panic!("显式实体名路径更新必须成功 {e}"));

    let row = latest_audit_row(&db).await;
    assert_eq!(
        row.resource_type.as_deref(),
        Some("explicit_accounting_period"),
        "调用方显式传入的 resource_type 必须原样落库，不被单点换名覆盖"
    );
}

#[tokio::test]
async fn list_filter_entity_and_resource_id_exact_semantics() {
    let db = Arc::new(setup_test_db().await);
    let role_id = seed_auditor_role(&db).await;
    let app = build_app((*db).clone(), role_id);

    let base = Utc::now();
    seed_audit_row(&db, "sales_quotations", "9", base - Duration::seconds(4)).await;
    seed_audit_row(&db, "sales_quotations", "15", base - Duration::seconds(3)).await;
    seed_audit_row(&db, "ar_invoices", "9", base - Duration::seconds(2)).await;
    seed_audit_row(&db, "auto_audit", "9", base - Duration::seconds(1)).await;

    // 性质一 带过滤只返回目标实体目标记录的行
    let (status, body) = call_list(&app, "resource_type=sales_quotations&resource_id=9").await;
    assert_eq!(status, StatusCode::OK, "组合过滤必须 200，实际响应 {body}");
    assert_eq!(
        body["data"]["total"].as_u64(),
        Some(1),
        "resource_id 等值过滤必须把同实体其他记录与跨实体同号记录都挡掉，total 应为 1"
    );
    let items = body["data"]["items"].as_array().expect("items 必须是数组");
    assert_eq!(items.len(), 1, "命中行数应为 1，实际 {body}");
    assert_eq!(items[0]["resource_type"].as_str(), Some("sales_quotations"));
    assert_eq!(items[0]["resource_id"].as_str(), Some("9"));

    // 等值与模糊的语义差对照 同值走 keyword 会连 15 一起命中（证明新参数不是兜底放行）
    let (_, kw_body) = call_list(&app, "keyword=9").await;
    let kw_total = kw_body["data"]["total"].as_u64().unwrap_or(0);
    assert!(
        kw_total >= 3,
        "keyword=9 子串命中 9 与 15 与 auto_audit 池中 9，total 应不小于 3，实际 {kw_body}"
    );

    // 性质二 不带新过滤参数时与加参数前行为一致（旧池内行仍可查，向后兼容）
    let (status2, all_body) = call_list(&app, "").await;
    assert_eq!(status2, StatusCode::OK, "空查询必须 200");
    assert_eq!(
        all_body["data"]["total"].as_u64(),
        Some(4),
        "不带过滤必须原样返回全部四行（含旧 auto_audit 池行），禁止参数引入改变缺省行为"
    );
    let (status3, legacy_body) = call_list(&app, "resource_type=auto_audit").await;
    assert_eq!(status3, StatusCode::OK);
    assert_eq!(
        legacy_body["data"]["total"].as_u64(),
        Some(1),
        "存量 auto_audit 行必须仍可按旧值查到，写入侧换真只影响新行"
    );

    // 性质三 过滤值不存在时返回空集合，不报错也不返回全量
    let (status4, miss_body) =
        call_list(&app, "resource_type=sales_quotations&resource_id=987654").await;
    assert_eq!(
        status4,
        StatusCode::OK,
        "不存在的 resource_id 必须 200 而不是错误响应"
    );
    assert_eq!(
        miss_body["data"]["total"].as_u64(),
        Some(0),
        "不存在的 resource_id 必须空集合而不是全量兜底放行"
    );
    let items_empty = miss_body["data"]["items"]
        .as_array()
        .expect("items 必须是数组");
    assert!(
        items_empty.is_empty(),
        "不存在的 resource_id 命中行必须为 0"
    );
    let (status5, miss_rt) = call_list(&app, "resource_type=no_such_entity_xyz").await;
    assert_eq!(status5, StatusCode::OK);
    assert_eq!(
        miss_rt["data"]["total"].as_u64(),
        Some(0),
        "不存在的 resource_type 同样必须空集合而非报错或全量"
    );
}
