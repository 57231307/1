//! 面料行业版销售订单状态门活体契约锁（update、approve、delete 三条腿）
//!
//! 钉住的缺陷形态（修复前事实，主干调查取证结论）。
//! - 更新腿允许调用方自由写入 status（传什么写什么，绕开状态机）。
//! - 审核腿无条件通过，pending 之外（已 approved、已 rejected）再审批仍返回 200，
//!   重复审批与首次审批不可区分（幂等假绿）。
//! - 删除腿无任何状态门，已审批单据可直接物理删除，审批追溯被破坏。
//!
//! 收口口径（与同族已合入实现对齐）。
//! - 状态写入收口走"缺键点名拒绝"（采购收货、应付发票两族同口径），更新腿收到
//!   status 键一律 400 VALIDATION_ERROR 并零写入，不忽略、不代改。
//! - 审核腿与拒绝腿（既有合规范式）同风格，事务内行锁与仅 pending 可推进门，
//!   非 pending 一律 400 BUSINESS_ERROR。
//! - 删除门仅放行尚未被审批链处置的 pending 行，非 pending 400 BUSINESS_ERROR。
//!
//! 判定方向（每条正向都配反向，反向必证库内状态零变化）。
//! - 更新腿，带 status 键被拒且整行未部分写入，不带 status 的正常字段真实落库。
//! - 审核腿，pending 推进后读回 approved 与审批人三件套，已 approved 再审批被拒
//!   且 approved_by 与 approved_at 未被覆盖，已 rejected 审批被拒，无审批键角色 403。
//! - 删除腿，pending 删除后主表与明细行都不存在，已 approved 删除被拒且行仍在，
//!   已 rejected 删除被拒且行仍在。
//!
//! 权限面。判定全部经真实 permission_middleware，按精确键 fabric-orders:update、
//! fabric-orders:approve、fabric-orders:delete 放行，不存在粗域展开，授权行为测试
//! 自种角色与授权行并自清理。
//!
//! 通道。连已迁移 PostgreSQL 真跑（test_common::setup_test_db 每次 TRUNCATE 业务表），
//! 属 CI 活库串行组（--test-threads=1），缺 TEST_DATABASE_URL 直接 panic。

mod test_common;

use axum::{
    Router,
    body::Body,
    extract::State,
    http::{Method, Request, StatusCode},
    middleware::{Next, from_fn_with_state},
    response::Response,
    routing::{post, put},
};
use bingxi_backend::container::AppState;
use bingxi_backend::handlers::sales_fabric_order_handler::{
    approve_fabric_order, delete_fabric_order, update_fabric_order,
};
use bingxi_backend::middleware::auth_context::AuthContext;
use bingxi_backend::middleware::permission::{invalidate_permission_cache, permission_middleware};
use bingxi_backend::models::status::{master_data, sales_fabric_order};
use bingxi_backend::models::{
    customer, product, role, role_permission, sales_order, sales_order_item, user,
};
use sea_orm::{ActiveModelTrait, ColumnTrait, DatabaseConnection, EntityTrait, QueryFilter, Set};
use serde_json::{Value, json};
use std::sync::Arc;
use tower::ServiceExt;

/// 种子身份（避开 contract_wave11 面料段的 9482/9483，两文件可在串行组内共存）
const FO_USER_ID: i32 = 9492;
const FO_OTHER_USER_ID: i32 = 9495;
const FO_CUSTOMER_ID: i32 = 9493;
const FO_PRODUCT_ID: i32 = 9494;

const ROLE_UPDATE_GRANTEE: &str = "it_fo_g_update";
const ROLE_APPROVE_GRANTEE: &str = "it_fo_g_approve";
const ROLE_DELETE_GRANTEE: &str = "it_fo_g_delete";
const ROLE_STRANGER: &str = "it_fo_stranger";
const ALL_FO_ROLES: [&str; 4] = [
    ROLE_UPDATE_GRANTEE,
    ROLE_APPROVE_GRANTEE,
    ROLE_DELETE_GRANTEE,
    ROLE_STRANGER,
];

fn now_ts() -> chrono::DateTime<chrono::Utc> {
    chrono::Utc::now()
}

/// 固定历史时刻（用于"已审批行未被重复审批覆盖"的可比读回基准）
fn fixed_past() -> chrono::DateTime<chrono::Utc> {
    chrono::DateTime::parse_from_rfc3339("2026-01-05T08:00:00Z")
        .expect("固定时刻字面量")
        .with_timezone(&chrono::Utc)
}

