//! RFM 评分出参形状契约锁（端点 `GET /api/v1/erp/crm/customers/:id/rfm`）
//!
//! 锁定的真实契约（唯一真相 = 源码）：
//! - `backend/src/services/crm/cust.rs::compute_rfm_score` 返回 `RfmScoreDetail`
//!   （R/F/M 三个分项 + 合成分 `score`，均为 f64）；
//! - `backend/src/services/crm/mod.rs::RfmScoreDetail` 无 `#[serde(rename_all)]`，
//!   序列化键即字段名 snake_case：`recency` / `frequency` / `monetary` / `score`；
//! - `backend/src/handlers/crm_handler.rs::get_rfm_score` 经
//!   `ApiResponse::success(serde_json::to_value(detail))` 出参 ⇒ `data` 必须是**对象**，
//!   四个键齐全且值均为 JSON number（f64 序列化，不是 Decimal 那种 JSON 字符串）。
//!
//! 为什么钉形状而不是钉数值：前端消费点（`frontend/src/api/crm-enhanced.ts::RfmScore`
//! 与 `views/crm/detail.vue`、`views/crm/enhanced/index.vue`）按子键取值；一旦本端点回退成
//! 裸 f64（历史形状）或任一分项被再次在服务层聚合时丢弃，前端就会把**真实存在的分数**渲染成
//! 空位。形状断言与阈值口径无关，故不锁具体分值、不锁任何文案。
//!
//! 夹具口径（路线一）：`test_common::setup_test_db()` 打已迁移 PostgreSQL；
//! `sales_orders.customer_id`/`created_by` 是真实外键（fk_sales_orders_customer、
//! fk_sales_orders_created_by），customers/users 迁移不播种 ⇒ 用例自插合法父行
//! （链：users(7) → customers(1) → sales_orders(1)）。CI 以 `--test-threads=1` 串行执行。

mod test_common;

use axum::{
    body::Body,
    extract::State,
    http::{Request, StatusCode},
    middleware::{from_fn_with_state, Next},
    response::Response,
    routing::get,
    Router,
};
use bingxi_backend::container::AppState;
use bingxi_backend::handlers::crm_handler;
use bingxi_backend::middleware::auth_context::AuthContext;
use bingxi_backend::models::{customer, sales_order, user};
use bingxi_backend::services::crm::cust::CrmService;
use chrono::Utc;
use rust_decimal::Decimal;
use sea_orm::{ActiveModelTrait, ActiveValue::Set};
use serde_json::Value;
use std::sync::Arc;
use tower::ServiceExt;

/// 契约键集（顺序无关，集合必须恰好相等——多一个"预留键"或少一个分项都算破坏契约）
const EXPECTED_KEYS: [&str; 4] = ["frequency", "monetary", "recency", "score"];

async fn live_db() -> sea_orm::DatabaseConnection {
    test_common::setup_test_db().await
}

/// FK 父行：users(7) → customers(1)（owner_id 指向真实用户）
async fn seed_user_and_customer(db: &sea_orm::DatabaseConnection) {
    let now = Utc::now();
    user::ActiveModel {
        id: Set(7),
        username: Set("rfm_shape_fixture_user".to_string()),
        password_hash: Set("test-only-not-a-real-hash".to_string()),
        real_name: Set(Some("RFM 形状夹具用户".to_string())),
        is_active: Set(true),
        is_totp_enabled: Set(false),
        created_at: Set(now),
        updated_at: Set(now),
        ..Default::default()
    }
    .insert(db)
    .await
    .expect("夹具：users 迁移不播种，必须自插归属用户");

    customer::ActiveModel {
        id: Set(1),
        customer_code: Set("RFM-SHAPE-0001".to_string()),
        customer_name: Set("RFM 形状夹具客户".to_string()),
        credit_limit: Set(Decimal::ZERO),
        payment_terms: Set(30),
        status: Set("active".to_string()),
        customer_type: Set("retail".to_string()),
        owner_id: Set(7),
        created_at: Set(now),
        updated_at: Set(now),
        ..Default::default()
    }
    .insert(db)
    .await
    .expect("夹具：customers 迁移不播种，必须自插客户行");
}

