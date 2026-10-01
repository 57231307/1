//! 采购订单明细更新写侧契约锁（任务 #145）
//!
//! 锁定的缺陷与修复（客诉实证"创建时填了完整内容，保存后再次编辑发现前面保存的
//! 内容不完整"）：`UpdateOrderItemRequest`（`backend/src/services/po/mod.rs`）
//! 原只有 material_id/unit_price/quantity_ordered/tax_rate/notes 五字段，
//! 编辑明细时 **折扣率 discount_percent / 交货允差 quantity_tolerance_pct /
//! 色号 color_no / 辅助数量 quantity_alt_ordered** 四类值改了也落不了库
//! （前端送过来也被 serde 忽略），重新打开还是旧值。
//!
//! 修复后形态（本测试逐项锁死）：
//! - DTO 补齐四写侧字段（键名与 `CreateOrderItemRequest` 同源：`color_no`/
//!   `quantity_alt_ordered`/`discount_percent`/`quantity_tolerance_pct`）；
//! - 允差范围校验复用创建路径同一权威函数 `validate_quantity_tolerance_pct`
//!   （0~100），越界拒绝经 `validate_write` 以 **business_displayable** 外显
//!   真实文案（不是脱敏的「业务处理失败」/「请求参数验证失败」）；
//! - 金额派生列（subtotal/tax_amount/discount_amount/total_amount）重算唯一
//!   权威 = 创建路径 `calculate_item_amounts`（`order_ops/crud.rs`），更新链路
//!   不得另写公式；
//! - 色号写 `color_code` 列与创建路径同口径分档：普通行直写列不反查；仅
//!   **转采购行**（行上已带 supplier_product_code 快照——创建路径仅在请求携带
//!   source_sales_order_id 时才建 SkuMappingService 并恒写该快照，快照列存在性
//!   即转采购行的持久化标记）改色号才经创建/更新共用私有函数
//!   `resolve_supplier_sku_snapshot`（product_colors.id 反查 + SkuMappingService
//!   对照解析）刷新保密快照列；把转采购门控错误套到普通行属语义漂移，一并锁死；
//! - 请求 DTO 结构性不含 supplier_* 字段，客户端伪造该两键只被 serde 忽略，
//!   快照值唯一来源是服务端解析；
//! - 「缺省即不改」：Option 为 None 的字段保持原值。
//!
//! 覆盖策略（对齐 `contract_wave1_purchase_return_dimensions_test.rs` /
//! `contract_wave2_contract_no_and_remark_test.rs` 先例，无 mock）：
//! - serde 解码（无 DB）：四新键双形态解码、缺键 = None、supplier_* 键被忽略、
//!   越界允差在 DTO 层即被共用校验函数拒绝且文案外显；
//! - sqlite::memory: 自建表 + 真实 handler（tower oneshot）：仅覆盖**不触库**的
//!   拒绝路径——越界允差在 handler 入口 `validate_write()?` 即返回 400
//!   （BUSINESS_ERROR + 真实文案），库存零变化；
//! - 成功路径端到端为**已迁移 PG 的 `#[ignore]` 活库用例**（由专用 job
//!   `ci-test-rust-ignored` 执行）：`update_order_item` 服务链对
//!   purchase_orders 加 `lock_exclusive()`，sqlite 方言不支持行锁（先例原话），
//!   不得伪装成 sqlite 用例；断言 PUT 后回读四项 == 提交值、派生列按权威口径
//!   重算、订单合计同步、转采购行快照经服务端反查刷新（伪造 supplier_* 不生效）、
//!   非法色号在转采购行被拒且事务回滚零残留、普通行直写色号不被转采购门控误拒、
//!   仅提交 notes 时四字段原样保持；
//! - 源码扫描防回潮锁：单一权威实现（禁止复制粘贴出第二套色号反查/金额公式），
//!   并锁定更新链路的转采购行门控。

mod test_common;

use axum::{
    Router,
    body::Body,
    extract::State,
    http::{Request, StatusCode},
    middleware::{Next, from_fn_with_state},
    response::Response,
    routing::put,
};
use bingxi_backend::container::AppState;
use bingxi_backend::handlers::purchase_order_handler;
use bingxi_backend::middleware::auth_context::AuthContext;
use bingxi_backend::models::user;
use bingxi_backend::models::{
    product, product_color, product_supplier_mapping, purchase_order, purchase_order_item,
    supplier, supplier_product, supplier_product_color, warehouse,
};
use bingxi_backend::services::po::UpdateOrderItemRequest;
use chrono::{TimeZone, Utc};
use rust_decimal::Decimal;
use sea_orm::{
    ActiveModelTrait, ActiveValue::Set, ConnectionTrait, DbBackend, EntityTrait, Statement,
};
use serde_json::{Value, json};
use std::str::FromStr;
use tower::ServiceExt;

fn dec(s: &str) -> Decimal {
    Decimal::from_str(s).unwrap()
}

/// JSON → Decimal：rust_decimal 出参标准形态是字符串，number 形态也接受（与前端
/// api/purchase.ts 声明的并集一致）；null 即 None（行级允差未指定走默认解析）
fn json_dec(v: &Value) -> Option<Decimal> {
    match v {
        Value::Null => None,
        Value::String(s) => Some(Decimal::from_str(s).unwrap_or_else(|e| {
            panic!("字符串 Decimal 解析失败 {s}: {e}");
        })),
        Value::Number(n) => Some(Decimal::from_str(&n.to_string()).unwrap()),
        other => panic!("期望 Decimal（字符串/数字/null），实际 {other}"),
    }
}

// =========================================================
// 1) serde 解码层（无 DB）
// =========================================================

