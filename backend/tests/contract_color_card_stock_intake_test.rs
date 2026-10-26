//! 色卡库存登记（PUT /erp/color-cards/{id} 的 stock_quantity）真实库契约锁
//!
//! 锁定的业务事实：色卡总库存是可通过 HTTP 登记的真实业务动作，语义与本仓
//! "缺省即不改"更新契约族一致，并且登记值受已发放量下界约束。
//!
//! 为什么必须有这条通道（发放侧的可用量口径）：
//! - 发放闸门 2 与事务内并发重检都按 `可用 = stock_quantity - issued_quantity` 判
//!   （`services/color_card_issue_service.rs`），发放只推进 issued_quantity；
//! - 建卡把 stock_quantity 落 0（`services/color_card_crud_service.rs::create`），
//!   印制与生产入库写的是另一张表 `inventory_stock` 的米数/公斤数，不回写色卡；
//! - 因此若没有任何 HTTP 入口能把 stock_quantity 抬起来，`POST /color-cards/issues`
//!   对任何经 HTTP 建立的色卡恒被闸门 2 拒绝，色卡发放业务在 HTTP 面上不可达。
//!
//! 本文件锁四件事，缺一即回归：
//! 1. 请求体不带 stock_quantity 键 ⇒ 原库存逐值保留（"缺省即不改"不得被写成"清零"）；
//! 2. 显式提交合法值 ⇒ 出参与库中回读同为该值；
//! 3. 提交负数 ⇒ HTTP 400 + 信封 code=VALIDATION_ERROR + 外显点名文案，库中零漂移；
//! 4. 提交低于已发放量的值 ⇒ 同族拒绝且 stock_quantity 与 issued_quantity 双零漂移；
//!    等值边界（库存恰好等于已发放量，可用 0）必须放行，不能被下界校验顺带吞掉。
//!
//! 脚手架：`test_common::setup_test_db()`（已迁移 PostgreSQL + TRUNCATE 业务表）
//! 组装真实 handler 并发真 HTTP 请求，行状态一律回读真库比对。
//! 夹具直插 seed 只铺前置（建卡端点本就把库存落 0，已发放量无法经 HTTP 抬升），
//! 被测动作本身全部经 HTTP 发出。

mod test_common;

use axum::{
    Router,
    body::Body,
    extract::State,
    http::{Method, Request, StatusCode},
    middleware::Next,
    response::Response,
    routing::put,
};
use serde_json::{Value, json};
use std::sync::Arc;
use tower::ServiceExt;

use bingxi_backend::container::AppState;
use bingxi_backend::handlers::color_card::update_color_card;
use bingxi_backend::middleware::auth_context::AuthContext;
use bingxi_backend::models::color_card::{ActiveModel as CardActive, Entity as ColorCardEntity};
use bingxi_backend::models::status::color_card as card_status;
use sea_orm::{ActiveModelTrait, ActiveValue::Set, EntityTrait};

// ---------------------------------------------------------------------------
// 脚手架：真实 handler + 注入会话身份
// ---------------------------------------------------------------------------

fn make_auth() -> AuthContext {
    AuthContext {
        user_id: 71,
        username: "color_card_stock_intake".to_string(),
        role_id: Some(2),
        department_id: Some(1),
        data_scope: Some("all".to_string()),
        dept_ids: None,
        dept_member_user_ids: None,
    }
}

async fn inject_auth(auth: State<AuthContext>, mut request: Request<Body>, next: Next) -> Response {
    request.extensions_mut().insert(auth.0);
    next.run(request).await
}

/// 只挂被测的色卡主档更新端点：路径形态与 routes/color_card.rs 的 `/{id}` PUT 一致
fn build_app(db: &Arc<sea_orm::DatabaseConnection>, auth: AuthContext) -> Router {
    let state = AppState {
        db: db.clone(),
        ..Default::default()
    };
    Router::new()
        .route("/erp/color-cards/{id}", put(update_color_card))
        .with_state(state)
        .layer(axum::middleware::from_fn_with_state(auth, inject_auth))
}

