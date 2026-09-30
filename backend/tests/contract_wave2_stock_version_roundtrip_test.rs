//! 库存乐观锁版本号出参回环锁（contract wave2）
//!
//! 锁定的契约（version 链路的三端同源）：
//! - `backend/src/models/inventory_stock.rs:74`（`version: i32` NOT NULL 真实列）
//! - `backend/src/handlers/inventory_stock_handler_dto.rs::StockResponse.version`
//!   （出参必须携带版本号——`UpdateStockWithVersionRequest.version` 必填，GET 出参是
//!   前端唯一合法取数点；缺它任何编辑都只能在 serde/乐观锁比对处失败）
//! - `backend/src/handlers/inventory_stock_handler.rs::to_stock_response`
//!   （真实列直映 `version: stock.version`，禁止 0 之类假值）
//! - `backend/src/handlers/inventory_stock_handler.rs::update_stock`
//!   （乐观锁比对 + `version = payload.version + 1` 递增）
//!
//! 覆盖策略：
//! - A 组为 serde/源码契约锁（**无需任何 DB**）
//! - B 组 `#[ignore]` 活库用例：GET 出参含 version → 带该 version 的 PUT 成功更新并递增；
//!   版本不符被拒且零写入；缺 version 的请求体在 serde 边界即被拒。

mod test_common;

use rust_decimal::Decimal;
use serde_json::json;
use std::str::FromStr;

use bingxi_backend::handlers::inventory_stock_handler_dto::{
    StockResponse, UpdateStockWithVersionRequest,
};

fn dec(s: &str) -> Decimal {
    Decimal::from_str(s).unwrap()
}

fn make_stock_response(version: i32) -> StockResponse {
    let now = chrono::Utc::now();
    StockResponse {
        id: 1,
        warehouse_id: 1,
        product_id: 100,
        quantity_on_hand: dec("100.00"),
        quantity_available: dec("90.00"),
        quantity_reserved: dec("10.00"),
        reorder_point: dec("20.00"),
        max_stock_point: dec("500.00"),
        bin_location: Some("A-01-01".to_string()),
        batch_no: "B001".to_string(),
        color_no: "C001".to_string(),
        dye_lot_no: Some("DL001".to_string()),
        grade: "一等品".to_string(),
        stock_status: "正常".to_string(),
        quality_status: "合格".to_string(),
        quantity_shipped: Decimal::ZERO,
        quantity_incoming: Decimal::ZERO,
        quantity_meters: dec("100.00"),
        quantity_kg: dec("50.00"),
        product_code: None,
        product_name: None,
        warehouse_name: None,
        version,
        created_at: now,
        updated_at: now,
    }
}

// =========================================================
// A) serde / 源码契约锁（无 DB）
// =========================================================

/// StockResponse 序列化必须携带 version，且 Decimal 数量按本仓口径出字符串
/// （前端 PUT 回传所需；出参缺键 = 编辑链路在请求边界必然被拒）
#[test]
fn stock_response_serializes_version_and_decimal_as_string() {
    let json = serde_json::to_value(make_stock_response(7)).expect("StockResponse 序列化失败");
    assert_eq!(
        json["version"], 7,
        "出参必须直含 version 键（前端乐观锁唯一取数点），实际: {json}"
    );
    assert_eq!(
        json["quantity_on_hand"], "100.00",
        "rust_decimal 出参为字符串（与库存台账既有口径一致）"
    );
}

/// PUT 入参 version 必填（乐观锁语义的入参侧锁定）：缺键必须在 serde 层被拒，
/// 不得给默认值——默认版本号的乐观锁等于没有锁
#[test]
fn update_request_version_is_required_at_serde_boundary() {
    let ok: UpdateStockWithVersionRequest = serde_json::from_value(json!({
        "quantity_on_hand": "88.00",
        "version": 3,
    }))
    .expect("带 version 的请求体必须可解码");
    assert_eq!(ok.version, 3);
    assert_eq!(ok.quantity_on_hand, Some(dec("88.00")));

    let err = serde_json::from_value::<UpdateStockWithVersionRequest>(json!({
        "quantity_on_hand": "88.00",
    }))
    .expect_err("缺 version 必须被 serde 拒绝（不得默认 0 蒙混乐观锁）");
    assert!(
        err.to_string().contains("version"),
        "拒绝原因应指向缺失的 version，实际: {err}"
    );
}