/// 四个新写侧字段必须真实解码（缺陷：此前 DTO 无这些键，前端提交被 serde 吞掉）
#[test]
fn decode_update_item_request_covers_four_recovered_fields() {
    let req: UpdateOrderItemRequest = serde_json::from_value(json!({
        "quantity_alt_ordered": "25.5",
        "discount_percent": 5,
        "quantity_tolerance_pct": "10.00",
        "color_no": "RED-01",
        "notes": "编辑补录"
    }))
    .expect("四字段写侧键必须解码成功");
    assert_eq!(req.quantity_alt_ordered, Some(dec("25.5")));
    assert_eq!(req.discount_percent, Some(dec("5")));
    assert_eq!(req.quantity_tolerance_pct, Some(dec("10.00")));
    assert_eq!(req.color_no.as_deref(), Some("RED-01"));
    assert_eq!(req.notes.as_deref(), Some("编辑补录"));
}

/// 缺省即不改：缺键 → None（不得把缺键当清空/当 0 落库）
#[test]
fn decode_update_item_request_missing_keys_are_none_not_cleared() {
    let req: UpdateOrderItemRequest =
        serde_json::from_value(json!({ "notes": "只改备注" })).expect("缺键应解码为 None");
    assert!(req.quantity_alt_ordered.is_none());
    assert!(req.discount_percent.is_none());
    assert!(req.quantity_tolerance_pct.is_none());
    assert!(req.color_no.is_none());
}

/// 保密约束：supplier_product_code/supplier_color_no 不在写侧请求中——
/// 前端伪造这两个键只会是未知键被忽略，DTO 结构上无法直写保密列
#[test]
fn forged_supplier_snapshot_keys_are_ignored() {
    let req: UpdateOrderItemRequest = serde_json::from_value(json!({
        "color_no": "RED-01",
        "supplier_product_code": "FORGED-CODE",
        "supplier_color_no": "FORGED-COLOR"
    }))
    .expect("未知 supplier_* 键不得导致解码失败");
    let back = serde_json::to_value(&req).unwrap();
    assert!(
        back.get("supplier_product_code").is_none() && back.get("supplier_color_no").is_none(),
        "UpdateOrderItemRequest 结构上必须无 supplier_* 字段，实际: {back}"
    );
}

/// 越界允差在 DTO 层就被权威函数拒绝（复用 validate_quantity_tolerance_pct，
/// 0~100），且 validate_write 以 business_displayable 携带真实文案（不得被
/// From<ValidationErrors> 脱敏成「请求参数验证失败」）
#[test]
fn out_of_range_tolerance_rejected_by_shared_validator_with_displayable_message() {
    for bad in ["150", "-1"] {
        let req: UpdateOrderItemRequest = serde_json::from_value(json!({
            "quantity_tolerance_pct": bad
        }))
        .unwrap();
        let err = req
            .validate_write()
            .expect_err(&format!("允差 {bad} 越界必须被拒绝"));
        let text = err.to_string();
        assert!(
            text.contains("交货允差百分比(quantity_tolerance_pct)必须在0~100之间"),
            "拒绝原因必须来自共用 validate_quantity_tolerance_pct 原文，实际: {text}"
        );
    }
    let ok: UpdateOrderItemRequest =
        serde_json::from_value(json!({ "quantity_tolerance_pct": "10.00" })).unwrap();
    ok.validate_write().expect("范围内允差必须通过");
    let none: UpdateOrderItemRequest = serde_json::from_value(json!({})).unwrap();
    none.validate_write()
        .expect("未提交允差不校验（缺省即不改）");
}

// =========================================================
// 2a) 真实 handler · 不触库拒绝路径（sqlite::memory: 自建表）
// =========================================================
//
// `update_order_item` 服务链对 purchase_orders 使用 `lock_exclusive()`
// （receipt.rs 行锁 + calculate_order_total_txn 重算行锁），sqlite 方言不支持
// ——本仓先例（contract_wave1_purchase_return_dimensions /
// contract_wave2_contract_no_and_remark）已将其成功路径全部落为 #[ignore] 活库
// 用例。sqlite 侧只保留**校验在取连接之前**、确定不触库的拒绝路径，
// 假绿零收益（对照先例：越界允差在 handler 入口 `validate_write()?` 即返回）。