async fn put_card(app: &Router, id: i64, body: Value) -> (StatusCode, Value) {
    let resp = app
        .clone()
        .oneshot(
            Request::builder()
                .method(Method::PUT)
                .uri(format!("/erp/color-cards/{id}"))
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
    (
        status,
        serde_json::from_slice(&bytes).unwrap_or_else(|e| {
            panic!(
                "色卡 {id} 更新响应非 JSON: {e}; body={}",
                String::from_utf8_lossy(&bytes)
            )
        }),
    )
}

/// 铺前置：按给定的库存与已发放量播种一张草稿态色卡，返回 id。
/// 状态取 card_status::DRAFT（更新门与 create 端点写入值同源），库存与发放量是夹具值，
/// 被测动作不发这里。
async fn seed_draft_card(
    db: &Arc<sea_orm::DatabaseConnection>,
    card_no: &str,
    stock_quantity: i32,
    issued_quantity: i32,
) -> i64 {
    let now = chrono::Utc::now();
    let card = CardActive {
        card_no: Set(card_no.to_string()),
        card_name: Set("色卡库存登记契约色卡".to_string()),
        card_type: Set("PANTONE".to_string()),
        status: Set(card_status::DRAFT.to_string()),
        stock_quantity: Set(stock_quantity),
        issued_quantity: Set(issued_quantity),
        created_at: Set(now),
        updated_at: Set(now),
        ..Default::default()
    }
    .insert(&**db)
    .await
    .unwrap_or_else(|e| panic!("播种草稿态色卡 {card_no} 失败: {e}"));
    card.id
}

/// 回读真库两列（判定只认库里真实值，不认响应体）
async fn read_back(db: &Arc<sea_orm::DatabaseConnection>, id: i64) -> (i32, i32) {
    let card = ColorCardEntity::find_by_id(id)
        .one(&**db)
        .await
        .unwrap()
        .unwrap_or_else(|| panic!("色卡 {id} 回读不到行"));
    (card.stock_quantity, card.issued_quantity)
}

/// 拒绝路径的统一信封断言：400 + VALIDATION_ERROR + 外显真实原因
fn assert_validation_rejected(status: StatusCode, body: &Value, expected_message: &str) {
    assert_eq!(
        status,
        StatusCode::BAD_REQUEST,
        "库存登记越界必须 400（不得裸 500、不得 2xx 放行），实际 {status}: {body}"
    );
    assert_eq!(
        body["code"].as_str(),
        Some("VALIDATION_ERROR"),
        "提交字段取值越界属校验族，信封 code 必须是 VALIDATION_ERROR，实际 {body}"
    );
    assert_eq!(
        body["message"].as_str(),
        Some(expected_message),
        "拒绝文案必须点名提交字段违反的公开规则并给出填法方向，实际 {body}"
    );
}

// ---------------------------------------------------------------------------
// 唯一用例：缺省不改 / 显式改并回读 / 负数拒 / 低于已发放量拒且零漂移 / 等值放行
// ---------------------------------------------------------------------------

#[tokio::test]
async fn stock_quantity_defaults_to_untouched_and_lower_bound_is_enforced() {
    let db = Arc::new(test_common::setup_test_db().await);
    let app = build_app(&db, make_auth());

    // —— ① 缺省即不改：不带 stock_quantity 键的更新不得触碰库存列 ——
    //    前置库存取非 0 夹具值 30，这样"被清零"与"保留"两种结果可区分
    let untouched_id = seed_draft_card(&db, "STOCK-DEFAULT-KEEP", 30, 0).await;
    let (status, body) = put_card(
        &app,
        untouched_id,
        json!({ "card_name": "只改卡名不改库存" }),
    )
    .await;
    assert_eq!(
        status,
        StatusCode::OK,
        "只改卡名的更新应 200，实际 {status}: {body}"
    );
    assert_eq!(
        body["code"].as_i64(),
        Some(200),
        "成功信封 code 应为 200，实际 {body}"
    );
    assert_eq!(
        body["data"]["stock_quantity"].as_i64(),
        Some(30),
        "出参 stock_quantity 应逐值保留 30"
    );
    assert_eq!(
        read_back(&db, untouched_id).await,
        (30, 0),
        "未提交 stock_quantity 时库中两列都必须零漂移（缺省即不改不得写成清零）"
    );

    // —— ② 显式提交合法值：出参与库中回读同为该值 ——
    let register_id = seed_draft_card(&db, "STOCK-REGISTER", 0, 0).await;
    let (status, body) = put_card(&app, register_id, json!({ "stock_quantity": 12 })).await;
    assert_eq!(
        status,
        StatusCode::OK,
        "登记 12 张库存应 200，实际 {status}: {body}"
    );
    assert_eq!(
        body["data"]["stock_quantity"].as_i64(),
        Some(12),
        "出参应回显登记值 12"
    );
    assert_eq!(
        read_back(&db, register_id).await,
        (12, 0),
        "库中 stock_quantity 必须落为登记值 12"
    );
    // 只提交库存时其余字段必须原样保留（"缺省即不改"对称成立）
    let card_after_register = ColorCardEntity::find_by_id(register_id)
        .one(&*db)
        .await
        .unwrap()
        .unwrap();
    assert_eq!(
        card_after_register.card_name, "色卡库存登记契约色卡",
        "只提交 stock_quantity 不得顺带改写 card_name"
    );

    // —— ③ 负数：点名拒绝且零漂移 ——
    let (status, body) = put_card(&app, register_id, json!({ "stock_quantity": -1 })).await;
    assert_validation_rejected(status, &body, "库存数量不能为负数");
    assert_eq!(
        read_back(&db, register_id).await,
        (12, 0),
        "负数被拒后库存与发放量必须零漂移"
    );

    // —— ④ 低于已发放量：点名拒绝且双列零漂移 ——
    //    前置已发放 5（该值无法经 HTTP 抬升，故由夹具直插）
    let under_id = seed_draft_card(&db, "STOCK-BELOW-ISSUED", 8, 5).await;
    let (status, body) = put_card(&app, under_id, json!({ "stock_quantity": 4 })).await;
    assert_validation_rejected(
        status,
        &body,
        "库存数量不能低于已发放数量，请填写不小于当前已发放数量的值",
    );
    assert_eq!(
        read_back(&db, under_id).await,
        (8, 5),
        "低于已发放量被拒后库存与发放量必须零漂移（不得部分写入）"
    );

    // —— ⑤ 等值边界放行：库存恰好等于已发放量（可用 0）是合法登记值 ——
    let (status, body) = put_card(&app, under_id, json!({ "stock_quantity": 5 })).await;
    assert_eq!(
        status,
        StatusCode::OK,
        "库存等于已发放量 5 应放行：下界是不低于，不是必须大于；实际 {status}: {body}"
    );
    assert_eq!(
        body["data"]["stock_quantity"].as_i64(),
        Some(5),
        "等值登记应回显 5"
    );
    assert_eq!(
        read_back(&db, under_id).await,
        (5, 5),
        "等值登记必须落库为 5"
    );

    // —— ⑥ 越界值不得因为同批提交里夹了合法字段而被半写半就 ——
    //    同一次请求同时提交合法卡名 + 越界库存：整笔必须被拒，卡名也不能改
    let atomic_id = seed_draft_card(&db, "STOCK-ATOMIC", 6, 2).await;
    let (status, body) = put_card(
        &app,
        atomic_id,
        json!({ "card_name": "越界同批不该写入的卡名", "stock_quantity": -3 }),
    )
    .await;
    assert_validation_rejected(status, &body, "库存数量不能为负数");
    let atomic_card = ColorCardEntity::find_by_id(atomic_id)
        .one(&*db)
        .await
        .unwrap()
        .unwrap();
    assert_eq!(
        (atomic_card.stock_quantity, atomic_card.issued_quantity),
        (6, 2),
        "越界拒绝后两列零漂移"
    );
    assert_eq!(
        atomic_card.card_name, "色卡库存登记契约色卡",
        "校验先于写入：同批合法字段不得被半写入"
    );
}