/// 源码契约锁：出参字段声明与构造点必须保持「真实列直映」，
/// 防止回退成"出参不带 version"（编辑不可用）或"假值 0"（乐观锁失真）
#[test]
fn stock_response_maps_version_from_real_column() {
    let dto = include_str!("../src/handlers/inventory_stock_handler_dto.rs");
    let stock_response_block = dto
        .split("pub struct StockResponse {")
        .nth(1)
        .and_then(|s| s.split("\n}").next())
        .expect("StockResponse 结构体应存在");
    assert!(
        stock_response_block.contains("pub version: i32"),
        "StockResponse 必须声明 version: i32（NOT NULL 列，不得 Option/缺省）"
    );

    let handler = include_str!("../src/handlers/inventory_stock_handler.rs");
    let to_response = handler
        .split("fn to_stock_response(")
        .nth(1)
        .and_then(|s| s.split("\n}").next())
        .expect("to_stock_response 函数体应存在");
    assert!(
        to_response.contains("version: stock.version"),
        "构造点必须直映 Model.version，实际: {to_response}"
    );
    assert!(
        !to_response.contains("version: 0"),
        "禁止以假值 0 蒙混乐观锁版本号"
    );

    let update_body = handler
        .split("pub async fn update_stock(")
        .nth(1)
        .and_then(|s| s.split("pub async fn ").next())
        .expect("update_stock 函数体应存在");
    assert!(
        update_body.contains("stock.version != payload.version"),
        "PUT 必须先做乐观锁比对"
    );
    assert!(
        update_body.contains("payload.version + 1"),
        "更新成功必须递增 version"
    );
}

// =========================================================
// B) 活库全链（真实 handler 回环）：#[ignore]
// =========================================================

