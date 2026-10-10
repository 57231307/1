//! 工艺流程推进集成测试
//!
//! 覆盖节点 start/pause/resume/complete/block 流程
//! 创建时间: 2026-06-17

use bingxi_backend::utils::process_state_machine::{CustomOrderStatus, default_process_nodes};

/// 将工艺节点类型映射为对应状态（测试辅助，原 lib 中 dead_code 移入）
fn node_type_to_status(node_type: &str) -> Option<CustomOrderStatus> {
    node_type.parse::<CustomOrderStatus>().ok()
}

#[test]
fn test_default_process_nodes_complete() {
    let nodes = default_process_nodes();
    assert_eq!(nodes.len(), 5);

    // 验证 5 阶段顺序
    assert_eq!(nodes[0].0, "yarn_purchasing");
    assert_eq!(nodes[1].0, "dyeing");
    assert_eq!(nodes[2].0, "finishing");
    assert_eq!(nodes[3].0, "delivery");
    assert_eq!(nodes[4].0, "after_sales");

    // 验证 sequence 1-5
    for (idx, (_, _, seq)) in nodes.iter().enumerate() {
        assert_eq!(*seq, (idx + 1) as i32);
    }
}

#[test]
fn test_node_type_to_status_mapping() {
    // 修复 P2-7（2026-06-25 综合审计）：集成测试属于外部 crate，
    // 不能用 `bingxi_backend::` 引用被测 crate 的私有路径，改用 `bingxi_backend::`。
    assert_eq!(
        node_type_to_status("yarn_purchasing"),
        Some(CustomOrderStatus::YarnPurchasing)
    );
    assert_eq!(
        node_type_to_status("dyeing"),
        Some(CustomOrderStatus::Dyeing)
    );
    assert_eq!(
        node_type_to_status("finishing"),
        Some(CustomOrderStatus::Finishing)
    );
    assert_eq!(
        node_type_to_status("delivery"),
        Some(CustomOrderStatus::Delivery)
    );
    assert_eq!(
        node_type_to_status("after_sales"),
        Some(CustomOrderStatus::AfterSales)
    );
    assert_eq!(node_type_to_status("invalid"), None);
}

// =========================================================
// 节点操作人身份只认服务端会话的站点活体锁（真 PostgreSQL + tower oneshot 真实 handler）：
// PUT /custom-orders/{oid}/nodes/{nid} 的操作人身份只准取会话
// （AuthContext.user_id）。请求体即便多带伪造的另一个用户 B，
// process_nodes.operator_id 也必须是 A ——审计归属不可被客户端伪造。
// 夹具形态照抄 contract_wave11_optional_json_body + wave5 状态合一锁
// （users/customers/products/custom_orders 全链路 FK 自种，
// 见 m0044_integrate_unreferenced_migrations.rs 的 custom_orders / process_nodes 建表段）。
// =========================================================

mod test_common;

use axum::{
    Router,
    body::Body,
    extract::State,
    http::{Method, Request, StatusCode, header::CONTENT_TYPE},
    middleware::{Next, from_fn_with_state},
    response::Response,
    routing::put,
};
use bingxi_backend::container::AppState;
use bingxi_backend::handlers::custom_order_handler;
use bingxi_backend::middleware::auth_context::AuthContext;
use bingxi_backend::models::status::production::process_node as node_status;
use bingxi_backend::models::{custom_order, customer, process_node, product, user};
use chrono::Utc;
use rust_decimal::Decimal;
use sea_orm::{ActiveModelTrait, ActiveValue::Set, EntityTrait};
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

async fn seed_user(db: &sea_orm::DatabaseConnection, tag: &str) -> i32 {
    let now = Utc::now();
    let row = user::ActiveModel {
        username: Set(format!("w323c_node_{tag}")),
        password_hash: Set("test-only-not-a-real-hash".to_string()),
        is_active: Set(true),
        is_totp_enabled: Set(false),
        created_at: Set(now),
        updated_at: Set(now),
        ..Default::default()
    }
    .insert(db)
    .await
    .unwrap_or_else(|e| panic!("种子用户 {tag} 失败: {e}"));
    row.id
}

