//! 坯布更新端点可空列三态清空与出入库入参门禁契约锁
//!
//! 锁定的真实行为（对齐 RFC 7396 JSON Merge Patch 与本仓既有先例
//! `contract_wave3_explicit_null_clear_test.rs` / `contract_wave12_greige_recipe_gates_test.rs`）：
//! 1. 三态语义（`PUT /production/greige-fabrics/{id}`，DTO
//!    `handlers/greige_fabric_handler.rs::UpdateGreigeFabricRequest`）：
//!    键缺席=保持原值、显式 `null`=清空为 NULL、有值=覆盖；可空文本列的空白串与显式
//!    `null` 同义（库里不存在「空串」与 NULL 的语义差别，白坯即 `color_no` 为空）。
//!    覆盖列取 `models/greige_fabric.rs` 与 system 域建坯布表迁移、
//!    `system/mod.rs` 声明为可空的那批：color_code/width_cm/weight_kg/length_m/
//!    supplier_id/batch_no/warehouse_id/location/quality_grade/remarks/safety_stock/
//!    reorder_point/max_stock_point/reorder_quantity/dye_lot_no/color_no。
//! 2. `status` 不开清空：删除门控（handler `delete_greige_fabric` 只拦 `在库`）与出库状态
//!    回写都按该中文 token 判定，端点不接受把本列置 NULL 的写法；越界值（含建表遗留的
//!    `available`）按取值域 `greige_fabric_status::ALL` 拒绝。
//! 3. 数值入参门禁：width_cm/weight_kg/length_m 与四个库存策略列拒绝负数与非有限值；
//!    入库数量与出库数量必须是正数（`stock_in` 是累加、`stock_out` 是扣减，0 或负数会把
//!    当前库存与累计入库量带偏）；出库超过现有量或超过累计入库量都拒成业务错误。
//! 4. 外键预检：supplier_id/warehouse_id 有库内外键（system 域建坯布表迁移），product_id/
//!    purchase_order_id/purchase_receipt_id 无库内外键，脏 id 一律预检成 4xx 业务错误，
//!    不许裸抛 500，也不许静默存成悬空引用。
//!
//! 覆盖策略：纯 serde 只锁 DTO 三态形状（不碰库）；其余全部走已迁移真库
//! （`test_common::setup_test_db()`，夹具自身 TRUNCATE），真实 handler + 真实表回读，
//! 无 mock、无同构 DDL。

mod test_common;
use test_common::setup_test_db;

use axum::{
    Router,
    body::Body,
    extract::State,
    http::{Method, Request, StatusCode},
    middleware::{Next, from_fn_with_state},
    response::Response,
};
use bingxi_backend::container::AppState;
use bingxi_backend::handlers::greige_fabric_handler;
use bingxi_backend::middleware::auth_context::AuthContext;
use bingxi_backend::models::greige_fabric;
use bingxi_backend::models::supplier;
use bingxi_backend::models::warehouse;
use chrono::{NaiveDate, Utc};
use rust_decimal::Decimal;
use sea_orm::ActiveValue::Set;
use sea_orm::{ActiveModelTrait, EntityTrait};
use serde_json::{Value, json};
use std::str::FromStr;
use std::sync::Arc;
use tower::ServiceExt;

fn dec(s: &str) -> Decimal {
    Decimal::from_str(s).unwrap()
}