/// 已迁移 PG 上走真实 GET/PUT handler：
/// ① GET 出参含 version 且等于落库真实列值；
/// ② PUT 携带该 version → 200，数量更新成功、出参 version 递增、DB 同步递增；
/// ③ PUT 携带过期 version → 400 BUSINESS_ERROR，DB 零写入（行原样）；
/// ④ PUT 缺 version 键 → 请求边界即拒（非 2xx），DB 依旧零写入。
#[tokio::test]
#[ignore = "需要 TEST_DATABASE_URL 已迁移 PostgreSQL（真实 insert + 两个 handler 全链）"]
async fn live_stock_version_roundtrip_and_conflict_rejected() {
    use axum::{
        Router,
        body::Body,
        extract::State,
        http::{Method, Request, StatusCode},
    };
    use bingxi_backend::container::AppState;
    use bingxi_backend::handlers::inventory_stock_handler;
    use bingxi_backend::middleware::auth_context::AuthContext;
    use bingxi_backend::models::status::purchase_inventory::{
        inventory_stock_quality_status, inventory_stock_status,
    };
    use bingxi_backend::models::{inventory_stock, product, warehouse};
    use chrono::Utc;
    use sea_orm::{ActiveModelTrait, ActiveValue::Set, ColumnTrait, EntityTrait, QueryFilter};
    use std::sync::Arc;
    use tower::ServiceExt;

    async fn inject_auth(
        State(auth): State<AuthContext>,
        mut request: Request<Body>,
        next: axum::middleware::Next,
    ) -> axum::response::Response {
        request.extensions_mut().insert(auth);
        next.run(request).await
    }

    let db = Arc::new(test_common::setup_test_db().await);
    let mut state = AppState::default();
    state.db = db.clone();
    let app = Router::new()
        .route(
            "/inventory/stock/{id}",
            axum::routing::get(inventory_stock_handler::get_stock)
                .put(inventory_stock_handler::update_stock),
        )
        .with_state(state)
        .layer(axum::middleware::from_fn_with_state(
            // role_id None：跳过数据权限字段过滤分支，锁定的是 version 回环本身
            AuthContext {
                user_id: 9621,
                username: "e2e_stock_w2_9621".to_string(),
                role_id: None,
                department_id: Some(1),
                data_scope: Some("self".to_string()),
                dept_ids: None,
                dept_member_user_ids: None,
            },
            inject_auth,
        ));

    // —— 播种主数据与库存行（version=3，前端 PUT 必须能从 GET 出参拿到它）——
    let ts = Utc::now().timestamp_nanos();
    let p = product::ActiveModel {
        name: Set("wave2版本号套件面料".to_string()),
        code: Set(format!("PRD-W2V-{ts}")),
        unit: Set("米".to_string()),
        status: Set("active".to_string()),
        is_deleted: Set(false),
        product_type: Set("fabric".to_string()),
        created_at: Set(Utc::now()),
        updated_at: Set(Utc::now()),
        ..Default::default()
    }
    .insert(&*db)
    .await
    .unwrap();
    let wh = warehouse::ActiveModel {
        warehouse_code: Set(format!("WH-W2V-{ts}")),
        name: Set("版本号回环仓".to_string()),
        is_default: Set(false),
        is_active: Set(true),
        created_at: Set(Utc::now()),
        updated_at: Set(Utc::now()),
        ..Default::default()
    }
    .insert(&*db)
    .await
    .unwrap();
    let stock = inventory_stock::ActiveModel {
        warehouse_id: Set(wh.id),
        product_id: Set(p.id),
        quantity_on_hand: Set(dec("100.00")),
        quantity_available: Set(dec("90.00")),
        quantity_reserved: Set(dec("10.00")),
        quantity_shipped: Set(Decimal::ZERO),
        quantity_incoming: Set(Decimal::ZERO),
        reorder_point: Set(dec("20.00")),
        max_stock_point: Set(dec("500.00")),
        reorder_quantity: Set(dec("50.00")),
        bin_location: Set(Some("A-01-01".to_string())),
        created_at: Set(Utc::now()),
        updated_at: Set(Utc::now()),
        batch_no: Set(format("B-W2V-{ts}")),
        color_no: Set("C001".to_string()),
        dye_lot_no: Set(Some("DL001".to_string())),
        grade: Set("一等品".to_string()),
        quantity_meters: Set(dec("100.00")),
        quantity_kg: Set(dec("50.00")),
        stock_status: Set(inventory_stock_status::NORMAL.to_string()),
        quality_status: Set(inventory_stock_quality_status::PASS.to_string()),
        version: Set(3),
        replenishment_strategy: Set("reorder_point".to_string()),
        ..Default::default()
    }
    .insert(&*db)
    .await
    .unwrap();

    async fn request_json(
        app: &Router,
        method: Method,
        uri: &str,
        body: Option<&serde_json::Value>,
    ) -> (StatusCode, serde_json::Value) {
        let builder = Request::builder().method(method).uri(uri);
        let req = match body {
            Some(v) => builder
                .header("content-type", "application/json")
                .body(Body::from(v.to_string()))
                .unwrap(),
            None => builder.body(Body::empty()).unwrap(),
        };
        let resp = app.clone().oneshot(req).await.unwrap();
        let status = resp.status();
        let bytes = axum::body::to_bytes(resp.into_body(), 1024 * 1024)
            .await
            .unwrap();
        let v = serde_json::from_slice(&bytes).unwrap_or(serde_json::Value::Null);
        (status, v)
    }

    async fn reload(db: &sea_orm::DatabaseConnection, id: i32) -> inventory_stock::Model {
        inventory_stock::Entity::find()
            .filter(inventory_stock::Column::Id.eq(id))
            .one(db)
            .await
            .unwrap()
            .expect("播种行应存在")
    }

    let uri = format!("/inventory/stock/{}", stock.id);

    // —— ① GET 出参含 version 且等于真实列 ——
    let (status, v) = request_json(&app, Method::GET, &uri, None).await;
    assert!(status.is_success(), "GET 必须 2xx，实际 {status}: {v}");
    assert_eq!(
        v["data"]["version"], 3,
        "GET 出参必须携带真实版本号（PUT 入参的唯一合法来源）"
    );

    // —— ② PUT 带 GET 回传的 version → 成功更新并递增 ——
    let (status, v) = request_json(
        &app,
        Method::PUT,
        &uri,
        Some(&json!({
            "quantity_on_hand": "88.00",
            "bin_location": "B-02-03",
            "version": 3,
        })),
    )
    .await;
    assert!(
        status.is_success(),
        "携带真实 version 的 PUT 必须成功（修复前出参无 version，编辑在 serde 层即被拒），实际 {status}: {v}"
    );
    assert_eq!(v["data"]["version"], 4, "更新成功后出参 version 递增");
    let q = v["data"]["quantity_on_hand"]
        .as_str()
        .expect("Decimal 出参为字符串");
    assert_eq!(Decimal::from_str(q).unwrap(), dec("88.00"));
    let row = reload(&db, stock.id).await;
    assert_eq!(row.version, 4, "DB 真实列同步递增");
    assert_eq!(row.quantity_on_hand, dec("88.00"));
    assert_eq!(row.bin_location.as_deref(), Some("B-02-03"));

    // —— ③ PUT 携带过期 version（①拿到的 3）→ 拒绝且零写入 ——
    let (status, v) = request_json(
        &app,
        Method::PUT,
        &uri,
        Some(&json!({
            "quantity_on_hand": "1.00",
            "version": 3,
        })),
    )
    .await;
    assert_eq!(
        status,
        StatusCode::BAD_REQUEST,
        "版本不符必须 400，实际: {v}"
    );
    assert_eq!(v["code"], "BUSINESS_ERROR", "乐观锁冲突是业务错误");
    let row = reload(&db, stock.id).await;
    assert_eq!(row.version, 4, "冲突拒绝不得产生任何写入");
    assert_eq!(row.quantity_on_hand, dec("88.00"));

    // —— ④ PUT 缺 version 键 → serde 边界即拒，零写入 ——
    let (status, _v) = request_json(
        &app,
        Method::PUT,
        &uri,
        Some(&json!({
            "quantity_on_hand": "2.00",
        })),
    )
    .await;
    assert!(
        !status.is_success(),
        "缺必填 version 的请求体必须在请求边界被拒（不得默认 0 蒙混），实际 {status}"
    );
    let row = reload(&db, stock.id).await;
    assert_eq!(row.quantity_on_hand, dec("88.00"), "被拒请求零写入");
    assert_eq!(row.version, 4);
}
