//! 出库四维（缸/色/批/匹）真实 sqlite 契约锁：调拨发运消耗染色匹（用户 2026-10-02 纠正口径）
//!
//! 锁定行为（判定唯一来源 `services/inv/fabric_class.rs`，出库包装
//! `services/inventory_deduction.rs::require_outbound_dimensions`）：
//! 1. 染色布出库明细缺匹号（m0066 前的存量 NULL 行走发运路径）→ 4xx 且信封
//!    code=VALIDATION_ERROR（字段必填=VALIDATION 边界；包装文案含内部产品 ID，
//!    出参走脱敏变体，本文件不断案 message 原文）；拒绝发生在任何库存写动作之前，
//!    随发运事务整体回滚——库存行与匹行必须零漂移；
//! 2. 同口径带真实匹号（inventory_piece 中该仓该缸该批的可用染色匹）→ 200，
//!    且匹级数据被**真实扣减**：inventory_piece 经事务内 CAS AVAILABLE→SHIPPED、
//!    调出仓库存真实扣 10、调入仓 quantity_incoming 加 10、TRANSFER_OUT 流水落库。
//!
//! 为什么此链能在 sqlite 真跑（对照 contract_wave1_transfer_dims 建单链不可 sqlite 化）：
//! `POST /transfers/{id}/ship` → `inv::batch::ship_transfer` 全链为 SeaORM 单表
//! 查询/条件更新（乐观锁 version CAS、匹 CAS），无 pg_advisory 取号、无 Postgres
//! 方言原生 SQL；`lock_exclusive()` 子句在 sea-query SqliteQueryBuilder 中按方言
//! 静默跳过（sea-query-1.0.2 backend/sqlite/query.rs "SQLite doesn't supports row
//! locking"），CAS 语义本身由 UPDATE ... WHERE status=AVAILABLE 在 sqlite 真实保证。
//!
//! 脚手架沿用 contract_wave6_crm_merge_owner_scope_test.rs：seeded_db（sqlite::memory:
//! + 与 models/*.rs 逐列对应的同构 DDL，Decimal→TEXT、DateTime→TEXT、bool→INTEGER，
//! 先例 contract_wave5/wave6 同形态）+ build_app + 注入 auth + 真发 HTTP 请求；
//! 行状态一律回读真库比对。状态字面量全部引自 models::status 词表，不手写第二套。

use axum::{
    Router,
    body::Body,
    http::{Method, Request, StatusCode},
    response::Response,
    routing::post,
};
use bingxi_backend::container::AppState;
use bingxi_backend::handlers::inventory_transfer_handler;
use bingxi_backend::middleware::auth_context::AuthContext;
use bingxi_backend::models::inventory_piece;
use bingxi_backend::models::inventory_stock;
use bingxi_backend::models::inventory_transaction;
use bingxi_backend::models::status::inventory_transfer as transfer_status;
use bingxi_backend::models::status::purchase_inventory::inventory_piece as piece_status;
use sea_orm::{ConnectionTrait, DbBackend, EntityTrait, Statement};
use serde_json::Value;
use std::str::FromStr;
use std::sync::Arc;
use tower::ServiceExt;

fn dec(s: &str) -> rust_decimal::Decimal {
    rust_decimal::Decimal::from_str(s).unwrap()
}

// ---------------------------------------------------------------------------
// 脚手架（wave6 同款）
// ---------------------------------------------------------------------------

fn make_auth() -> AuthContext {
    AuthContext {
        user_id: 60,
        username: "piece_four_dim_ship".to_string(),
        role_id: Some(2),
        department_id: Some(1),
        data_scope: Some("all".to_string()),
        dept_ids: None,
        dept_member_user_ids: None,
    }
}

async fn inject_auth(
    auth: axum::extract::State<AuthContext>,
    mut request: Request<Body>,
    next: axum::middleware::Next,
) -> Response {
    request.extensions_mut().insert(auth.0);
    next.run(request).await
}

