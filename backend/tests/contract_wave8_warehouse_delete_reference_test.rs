//! 仓库删除引用预检契约锁（CI run #4672：`DELETE /warehouses/3` 被
//! `fk_greige_fabric_warehouse` 拒后 DbErr 裸冒 500 `DATABASE_ERROR`，本波修复 →
//! 400 `BUSINESS_ERROR` + 可外显文案）
//!
//! 锁定的契约（`backend/src/services/warehouse_service.rs` delete /
//! find_warehouse_references / map_warehouse_fk_error）：
//! - 引用存在性预检命中 → `AppError::business_displayable`
//!   → HTTP 400 + code=BUSINESS_ERROR + **真实文案外显**（不得被脱敏成"业务处理失败"）
//! - 文案形态 = "该仓库已被 {N}{业务名}占用，无法删除，请先处理关联数据"：
//!   含用户可行动的数量，**不含**表名/约束名/列名/内部 ID（双向核法见用例断言）
//! - 拒绝路径不删行、不 CASCADE 静默删子行；无引用 → 硬删 + 审计落库（真实 user_id）
//! - `products.warehouse_id`（m0001 遗留列、实体未映射、FK fk_products_warehouse 存活）
//!   由参数化原生 COUNT 覆盖，本文件锁该分支不回归 500
//! - 不存在的仓库 → 404 NOT_FOUND（与修复前语义一致）
//!
//! 覆盖策略（路线一，同 contract_wave1_crm_lead_delete_reference_test）：
//! `test_common::setup_test_db()` 打已迁移 PostgreSQL（每次 TRUNCATE 业务表，用例互不串库）。
//!
//! 已知不可判定分支（诚实记录）：竞态兜底 `map_warehouse_fk_error`（预检清单外的 FK 在
//! DELETE 阶段命中）无法在单连接测试上确定性触发；预检与删除已同事务 + 父行
//! lock_exclusive 收窄窗口（PG 子表插入对父行 FOR KEY SHARE 与 FOR UPDATE 互斥），
//! 兜底分支未在本文件断言范围，见交付报告。

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
use bingxi_backend::handlers::warehouse_handler;
use bingxi_backend::middleware::auth_context::AuthContext;
use bingxi_backend::models::{audit_log, greige_fabric, warehouse};
use bingxi_backend::services::warehouse_service::WarehouseService;
use bingxi_backend::utils::error::AppError;
use bingxi_backend::utils::messages::biz_msg;
use chrono::Utc;
use sea_orm::{
    ActiveModelTrait, ActiveValue::Set, ColumnTrait, ConnectionTrait, DatabaseBackend, EntityTrait,
    PaginatorTrait, QueryFilter, Statement,
};
use serde_json::Value;
use std::sync::Arc;
use tower::ServiceExt;

const WH_REFERENCED_ID: i32 = 9471;
const WH_CLEAN_ID: i32 = 9472;
const WH_PRODUCTS_ID: i32 = 9473;

/// 预检命中（实体列分支，CI 事故同型：greige_fabric 引用 2 行）
const REFUSED_GREIGE: &str = "该仓库已被 2条坯布库存记录占用，无法删除，请先处理关联数据";
/// 预检命中（products.warehouse_id 遗留列原生 COUNT 分支）
const REFUSED_PRODUCTS: &str = "该仓库已被 1个产品档案的默认仓关联占用，无法删除，请先处理关联数据";

async fn live_db() -> sea_orm::DatabaseConnection {
    test_common::setup_test_db().await
}

async fn seed_warehouse(db: &sea_orm::DatabaseConnection, id: i32) {
    let now = Utc::now();
    warehouse::ActiveModel {
        id: Set(id),
        warehouse_code: Set(format!("W8WH{id}")),
        name: Set(format!("波8删除契约仓库 {id}")),
        is_default: Set(false),
        is_active: Set(true),
        created_at: Set(now),
        updated_at: Set(now),
        ..Default::default()
    }
    .insert(db)
    .await
    .expect("seed 仓库失败");
}

/// 引用行：真表 greige_fabric（FK fk_greige_fabric_warehouse → warehouses(id)，
/// 即 CI #4672 日志里拒绝 DELETE 的那条约束）。
async fn seed_greige_ref(db: &sea_orm::DatabaseConnection, warehouse_id: i32, n: i32) {
    for seq in 1..=n {
        greige_fabric::ActiveModel {
            fabric_no: Set(format!("W8GF-{warehouse_id}-{seq}")),
            fabric_name: Set("波8契约探针坯布".to_string()),
            fabric_type: Set("坯布".to_string()),
            warehouse_id: Set(Some(warehouse_id)),
            ..Default::default()
        }
        .insert(db)
        .await
        .expect("seed 坯布引用行失败");
    }
}