fn make_auth(user_id: i32) -> AuthContext {
    AuthContext {
        user_id,
        username: format!("e2e_user_{user_id}"),
        role_id: Some(1),
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

fn item_router(db: sea_orm::DatabaseConnection) -> Router {
    let mut state = AppState::default();
    state.db = std::sync::Arc::new(db);
    Router::new()
        .route(
            "/orders/{id}/items/{item_id}",
            put(purchase_order_handler::update_order_item),
        )
        .with_state(state)
        .layer(from_fn_with_state(make_auth(100), inject_auth))
}

/// 各表最小 DDL：列与 `models/*.rs::Model` 逐列对应（Decimal 列用 TEXT、bool 用
/// INTEGER，先例 `contract_wave1_ar_payment_error_mapping_test.rs`）；种子行统一
/// 经 SeaORM ActiveModel 写入，由类型系统保证编码正确。
async fn create_tables(db: &sea_orm::DatabaseConnection) {
    let ddls = [
        r#"CREATE TABLE purchase_orders (
            id INTEGER PRIMARY KEY,
            order_no TEXT NOT NULL UNIQUE, supplier_id INTEGER NOT NULL,
            order_date TEXT NOT NULL, expected_delivery_date TEXT, actual_delivery_date TEXT,
            warehouse_id INTEGER NOT NULL, department_id INTEGER NOT NULL,
            purchaser_id INTEGER NOT NULL, currency TEXT NOT NULL,
            exchange_rate TEXT NOT NULL, total_amount TEXT NOT NULL,
            total_amount_foreign TEXT NOT NULL, total_quantity TEXT NOT NULL,
            total_quantity_alt TEXT NOT NULL, order_status TEXT NOT NULL,
            payment_terms TEXT, shipping_terms TEXT, notes TEXT, attachment_urls TEXT,
            created_by INTEGER NOT NULL, created_at TEXT NOT NULL,
            updated_by INTEGER, updated_at TEXT NOT NULL,
            approved_by INTEGER, approved_at TEXT, rejected_reason TEXT
        )"#,
        r#"CREATE TABLE purchase_order_item (
            id INTEGER PRIMARY KEY,
            order_id INTEGER NOT NULL, line_no INTEGER NOT NULL, product_id INTEGER NOT NULL,
            quantity TEXT NOT NULL, quantity_alt TEXT NOT NULL,
            unit_price TEXT NOT NULL, unit_price_foreign TEXT NOT NULL,
            discount_percent TEXT NOT NULL, tax_percent TEXT NOT NULL,
            subtotal TEXT NOT NULL, tax_amount TEXT NOT NULL,
            discount_amount TEXT NOT NULL, total_amount TEXT NOT NULL,
            received_quantity TEXT NOT NULL, received_quantity_alt TEXT NOT NULL,
            quantity_tolerance_pct TEXT, notes TEXT,
            created_at TEXT NOT NULL, updated_at TEXT NOT NULL,
            color_code TEXT, lot_no TEXT, batch_no TEXT,
            supplier_product_code TEXT, supplier_color_no TEXT
        )"#,
        r#"CREATE TABLE product_colors (
            id INTEGER PRIMARY KEY,
            product_id INTEGER NOT NULL, color_no TEXT NOT NULL, color_name TEXT NOT NULL,
            pantone_code TEXT, color_type TEXT NOT NULL, dye_formula TEXT,
            extra_cost TEXT NOT NULL, is_active INTEGER NOT NULL,
            created_at TEXT NOT NULL, updated_at TEXT NOT NULL
        )"#,
        r#"CREATE TABLE supplier_products (
            id INTEGER PRIMARY KEY,
            supplier_id INTEGER NOT NULL, product_code TEXT NOT NULL, product_name TEXT NOT NULL,
            product_description TEXT, unit TEXT NOT NULL, is_enabled INTEGER NOT NULL,
            created_at TEXT NOT NULL, updated_at TEXT NOT NULL,
            created_by INTEGER, updated_by INTEGER, remarks TEXT
        )"#,
        r#"CREATE TABLE supplier_product_colors (
            id INTEGER PRIMARY KEY,
            supplier_product_id INTEGER NOT NULL, color_no TEXT NOT NULL, color_name TEXT NOT NULL,
            pantone_code TEXT, extra_cost TEXT NOT NULL, is_enabled INTEGER NOT NULL,
            created_at TEXT NOT NULL, updated_at TEXT NOT NULL, remarks TEXT
        )"#,
        r#"CREATE TABLE product_supplier_mappings (
            id INTEGER PRIMARY KEY,
            product_id INTEGER NOT NULL, product_color_id INTEGER, supplier_id INTEGER NOT NULL,
            supplier_product_id INTEGER NOT NULL, supplier_product_color_id INTEGER,
            is_primary INTEGER NOT NULL, priority INTEGER NOT NULL,
            supplier_price TEXT, min_order_quantity TEXT, lead_time INTEGER,
            is_enabled INTEGER NOT NULL, created_at TEXT NOT NULL, updated_at TEXT NOT NULL,
            created_by INTEGER, updated_by INTEGER, remarks TEXT
        )"#,
        r#"CREATE TABLE users (
            id INTEGER PRIMARY KEY,
            username TEXT NOT NULL, password_hash TEXT NOT NULL,
            real_name TEXT, avatar TEXT, email TEXT, phone TEXT,
            role_id INTEGER, department_id INTEGER, is_active INTEGER NOT NULL,
            totp_secret TEXT, is_totp_enabled INTEGER NOT NULL, totp_recovery_codes TEXT,
            last_login_at TEXT, password_changed_at TEXT, agreed_to_terms_at TEXT,
            created_at TEXT NOT NULL, updated_at TEXT NOT NULL,
            gender TEXT, birth_date TEXT
        )"#,
        r#"CREATE TABLE audit_logs (
            id INTEGER PRIMARY KEY,
            user_id INTEGER, username TEXT, action TEXT NOT NULL,
            resource_type TEXT, resource_id TEXT, resource_name TEXT, description TEXT,
            ip_address TEXT, user_agent TEXT, request_method TEXT, request_path TEXT,
            request_body TEXT, response_status INTEGER, duration_ms INTEGER,
            old_value TEXT, new_value TEXT, created_at TEXT,
            operation_type TEXT, severity TEXT, request_id TEXT,
            before_snapshot TEXT, after_snapshot TEXT, condition TEXT,
            export_record_count INTEGER, export_query_filter TEXT, export_file_format TEXT,
            export_approval_token TEXT, export_watermark_user TEXT
        )"#,
    ];
    for ddl in ddls {
        db.execute_raw(Statement::from_sql_and_values(
            DbBackend::Sqlite,
            ddl,
            Vec::new(),
        ))
        .await
        .unwrap_or_else(|e| panic!("DDL 执行失败: {e}"));
    }
}