// ---------------------------------------------------------------------------
// 种子与请求 helper
// ---------------------------------------------------------------------------

/// 清掉上一轮可能残留的自种角色与授权行（roles 与 role_permissions 为种子参照表，
/// 不在夹具 TRUNCATE 清单内，必须显式自清理，与 contract_wave11 面料段同法）
async fn cleanup_fo_roles(db: &DatabaseConnection) {
    let role_ids = role::Entity::find()
        .filter(role::Column::Code.is_in(ALL_FO_ROLES))
        .all(db)
        .await
        .unwrap()
        .into_iter()
        .map(|r| r.id)
        .collect::<Vec<i32>>();
    if !role_ids.is_empty() {
        role_permission::Entity::delete_many()
            .filter(role_permission::Column::RoleId.is_in(role_ids.clone()))
            .exec(db)
            .await
            .unwrap();
        for rid in &role_ids {
            invalidate_permission_cache(*rid);
        }
    }
    role::Entity::delete_many()
        .filter(role::Column::Code.is_in(ALL_FO_ROLES))
        .exec(db)
        .await
        .unwrap();
}

async fn seed_user(db: &DatabaseConnection) {
    user::ActiveModel {
        id: Set(FO_USER_ID),
        username: Set("it_fo_gates".to_string()),
        password_hash: Set("test-only-not-a-real-hash".to_string()),
        real_name: Set(Some("面料状态门操作人".to_string())),
        is_active: Set(true),
        is_totp_enabled: Set(false),
        created_at: Set(now_ts()),
        updated_at: Set(now_ts()),
        ..Default::default()
    }
    .insert(db)
    .await
    .unwrap_or_else(|e| panic!("种子操作人失败: {e}"));
}

async fn seed_role(db: &DatabaseConnection, code: &str) -> i32 {
    let model = role::ActiveModel {
        name: Set(format!("面料状态门角色 {code}")),
        code: Set(code.to_string()),
        is_system: Set(false),
        data_scope: Set("all".to_string()),
        created_at: Set(now_ts()),
        updated_at: Set(now_ts()),
        ..Default::default()
    }
    .insert(db)
    .await
    .unwrap_or_else(|e| panic!("种子角色 {code} 失败: {e}"));
    model.id
}

/// 角色级 allowed=true 授权行（resource_id=NULL，运行时按精确键 fabric-orders:action 命中）
async fn seed_grant(db: &DatabaseConnection, role_id: i32, resource: &str, action: &str) {
    role_permission::ActiveModel {
        role_id: Set(role_id),
        resource_type: Set(resource.to_string()),
        resource_id: Set(None),
        action: Set(action.to_string()),
        allowed: Set(true),
        permission_code: Set(None),
        created_at: Set(now_ts()),
        updated_at: Set(now_ts()),
        ..Default::default()
    }
    .insert(db)
    .await
    .unwrap_or_else(|e| panic!("种子授权行 {resource}:{action} 失败: {e}"));
}

async fn seed_customer(db: &DatabaseConnection) {
    customer::ActiveModel {
        id: Set(FO_CUSTOMER_ID),
        customer_code: Set("CUS-FO-GATES".to_string()),
        customer_name: Set("面料状态门客户".to_string()),
        customer_type: Set("retail".to_string()),
        credit_limit: Set(rust_decimal::Decimal::ZERO),
        payment_terms: Set(30),
        status: Set(master_data::ACTIVE.to_string()),
        owner_id: Set(FO_USER_ID),
        created_at: Set(now_ts()),
        updated_at: Set(now_ts()),
        ..Default::default()
    }
    .insert(db)
    .await
    .unwrap_or_else(|e| panic!("种子客户失败: {e}"));
}

async fn seed_product(db: &DatabaseConnection) {
    product::ActiveModel {
        id: Set(FO_PRODUCT_ID),
        name: Set("面料状态门样品布".to_string()),
        code: Set("PRD-FO-GATES-T1".to_string()),
        unit: Set("米".to_string()),
        status: Set("active".to_string()),
        is_deleted: Set(false),
        product_type: Set("fabric".to_string()),
        created_at: Set(now_ts()),
        updated_at: Set(now_ts()),
        ..Default::default()
    }
    .insert(db)
    .await
    .unwrap_or_else(|e| panic!("种子产品失败: {e}"));
}