/// 订单行（RFM 三项的输入源：R 取 created_at、F 取行数、M 取 total_amount 求和）
async fn seed_order(db: &sea_orm::DatabaseConnection, id: i32, amount: Decimal) {
    let now = Utc::now();
    sales_order::ActiveModel {
        id: Set(id),
        order_no: Set(format!("SO-RFM-SHAPE-{id}")),
        customer_id: Set(1),
        order_date: Set(now),
        required_date: Set(Some(now)),
        status: Set("approved".to_string()),
        subtotal: Set(amount),
        tax_amount: Set(Decimal::ZERO),
        discount_amount: Set(Decimal::ZERO),
        shipping_cost: Set(Decimal::ZERO),
        total_amount: Set(amount),
        paid_amount: Set(Decimal::ZERO),
        balance_amount: Set(amount),
        created_by: Set(Some(7)),
        created_at: Set(now),
        updated_at: Set(now),
        ..Default::default()
    }
    .insert(db)
    .await
    .expect("夹具：sales_orders 有 customers/users 外键，父行已自插");
}

/// 把"对象 + 恰好这四个键 + 值全为 number"三段判据一次说完
fn assert_rfm_detail_shape(detail: &Value) {
    assert!(
        !detail.is_number(),
        "回归锁：data 不得回退成裸 f64（前端按子键取值会恒 undefined）"
    );
    let obj = detail
        .as_object()
        .unwrap_or_else(|| panic!("data 必须是 JSON 对象，实际: {detail}"));
    let mut keys: Vec<&str> = obj.keys().map(|k| k.as_str()).collect();
    keys.sort_unstable();
    assert_eq!(
        keys, EXPECTED_KEYS,
        "出参键集必须与 RfmScoreDetail 字段集相等"
    );
    for key in EXPECTED_KEYS {
        assert!(
            obj[key].is_number(),
            "键 {key} 必须是 JSON number（f64 出参不是 Decimal 字符串），实际: {:?}",
            obj[key]
        );
    }
}

fn make_auth() -> AuthContext {
    AuthContext {
        user_id: 7,
        username: "rfm_shape_fixture_user".to_string(),
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

fn build_app(state: AppState) -> Router {
    Router::new()
        .route("/crm/customers/{id}/rfm", get(crm_handler::get_rfm_score))
        .with_state(state)
        .layer(from_fn_with_state(make_auth(), inject_auth))
}

// =========================================================
// 1) service 层出参形状（真实 PostgreSQL）
// =========================================================

/// 有订单客户：compute_rfm_score 序列化后必须是四键对象，分项与合成分同时在场
/// （历史缺陷形态=只出合成分、三个分项在函数内被均值吞掉 ⇒ 前端 R/F/M 三列恒空）
#[tokio::test]
async fn compute_rfm_score_serializes_as_structured_detail() {
    let db = live_db().await;
    seed_user_and_customer(&db).await;
    seed_order(&db, 1, Decimal::from(50_000)).await;

    let svc = CrmService::new(Arc::new(db));
    let detail = svc
        .compute_rfm_score(1)
        .await
        .expect("有订单客户的 RFM 计算不得报错");
    let value = serde_json::to_value(&detail).expect("RfmScoreDetail 必须可序列化");
    assert_rfm_detail_shape(&value);
}

/// 无订单客户同样必须是四键对象：后端对无消费客户给的是最低分（分项 1.0），
/// 而不是"未计算"，故出参形状不得随数据缺失退化成 null / 缺键 / 裸数
#[tokio::test]
async fn compute_rfm_score_shape_stable_for_customer_without_orders() {
    let db = live_db().await;
    seed_user_and_customer(&db).await;

    let svc = CrmService::new(Arc::new(db));
    let detail = svc
        .compute_rfm_score(1)
        .await
        .expect("无订单也属正常分支（最低分语义），不可报错");
    let value = serde_json::to_value(&detail).expect("RfmScoreDetail 必须可序列化");
    assert_rfm_detail_shape(&value);
}

// =========================================================
// 2) HTTP 层信封与 data 形状（真实 PostgreSQL）
// =========================================================

/// GET /crm/customers/:id/rfm → 200 + 成功信封 code=200 + data 为结构化对象
#[tokio::test]
async fn http_get_rfm_score_returns_success_envelope_with_object_data() {
    let db = live_db().await;
    seed_user_and_customer(&db).await;
    seed_order(&db, 1, Decimal::from(120_000)).await;
    seed_order(&db, 2, Decimal::from(30_000)).await;

    let app = build_app(AppState {
        db: Arc::new(db),
        ..Default::default()
    });
    let req = Request::builder()
        .method("GET")
        .uri("/crm/customers/1/rfm")
        .body(Body::empty())
        .unwrap();
    let resp = app.oneshot(req).await.unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
    let bytes = axum::body::to_bytes(resp.into_body(), 1024 * 1024)
        .await
        .unwrap();
    let v: Value = serde_json::from_slice(&bytes).unwrap();
    assert_eq!(
        v["code"], 200,
        "成功信封 code 契约（utils/response.rs::ApiResponse）"
    );
    assert_rfm_detail_shape(&v["data"]);
}