async fn exec(db: &sea_orm::DatabaseConnection, sql: &str) {
    db.execute_raw(Statement::from_sql_and_values(
        DbBackend::Sqlite,
        sql,
        Vec::new(),
    ))
    .await
    .unwrap_or_else(|e| panic!("DDL/种子 执行失败: {e}\nSQL: {sql}"));
}

/// 与 models/inventory_transfer.rs::Model 逐列对应
const DDL_TRANSFERS: &str = r#"CREATE TABLE inventory_transfers (
    id INTEGER PRIMARY KEY,
    transfer_no TEXT NOT NULL, from_warehouse_id INTEGER NOT NULL,
    to_warehouse_id INTEGER NOT NULL, transfer_date TEXT NOT NULL,
    status TEXT NOT NULL, total_quantity TEXT NOT NULL, notes TEXT,
    created_by INTEGER, approved_by INTEGER, approved_at TEXT,
    shipped_at TEXT, received_at TEXT,
    created_at TEXT NOT NULL, updated_at TEXT NOT NULL,
    approval_level TEXT, approved_by_role TEXT, total_amount TEXT NOT NULL
)"#;

/// 与 models/inventory_transfer_item.rs::Model 逐列对应（piece_no 为 m0066 补列）
const DDL_TRANSFER_ITEMS: &str = r#"CREATE TABLE inventory_transfer_items (
    id INTEGER PRIMARY KEY, transfer_id INTEGER NOT NULL, product_id INTEGER NOT NULL,
    quantity TEXT NOT NULL, shipped_quantity TEXT NOT NULL, received_quantity TEXT NOT NULL,
    unit_cost TEXT, notes TEXT, created_at TEXT NOT NULL, updated_at TEXT NOT NULL,
    color_no TEXT NOT NULL, dye_lot_no TEXT, batch_no TEXT NOT NULL, piece_no TEXT
)"#;

/// 与 models/inventory_stock.rs::Model 逐列对应
const DDL_STOCKS: &str = r#"CREATE TABLE inventory_stocks (
    id INTEGER PRIMARY KEY,
    warehouse_id INTEGER NOT NULL, product_id INTEGER NOT NULL,
    quantity_on_hand TEXT NOT NULL, quantity_available TEXT NOT NULL,
    quantity_reserved TEXT NOT NULL, quantity_shipped TEXT NOT NULL,
    quantity_incoming TEXT NOT NULL, reorder_point TEXT NOT NULL,
    max_stock_point TEXT NOT NULL, reorder_quantity TEXT NOT NULL,
    bin_location TEXT, last_count_date TEXT, last_movement_date TEXT,
    created_at TEXT NOT NULL, updated_at TEXT NOT NULL,
    batch_no TEXT NOT NULL, color_no TEXT NOT NULL, dye_lot_no TEXT, grade TEXT NOT NULL,
    production_date TEXT, expiry_date TEXT,
    quantity_meters TEXT NOT NULL, quantity_kg TEXT NOT NULL,
    gram_weight TEXT, width TEXT, location_id INTEGER, shelf_no TEXT, layer_no TEXT,
    stock_status TEXT NOT NULL, quality_status TEXT NOT NULL,
    version INTEGER NOT NULL, replenishment_strategy TEXT NOT NULL
)"#;