/// 种一行指定状态的面料订单（sales_orders 表，订单号带毫秒保证并行用例互不撞）
async fn seed_fabric_order_with_status(
    db: &DatabaseConnection,
    status: &str,
    approved_by: Option<i32>,
    approved_at: Option<chrono::DateTime<chrono::Utc>>,
    rejected_reason: Option<&str>,
) -> i32 {
    let order = sales_order::ActiveModel {
        order_no: Set(format!(
            "SO-FO-GATES-{status}-{}",
            chrono::Utc::now().timestamp_nanos_opt().unwrap_or_default()
        )),
        customer_id: Set(FO_CUSTOMER_ID),
        order_date: Set(now_ts()),
        required_date: Set(Some(now_ts())),
        status: Set(status.to_string()),
        subtotal: Set(rust_decimal::Decimal::new(10000, 2)),
        tax_amount: Set(rust_decimal::Decimal::ZERO),
        discount_amount: Set(rust_decimal::Decimal::ZERO),
        shipping_cost: Set(rust_decimal::Decimal::ZERO),
        total_amount: Set(rust_decimal::Decimal::new(10000, 2)),
        paid_amount: Set(rust_decimal::Decimal::ZERO),
        balance_amount: Set(rust_decimal::Decimal::new(10000, 2)),
        created_by: Set(Some(FO_USER_ID)),
        approved_by: Set(approved_by),
        approved_at: Set(approved_at),
        rejected_reason: Set(rejected_reason.map(str::to_string)),
        created_at: Set(now_ts()),
        updated_at: Set(now_ts()),
        ..Default::default()
    }
    .insert(db)
    .await
    .unwrap_or_else(|e| panic!("种子面料订单（{status}）失败: {e}"));
    order.id
}

async fn seed_pending(db: &DatabaseConnection) -> i32 {
    seed_fabric_order_with_status(db, sales_fabric_order::PENDING, None, None, None).await
}

async fn seed_approved(db: &DatabaseConnection) -> i32 {
    seed_fabric_order_with_status(
        db,
        sales_fabric_order::APPROVED,
        Some(FO_OTHER_USER_ID),
        Some(fixed_past()),
        None,
    )
    .await
}

async fn seed_rejected(db: &DatabaseConnection) -> i32 {
    seed_fabric_order_with_status(
        db,
        sales_fabric_order::REJECTED,
        Some(FO_OTHER_USER_ID),
        Some(fixed_past()),
        Some("色差超出允收范围"),
    )
    .await
}

/// 种一行订单明细（列集合与 fabric 建单路径同一必填集）
async fn seed_fabric_order_item(db: &DatabaseConnection, order_id: i32) -> i32 {
    let item = sales_order_item::ActiveModel {
        order_id: Set(order_id),
        product_id: Set(FO_PRODUCT_ID),
        quantity: Set(rust_decimal::Decimal::new(1000, 2)),
        unit_price: Set(rust_decimal::Decimal::new(5000, 2)),
        discount_percent: Set(rust_decimal::Decimal::ZERO),
        tax_percent: Set(rust_decimal::Decimal::ZERO),
        subtotal: Set(rust_decimal::Decimal::new(5000, 2)),
        tax_amount: Set(rust_decimal::Decimal::ZERO),
        discount_amount: Set(rust_decimal::Decimal::ZERO),
        total_amount: Set(rust_decimal::Decimal::new(5000, 2)),
        shipped_quantity: Set(rust_decimal::Decimal::ZERO),
        color_no: Set("FO-TEST-COLOR".to_string()),
        quantity_meters: Set(rust_decimal::Decimal::new(1000, 2)),
        quantity_kg: Set(rust_decimal::Decimal::ZERO),
        color_extra_cost: Set(rust_decimal::Decimal::ZERO),
        grade_price_diff: Set(rust_decimal::Decimal::ZERO),
        shipped_quantity_meters: Set(rust_decimal::Decimal::ZERO),
        shipped_quantity_kg: Set(rust_decimal::Decimal::ZERO),
        created_at: Set(now_ts()),
        updated_at: Set(now_ts()),
        ..Default::default()
    }
    .insert(db)
    .await
    .unwrap_or_else(|e| panic!("种子订单明细失败: {e}"));
    item.id
}