/// sqlite 种子：DRAFT 采购订单 1（created_by=100=操作人）+ 明细行 1
/// （数量 10 × 单价 100，tax 13%，无折扣/允差/色号/辅量）。仅供不触库的
/// 拒绝路径用例确认「被拒后库里零变化」。
async fn seed(db: &sea_orm::DatabaseConnection) {
    let now = Utc::now();
    user::ActiveModel {
        id: sea_orm::ActiveValue::Set(100),
        username: sea_orm::ActiveValue::Set("po_item_editor".to_string()),
        password_hash: sea_orm::ActiveValue::Set("x".to_string()),
        is_active: sea_orm::ActiveValue::Set(true),
        is_totp_enabled: sea_orm::ActiveValue::Set(false),
        created_at: sea_orm::ActiveValue::Set(now),
        updated_at: sea_orm::ActiveValue::Set(now),
        ..Default::default()
    }
    .insert(db)
    .await
    .unwrap();

    purchase_order::ActiveModel {
        id: sea_orm::ActiveValue::Set(1),
        order_no: sea_orm::ActiveValue::Set("PO20260920001".to_string()),
        supplier_id: sea_orm::ActiveValue::Set(7),
        order_date: sea_orm::ActiveValue::Set(
            chrono::NaiveDate::from_ymd_opt(2026, 9, 20).unwrap(),
        ),
        warehouse_id: sea_orm::ActiveValue::Set(1),
        department_id: sea_orm::ActiveValue::Set(1),
        purchaser_id: sea_orm::ActiveValue::Set(100),
        currency: sea_orm::ActiveValue::Set("CNY".to_string()),
        exchange_rate: sea_orm::ActiveValue::Set(Decimal::new(1, 0)),
        total_amount: sea_orm::ActiveValue::Set(dec("1130")),
        total_amount_foreign: sea_orm::ActiveValue::Set(dec("1130")),
        total_quantity: sea_orm::ActiveValue::Set(Decimal::new(10, 0)),
        total_quantity_alt: sea_orm::ActiveValue::Set(Decimal::ZERO),
        order_status: sea_orm::ActiveValue::Set(
            bingxi_backend::models::status::purchase_inventory::purchase_order::DRAFT.to_string(),
        ),
        created_by: sea_orm::ActiveValue::Set(100),
        created_at: sea_orm::ActiveValue::Set(now),
        updated_at: sea_orm::ActiveValue::Set(now),
        ..Default::default()
    }
    .insert(db)
    .await
    .unwrap();

    purchase_order_item::ActiveModel {
        id: sea_orm::ActiveValue::Set(1),
        order_id: sea_orm::ActiveValue::Set(1),
        line_no: sea_orm::ActiveValue::Set(1),
        product_id: sea_orm::ActiveValue::Set(5),
        quantity: sea_orm::ActiveValue::Set(Decimal::new(10, 0)),
        quantity_alt: sea_orm::ActiveValue::Set(Decimal::ZERO),
        unit_price: sea_orm::ActiveValue::Set(Decimal::new(100, 0)),
        unit_price_foreign: sea_orm::ActiveValue::Set(Decimal::new(100, 0)),
        discount_percent: sea_orm::ActiveValue::Set(Decimal::ZERO),
        tax_percent: sea_orm::ActiveValue::Set(dec("13")),
        subtotal: sea_orm::ActiveValue::Set(dec("1000")),
        tax_amount: sea_orm::ActiveValue::Set(dec("130")),
        discount_amount: sea_orm::ActiveValue::Set(Decimal::ZERO),
        total_amount: sea_orm::ActiveValue::Set(dec("1130")),
        received_quantity: sea_orm::ActiveValue::Set(Decimal::ZERO),
        received_quantity_alt: sea_orm::ActiveValue::Set(Decimal::ZERO),
        created_at: sea_orm::ActiveValue::Set(now),
        updated_at: sea_orm::ActiveValue::Set(now),
        ..Default::default()
    }
    .insert(db)
    .await
    .unwrap();
}

async fn seeded_app() -> (Router, sea_orm::DatabaseConnection) {
    let db = sea_orm::Database::connect("sqlite::memory:")
        .await
        .expect("sqlite::memory: 连接失败");
    create_tables(&db).await;
    seed(&db).await;
    (item_router(db.clone()), db)
}