/// 引用行：products.warehouse_id 为 m0001 遗留列（models/product.rs 实体未映射），
/// 按 DDL 用参数化原生 INSERT 种子（同 contract_wave3_after_sales_customer_name_test 先例）。
async fn seed_product_ref(db: &sea_orm::DatabaseConnection, warehouse_id: i32) {
    db.execute_raw(Statement::from_sql_and_values(
        DatabaseBackend::Postgres,
        "INSERT INTO products (code, name, warehouse_id) VALUES ($1, $2, $3)",
        [
            format!("W8PROD-{warehouse_id}").into(),
            "波8产品档案默认仓探针".into(),
            warehouse_id.into(),
        ],
    ))
    .await
    .expect("seed products 引用行失败（warehouse_id 遗留列应可写）");
}

fn make_auth(user_id: i32) -> AuthContext {
    AuthContext {
        user_id,
        username: format!("e2e_user_{user_id}"),
        role_id: Some(2),
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

fn build_app(state: AppState, auth: AuthContext) -> Router {
    Router::new()
        .route("/warehouses/{id}", delete(warehouse_handler::delete))
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

fn del(uri: &str) -> Request<Body> {
    Request::builder()
        .method("DELETE")
        .uri(uri)
        .body(Body::empty())
        .unwrap()
}

/// 出参泄露面反向断言：文案只含数量与业务名，不得混入表名/约束名/列名/内部 ID。
fn assert_no_internal_leak(msg: &str, hidden_id: i32) {
    for token in [
        "fk_",
        "greige_fabric",
        "products",
        "warehouse_id",
        "数据关联错误",
        "约束",
        "SQL",
    ] {
        assert!(
            !msg.contains(token),
            "出参文案泄露内部实现细节 {token:?}：{msg}"
        );
    }
    assert!(
        !msg.contains(&hidden_id.to_string()),
        "出参文案不得混入被删实体内部 ID {hidden_id}：{msg}"
    );
}

// =========================================================
// service 层（真实 PostgreSQL）
// =========================================================

/// 被坯布引用的仓库删除 → 预检拒绝（CI #4672 事故同型）：BusinessErrorDisplayable 族、
/// 文案逐字锁死、仓库必须仍在（拒绝发生在 DELETE 之前，不静默删、不 CASCADE）
#[tokio::test]
async fn delete_warehouse_referenced_by_greige_rejected_with_displayable_business_error() {
    let db = live_db().await;
    seed_warehouse(&db, WH_REFERENCED_ID).await;
    seed_greige_ref(&db, WH_REFERENCED_ID, 2).await;

    let svc = WarehouseService::new(Arc::new(db.clone()));
    let err = svc
        .delete(WH_REFERENCED_ID, 100)
        .await
        .expect_err("被坯布（fk_greige_fabric_warehouse）引用的仓库不可删除（修复前此处为 FK 裸 500 DATABASE_ERROR）");

    match &err {
        AppError::BusinessErrorDisplayable(_) => {}
        other => panic!("必须用 business_displayable（文案须可外显），实际: {other:?}"),
    }
    assert_eq!(err.error_code(), "BUSINESS_ERROR");
    assert_eq!(err.to_string(), format!("业务错误：{REFUSED_GREIGE}"));
    assert_no_internal_leak(REFUSED_GREIGE, WH_REFERENCED_ID);

    let still = warehouse::Entity::find_by_id(WH_REFERENCED_ID)
        .one(&db)
        .await
        .unwrap();
    assert!(still.is_some(), "拒绝路径不得删掉仓库");
    let refs = greige_fabric::Entity::find()
        .filter(greige_fabric::Column::WarehouseId.eq(WH_REFERENCED_ID))
        .count(&db)
        .await
        .unwrap();
    assert_eq!(refs, 2, "拒绝路径不得 CASCADE 静默删引用行");

    // HTTP 信封（构造点直连 IntoResponse）：400 + code + 真实文案外显
    let resp = err.into_response();
    assert_eq!(resp.status(), StatusCode::BAD_REQUEST);
    let bytes = axum::body::to_bytes(resp.into_body(), 1024 * 1024)
        .await
        .unwrap();
    let v: Value = serde_json::from_slice(&bytes).unwrap();
    assert_eq!(v["code"], "BUSINESS_ERROR");
    assert_eq!(v["message"], REFUSED_GREIGE);
    assert_ne!(v["message"], "业务处理失败", "displayable 族不得被脱敏吞掉");
    assert_ne!(v["code"], "DATABASE_ERROR", "回归锁：不得回到裸 500 映射");
    assert_ne!(v["message"], "数据关联错误", "FK 归类文案不得外溢到出参");
}

/// products.warehouse_id 遗留列（实体未映射）分支：原生 COUNT 预检必须同样拦截，
/// 否则该 FK 命中仍会裸冒 500（本分支若不覆盖，修复只堵住 CI 日志那一条约束）
#[tokio::test]
async fn delete_warehouse_referenced_by_legacy_products_column_rejected() {
    let db = live_db().await;
    seed_warehouse(&db, WH_PRODUCTS_ID).await;
    seed_product_ref(&db, WH_PRODUCTS_ID).await;

    let svc = WarehouseService::new(Arc::new(db.clone()));
    let err = svc
        .delete(WH_PRODUCTS_ID, 100)
        .await
        .expect_err("products.warehouse_id（fk_products_warehouse）引用必须被预检拦截");

    assert!(
        matches!(err, AppError::BusinessErrorDisplayable(_)),
        "实际: {err:?}"
    );
    assert_eq!(err.error_code(), "BUSINESS_ERROR");
    assert_eq!(err.to_string(), format!("业务错误：{REFUSED_PRODUCTS}"));
    assert_no_internal_leak(REFUSED_PRODUCTS, WH_PRODUCTS_ID);
}

/// 无引用仓库删除 → 硬删成功 + 审计行真实落库（delete_with_audit 非静默，同事务提交）
#[tokio::test]
async fn delete_unreferenced_warehouse_succeeds_and_writes_audit() {
    let db = live_db().await;
    seed_warehouse(&db, WH_CLEAN_ID).await;

    let svc = WarehouseService::new(Arc::new(db.clone()));
    svc.delete(WH_CLEAN_ID, 100)
        .await
        .expect("无引用必须删除成功");

    assert!(
        warehouse::Entity::find_by_id(WH_CLEAN_ID)
            .one(&db)
            .await
            .unwrap()
            .is_none(),
        "仓库已硬删"
    );
    let audits = audit_log::Entity::find()
        .filter(audit_log::Column::ResourceType.eq("warehouse"))
        .filter(audit_log::Column::ResourceId.eq(WH_CLEAN_ID.to_string()))
        .all(&db)
        .await
        .unwrap();
    assert_eq!(audits.len(), 1);
    assert_eq!(audits[0].action, "DELETE");
    assert_eq!(audits[0].user_id, Some(100), "审计操作人须为真实 user_id");
    assert!(audits[0].before_snapshot.is_some(), "删除前快照必须留存");
}

/// 不存在的仓库 → 404 NOT_FOUND（与修复前语义一致，预检改动不得把 404 拉成 400/500）
#[tokio::test]
async fn delete_missing_warehouse_is_404() {
    let db = live_db().await;
    let svc = WarehouseService::new(Arc::new(db));
    let err = svc
        .delete(99999, 100)
        .await
        .expect_err("不存在的仓库不可删除");
    assert_eq!(err.error_code(), "NOT_FOUND");
    assert_eq!(err.to_string(), "未找到：仓库不存在");
}

// =========================================================
// HTTP handler 层（define_crud_handlers! 生成的 delete）
// =========================================================

async fn seeded_router(auth_user: i32) -> Router {
    let db = live_db().await;
    seed_warehouse(&db, WH_REFERENCED_ID).await;
    seed_greige_ref(&db, WH_REFERENCED_ID, 2).await;
    seed_warehouse(&db, WH_CLEAN_ID).await;
    let state = AppState {
        db: Arc::new(db),
        ..Default::default()
    };
    build_app(state, make_auth(auth_user))
}

/// DELETE /warehouses/9471 被引用 → 400 BUSINESS_ERROR（非裸 500），文案外显且无内部细节
#[tokio::test]
async fn http_delete_referenced_warehouse_400_business_error() {
    let app = seeded_router(100).await;
    let (status, v) = request_json(&app, del(&format!("/warehouses/{WH_REFERENCED_ID}"))).await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
    assert_eq!(v["code"], "BUSINESS_ERROR");
    assert_eq!(v["message"], REFUSED_GREIGE);
    assert_ne!(v["message"], "业务处理失败", "不得被包装层重新降级脱敏");
    let msg = v["message"].as_str().unwrap_or_default();
    assert_no_internal_leak(msg, WH_REFERENCED_ID);
}

/// DELETE 无引用 → 200，data 为 biz_msg::DELETE_OK 常量原文
#[tokio::test]
async fn http_delete_unreferenced_warehouse_200() {
    let app = seeded_router(100).await;
    let (status, v) = request_json(&app, del(&format!("/warehouses/{WH_CLEAN_ID}"))).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(v["code"], 200);
    assert_eq!(v["data"], biz_msg::DELETE_OK);
}