/// 按真实挂载路径（routes/sales.rs 面料段）建路由并挂真实权限中间件
fn fo_app(db: Arc<DatabaseConnection>, role_id: i32) -> Router {
    let state = AppState {
        db: db.clone(),
        ..Default::default()
    };
    async fn inject_auth(
        auth: State<AuthContext>,
        mut request: Request<Body>,
        next: Next,
    ) -> Response {
        request.extensions_mut().insert(auth.0);
        next.run(request).await
    }
    let auth = AuthContext {
        user_id: FO_USER_ID,
        username: "it_fo_gates".to_string(),
        role_id: Some(role_id),
        department_id: None,
        data_scope: Some("all".to_string()),
        dept_ids: None,
        dept_member_user_ids: None,
    };
    Router::new()
        .route(
            "/api/v1/erp/sales/fabric-orders/{id}",
            put(update_fabric_order).delete(delete_fabric_order),
        )
        .route(
            "/api/v1/erp/sales/fabric-orders/{id}/approve",
            post(approve_fabric_order),
        )
        .layer(from_fn_with_state(state.clone(), permission_middleware))
        .layer(from_fn_with_state(auth, inject_auth))
        .with_state(state)
}

async fn send(app: &Router, method: Method, uri: &str, body: Option<Value>) -> (StatusCode, Value) {
    let builder = Request::builder()
        .method(method)
        .uri(uri)
        .header("content-type", "application/json");
    let req = match body {
        Some(v) => builder.body(Body::from(v.to_string())).unwrap(),
        None => builder.body(Body::empty()).unwrap(),
    };
    let resp = app.clone().oneshot(req).await.unwrap();
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

async fn put_fo(app: &Router, id: i32, payload: Value) -> (StatusCode, Value) {
    send(
        app,
        Method::PUT,
        &format!("/api/v1/erp/sales/fabric-orders/{id}"),
        Some(payload),
    )
    .await
}

async fn approve_fo(app: &Router, id: i32) -> (StatusCode, Value) {
    send(
        app,
        Method::POST,
        &format!("/api/v1/erp/sales/fabric-orders/{id}/approve"),
        None,
    )
    .await
}

async fn delete_fo(app: &Router, id: i32) -> (StatusCode, Value) {
    send(
        app,
        Method::DELETE,
        &format!("/api/v1/erp/sales/fabric-orders/{id}"),
        None,
    )
    .await
}

async fn reload_order(db: &DatabaseConnection, id: i32) -> sales_order::Model {
    sales_order::Entity::find_by_id(id)
        .one(db)
        .await
        .unwrap()
        .unwrap_or_else(|| panic!("订单行 {id} 必须仍在库（读回真值）"))
}

/// 每用例统一前置，清残留、种身份与参照行，返回（更新腿角色，审批腿角色，删除腿角色，陌生人角色）
async fn setup_fo_fixture(db: &DatabaseConnection) -> (i32, i32, i32, i32) {
    cleanup_fo_roles(db).await;
    seed_user(db).await;
    seed_customer(db).await;
    let role_update = seed_role(db, ROLE_UPDATE_GRANTEE).await;
    seed_grant(db, role_update, "fabric-orders", "update").await;
    let role_approve = seed_role(db, ROLE_APPROVE_GRANTEE).await;
    seed_grant(db, role_approve, "fabric-orders", "approve").await;
    let role_delete = seed_role(db, ROLE_DELETE_GRANTEE).await;
    seed_grant(db, role_delete, "fabric-orders", "delete").await;
    let role_stranger = seed_role(db, ROLE_STRANGER).await;
    for rid in [role_update, role_approve, role_delete, role_stranger] {
        invalidate_permission_cache(rid);
    }
    (role_update, role_approve, role_delete, role_stranger)
}

// ---------------------------------------------------------------------------
// 一、更新腿，status 键点名拒绝，正常字段真实落库
// ---------------------------------------------------------------------------

/// 反向，pending 行更新携带 status 键得 400 VALIDATION_ERROR，整行零写入（无部分落库）
#[tokio::test]
async fn update_leg_with_status_key_rejected_and_row_untouched() {
    let db = Arc::new(test_common::setup_test_db().await);
    let (role_update, _, _, _) = setup_fo_fixture(&db).await;
    let order_id = seed_pending(&db).await;
    let before = reload_order(&db, order_id).await;

    let app = fo_app(db.clone(), role_update);
    let (status, body) = put_fo(
        &app,
        order_id,
        json!({"status": "approved", "shipping_address": "杭州市余杭区文一西路"}),
    )
    .await;
    assert_eq!(
        status,
        StatusCode::BAD_REQUEST,
        "更新腿携带 status 键必须缺键点名拒绝（400），实得 {status} {body}"
    );
    assert_eq!(
        body["code"].as_str(),
        Some("VALIDATION_ERROR"),
        "更新腿 status 键拒绝的机器码必须为 VALIDATION_ERROR，实得 {body}"
    );

    let after = reload_order(&db, order_id).await;
    assert_eq!(
        after.status,
        sales_fabric_order::PENDING,
        "被拒的更新不得改动状态（非法状态跃迁必须零落库）"
    );
    assert_eq!(
        after.shipping_address, before.shipping_address,
        "拒绝必须整行零写入：同载荷里的正常字段也不得部分落库"
    );
    assert_eq!(
        after.updated_at, before.updated_at,
        "被拒的更新不得推进 updated_at（写后读回真值证明 UPDATE 未发生）"
    );
}

/// 正向，不带 status 的正常字段更新得 200，写后读回库内真值，状态保持 pending
#[tokio::test]
async fn update_leg_without_status_persists_and_reads_back() {
    let db = Arc::new(test_common::setup_test_db().await);
    let (role_update, _, _, _) = setup_fo_fixture(&db).await;
    let order_id = seed_pending(&db).await;

    let app = fo_app(db.clone(), role_update);
    let (status, body) = put_fo(
        &app,
        order_id,
        json!({"shipping_address": "杭州市余杭区文一西路"}),
    )
    .await;
    assert_eq!(
        status,
        StatusCode::OK,
        "不带 status 的正常更新必须放行，实得 {status} {body}"
    );

    let after = reload_order(&db, order_id).await;
    assert_eq!(
        after.shipping_address.as_deref(),
        Some("杭州市余杭区文一西路"),
        "正常字段必须真实落库（写后读回真值）"
    );
    assert_eq!(
        after.status,
        sales_fabric_order::PENDING,
        "更新腿不得改动状态，读回应仍为 pending"
    );
}

// ---------------------------------------------------------------------------
// 二、审核腿，仅 pending 可推进，非 pending 拒绝且审批三件套零覆盖
// ---------------------------------------------------------------------------

/// 正向，pending 行审批得 200，读回 status=approved、approved_by、approved_at 三件套
#[tokio::test]
async fn approve_leg_pending_flips_and_reads_back() {
    let db = Arc::new(test_common::setup_test_db().await);
    let (_, role_approve, _, _) = setup_fo_fixture(&db).await;
    let order_id = seed_pending(&db).await;

    let app = fo_app(db.clone(), role_approve);
    let (status, body) = approve_fo(&app, order_id).await;
    assert_eq!(
        status,
        StatusCode::OK,
        "持 fabric-orders:approve 的角色对 pending 行应放行，实得 {status} {body}"
    );

    let after = reload_order(&db, order_id).await;
    assert_eq!(
        after.status,
        sales_fabric_order::APPROVED,
        "审批后状态必须真实翻为 approved（读回库内真值）"
    );
    assert_eq!(
        after.approved_by,
        Some(FO_USER_ID),
        "审批人必须按会话身份落库 approved_by"
    );
    assert!(
        after.approved_at.is_some(),
        "审批时刻必须落库 approved_at（重复审批与首次审批的追溯基准）"
    );
}

/// 反向，已 approved 行再审批得 400 BUSINESS_ERROR，approved_by 与 approved_at 零覆盖
#[tokio::test]
async fn approve_leg_on_approved_returns_business_error_with_zero_change() {
    let db = Arc::new(test_common::setup_test_db().await);
    let (_, role_approve, _, _) = setup_fo_fixture(&db).await;
    let order_id = seed_approved(&db).await;

    let app = fo_app(db.clone(), role_approve);
    let (status, body) = approve_fo(&app, order_id).await;
    assert_eq!(
        status,
        StatusCode::BAD_REQUEST,
        "已审批单再审批不得返回成功（幂等假绿），必须 400，实得 {status} {body}"
    );
    assert_eq!(
        body["code"].as_str(),
        Some("BUSINESS_ERROR"),
        "重复审批拒绝的机器码必须为 BUSINESS_ERROR，实得 {body}"
    );

    let after = reload_order(&db, order_id).await;
    assert_eq!(
        after.status,
        sales_fabric_order::APPROVED,
        "非法状态跃迁被拒后状态不得改动"
    );
    assert_eq!(
        after.approved_by,
        Some(FO_OTHER_USER_ID),
        "被拒的重复审批不得覆盖原审批人（追溯链完整性）"
    );
    assert_eq!(
        after.approved_at,
        Some(fixed_past()),
        "被拒的重复审批不得覆盖原审批时刻"
    );
}

/// 反向，已 rejected 行审批得 400 BUSINESS_ERROR，状态与拒绝理由零变化
#[tokio::test]
async fn approve_leg_on_rejected_returns_business_error_with_zero_change() {
    let db = Arc::new(test_common::setup_test_db().await);
    let (_, role_approve, _, _) = setup_fo_fixture(&db).await;
    let order_id = seed_rejected(&db).await;

    let app = fo_app(db.clone(), role_approve);
    let (status, body) = approve_fo(&app, order_id).await;
    assert_eq!(
        status,
        StatusCode::BAD_REQUEST,
        "已拒绝行审批必须被状态门拒绝（400），实得 {status} {body}"
    );
    assert_eq!(
        body["code"].as_str(),
        Some("BUSINESS_ERROR"),
        "已拒绝行审批拒绝的机器码必须为 BUSINESS_ERROR，实得 {body}"
    );

    let after = reload_order(&db, order_id).await;
    assert_eq!(
        after.status,
        sales_fabric_order::REJECTED,
        "被拒审批不得改动 rejected 状态"
    );
    assert_eq!(
        after.rejected_reason.as_deref(),
        Some("色差超出允收范围"),
        "被拒审批不得覆盖既有拒绝理由"
    );
}

/// RBAC 反向，不持 fabric-orders:approve 精确键的角色得 403 FORBIDDEN，行零变化
#[tokio::test]
async fn approve_leg_without_key_returns_403_and_row_untouched() {
    let db = Arc::new(test_common::setup_test_db().await);
    let (_, _, _, role_stranger) = setup_fo_fixture(&db).await;
    let order_id = seed_pending(&db).await;

    let app = fo_app(db.clone(), role_stranger);
    let (status, body) = approve_fo(&app, order_id).await;
    assert_eq!(
        status,
        StatusCode::FORBIDDEN,
        "无审批键必须被门层 403（精确键判定，不做粗域放行），实得 {status} {body}"
    );
    assert_eq!(
        body["code"].as_str(),
        Some("FORBIDDEN"),
        "403 机器码必须为 FORBIDDEN，实得 {body}"
    );

    let after = reload_order(&db, order_id).await;
    assert_eq!(
        after.status,
        sales_fabric_order::PENDING,
        "403 必须发生在 RBAC 门层：不得改业务状态"
    );
}

// ---------------------------------------------------------------------------
// 三、删除腿，仅 pending 可删，非 pending 拒绝且行存活
// ---------------------------------------------------------------------------

/// 正向，pending 行删除得 200，读回主表行与明细行都不存在
#[tokio::test]
async fn delete_leg_pending_removes_row_and_items() {
    let db = Arc::new(test_common::setup_test_db().await);
    let (_, _, role_delete, _) = setup_fo_fixture(&db).await;
    seed_product(&db).await;
    let order_id = seed_pending(&db).await;
    let item_id = seed_fabric_order_item(&db, order_id).await;

    let app = fo_app(db.clone(), role_delete);
    let (status, body) = delete_fo(&app, order_id).await;
    assert_eq!(
        status,
        StatusCode::OK,
        "持 fabric-orders:delete 的角色对 pending 行应放行，实得 {status} {body}"
    );

    let gone = sales_order::Entity::find_by_id(order_id)
        .one(db.as_ref())
        .await
        .unwrap();
    assert!(gone.is_none(), "删除生效后主表行必须不存在（读回真值）");
    let item_gone = sales_order_item::Entity::find_by_id(item_id)
        .one(db.as_ref())
        .await
        .unwrap();
    assert!(item_gone.is_none(), "明细行必须随主表清理（外键不得悬空）");
}

/// 反向，已 approved 行删除得 400 BUSINESS_ERROR，行仍在且状态零变化
#[tokio::test]
async fn delete_leg_on_approved_returns_business_error_and_row_survives() {
    let db = Arc::new(test_common::setup_test_db().await);
    let (_, _, role_delete, _) = setup_fo_fixture(&db).await;
    let order_id = seed_approved(&db).await;

    let app = fo_app(db.clone(), role_delete);
    let (status, body) = delete_fo(&app, order_id).await;
    assert_eq!(
        status,
        StatusCode::BAD_REQUEST,
        "已审批单据删除必须被状态门拒绝（400），实得 {status} {body}"
    );
    assert_eq!(
        body["code"].as_str(),
        Some("BUSINESS_ERROR"),
        "删除状态门拒绝的机器码必须为 BUSINESS_ERROR，实得 {body}"
    );

    let after = reload_order(&db, order_id).await;
    assert_eq!(
        after.status,
        sales_fabric_order::APPROVED,
        "被拒删除不得改动状态，行必须仍完整存活"
    );
    assert_eq!(
        after.approved_by,
        Some(FO_OTHER_USER_ID),
        "被拒删除不得清空审批人追溯"
    );
}

/// 反向，已 rejected 行删除得 400 BUSINESS_ERROR，行仍在
#[tokio::test]
async fn delete_leg_on_rejected_returns_business_error_and_row_survives() {
    let db = Arc::new(test_common::setup_test_db().await);
    let (_, _, role_delete, _) = setup_fo_fixture(&db).await;
    let order_id = seed_rejected(&db).await;

    let app = fo_app(db.clone(), role_delete);
    let (status, body) = delete_fo(&app, order_id).await;
    assert_eq!(
        status,
        StatusCode::BAD_REQUEST,
        "已拒绝单据删除必须被状态门拒绝（400），实得 {status} {body}"
    );
    assert_eq!(
        body["code"].as_str(),
        Some("BUSINESS_ERROR"),
        "已拒绝行删除拒绝的机器码必须为 BUSINESS_ERROR，实得 {body}"
    );

    let after = reload_order(&db, order_id).await;
    assert_eq!(
        after.status,
        sales_fabric_order::REJECTED,
        "被拒删除后行必须仍在且状态不变"
    );
}

// ---------------------------------------------------------------------------
// 四、源码棘轮，三条腿的状态门结构锁（不依赖活库，变异自检的第一道靶）
// ---------------------------------------------------------------------------

/// 从源码文本里切出一个 async fn 的函数体（与 contract_wave6 回潮棘轮同法）
fn split_fn_body(src: &str, name: &str) -> String {
    src.split(&format!("pub async fn {name}"))
        .nth(1)
        .unwrap_or_else(|| panic!("{name} 定义缺失"))
        .split("pub async fn ")
        .next()
        .unwrap_or_else(|| panic!("{name} 函数体边界缺失"))
        .to_string()
}

/// 结构锁，更新腿不得存在 status 直写路径，审核腿与删除腿必须具备行锁与 pending 状态门。
/// 若修复回潮（删门、改回无条件写入），本锁直接判红，为变异自检提供静态靶。
#[test]
fn fabric_legs_ratchet_state_gates_present() {
    let src = include_str!("../src/services/so/fabric_order.rs");

    let update_body = split_fn_body(src, "update_fabric_order");
    assert!(
        update_body.contains("req.status.is_some()"),
        "回潮棘轮：更新腿又放行调用方直写 status（缺键点名拒绝被移除）"
    );
    assert!(
        !update_body.contains("order.status = Set("),
        "回潮棘轮：更新腿出现 status 直写语句（状态机旁路回潮）"
    );

    let approve_body = split_fn_body(src, "approve_fabric_order");
    assert!(
        approve_body.contains("lock_exclusive()"),
        "回潮棘轮：审核腿丢失事务行锁（并发重复审批竞态回潮）"
    );
    assert!(
        approve_body.contains("order.status != fabric_status::PENDING"),
        "回潮棘轮：审核腿丢失 pending 状态门（无条件审批假绿回潮）"
    );
    assert!(
        approve_body.contains("update_with_audit("),
        "回潮棘轮：审核腿审计落库丢失（与拒绝腿同风格的追溯要求）"
    );

    let delete_body = split_fn_body(src, "delete_fabric_order");
    assert!(
        delete_body.contains("lock_exclusive()"),
        "回潮棘轮：删除腿丢失事务行锁"
    );
    assert!(
        delete_body.contains("order.status != fabric_status::PENDING"),
        "回潮棘轮：删除腿丢失 pending 状态门（已审批单据可直接删除回潮）"
    );
}