/// PUT 更新并返回响应（ApiResponse.data = 更新后的 purchase_order_item::Model
/// 序列化——与 GET /purchase/orders/{id} 详情 items 同一键形态，即"重新打开编辑"
/// 实际读到的东西；`list_order_items` 端点的 returned_quantity 用 PG 专属
/// `CAST(... AS NUMERIC)` 直映，sqlite 下无方言保真，不在此重复覆盖）。
async fn put_item(app: &Router, uri: &str, body: Value) -> (StatusCode, Value) {
    let resp = app
        .clone()
        .oneshot(
            Request::builder()
                .method("PUT")
                .uri(uri)
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
    (status, serde_json::from_slice(&bytes).unwrap())
}

/// 越界允差（>100 / <0）必须被业务错误拒绝，且拒绝文案以 business_displayable
/// 外显真实原因——不是脱敏的「业务处理失败」，也不是走样的「请求参数验证失败」；
/// 校验复用创建路径同一 validate_quantity_tolerance_pct（0~100）。
/// handler 入口即拒绝、不触达任何 SQL（sqlite 方言限制不构成本用例障碍）。
#[tokio::test]
async fn update_item_out_of_range_tolerance_rejected_with_displayable_message() {
    let (app, db) = seeded_app().await;
    for bad in [json!(150), json!(-1)] {
        let (status, v) = put_item(
            &app,
            "/orders/1/items/1",
            json!({ "quantity_tolerance_pct": bad }),
        )
        .await;
        assert_eq!(
            status,
            StatusCode::BAD_REQUEST,
            "越界允差必须 400，实际体: {v}"
        );
        assert_eq!(v["code"], "BUSINESS_ERROR", "实际体: {v}");
        assert_eq!(
            v["message"], "交货允差百分比(quantity_tolerance_pct)必须在0~100之间",
            "business_displayable 必须外显真实拒绝文案，不得脱敏"
        );
        assert_ne!(v["message"], "业务处理失败");
        assert_ne!(v["message"], "请求参数验证失败");
    }
    // 拒绝路径零写入：允差仍是 seed 的 NULL
    let row = purchase_order_item::Entity::find_by_id(1)
        .one(&db)
        .await
        .unwrap()
        .unwrap();
    assert_eq!(row.quantity_tolerance_pct, None);
}

// =========================================================
// 2b) 真实 handler 成功路径端到端（已迁移 PG 活库，#[ignore]，
//     由专用 job ci-test-rust-ignored 执行）
// =========================================================

/// 活库播种：产品/仓库/供应商/DRAFT 采购订单/明细行（先例
/// `contract_wave1_purchase_return_dimensions_test.rs::seed_full_po_context`
/// 同式，唯一后缀防并行撞约束）。`transfer_snapshot` 非 None 时把明细写成
/// **转采购行**（带旧供应商快照列——创建路径 source_sales_order_id 分支的
/// 持久化形态），用于锁「改色号刷新快照」；None 为普通行。
/// 返回 (order_id, item_id, supplier_id, product_id)。
async fn seed_live_po(
    db: &sea_orm::DatabaseConnection,
    transfer_snapshot: Option<(&str, &str)>,
) -> (i32, i32, i32, i32) {
    let suffix = Utc::now().timestamp_nanos_opt().unwrap();
    let now = Utc::now();

    let p = product::ActiveModel {
        name: Set(format!("波次测试坯布-{suffix}")),
        code: Set(format!("FAB-W145-{suffix}")),
        unit: Set("米".to_string()),
        status: Set("active".to_string()),
        is_deleted: Set(false),
        product_type: Set("fabric".to_string()),
        created_at: Set(now),
        updated_at: Set(now),
        ..Default::default()
    }
    .insert(db)
    .await
    .unwrap();

    let wh = warehouse::ActiveModel {
        warehouse_code: Set(format!("WH-W145-{suffix}")),
        name: Set("波次测试仓".to_string()),
        is_default: Set(false),
        is_active: Set(true),
        created_at: Set(now),
        updated_at: Set(now),
        ..Default::default()
    }
    .insert(db)
    .await
    .unwrap();

    let zero_tz = chrono::FixedOffset::east_opt(0).unwrap();
    let sup_now = zero_tz.from_utc_datetime(&now.naive_utc());
    let sup = supplier::ActiveModel {
        supplier_code: Set(format!("SUP-W145-{suffix}")),
        supplier_name: Set(format!("波次测试供应商-{suffix}")),
        supplier_short_name: Set("波供".to_string()),
        supplier_type: Set("面料供应商".to_string()),
        credit_code: Set("91330000TEST00000X".to_string()),
        registered_address: Set("测试注册地址".to_string()),
        legal_representative: Set("测试法人".to_string()),
        registered_capital: Set(Decimal::ZERO),
        establishment_date: Set(chrono::NaiveDate::from_ymd_opt(2020, 1, 1).unwrap()),
        taxpayer_type: Set("一般纳税人".to_string()),
        bank_name: Set("测试银行".to_string()),
        bank_account: Set("6222000000000000".to_string()),
        contact_phone: Set("13800000000".to_string()),
        created_at: Set(sup_now),
        updated_at: Set(sup_now),
        is_processor: Set(false),
        ..Default::default()
    }
    .insert(db)
    .await
    .unwrap();

    let po = purchase_order::ActiveModel {
        order_no: Set(format!("PO-W145-{suffix}")),
        supplier_id: Set(sup.id),
        order_date: Set(chrono::NaiveDate::from_ymd_opt(2026, 9, 20).unwrap()),
        warehouse_id: Set(wh.id),
        department_id: Set(1),
        purchaser_id: Set(100),
        currency: Set("CNY".to_string()),
        exchange_rate: Set(Decimal::ONE),
        total_amount: Set(dec("1130")),
        total_amount_foreign: Set(dec("1130")),
        total_quantity: Set(Decimal::new(10, 0)),
        total_quantity_alt: Set(Decimal::ZERO),
        order_status: Set(
            bingxi_backend::models::status::purchase_inventory::purchase_order::DRAFT.to_string(),
        ),
        created_by: Set(100),
        created_at: Set(now),
        updated_at: Set(now),
        ..Default::default()
    }
    .insert(db)
    .await
    .unwrap();

    let mut item = purchase_order_item::ActiveModel {
        order_id: Set(po.id),
        line_no: Set(1),
        product_id: Set(p.id),
        quantity: Set(Decimal::new(10, 0)),
        quantity_alt: Set(Decimal::ZERO),
        unit_price: Set(Decimal::new(100, 0)),
        unit_price_foreign: Set(Decimal::new(100, 0)),
        discount_percent: Set(Decimal::ZERO),
        tax_percent: Set(dec("13")),
        subtotal: Set(dec("1000")),
        tax_amount: Set(dec("130")),
        discount_amount: Set(Decimal::ZERO),
        total_amount: Set(dec("1130")),
        received_quantity: Set(Decimal::ZERO),
        received_quantity_alt: Set(Decimal::ZERO),
        created_at: Set(now),
        updated_at: Set(now),
        ..Default::default()
    };
    if let Some((code, color)) = transfer_snapshot {
        item.supplier_product_code = Set(Some(code.to_string()));
        item.supplier_color_no = Set(Some(color.to_string()));
    }
    let poi = item.insert(db).await.unwrap();

    (po.id, poi.id, sup.id, p.id)
}

/// 活库播种色卡与供应商 SKU 对照：product_colors(RED-01) +
/// supplier_products(SP-999) + supplier_product_colors(SC-77) +
/// product_supplier_mappings（is_primary/is_enabled，priority=1，与
/// SkuMappingService::resolve_supplier_sku 的过滤条件逐项对应）。
async fn seed_live_color_and_mapping(
    db: &sea_orm::DatabaseConnection,
    product_id: i32,
    supplier_id: i32,
) {
    let now = Utc::now();
    let pc = product_color::ActiveModel {
        product_id: Set(product_id),
        color_no: Set("RED-01".to_string()),
        color_name: Set("大红".to_string()),
        color_type: Set("常规色".to_string()),
        extra_cost: Set(Decimal::ZERO),
        is_active: Set(true),
        created_at: Set(now),
        updated_at: Set(now),
        ..Default::default()
    }
    .insert(db)
    .await
    .unwrap();

    let sp = supplier_product::ActiveModel {
        supplier_id: Set(supplier_id),
        product_code: Set("SP-999".to_string()),
        product_name: Set("供应商面料".to_string()),
        unit: Set("米".to_string()),
        is_enabled: Set(true),
        created_at: Set(now),
        updated_at: Set(now),
        ..Default::default()
    }
    .insert(db)
    .await
    .unwrap();

    let spc = supplier_product_color::ActiveModel {
        supplier_product_id: Set(sp.id),
        color_no: Set("SC-77".to_string()),
        color_name: Set("大红".to_string()),
        extra_cost: Set(Decimal::ZERO),
        is_enabled: Set(true),
        created_at: Set(now),
        updated_at: Set(now),
        ..Default::default()
    }
    .insert(db)
    .await
    .unwrap();

    product_supplier_mapping::ActiveModel {
        product_id: Set(product_id),
        product_color_id: Set(Some(pc.id)),
        supplier_id: Set(supplier_id),
        supplier_product_id: Set(sp.id),
        supplier_product_color_id: Set(Some(spc.id)),
        is_primary: Set(true),
        priority: Set(1),
        is_enabled: Set(true),
        created_at: Set(now),
        updated_at: Set(now),
        ..Default::default()
    }
    .insert(db)
    .await
    .unwrap();
}

/// 核心回归锁（客诉场景）：编辑明细补录折扣/允差/色号/辅量 → PUT 成功，
/// 回读四项 == 提交值（修复前 serde 吞键、值根本落不了库，回读恒为旧值/NULL）。
/// 同时锁定普通行语义：color_code 直写列，**不得**因缺 SKU 对照被转采购门控
/// 误拒（创建路径普通单同样直写、不反查，读写口径同源）。
#[tokio::test]
#[ignore = "需要 TEST_DATABASE_URL 已迁移 PostgreSQL：update_order_item 服务链对 purchase_orders 加 lock_exclusive，sqlite 方言不支持行锁"]
async fn update_item_persists_four_fields_and_reads_back() {
    let db = test_common::setup_test_db().await;
    let (order_id, item_id, _sup_id, _prod_id) = seed_live_po(&db, None).await;
    let app = item_router(db.clone());
    let uri = format!("/orders/{order_id}/items/{item_id}");
    let (status, v) = put_item(
        &app,
        &uri,
        json!({
            "quantity_alt_ordered": "25.5",
            "discount_percent": 5,
            "quantity_tolerance_pct": "10.00",
            "color_no": "RED-01"
        }),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "四字段更新必须 200，实际体: {v}");

    // 出参回读（Model serde，键与详情 items 一致）
    let item = &v["data"];
    assert_eq!(json_dec(&item["discount_percent"]), Some(dec("5")));
    assert_eq!(
        json_dec(&item["quantity_tolerance_pct"]),
        Some(dec("10.00"))
    );
    assert_eq!(item["color_code"], "RED-01");
    assert_eq!(json_dec(&item["quantity_alt"]), Some(dec("25.5")));

    // DB 真相层同步锁死（重新读取实体 == 提交值）
    let row = purchase_order_item::Entity::find_by_id(item_id)
        .one(&db)
        .await
        .unwrap()
        .unwrap();
    assert_eq!(row.discount_percent, dec("5"));
    assert_eq!(row.quantity_tolerance_pct, Some(dec("10.00")));
    assert_eq!(row.color_code.as_deref(), Some("RED-01"));
    assert_eq!(row.quantity_alt, dec("25.5"));
    // 普通行：不反查、不刷快照（与创建路径普通单口径一致）
    assert_eq!(row.supplier_product_code, None, "普通行不得被写入反查快照");
    assert_eq!(row.supplier_color_no, None);
}

/// 派生金额列按创建路径唯一权威 `calculate_item_amounts` 口径重算：
/// subtotal=10×100=1000，discount_amount=1000×5%=50，tax_amount=130，
/// total=1000+130-50=1080；订单合计同步（calculate_order_total_txn）
#[tokio::test]
#[ignore = "需要 TEST_DATABASE_URL 已迁移 PostgreSQL（update_order_item 链路 lock_exclusive，同上前例）"]
async fn update_item_recalculates_derived_amounts_with_create_authority() {
    let db = test_common::setup_test_db().await;
    let (order_id, item_id, _sup, _prod) = seed_live_po(&db, None).await;
    let app = item_router(db.clone());
    let uri = format!("/orders/{order_id}/items/{item_id}");
    let (status, v) = put_item(
        &app,
        &uri,
        json!({ "discount_percent": 5, "quantity_alt_ordered": "25.5" }),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "实际体: {v}");

    let item = &v["data"];
    assert_eq!(json_dec(&item["subtotal"]), Some(dec("1000")));
    assert_eq!(json_dec(&item["discount_amount"]), Some(dec("50")));
    assert_eq!(json_dec(&item["tax_amount"]), Some(dec("130")));
    assert_eq!(json_dec(&item["total_amount"]), Some(dec("1080")));

    let order = purchase_order::Entity::find_by_id(order_id)
        .one(&db)
        .await
        .unwrap()
        .unwrap();
    assert_eq!(
        order.total_amount,
        dec("1080"),
        "订单合计必须按重算后的行总额"
    );
    assert_eq!(order.total_quantity_alt, dec("25.5"));
}

/// 色号权威反查锁（转采购行）：改色号后经与创建路径同一套逻辑反查
/// product_colors.id 并解析供应商保密对照，旧快照（OLD-CODE/OLD-COLOR）被刷新为
/// 服务端解析值；请求体里**伪造** supplier_product_code/supplier_color_no 两键
/// 完全不生效（serde 忽略，快照值唯一来源是服务端解析）。
#[tokio::test]
#[ignore = "需要 TEST_DATABASE_URL 已迁移 PostgreSQL（update_order_item 链路 lock_exclusive，同上前例）"]
async fn update_item_color_no_refreshes_supplier_snapshot_via_authoritative_lookup() {
    let db = test_common::setup_test_db().await;
    let (order_id, item_id, sup_id, prod_id) =
        seed_live_po(&db, Some(("OLD-CODE", "OLD-COLOR"))).await;
    seed_live_color_and_mapping(&db, prod_id, sup_id).await;
    let app = item_router(db.clone());
    let uri = format!("/orders/{order_id}/items/{item_id}");
    let (status, v) = put_item(
        &app,
        &uri,
        json!({
            "color_no": "RED-01",
            "supplier_product_code": "FORGED-CODE",
            "supplier_color_no": "FORGED-COLOR"
        }),
    )
    .await;
    assert_eq!(
        status,
        StatusCode::OK,
        "转采购行改色号必须 200，实际体: {v}"
    );

    let row = purchase_order_item::Entity::find_by_id(item_id)
        .one(&db)
        .await
        .unwrap()
        .unwrap();
    assert_eq!(row.color_code.as_deref(), Some("RED-01"));
    assert_eq!(
        row.supplier_product_code.as_deref(),
        Some("SP-999"),
        "快照必须由服务端权威反查刷新：伪造键不生效、旧快照不留存"
    );
    assert_eq!(row.supplier_color_no.as_deref(), Some("SC-77"));
}

/// 转采购行提交了产品下不存在的色号 → 服务端如实拒绝（复用创建路径同一反查
/// 错误分支），事务回滚：不得静默落一个对不上号的 color_code，也不得留下
/// 与新色不一致的旧快照
#[tokio::test]
#[ignore = "需要 TEST_DATABASE_URL 已迁移 PostgreSQL（update_order_item 链路 lock_exclusive，同上前例）"]
async fn update_item_unknown_color_no_rejected_and_nothing_persisted() {
    let db = test_common::setup_test_db().await;
    let (order_id, item_id, sup_id, prod_id) =
        seed_live_po(&db, Some(("OLD-CODE", "OLD-COLOR"))).await;
    seed_live_color_and_mapping(&db, prod_id, sup_id).await;
    let app = item_router(db.clone());
    let uri = format!("/orders/{order_id}/items/{item_id}");
    let (status, _v) = put_item(&app, &uri, json!({ "color_no": "NO-SUCH-COLOR" })).await;
    assert_eq!(status, StatusCode::BAD_REQUEST, "非法色号必须 400");
    let row = purchase_order_item::Entity::find_by_id(item_id)
        .one(&db)
        .await
        .unwrap()
        .unwrap();
    assert_eq!(row.color_code, None, "被拒绝的更新不得留下任何脏值");
    assert_eq!(row.supplier_product_code.as_deref(), Some("OLD-CODE"));
    assert_eq!(row.supplier_color_no.as_deref(), Some("OLD-COLOR"));
}

/// 「缺省即不改」语义锁：只提交 notes 时，折扣/允差/色号/辅量四字段保持原值
/// （None 不得被当成清空或 0 ——那会把用户创建时填的内容二次弄丢）
#[tokio::test]
#[ignore = "需要 TEST_DATABASE_URL 已迁移 PostgreSQL（update_order_item 链路 lock_exclusive，同上前例）"]
async fn update_item_partial_request_keeps_untouched_fields() {
    let db = test_common::setup_test_db().await;
    let (order_id, item_id, sup_id, prod_id) = seed_live_po(&db, None).await;
    seed_live_color_and_mapping(&db, prod_id, sup_id).await;
    let app = item_router(db.clone());
    let uri = format!("/orders/{order_id}/items/{item_id}");
    let (status, v) = put_item(
        &app,
        &uri,
        json!({
            "quantity_alt_ordered": "25.5",
            "discount_percent": 5,
            "quantity_tolerance_pct": "10.00",
            "color_no": "RED-01"
        }),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "先补齐四字段: {v}");

    let (status, v) = put_item(&app, &uri, json!({ "notes": "只改备注" })).await;
    assert_eq!(status, StatusCode::OK, "实际体: {v}");
    let item = &v["data"];
    assert_eq!(json_dec(&item["discount_percent"]), Some(dec("5")));
    assert_eq!(
        json_dec(&item["quantity_tolerance_pct"]),
        Some(dec("10.00"))
    );
    assert_eq!(item["color_code"], "RED-01");
    assert_eq!(json_dec(&item["quantity_alt"]), Some(dec("25.5")));
    assert_eq!(item["notes"], "只改备注");

    // DB 真相层同样保持（None 未被当成清空/0）
    let row = purchase_order_item::Entity::find_by_id(item_id)
        .one(&db)
        .await
        .unwrap()
        .unwrap();
    assert_eq!(row.quantity_tolerance_pct, Some(dec("10.00")));
    assert_eq!(row.color_code.as_deref(), Some("RED-01"));
    assert_eq!(row.discount_percent, dec("5"));
    assert_eq!(row.quantity_alt, dec("25.5"));
}

// =========================================================
// 3) 源码扫描防回潮锁（无 DB）
// =========================================================

/// 从源码截取一个顶层块：anchor 起，到首个 "\n}"（先例 contract_wave2 系列；
/// 先剔除 \r 防 CRLF 工作树使跨行断言漏检）
fn extract_block(src: &str, anchor: &str) -> String {
    let src = src.replace('\r', "");
    let i = src
        .find(anchor)
        .unwrap_or_else(|| panic!("源码锚点丢失: {anchor}"));
    let j = src[i..]
        .find("\n}")
        .unwrap_or_else(|| panic!("块结束定位失败: {anchor}"));
    src[i..i + j + 2].to_string()
}

/// 截取 impl 块内的方法（4 空格缩进、以 "\n    }" 结束）：方法体内嵌套块
/// 缩进更深，首个 4 空格收括号即方法收口——用于精确圈定 receipt.rs 的
/// update_order_item 方法体（顶层 extract_block 会一路圈到整个 impl 尾部，
/// 稀释「更新链路不得内联第二套公式」的判据）
fn extract_impl_method(src: &str, anchor: &str) -> String {
    let src = src.replace('\r', "");
    let i = src
        .find(anchor)
        .unwrap_or_else(|| panic!("源码锚点丢失: {anchor}"));
    let j = src[i..]
        .find("\n    }")
        .unwrap_or_else(|| panic!("方法结束定位失败: {anchor}"));
    src[i..i + j + 6].to_string()
}

/// 写侧 DTO 必须覆盖四字段并挂共用校验；结构上不得含 supplier_* 保密列
#[test]
fn source_scan_update_dto_covers_fields_and_hides_supplier_columns() {
    let src = include_str!("../src/services/po/mod.rs");
    let dto = extract_block(src, "pub struct UpdateOrderItemRequest");
    for field in [
        "quantity_alt_ordered",
        "discount_percent",
        "quantity_tolerance_pct",
        "color_no",
    ] {
        assert!(
            dto.contains(&format!("pub {field}: Option<")),
            "UpdateOrderItemRequest 必须含写侧字段 {field}，实际块:\n{dto}"
        );
    }
    assert!(
        dto.contains("#[validate(custom(function = \"validate_quantity_tolerance_pct\"))]"),
        "允差必须复用创建路径同一校验函数，禁止另写校验，实际块:\n{dto}"
    );
    for forbidden in ["supplier_product_code", "supplier_color_no"] {
        assert!(
            !dto.contains(forbidden),
            "写侧请求不得出现保密快照列 {forbidden}，实际块:\n{dto}"
        );
    }
}

/// 更新链路必须调用同一权威实现（金额口径 + 色号反查），禁止复制公式/规则；
/// 且色号反查解析在 po 模块内只允许一份实现；快照反查必须由转采购行门控
/// 包裹（防止把创建路径 source_sales_order_id 分支的门控错误套到普通行——
/// 语义漂移回归锁）
#[test]
fn source_scan_update_path_reuses_single_authority() {
    let crud = include_str!("../src/services/po/order_ops/crud.rs");
    let receipt = include_str!("../src/services/po/receipt.rs");

    let update_body = extract_impl_method(receipt, "pub async fn update_order_item");
    assert!(
        update_body.contains("Self::calculate_item_amounts(&merged)"),
        "更新明细派生金额必须复用创建路径 calculate_item_amounts，实际块:\n{update_body}"
    );
    assert!(
        update_body.contains("Self::resolve_supplier_sku_snapshot("),
        "更新明细色号必须走共用权威反查，实际块:\n{update_body}"
    );
    assert!(
        update_body.contains("current.supplier_product_code.is_some()"),
        "保密快照反查仅限转采购行（行上已有 supplier_product_code 快照），普通行强制反查会把创建路径的转采购门控错误套到普通编辑上，实际块:\n{update_body}"
    );
    assert!(
        !update_body.contains("round_dp"),
        "更新链路不得内联第二套金额公式（round_dp 归一化只允许存在于 calculate_item_amounts），实际块:\n{update_body}"
    );

    let def_count = crud.matches("fn resolve_supplier_sku_snapshot(").count()
        + receipt.matches("fn resolve_supplier_sku_snapshot(").count();
    assert_eq!(
        def_count, 1,
        "色号反查+对照解析在 po 模块内必须只有一份实现（禁止复制粘贴出第二套规则）"
    );
    let calc_count = crud.matches("fn calculate_item_amounts(").count()
        + receipt.matches("fn calculate_item_amounts(").count();
    assert_eq!(calc_count, 1, "金额口径函数只允许一份实现");

    // 创建路径（crud）与更新路径（receipt）都必须是调用方而非各自实现
    assert!(
        crud.contains("Self::resolve_supplier_sku_snapshot("),
        "创建路径必须同样调用共用反查函数"
    );
}

/// handler 更新入口必须执行写侧校验（否则越界允差直达落库）
#[test]
fn source_scan_handler_validates_update_request() {
    let src = include_str!("../src/handlers/purchase_order_handler.rs");
    let body = extract_block(src, "pub async fn update_order_item");
    assert!(
        body.contains("req.validate_write()?"),
        "update_order_item 入口必须校验（复用 validate_quantity_tolerance_pct），实际块:\n{body}"
    );
    assert!(
        !body.contains(".map_err("),
        "AppError 结果必须 `?` 原样传播，禁止强转 500，实际块:\n{body}"
    );
}