/// 与 models/inventory_piece.rs::Model 逐列对应（先例 contract_wave5_outsource_issue_guard）
const DDL_PIECE: &str = r#"CREATE TABLE inventory_piece (
    id INTEGER PRIMARY KEY,
    piece_no TEXT NOT NULL, piece_type TEXT NOT NULL, dye_lot_id INTEGER,
    machine_no TEXT, machine_operator TEXT, warehouse_in_at TEXT, supplier_piece_no TEXT,
    length TEXT NOT NULL, weight TEXT, width TEXT, gram_weight TEXT,
    position_no TEXT, package_no TEXT, production_date TEXT, shelf_life INTEGER,
    quality_status TEXT, inventory_status TEXT, warehouse_id INTEGER NOT NULL,
    remarks TEXT, barcode TEXT, product_id INTEGER NOT NULL, batch_no TEXT NOT NULL,
    color_no TEXT NOT NULL, dye_lot_no TEXT NOT NULL,
    parent_piece_id INTEGER, inspection_id INTEGER, piece_seq INTEGER,
    location_id INTEGER, scan_type TEXT, status TEXT NOT NULL,
    created_at TEXT NOT NULL, updated_at TEXT NOT NULL,
    created_by INTEGER, updated_by INTEGER, original_length TEXT, original_weight TEXT
)"#;

/// 与 models/inventory_transaction.rs::Model 逐列对应（发运写 TRANSFER_OUT 流水）
const DDL_TRANSACTIONS: &str = r#"CREATE TABLE inventory_transactions (
    id INTEGER PRIMARY KEY,
    transaction_type TEXT NOT NULL,
    product_id INTEGER NOT NULL, warehouse_id INTEGER NOT NULL,
    batch_no TEXT NOT NULL, color_no TEXT NOT NULL, dye_lot_no TEXT, grade TEXT NOT NULL,
    quantity_meters TEXT NOT NULL, quantity_kg TEXT NOT NULL,
    source_bill_type TEXT, source_bill_no TEXT, source_bill_id INTEGER,
    quantity_before_meters TEXT, quantity_before_kg TEXT,
    quantity_after_meters TEXT, quantity_after_kg TEXT,
    notes TEXT, created_by INTEGER, created_at TEXT NOT NULL
)"#;

/// 与 models/user.rs::Model 逐列对应（ship 链 fetch_username + 详情 JOIN real_name）
const DDL_USERS: &str = r#"CREATE TABLE users (
    id INTEGER PRIMARY KEY,
    username TEXT NOT NULL, password_hash TEXT NOT NULL,
    real_name TEXT, avatar TEXT, email TEXT, phone TEXT,
    role_id INTEGER, department_id INTEGER, is_active INTEGER NOT NULL,
    totp_secret TEXT, is_totp_enabled INTEGER NOT NULL, totp_recovery_codes TEXT,
    last_login_at TEXT, password_changed_at TEXT, agreed_to_terms_at TEXT,
    created_at TEXT NOT NULL, updated_at TEXT NOT NULL,
    gender TEXT, birth_date TEXT
)"#;

/// 与 models/audit_log.rs::Model 对应（AuditLogService::update_with_audit 落审计）
const DDL_AUDIT_LOGS: &str = r#"CREATE TABLE audit_logs (
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
)"#;

/// JOIN 富化仅需被引用列（详情查询 expr_as/column_as 触及的列）
const DDL_WAREHOUSES: &str = "CREATE TABLE warehouses (id INTEGER PRIMARY KEY, name TEXT NOT NULL)";
const DDL_PRODUCTS: &str = r#"CREATE TABLE products (
    id INTEGER PRIMARY KEY, code TEXT NOT NULL, name TEXT NOT NULL,
    unit TEXT NOT NULL, product_grade TEXT
)"#;