fn make_auth(user_id: i32) -> AuthContext {
    AuthContext {
        user_id,
        username: format!("test_user_{user_id}"),
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

/// 坯布写面路由：与 `routes/production.rs` 的更新/删除/入库/出库四端点同 handler。
/// 路径参数写法跟 axum 0.8 的 `{id}`，与既有 greige 用例同形。
fn build_app(db: sea_orm::DatabaseConnection, auth: AuthContext) -> Router {
    let state = AppState {
        db: Arc::new(db),
        ..Default::default()
    };
    Router::new()
        .route(
            "/greige-fabrics/{id}",
            axum::routing::put(greige_fabric_handler::update_greige_fabric)
                .delete(greige_fabric_handler::delete_greige_fabric),
        )
        .route(
            "/greige-fabrics/{id}/stock-in",
            axum::routing::post(greige_fabric_handler::stock_in),
        )
        .route(
            "/greige-fabrics/{id}/stock-out",
            axum::routing::post(greige_fabric_handler::stock_out),
        )
        .route(
            "/greige-fabrics",
            axum::routing::post(greige_fabric_handler::create_greige_fabric),
        )
        .with_state(state)
        .layer(from_fn_with_state(auth, inject_auth))
}

async fn call(app: &Router, method: Method, uri: &str, body: Option<Value>) -> (StatusCode, Value) {
    let mut builder = Request::builder().method(method).uri(uri);
    let req = match body {
        Some(v) => {
            builder = builder.header("content-type", "application/json");
            builder.body(Body::from(v.to_string())).unwrap()
        }
        None => builder.body(Body::empty()).unwrap(),
    };
    let resp = app.clone().oneshot(req).await.unwrap();
    let status = resp.status();
    let bytes = axum::body::to_bytes(resp.into_body(), 4 * 1024 * 1024)
        .await
        .unwrap();
    (status, serde_json::from_slice(&bytes).unwrap())
}

/// 种一条全字段有值的坯布：可空列全部非 NULL，三态用例才有「保持/清空」两种可观测态。
/// supplier_id 指向固定 7，故先补种 suppliers(7) 满足 fk_greige_fabric_supplier，
/// 否则夹具 insert 会撞外键违例（setup_test_db 每用例先 TRUNCATE，这里内联种子即可自洽）。
async fn seed_supplier_7(db: &sea_orm::DatabaseConnection) {
    let now = Utc::now().fixed_offset();
    supplier::ActiveModel {
        id: Set(7),
        supplier_code: Set("SUP-GREIGE-007".to_string()),
        supplier_name: Set("坯布契约供应商".to_string()),
        supplier_short_name: Set("坯供".to_string()),
        supplier_type: Set("面料供应商".to_string()),
        credit_code: Set("91330000GREIGE007X1".to_string()),
        registered_address: Set("契约测试注册地址".to_string()),
        legal_representative: Set("契约法人".to_string()),
        registered_capital: Set(Decimal::ZERO),
        establishment_date: Set(NaiveDate::from_ymd_opt(2020, 1, 1).expect("夹具日期常量")),
        taxpayer_type: Set("一般纳税人".to_string()),
        bank_name: Set("契约测试银行".to_string()),
        bank_account: Set("6222000000000007".to_string()),
        contact_phone: Set("13800000000".to_string()),
        is_processor: Set(false),
        created_at: Set(now),
        updated_at: Set(now),
        ..Default::default()
    }
    .insert(db)
    .await
    .unwrap_or_else(|e| panic!("seed 供应商 7 失败: {e}"));
}

async fn seed_full_greige(db: &sea_orm::DatabaseConnection, id: i32) {
    seed_supplier_7(db).await;
    greige_fabric::ActiveModel {
        id: Set(id),
        fabric_no: Set(format!("GF-TEST-{id:04}")),
        fabric_name: Set(format!("测试坯布-{id}")),
        fabric_type: Set("针织".to_string()),
        status: Set(Some("在库".to_string())),
        is_deleted: Set(Some(false)),
        created_by: Set(Some(100)),
        color_code: Set(Some("CC-OLD".to_string())),
        batch_no: Set(Some("BATCH-OLD".to_string())),
        location: Set(Some("A-01".to_string())),
        quality_grade: Set(Some("A".to_string())),
        remarks: Set(Some("旧备注".to_string())),
        dye_lot_no: Set(Some("DYE-OLD".to_string())),
        color_no: Set(Some("COL-OLD".to_string())),
        width_cm: Set(Some(dec("180.00"))),
        weight_kg: Set(Some(dec("100.00"))),
        length_m: Set(Some(dec("200.00"))),
        quantity_kg: Set(Some(dec("100.00"))),
        quantity_meters: Set(Some(dec("200.00"))),
        supplier_id: Set(Some(7)),
        warehouse_id: Set(Some(1)),
        safety_stock: Set(Some(dec("10.00"))),
        reorder_point: Set(Some(dec("20.00"))),
        max_stock_point: Set(Some(dec("500.00"))),
        reorder_quantity: Set(Some(dec("50.00"))),
        created_at: Set(Utc::now().fixed_offset()),
        updated_at: Set(Utc::now().fixed_offset()),
        ..Default::default()
    }
    .insert(db)
    .await
    .unwrap();
}

async fn seed_warehouse(db: &sea_orm::DatabaseConnection, id: i32) {
    warehouse::ActiveModel {
        id: Set(id),
        warehouse_code: Set(format!("WH-TEST-{id:04}")),
        name: Set(format!("测试仓库-{id}")),
        is_default: Set(false),
        is_active: Set(true),
        created_at: Set(Utc::now()),
        updated_at: Set(Utc::now()),
        ..Default::default()
    }
    .insert(db)
    .await
    .unwrap();
}

async fn find_row(db: &sea_orm::DatabaseConnection, id: i32) -> greige_fabric::Model {
    greige_fabric::Entity::find_by_id(id)
        .one(db)
        .await
        .unwrap()
        .unwrap_or_else(|| panic!("坯布行 {id} 应存在于测试库"))
}

// ============================================================
// A) DTO 三态形状（纯 serde，不碰库）
// ============================================================

#[test]
fn dto_absent_key_is_keep_and_explicit_null_is_clear() {
    // 键缺席的三态直接判定反序列化后的 Rust 字段：`color_code` 声明为 `Option<Option<String>>`
    // 且挂 `serde(default)`，键缺席即外层 None（保持原值）。不借助回序列化——Update 是入参
    // DTO，只派生 Deserialize，本用例与同组 Some(None)/Some(Some) 断言同源，一律直接读字段。
    let absent = serde_json::from_str::<greige_fabric_handler::UpdateGreigeFabricRequest>("{}")
        .expect("空对象应能反序列化为更新请求");
    assert!(
        absent.color_code.is_none(),
        "键缺席时 color_code 应是外层 None（保持原值），实际 {:?}",
        absent.color_code
    );

    let cleared =
        serde_json::from_str::<greige_fabric_handler::UpdateGreigeFabricRequest>(
            r#"{"color_code": null, "width_cm": null, "supplier_id": null}"#,
        )
        .expect("显式 null 应能反序列化为更新请求");
    assert!(
        matches!(cleared.color_code, Some(None)),
        "显式 null 的 color_code 必须是 Some(None)（清空），实际 {:?}",
        cleared.color_code
    );
    assert!(
        matches!(cleared.width_cm, Some(None)),
        "显式 null 的 width_cm 必须是 Some(None)（清空），实际 {:?}",
        cleared.width_cm
    );
    assert!(
        matches!(cleared.supplier_id, Some(None)),
        "显式 null 的 supplier_id 必须是 Some(None)（清空），实际 {:?}",
        cleared.supplier_id
    );

    let set = serde_json::from_str::<greige_fabric_handler::UpdateGreigeFabricRequest>(
        r#"{"color_code": "CC-NEW", "width_cm": 190.5}"#,
    )
    .expect("有值应能反序列化为更新请求");
    assert_eq!(
        set.color_code,
        Some(Some("CC-NEW".to_string())),
        "有值的 color_code 必须是 Some(Some(v))（覆盖）"
    );
    assert_eq!(
        set.width_cm,
        Some(Some(190.5)),
        "有值的 width_cm 必须是 Some(Some(v))（覆盖）"
    );

    // status 有意不开三态：清空入口不存在，类型上就不是 Option<Option<String>>
    let status_req =
        serde_json::from_str::<greige_fabric_handler::UpdateGreigeFabricRequest>(
            r#"{"status": null}"#,
        )
        .expect("status 显式 null 应折叠成缺席而不是报错");
    assert_eq!(
        status_req.status, None,
        "status 保持裸 Option：显式 null 与键缺席同义（不覆盖、不清空）"
    );
}

// ============================================================
// B) 真库三态回环：清空 / 保持 / 覆盖
// ============================================================

#[tokio::test]
async fn update_explicit_null_clears_nullable_columns_to_db_null() {
    let db = setup_test_db().await;
    seed_full_greige(&db, 1).await;

    let app = build_app(db.clone(), make_auth(100));
    let (status, v) = call(
        &app,
        Method::PUT,
        "/greige-fabrics/1",
        Some(json!({
            "color_code": null,
            "batch_no": null,
            "location": null,
            "quality_grade": null,
            "remarks": null,
            "dye_lot_no": null,
            "color_no": null,
            "width_cm": null,
            "weight_kg": null,
            "length_m": null,
            "supplier_id": null,
            "warehouse_id": null,
            "safety_stock": null,
            "reorder_point": null,
            "max_stock_point": null,
            "reorder_quantity": null,
        })),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "清空响应应为 200，实际 {v}");

    let row = find_row(&db, 1).await;
    for (name, actual) in [
        ("color_code", row.color_code.clone()),
        ("batch_no", row.batch_no.clone()),
        ("location", row.location.clone()),
        ("quality_grade", row.quality_grade.clone()),
        ("remarks", row.remarks.clone()),
        ("dye_lot_no", row.dye_lot_no.clone()),
        ("color_no", row.color_no.clone()),
    ] {
        assert!(
            actual.is_none(),
            "显式 null 应把可空文本列 {name} 落成 NULL，实际 {actual:?}"
        );
    }
    for (name, actual) in [
        ("width_cm", row.width_cm),
        ("weight_kg", row.weight_kg),
        ("length_m", row.length_m),
        ("safety_stock", row.safety_stock),
        ("reorder_point", row.reorder_point),
        ("max_stock_point", row.max_stock_point),
        ("reorder_quantity", row.reorder_quantity),
    ] {
        assert!(
            actual.is_none(),
            "显式 null 应把可空数值列 {name} 落成 NULL，实际 {actual:?}"
        );
    }
    assert!(
        row.supplier_id.is_none() && row.warehouse_id.is_none(),
        "显式 null 应把裸外键列清成 NULL（supplier_id={:?} warehouse_id={:?}）",
        row.supplier_id,
        row.warehouse_id
    );
    // 三态的另一面：清空数量列不等于清空状态，状态必须原样保住
    assert_eq!(
        row.status.as_deref(),
        Some("在库"),
        "清空其它列不应顺带改写 status"
    );
}

#[tokio::test]
async fn update_absent_keys_keep_original_values() {
    let db = setup_test_db().await;
    seed_full_greige(&db, 1).await;

    let app = build_app(db.clone(), make_auth(100));
    let (status, v) = call(
        &app,
        Method::PUT,
        "/greige-fabrics/1",
        Some(json!({"fabric_name": "只改名称"})),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "只改名称应为 200，实际 {v}");

    let row = find_row(&db, 1).await;
    assert_eq!(row.fabric_name, "只改名称", "提交的 fabric_name 应覆盖");
    assert_eq!(
        row.color_code.as_deref(),
        Some("CC-OLD"),
        "键缺席的 color_code 必须保持原值"
    );
    assert_eq!(
        row.batch_no.as_deref(),
        Some("BATCH-OLD"),
        "键缺席的 batch_no 必须保持原值"
    );
    assert_eq!(row.weight_kg, Some(dec("100.00")), "键缺席的 weight_kg 必须保持原值");
    assert_eq!(
        row.safety_stock, Some(dec("10.00")),
        "键缺席的 safety_stock 必须保持原值"
    );
    assert_eq!(row.supplier_id, Some(7), "键缺席的 supplier_id 必须保持原值");
}

#[tokio::test]
async fn update_blank_text_is_normalized_to_null_not_empty_string() {
    let db = setup_test_db().await;
    seed_full_greige(&db, 1).await;

    let app = build_app(db.clone(), make_auth(100));
    let (status, v) = call(
        &app,
        Method::PUT,
        "/greige-fabrics/1",
        Some(json!({"color_no": "   ", "remarks": "", "location": "  B-02  "})),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "空白文本提交应为 200，实际 {v}");

    let row = find_row(&db, 1).await;
    assert!(
        row.color_no.is_none(),
        "空白色号应落 NULL（白坯口径按是否为空判定），实际 {:?}",
        row.color_no
    );
    assert!(
        row.remarks.is_none(),
        "空串备注应落 NULL，禁止空串与 NULL 两种存法并存，实际 {:?}",
        row.remarks
    );
    assert_eq!(
        row.location.as_deref(),
        Some("B-02"),
        "有值文本应去首尾空白后覆盖"
    );
}

#[tokio::test]
async fn update_rejects_negative_and_keeps_row_untouched() {
    let db = setup_test_db().await;
    seed_full_greige(&db, 1).await;

    let app = build_app(db.clone(), make_auth(100));
    let (status, v) = call(
        &app,
        Method::PUT,
        "/greige-fabrics/1",
        Some(json!({"weight_kg": -5.0})),
    )
    .await;
    assert_eq!(
        status,
        StatusCode::BAD_REQUEST,
        "负数重量应被拒成 400，实际 {status} {v}"
    );
    assert_eq!(v["code"], "VALIDATION_ERROR", "错误码应为校验码：{v}");
    assert!(
        v["message"]
            .as_str()
            .is_some_and(|m| m.contains("重量(kg)")),
        "拒绝文案必须点名被拒字段，实际 {v}"
    );

    let row = find_row(&db, 1).await;
    assert_eq!(
        row.weight_kg,
        Some(dec("100.00")),
        "被拒的更新不得产生任何列漂移"
    );

    // 0 是合法声明值（清库后的真实库存），不被负数门禁误伤
    let (status_zero, v_zero) = call(
        &app,
        Method::PUT,
        "/greige-fabrics/1",
        Some(json!({"weight_kg": 0})),
    )
    .await;
    assert_eq!(
        status_zero,
        StatusCode::OK,
        "提交 0 重量是合法覆盖，应为 200，实际 {v_zero}"
    );
    assert_eq!(
        find_row(&db, 1).await.weight_kg,
        Some(dec("0.00")),
        "0 应如实落库"
    );
}

#[tokio::test]
async fn update_status_still_enforces_chinese_vocab() {
    let db = setup_test_db().await;
    seed_full_greige(&db, 1).await;

    let app = build_app(db.clone(), make_auth(100));
    let (status, v) = call(
        &app,
        Method::PUT,
        "/greige-fabrics/1",
        Some(json!({"status": "available"})),
    )
    .await;
    assert_eq!(
        status,
        StatusCode::BAD_REQUEST,
        "建表遗留英文值 available 必须按取值域拒绝，实际 {status} {v}"
    );
    assert!(
        v["message"]
            .as_str()
            .is_some_and(|m| m.contains("在库") && m.contains("已出库")),
        "拒绝文案必须回显允许值，实际 {v}"
    );
    assert_eq!(
        find_row(&db, 1).await.status.as_deref(),
        Some("在库"),
        "越界状态提交不得落库"
    );

    let (ok_status, ok_body) = call(
        &app,
        Method::PUT,
        "/greige-fabrics/1",
        Some(json!({"status": "已出库"})),
    )
    .await;
    assert_eq!(
        ok_status,
        StatusCode::OK,
        "词表内中文值应通过，实际 {ok_body}"
    );
    assert_eq!(
        find_row(&db, 1).await.status.as_deref(),
        Some("已出库"),
        "词表内值应如实覆盖"
    );
}

#[tokio::test]
async fn update_dirty_foreign_id_is_business_error_not_500() {
    let db = setup_test_db().await;
    seed_full_greige(&db, 1).await;

    let app = build_app(db.clone(), make_auth(100));
    // supplier_id=7 与 warehouse_id=1 是种子值，但本用例不种父表：脏 id 必须预检成 4xx
    for (field, dirty) in [("supplier_id", 99999997_i64), ("warehouse_id", 99999998)] {
        let (status, v) = call(
            &app,
            Method::PUT,
            "/greige-fabrics/1",
            Some(json!({field: dirty})),
        )
        .await;
        assert!(
            status.is_client_error(),
            "更新提交不存在的 {field} 应被预检成 4xx，实际 {status} {v}"
        );
        assert_ne!(
            status,
            StatusCode::INTERNAL_SERVER_ERROR,
            "{field} 脏值不得裸抛 500（调用方无法区分数据错误与系统故障）：{v}"
        );
    }
    assert_eq!(
        find_row(&db, 1).await.supplier_id,
        Some(7),
        "被拒的 supplier_id 提交不得改库"
    );
}

// ============================================================
// C) 建单面外键预检（三列无库内外键，靠 handler 预检防悬空引用）
// ============================================================

#[tokio::test]
async fn create_rejects_dirty_reference_ids_without_500() {
    let db = setup_test_db().await;
    let app = build_app(db.clone(), make_auth(100));

    for (field, dirty) in [
        ("product_id", 99999991_i64),
        ("purchase_order_id", 99999992),
        ("purchase_receipt_id", 99999993),
    ] {
        let (status, v) = call(
            &app,
            Method::POST,
            "/greige-fabrics",
            Some(json!({
                "fabric_no": format!("GF-BAD-{field}"),
                "fabric_name": "脏引用建单",
                "fabric_type": "针织",
                field: dirty,
            })),
        )
        .await;
        assert!(
            status.is_client_error(),
            "建单提交不存在的 {field} 应被预检成 4xx，实际 {status} {v}"
        );
        assert_ne!(
            status,
            StatusCode::INTERNAL_SERVER_ERROR,
            "{field} 不在库内外键保护范围内，脏值不得冒 500：{v}"
        );
        assert_eq!(v["code"], "BUSINESS_ERROR", "{field} 拒绝应为业务错误码：{v}");
    }
}

#[tokio::test]
async fn create_rejects_negative_quantity_inputs() {
    let db = setup_test_db().await;
    let app = build_app(db.clone(), make_auth(100));

    let (status, v) = call(
        &app,
        Method::POST,
        "/greige-fabrics",
        Some(json!({
            "fabric_no": "GF-NEG-0001",
            "fabric_name": "负数建单",
            "fabric_type": "针织",
            "weight_kg": -1.0,
        })),
    )
    .await;
    assert_eq!(
        status,
        StatusCode::BAD_REQUEST,
        "建单负数重量应被拒成 400，实际 {status} {v}"
    );
    assert_eq!(v["code"], "VALIDATION_ERROR", "错误码应为校验码：{v}");
}

// ============================================================
// D) 入库入参门禁与真实累加
// ============================================================

#[tokio::test]
async fn stock_in_accumulates_with_decimal_precision() {
    let db = setup_test_db().await;
    seed_warehouse(&db, 1).await;
    seed_full_greige(&db, 1).await;

    let app = build_app(db.clone(), make_auth(100));
    let (status, v) = call(
        &app,
        Method::POST,
        "/greige-fabrics/1/stock-in",
        Some(json!({
            "warehouse_id": 1,
            "weight_kg": 0.01,
            "length_m": 0.01,
            "location": "A-02",
            "quality_grade": "B",
            "remarks": "入库补量",
        })),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "正常入库应为 200，实际 {v}");
    // 100.00 + 0.01 必须精确到分位：原实现经 f64 往返会把分位抖掉
    assert_eq!(
        v["data"]["weight_kg"], "100.01",
        "入库重量应精确累加，实际 {v}"
    );
    assert_eq!(
        v["data"]["quantity_kg"], "100.01",
        "累计入库重量应同步累加，实际 {v}"
    );
    assert_eq!(v["data"]["status"], "在库", "入库后状态应为在库：{v}");

    let row = find_row(&db, 1).await;
    assert_eq!(row.weight_kg, Some(dec("100.01")), "回读落库重量应精确累加");
    assert_eq!(row.length_m, Some(dec("200.01")), "回读落库长度应精确累加");
    assert_eq!(
        row.location.as_deref(),
        Some("A-02"),
        "入库应写入库位"
    );
    assert_eq!(
        row.quality_grade.as_deref(),
        Some("B"),
        "入库应写入等级"
    );
    assert_eq!(
        row.remarks.as_deref(),
        Some("入库补量"),
        "入库应写入备注"
    );
}

#[tokio::test]
async fn stock_in_rejects_non_positive_and_dirty_warehouse() {
    let db = setup_test_db().await;
    seed_warehouse(&db, 1).await;
    seed_full_greige(&db, 1).await;

    let app = build_app(db.clone(), make_auth(100));
    for (label, body) in [
        ("负数入库", json!({"warehouse_id": 1, "weight_kg": -5.0, "length_m": 10.0})),
        ("零值入库", json!({"warehouse_id": 1, "weight_kg": 0.0, "length_m": 10.0})),
    ] {
        let (status, v) = call(&app, Method::POST, "/greige-fabrics/1/stock-in", Some(body)).await;
        assert_eq!(
            status,
            StatusCode::BAD_REQUEST,
            "{label}应被拒成 400，实际 {status} {v}"
        );
    }

    let (status, v) = call(
        &app,
        Method::POST,
        "/greige-fabrics/1/stock-in",
        Some(json!({"warehouse_id": 99999999, "weight_kg": 5.0, "length_m": 10.0})),
    )
    .await;
    assert!(
        status.is_client_error(),
        "不存在的仓库应预检成 4xx，实际 {status} {v}"
    );
    assert_ne!(
        status,
        StatusCode::INTERNAL_SERVER_ERROR,
        "脏 warehouse_id 不得裸抛 500（库内有外键，裸撞会撞成 DATABASE_ERROR）：{v}"
    );

    let row = find_row(&db, 1).await;
    assert_eq!(
        row.weight_kg,
        Some(dec("100.00")),
        "被拒的入库调用不得改库存"
    );
    assert_eq!(
        row.warehouse_id,
        Some(1),
        "被拒的入库调用不得改仓库归属"
    );
}

// ============================================================
// E) 出库入参门禁与状态回写
// ============================================================

#[tokio::test]
async fn stock_out_requires_at_least_one_positive_quantity() {
    let db = setup_test_db().await;
    seed_full_greige(&db, 1).await;

    let app = build_app(db.clone(), make_auth(100));
    let (status, v) = call(
        &app,
        Method::POST,
        "/greige-fabrics/1/stock-out",
        Some(json!({"remarks": "空调用"})),
    )
    .await;
    assert_eq!(
        status,
        StatusCode::BAD_REQUEST,
        "不带任何数量的出库调用应被拒，实际 {status} {v}"
    );
    assert_eq!(v["code"], "VALIDATION_ERROR", "错误码应为校验码：{v}");

    let row = find_row(&db, 1).await;
    assert_eq!(
        row.remarks.as_deref(),
        Some("旧备注"),
        "被拒的出库调用不得顺带改写备注"
    );

    let (neg_status, neg_body) = call(
        &app,
        Method::POST,
        "/greige-fabrics/1/stock-out",
        Some(json!({"weight_kg": -1.0})),
    )
    .await;
    assert_eq!(
        neg_status,
        StatusCode::BAD_REQUEST,
        "负数出库量应被拒成 400（负数等于凭空加库存），实际 {neg_body}"
    );
}

#[tokio::test]
async fn stock_out_partial_keeps_in_stock_and_full_drains_to_stocked_out() {
    let db = setup_test_db().await;
    seed_full_greige(&db, 1).await;
    seed_full_greige(&db, 2).await;

    let app = build_app(db.clone(), make_auth(100));
    let (status, v) = call(
        &app,
        Method::POST,
        "/greige-fabrics/1/stock-out",
        Some(json!({"weight_kg": 40.0, "length_m": 50.0, "remarks": "领用投产"})),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "部分出库应为 200，实际 {v}");
    assert_eq!(v["data"]["weight_kg"], "60.00", "扣减后重量应如实，实际 {v}");
    assert_eq!(v["data"]["quantity_kg"], "60.00", "累计量应同步扣减，实际 {v}");
    assert_eq!(v["data"]["status"], "在库", "仍有剩余应维持在库：{v}");

    let (status_full, v_full) = call(
        &app,
        Method::POST,
        "/greige-fabrics/2/stock-out",
        Some(json!({"weight_kg": 100.0, "length_m": 200.0})),
    )
    .await;
    assert_eq!(status_full, StatusCode::OK, "整卷出库应为 200，实际 {v_full}");
    assert_eq!(
        v_full["data"]["status"],
        "已出库",
        "重量与长度同时归零应置已出库：{v_full}"
    );

    // 已出库态是可删的前提：删除门控只拦「在库」，此处双向钉住门控与状态词表同源
    let (del_status, del_body) = call(&app, Method::DELETE, "/greige-fabrics/2", None).await;
    assert_eq!(
        del_status,
        StatusCode::OK,
        "已出库坯布应允许软删，实际 {del_body}"
    );
    let (blocked_status, blocked_body) = call(&app, Method::DELETE, "/greige-fabrics/1", None).await;
    assert_eq!(
        blocked_status,
        StatusCode::BAD_REQUEST,
        "在库坯布删除必须被拒，实际 {blocked_status} {blocked_body}"
    );
    assert_eq!(
        blocked_body["code"],
        "BUSINESS_ERROR",
        "删除门控应为业务错误：{blocked_body}"
    );
}

#[tokio::test]
async fn stock_out_rejects_overdraft_on_both_current_and_cumulative() {
    let db = setup_test_db().await;
    seed_full_greige(&db, 1).await;

    // 现有量 100、累计量 100：超出现有量先被挡
    let app = build_app(db.clone(), make_auth(100));
    let (status, v) = call(
        &app,
        Method::POST,
        "/greige-fabrics/1/stock-out",
        Some(json!({"weight_kg": 101.0})),
    )
    .await;
    assert_eq!(
        status,
        StatusCode::BAD_REQUEST,
        "出库超过现有重量应被拒，实际 {status} {v}"
    );
    assert_eq!(v["code"], "BUSINESS_ERROR", "超额出库应为业务错误码：{v}");
    assert!(
        v["message"]
            .as_str()
            .is_some_and(|m| m.contains("100") && m.contains("101")),
        "拒绝文案必须点名现有量与本次量，实际 {v}"
    );
    assert_eq!(
        find_row(&db, 1).await.weight_kg,
        Some(dec("100.00")),
        "被拒出库不得改库存"
    );

    // 把累计量单独压到低于现有量：只挡现有量一处的话，这一步会把累计量减成负数
    let mut adjust: greige_fabric::ActiveModel = find_row(&db, 1).await.into();
    adjust.quantity_kg = Set(Some(dec("30.00")));
    adjust.updated_at = Set(Utc::now().fixed_offset());
    let adjusted = adjust.update(&db).await.unwrap();
    assert_eq!(adjusted.quantity_kg, Some(dec("30.00")));

    let (status_qty, v_qty) = call(
        &app,
        Method::POST,
        "/greige-fabrics/1/stock-out",
        Some(json!({"weight_kg": 50.0})),
    )
    .await;
    assert_eq!(
        status_qty,
        StatusCode::BAD_REQUEST,
        "现有量够但累计量不够时也必须拒绝（否则累计量被带负，成本分母失真），实际 {status_qty} {v_qty}"
    );
    let row = find_row(&db, 1).await;
    assert_eq!(
        row.quantity_kg,
        Some(dec("30.00")),
        "被拒出库不得把累计量减成负数"
    );
    assert_eq!(
        row.weight_kg,
        Some(dec("100.00")),
        "被拒出库不得动现有量"
    );
}

// ============================================================
// F) 米数与公斤数双单位一致性（单源 utils/dual_unit_converter）
// ============================================================

/// 种一条配了线密度（克重 g/m² 与幅宽 cm 均正）的坯布，双单位换算才有可核对口径。
/// 功能：为双单位一致性用例准备克重=200、幅宽=180 的档案；米↔公斤换算率随之固定为
///      每公斤 = 1000 / (200 * 1.8) = 2.7778 米。
/// 调用方：本文件 F 组三个双单位用例。
/// 入参：db 活库连接，id 主键。
/// 传给谁：直接 insert 到 greige_fabric 表。
/// 存什么：一条在库、created_by=100、带克重与幅宽的坯布，库存与累计量给足正数。
/// 存哪里：greige_fabric 表，夹具自身 TRUNCATE 清理。
async fn seed_greige_with_density(db: &sea_orm::DatabaseConnection, id: i32) {
    greige_fabric::ActiveModel {
        id: Set(id),
        fabric_no: Set(format!("GF-DENS-{id:04}")),
        fabric_name: Set(format!("密度坯布-{id}")),
        fabric_type: Set("针织".to_string()),
        status: Set(Some("在库".to_string())),
        is_deleted: Set(Some(false)),
        created_by: Set(Some(100)),
        warehouse_id: Set(Some(1)),
        gram_weight: Set(Some(dec("200.00"))),
        width_cm: Set(Some(dec("180.00"))),
        weight_kg: Set(Some(dec("100.00"))),
        length_m: Set(Some(dec("200.00"))),
        quantity_kg: Set(Some(dec("100.00"))),
        quantity_meters: Set(Some(dec("200.00"))),
        created_at: Set(Utc::now().fixed_offset()),
        updated_at: Set(Utc::now().fixed_offset()),
        ..Default::default()
    }
    .insert(db)
    .await
    .unwrap();
}

/// 正向：入库米数与公斤数按档案克重幅宽自洽（10m×200g/m²×1.8m=3.6kg），应放行并累加。
#[tokio::test]
async fn stock_in_accepts_consistent_dual_unit() {
    let db = setup_test_db().await;
    seed_warehouse(&db, 1).await;
    seed_greige_with_density(&db, 1).await;

    let app = build_app(db.clone(), make_auth(100));
    let (status, v) = call(
        &app,
        Method::POST,
        "/greige-fabrics/1/stock-in",
        Some(json!({"warehouse_id": 1, "weight_kg": 3.6, "length_m": 10.0})),
    )
    .await;
    assert_eq!(
        status,
        StatusCode::OK,
        "自洽的双单位入库应放行，实际 {status} {v}"
    );
    // 100.00 + 3.6 精确累加，回读落库真值
    let row = find_row(&db, 1).await;
    assert_eq!(row.weight_kg, Some(dec("103.60")), "入库重量应累加：{row:?}");
    assert_eq!(row.length_m, Some(dec("210.00")), "入库长度应累加：{row:?}");
}

/// 反向：入库米数与公斤数明显对不上线密度（10m 却报 50kg），应拒成校验错误且零副作用。
/// 变异自证：若把 ensure_dual_unit_consistent 的拒绝改成恒 Ok 或删除，本用例必转 200 而失败。
#[tokio::test]
async fn stock_in_rejects_inconsistent_dual_unit() {
    let db = setup_test_db().await;
    seed_warehouse(&db, 1).await;
    seed_greige_with_density(&db, 1).await;

    let app = build_app(db.clone(), make_auth(100));
    let (status, v) = call(
        &app,
        Method::POST,
        "/greige-fabrics/1/stock-in",
        Some(json!({"warehouse_id": 1, "weight_kg": 50.0, "length_m": 10.0})),
    )
    .await;
    assert_eq!(
        status,
        StatusCode::BAD_REQUEST,
        "矛盾的双单位入库应被拒成 400，实际 {status} {v}"
    );
    assert_eq!(v["code"], "VALIDATION_ERROR", "错误码应为校验码：{v}");
    assert!(
        v["message"]
            .as_str()
            .is_some_and(|m| m.contains("米数") && m.contains("公斤数")),
        "拒绝文案须点名米数与公斤数不一致，实际 {v}"
    );
    // 零副作用：库存与累计量都不得被这次被拒调用改动
    let row = find_row(&db, 1).await;
    assert_eq!(row.weight_kg, Some(dec("100.00")), "被拒入库不得改库存");
    assert_eq!(row.length_m, Some(dec("200.00")), "被拒入库不得改库存长度");
    assert_eq!(
        row.quantity_kg,
        Some(dec("100.00")),
        "被拒入库不得改累计量"
    );
}

/// 边界：档案未配克重（gram_weight 为 NULL）时米与公斤是两个无从互推的独立量，
/// 即便录入值对不上任何换算率也必须放行，不臆造线密度把正常录入判成非法。
/// 变异自证：若把 ensure_dual_unit_consistent 的「克重缺席即跳过」去掉，本用例会被误拒而失败。
#[tokio::test]
async fn stock_in_skips_dual_unit_check_when_density_absent() {
    let db = setup_test_db().await;
    seed_warehouse(&db, 1).await;
    // seed_full_greige 配了 width_cm=180 但克重缺省为 NULL
    seed_full_greige(&db, 1).await;

    let app = build_app(db.clone(), make_auth(100));
    let (status, v) = call(
        &app,
        Method::POST,
        "/greige-fabrics/1/stock-in",
        Some(json!({"warehouse_id": 1, "weight_kg": 50.0, "length_m": 10.0})),
    )
    .await;
    assert_eq!(
        status,
        StatusCode::OK,
        "克重未配时无从换算，双单位校验必须跳过并放行，实际 {status} {v}"
    );
}

/// 反向：建单累计量米数与公斤数对不上已配线密度，应拒成校验错误且不落库。
/// 变异自证：删除 create 里的累计量一致性检查会让本用例建单成功（返回 200）而失败。
#[tokio::test]
async fn create_rejects_inconsistent_cumulative_dual_unit() {
    let db = setup_test_db().await;
    let app = build_app(db.clone(), make_auth(100));
    let (status, v) = call(
        &app,
        Method::POST,
        "/greige-fabrics",
        Some(json!({
            "fabric_type": "针织",
            "gram_weight": 200.0,
            "width_cm": 180.0,
            "quantity_meters": 100.0,
            "quantity_kg": 90.0,
        })),
    )
    .await;
    assert_eq!(
        status,
        StatusCode::BAD_REQUEST,
        "建单累计量米↔公斤不自洽应被拒成 400，实际 {status} {v}"
    );
    assert_eq!(v["code"], "VALIDATION_ERROR", "错误码应为校验码：{v}");

    // 自洽的累计量（100m×200g/m²×1.8m=36kg）应放行
    let (ok_status, ok_v) = call(
        &app,
        Method::POST,
        "/greige-fabrics",
        Some(json!({
            "fabric_type": "针织",
            "gram_weight": 200.0,
            "width_cm": 180.0,
            "quantity_meters": 100.0,
            "quantity_kg": 36.0,
        })),
    )
    .await;
    assert_eq!(
        ok_status,
        StatusCode::OK,
        "自洽的建单累计量应放行，实际 {ok_status} {ok_v}"
    );
}