/// 种子一个 pending 节点及其全部 FK 父链，返回 (节点 id, 会话用户 A id, 伪造目标 B id)
async fn seed_node_chain(db: &sea_orm::DatabaseConnection) -> (i64, i32, i32) {
    let now = Utc::now();
    let user_a = seed_user(db, "a").await;
    // B 必须真实存在：operator_id → users(id) 有 FK（见 m0044 迁移的 process_nodes 建表段），
    // 若 B 无行，修复被回退时锁会先炸成 500 FK 违反而不是断言红，失去指向性。
    let user_b = seed_user(db, "b").await;

    let c = customer::ActiveModel {
        customer_code: Set(format!("CUST-W323C-{now:}")),
        customer_name: Set("323c会话身份锁客户".to_string()),
        credit_limit: Set(Decimal::ZERO),
        payment_terms: Set(30),
        status: Set("active".to_string()),
        customer_type: Set(bingxi_backend::constants::customer_type::OTHER.to_string()),
        owner_id: Set(user_a),
        created_by: Set(Some(user_a)),
        created_at: Set(now),
        updated_at: Set(now),
        ..Default::default()
    }
    .insert(db)
    .await
    .expect("夹具：种 customers 父行失败");

    let p = product::ActiveModel {
        name: Set(format!("323c会话身份锁产品-{now:}")),
        code: Set(format!("PRD-W323C-{now:}")),
        unit: Set("m".to_string()),
        status: Set("active".to_string()),
        is_deleted: Set(false),
        product_type: Set("fabric".to_string()),
        created_at: Set(now),
        updated_at: Set(now),
        ..Default::default()
    }
    .insert(db)
    .await
    .expect("夹具：种 products 父行失败");

    let order = custom_order::ActiveModel {
        order_no: Set(format!("CO-W323C-{now:}")),
        customer_id: Set(c.id as i64),
        product_id: Set(p.id as i64),
        spec: Set("测试规格".to_string()),
        quantity: Set(Decimal::new(10, 0)),
        unit: Set("m".to_string()),
        custom_requirements: Set(serde_json::json!({})),
        status: Set("draft".to_string()),
        currency: Set("CNY".to_string()),
        created_at: Set(now),
        updated_at: Set(now),
        ..Default::default()
    }
    .insert(db)
    .await
    .expect("夹具：种 custom_order 父行失败");

    let node = process_node::ActiveModel {
        id: Default::default(),
        custom_order_id: Set(order.id),
        node_type: Set("dyeing".to_string()),
        node_name: Set("染色".to_string()),
        sequence: Set(2),
        status: Set(node_status::PENDING.to_string()),
        planned_start_date: Set(None),
        planned_end_date: Set(None),
        actual_start_date: Set(None),
        actual_end_date: Set(None),
        operator_id: Set(None),
        notes: Set(None),
        created_at: Set(now),
        updated_at: Set(now),
    }
    .insert(db)
    .await
    .expect("夹具：种 process_node 失败");

    (node.id, user_a, user_b)
}

#[tokio::test]
async fn update_node_operator_identity_must_come_from_session_not_body() {
    let db = Arc::new(test_common::setup_test_db().await);
    let (node_id, user_a, user_b) = seed_node_chain(&db).await;

    let auth = AuthContext {
        user_id: user_a,
        username: "w323c_node_a".to_string(),
        role_id: Some(2),
        department_id: None,
        data_scope: Some("all".to_string()),
        dept_ids: None,
        dept_member_user_ids: None,
    };
    let app = Router::new()
        .route(
            "/custom-orders/{oid}/nodes/{nid}",
            put(custom_order_handler::update_process_node),
        )
        .with_state(AppState {
            db: db.clone(),
            ..Default::default()
        })
        .layer(from_fn_with_state(auth, inject_auth));

    // 会话是 A，请求体伪造 B：B 必须被忽略（DTO 已删除该字段，serde 默认放行未知键）
    let resp = app
        .oneshot(
            Request::builder()
                .method(Method::PUT)
                .uri(format!("/custom-orders/1/nodes/{node_id}"))
                .header(CONTENT_TYPE, "application/json")
                .body(Body::from(
                    json!({
                        "status": node_status::IN_PROGRESS,
                        "notes": "会话身份锁",
                        "operator_id": user_b,
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
        "合法更新请求必须 200，实得 {status} {}",
        String::from_utf8_lossy(&bytes)
    );

    let row = process_node::Entity::find_by_id(node_id)
        .one(db.as_ref())
        .await
        .expect("节点直读失败")
        .expect("节点必须存在");
    assert_eq!(
        row.operator_id,
        Some(user_a),
        "process_nodes.operator_id 必须等于会话用户 A（{user_a}），实得 {:?}",
        row.operator_id
    );
    assert_ne!(
        row.operator_id,
        Some(user_b),
        "请求体伪造的用户 B（{user_b}）绝不允许落操作人列"
    );
    assert_eq!(
        row.status,
        node_status::IN_PROGRESS,
        "状态门与落库列不得因身份收口而改动"
    );
}