/// 种子布局：
/// - 调拨单 1（approved，item piece_no='P-9' 指向真实可用染色匹）→ 发运成功组；
/// - 调拨单 2（approved，item piece_no IS NULL 的染色布存量行）→ 缺匹号拒绝组；
/// - 调出仓(1)库存 50、调入仓(2)库存 0；染色匹 P-9 AVAILABLE。
async fn seeded_db() -> Arc<sea_orm::DatabaseConnection> {
    let db = sea_orm::Database::connect("sqlite::memory:")
        .await
        .expect("sqlite::memory: 连接失败");
    for ddl in [
        DDL_TRANSFERS,
        DDL_TRANSFER_ITEMS,
        DDL_STOCKS,
        DDL_PIECE,
        DDL_TRANSACTIONS,
        DDL_USERS,
        DDL_AUDIT_LOGS,
        DDL_WAREHOUSES,
        DDL_PRODUCTS,
    ] {
        exec(&db, ddl).await;
    }
    exec(
        &db,
        "INSERT INTO warehouses (id,name) VALUES (1,'胚布成品调出仓'),(2,'分仓')",
    )
    .await;
    exec(
        &db,
        "INSERT INTO products (id,code,name,unit,product_grade)
         VALUES (5,'FAB-RED','红色染色布','米','一等品')",
    )
    .await;
    exec(
        &db,
        &format!(
            "INSERT INTO inventory_transfers
             (id,transfer_no,from_warehouse_id,to_warehouse_id,transfer_date,status,
              total_quantity,created_by,created_at,updated_at,total_amount) VALUES
             (1,'TRF-P4D-001',1,2,'2026-01-01T00:00:00Z','{}','10.00',NULL,
              '2026-01-01T00:00:00Z','2026-01-01T00:00:00Z','0.00'),
             (2,'TRF-P4D-002',1,2,'2026-01-01T00:00:00Z','{}','10.00',NULL,
              '2026-01-01T00:00:00Z','2026-01-01T00:00:00Z','0.00')",
            transfer_status::APPROVED,
            transfer_status::APPROVED
        ),
    )
    .await;
    exec(
        &db,
        "INSERT INTO inventory_transfer_items
         (id,transfer_id,product_id,quantity,shipped_quantity,received_quantity,
          created_at,updated_at,color_no,dye_lot_no,batch_no,piece_no) VALUES
         (1,1,5,'10.00','0','0','2026-01-01T00:00:00Z','2026-01-01T00:00:00Z',
          'RED','DL-A','B1','P-9'),
         (2,2,5,'10.00','0','0','2026-01-01T00:00:00Z','2026-01-01T00:00:00Z',
          'RED','DL-A','B1',NULL)",
    )
    .await;
    exec(
        &db,
        "INSERT INTO inventory_stocks
         (id,warehouse_id,product_id,quantity_on_hand,quantity_available,
          quantity_reserved,quantity_shipped,quantity_incoming,reorder_point,
          max_stock_point,reorder_quantity,created_at,updated_at,batch_no,color_no,
          dye_lot_no,grade,quantity_meters,quantity_kg,stock_status,quality_status,
          version,replenishment_strategy) VALUES
         (1,1,5,'50.00','50.00','0','0','0','0','0','0',
          '2026-01-01T00:00:00Z','2026-01-01T00:00:00Z','B1','RED','DL-A',
          '一等品','50.00','10.00','正常','合格',0,'reorder_point'),
         (2,2,5,'0.00','0.00','0','0','0','0','0','0',
          '2026-01-01T00:00:00Z','2026-01-01T00:00:00Z','B1','RED','DL-A',
          '一等品','0.00','0.00','正常','合格',0,'reorder_point')",
    )
    .await;
    exec(
        &db,
        &format!(
            "INSERT INTO inventory_piece
             (id,piece_no,piece_type,warehouse_id,product_id,batch_no,color_no,
              dye_lot_no,length,status,created_at,updated_at) VALUES
             (1,'P-9','dyed',1,5,'B1','RED','DL-A','30.00','{}',
              '2026-01-01T00:00:00Z','2026-01-01T00:00:00Z')",
            piece_status::AVAILABLE
        ),
    )
    .await;
    Arc::new(db)
}

fn build_app(db: &Arc<sea_orm::DatabaseConnection>, auth: AuthContext) -> Router {
    let state = AppState {
        db: db.clone(),
        ..Default::default()
    };
    Router::new()
        .route(
            "/erp/inventory/transfers/{id}/ship",
            post(inventory_transfer_handler::ship_transfer),
        )
        .with_state(state)
        .layer(axum::middleware::from_fn_with_state(auth, inject_auth))
}

async fn ship(app: &Router, id: i32) -> (StatusCode, Value) {
    let resp = app
        .clone()
        .oneshot(
            Request::builder()
                .method(Method::POST)
                .uri(format!("/erp/inventory/transfers/{id}/ship"))
                .body(Body::empty())
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
                "发运 {id} 响应非 JSON: {e}; body={}",
                String::from_utf8_lossy(&bytes)
            )
        }),
    )
}

// ---------------------------------------------------------------------------
// 唯一用例：缺匹号 4xx VALIDATION_ERROR 且零漂移 → 带真实匹号 200 且匹级真实扣减
// ---------------------------------------------------------------------------

#[tokio::test]
async fn dyed_outbound_ship_requires_real_piece_and_consumes_it() {
    let db = seeded_db().await;
    let app = build_app(&db, make_auth());

    // —— ① 染色布存量明细缺匹号：发运必须 4xx + VALIDATION_ERROR（字段必填族）——
    let (status, v) = ship(&app, 2).await;
    assert!(
        status.is_client_error(),
        "染色布缺匹号发运必须 4xx（禁裸 500），实际 {status}: {v}"
    );
    assert_eq!(
        v["code"], "VALIDATION_ERROR",
        "字段必填/取值错误按本仓边界必须归 VALIDATION_ERROR，实际信封: {v}"
    );

    // 拒绝先于任何库存写动作：库存行与匹行必须零漂移
    let stock_before = inventory_stock::Entity::find_by_id(1)
        .one(&*db)
        .await
        .unwrap()
        .unwrap();
    assert_eq!(
        stock_before.quantity_available,
        dec("50.00"),
        "缺匹号拒绝不得动调出仓库存"
    );
    let piece_before = inventory_piece::Entity::find_by_id(1)
        .one(&*db)
        .await
        .unwrap()
        .unwrap();
    assert_eq!(
        piece_before.status,
        piece_status::AVAILABLE,
        "缺匹号拒绝不得消耗匹"
    );

    // —— ② 带真实匹号发运：200，匹级数据被真实扣减（CAS AVAILABLE→SHIPPED）——
    let (status, v) = ship(&app, 1).await;
    assert_eq!(status, StatusCode::OK, "四维齐全+真实可用匹应发运成功: {v}");
    assert_eq!(v["data"]["status"], transfer_status::SHIPPED);

    let piece = inventory_piece::Entity::find_by_id(1)
        .one(&*db)
        .await
        .unwrap()
        .unwrap();
    assert_eq!(
        piece.status,
        piece_status::SHIPPED,
        "匹级数据必须被真实扣减（发运事务内 CAS 落 SHIPPED）"
    );

    let stock = inventory_stock::Entity::find_by_id(1)
        .one(&*db)
        .await
        .unwrap()
        .unwrap();
    assert_eq!(stock.quantity_available, dec("40"), "调出仓可用量真实扣 10");
    assert_eq!(stock.quantity_on_hand, dec("40"), "在库量同步扣减");
    assert_eq!(stock.quantity_meters, dec("40.00"), "米数按四维行如实扣减");
    assert_eq!(
        stock.version,
        stock_before.version + 1,
        "乐观锁 version 必须随扣减递增"
    );

    let target = inventory_stock::Entity::find_by_id(2)
        .one(&*db)
        .await
        .unwrap()
        .unwrap();
    assert_eq!(target.quantity_incoming, dec("10"), "调入仓在途同步加 10");

    let txn_rows = inventory_transaction::Entity::find()
        .all(&*db)
        .await
        .unwrap();
    assert_eq!(
        txn_rows.len(),
        1,
        "成功发运应落且仅落一条 TRANSFER_OUT 流水"
    );
    assert_eq!(txn_rows[0].transaction_type, "TRANSFER_OUT");
}
