//! 采购收货「合格方可入库/结算」门控契约锁（wave6）
//!
//! 前置事实（commit 2fc3a7c1 已备好数据链）：采购质检完成已在同一事务回写
//! `purchase_receipt.inspection_status`（权威词表 models/status/purchase_inventory.rs
//! `purchase_receipt_inspection`：PENDING/PASSED/REJECTED，映射 pass→PASSED、
//! fail/partial→REJECTED，由 contract_wave5_inspection_result_authority_test 钉住）。
//! 本锁在其上落地用户裁定的门控：**质检合格方可确认入库/结算**。
//!
//! 门控落点（判定唯一实现 = PurchaseReceiptService::ensure_receipt_inspection_allows_flow，
//! 任何库存写入/状态推进/应付生成之前）：
//! - `services/purchase_receipt_ops/state.rs::confirm_receipt`——锁内事务时序由源码扫描锁①钉住：
//!   门控先于 update_order_received_quantity（PO 进度/到货日/状态）、update_inventory_txn
//!   （库存行+流水）与 COMPLETED 写入；被拒时事务零写入。
//! - `services/ap_invoice_ops/receipt.rs::find_receipt_and_check_exists`——结算入口与确认入库
//!   同一口径（词表语义 PASSED=允许后续入库/**结算**流转；auto_generate HTTP 端点可绕过
//!   confirm 直连，必须同门），复用同一函数、门控先于重复生成检查与任何应付写入。
//!
//! PENDING/NULL 裁定（证据全文见门控函数文档注释）：每一张收货单都必须先有质检结论回写。
//! - 生产 DDL（m0009）该列 `VARCHAR(20) NOT NULL DEFAULT 'PENDING'`，建单固定置 PENDING
//!   （build_receipt_active_model）——不存在"免检收货"第四态，NULL 在类型层（String 非
//!   Option<String>）不可达；
//! - 全仓无按品类/配置豁免质检的开关（免检/需质检/inspection_required/need_inspection
//!   检索零命中；化料 chemical_lot、验布 fabric_inspection 是另表另列，不影响本列语义）；
//! - 词表 PASSED 定义即「质检合格：允许后续入库/结算流转」。放行 PENDING（新建单的恒初值）
//!   等于门控形同虚设 ⇒ **PENDING 拒绝**，文案给出可行动路径（先完成质检并录入结论）。
//!   词表外坏值 fail-closed 走脱敏 business，绝不静默放行。
//! - **禁止"配置未开即放行"旁路**：源码扫描锁④钉死门控链路上不存在任何 config/feature/
//!   setting 判定分支。
//!
//! 覆盖策略（无 mock、真实 service 调用）：
//! - sqlite 同构：结算入口（auto_generate_from_receipt，链路无 lock_exclusive，sqlite 可承载）
//!   对 REJECTED/PENDING/词表外/ PASSED 四向判定与错误信封（400+BUSINESS_ERROR+真实文案；
//!   本 fixture **不建 ap_invoice 表**——若门控被移除/后移，请求将直达重复生成检查触发
//!   DATABASE_ERROR，PASSED 用例正是以此证明"合格数据不被门控误伤"）；
//! - sqlite 同构正向推进：真实调用 confirm_receipt 事务内的两个写入口
//!   （update_order_received_quantity + update_inventory_txn，先例
//!   contract_wave5_purchase_actual_delivery_writeback_test.rs），回读 PO 进度/库存行/流水
//!   真实落库（不只断调用成功）；
//! - `#[ignore]` 活库用例（TEST_DATABASE_URL→已迁移 PG，ci-test-rust-ignored 执行；缺变量时
//!   require_postgres **显式失败**，禁止条件跳过假绿）：完整 confirm 链路（含 lock_exclusive）
//!   PASSED 正向 COMPLETED+库存回读、REJECTED/PENDING 反向零漂移（状态未推进、库存行数与
//!   数量不变、无应付/流水落库）；
//! - 判定矩阵与文案逐字符锁在
//!   `services/purchase_receipt_service.rs::inspection_gate_tests` 内联单测（真实门控函数）。

mod test_common;

use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use bingxi_backend::models::status::purchase_inventory::purchase_receipt_inspection;
use bingxi_backend::models::status::{
    purchase_order as po_status, purchase_receipt as receipt_status,
};
use bingxi_backend::models::{
    ap_invoice, inventory_stock, inventory_transaction, purchase_order, purchase_order_item,
    purchase_receipt, purchase_receipt_item, user,
};
use bingxi_backend::services::ap_invoice_service::ApInvoiceService;
use bingxi_backend::services::purchase_receipt_service::PurchaseReceiptService;
use bingxi_backend::utils::error::AppError;
use chrono::{NaiveDate, Utc};
use rust_decimal::Decimal;
use sea_orm::{
    ActiveModelTrait, ColumnTrait, ConnectionTrait, DatabaseConnection, DbBackend, EntityTrait,
    PaginatorTrait, QueryFilter, Set, Statement, TransactionTrait,
};
use std::str::FromStr;
use std::sync::Arc;

fn dec(s: &str) -> Decimal {
    Decimal::from_str(s).unwrap()
}

fn date(y: i32, m: u32, d: u32) -> NaiveDate {
    NaiveDate::from_ymd_opt(y, m, d).unwrap()
}

fn unique_suffix() -> i64 {
    Utc::now().timestamp_nanos_opt().unwrap()
}

// =========================================================
// sqlite 同构表（列与 models/*.rs::Model 逐列对应；DDL 形态先例：
// purchase_receipt/purchase_receipt_item 取 contract_wave5_inspection_result_authority_test.rs
// 与 contract_wave5_purchase_actual_delivery_writeback_test.rs，inventory_stocks 取
// contract_wave3_explicit_null_clear_test.rs，inventory_transactions 按
// models/inventory_transaction.rs 全列）
// =========================================================

const TABLE_DDLS: &[&str] = &[
    r#"CREATE TABLE purchase_receipt (
        id INTEGER PRIMARY KEY,
        receipt_no TEXT NOT NULL UNIQUE,
        order_id INTEGER,
        supplier_id INTEGER NOT NULL,
        receipt_date TEXT NOT NULL,
        warehouse_id INTEGER NOT NULL,
        department_id INTEGER,
        receiver_id INTEGER,
        inspector_id INTEGER,
        inspection_status TEXT NOT NULL,
        receipt_status TEXT NOT NULL,
        total_quantity TEXT NOT NULL,
        total_quantity_alt TEXT NOT NULL,
        total_amount TEXT NOT NULL,
        notes TEXT,
        attachment_urls TEXT,
        created_by INTEGER NOT NULL,
        created_at TEXT NOT NULL,
        updated_by INTEGER,
        updated_at TEXT NOT NULL,
        confirmed_at TEXT,
        confirmed_by INTEGER
    )"#,
    r#"CREATE TABLE purchase_receipt_item (
        id INTEGER PRIMARY KEY,
        receipt_id INTEGER NOT NULL, order_item_id INTEGER,
        line_no INTEGER NOT NULL, product_id INTEGER NOT NULL,
        material_code TEXT NOT NULL, material_name TEXT NOT NULL,
        batch_no TEXT, color_code TEXT, lot_no TEXT, grade TEXT,
        gram_weight TEXT, width TEXT,
        quantity TEXT NOT NULL, quantity_alt TEXT,
        unit_master TEXT NOT NULL, unit_alt TEXT,
        unit_price TEXT, amount TEXT, location_code TEXT,
        piece_no TEXT, package_no TEXT, production_date TEXT,
        shelf_life INTEGER, notes TEXT, created_at TEXT,
        internal_dye_lot_id INTEGER, internal_dye_lot_no TEXT,
        internal_piece_ids TEXT, internal_piece_nos TEXT,
        supplier_dye_lot_no TEXT, supplier_piece_nos TEXT,
        batch_conversion_log_id INTEGER
    )"#,
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
    r#"CREATE TABLE inventory_stocks (
        id INTEGER PRIMARY KEY,
        warehouse_id INTEGER, product_id INTEGER,
        quantity_on_hand TEXT, quantity_available TEXT, quantity_reserved TEXT,
        quantity_shipped TEXT, quantity_incoming TEXT,
        reorder_point TEXT, max_stock_point TEXT, reorder_quantity TEXT,
        bin_location TEXT, last_count_date TEXT, last_movement_date TEXT,
        created_at TEXT NOT NULL, updated_at TEXT NOT NULL,
        batch_no TEXT, color_no TEXT, dye_lot_no TEXT, grade TEXT,
        production_date TEXT, expiry_date TEXT,
        quantity_meters TEXT, quantity_kg TEXT, gram_weight TEXT, width TEXT,
        location_id INTEGER, shelf_no TEXT, layer_no TEXT,
        stock_status TEXT, quality_status TEXT,
        version INTEGER, replenishment_strategy TEXT
    )"#,
    r#"CREATE TABLE inventory_transactions (
        id INTEGER PRIMARY KEY,
        transaction_type TEXT NOT NULL,
        product_id INTEGER NOT NULL, warehouse_id INTEGER NOT NULL,
        batch_no TEXT NOT NULL, color_no TEXT NOT NULL, dye_lot_no TEXT, grade TEXT NOT NULL,
        quantity_meters TEXT NOT NULL, quantity_kg TEXT NOT NULL,
        source_bill_type TEXT, source_bill_no TEXT, source_bill_id INTEGER,
        quantity_before_meters TEXT, quantity_before_kg TEXT,
        quantity_after_meters TEXT, quantity_after_kg TEXT,
        notes TEXT, created_by INTEGER, created_at TEXT NOT NULL
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

async fn exec(db: &DatabaseConnection, sql: &str) {
    db.execute_raw(Statement::from_sql_and_values(
        DbBackend::Sqlite,
        sql,
        Vec::<sea_orm::Value>::new(),
    ))
    .await
    .unwrap_or_else(|e| panic!("DDL 执行失败: {e}\nSQL: {sql}"));
}

async fn sqlite_db() -> DatabaseConnection {
    sea_orm::Database::connect("sqlite::memory:")
        .await
        .expect("sqlite::memory: 连接失败")
}

/// 刻意**只建除 ap_invoice 外**的表：门控若失位，auto_generate 会直达重复生成检查
/// 命中缺失的 ap_invoice 表报 DATABASE_ERROR——PASSED 用例正是以"错误不再是门控的
/// BUSINESS_ERROR"证明合格数据不被误伤（见测试 1）。
async fn sqlite_without_ap_invoice() -> DatabaseConnection {
    let db = sqlite_db().await;
    for ddl in TABLE_DDLS {
        exec(&db, ddl).await;
    }
    db
}

async fn seed_receipt(db: &DatabaseConnection, inspection_status: &str) -> purchase_receipt::Model {
    purchase_receipt::ActiveModel {
        receipt_no: Set(format!("GR-W6-{suffix}", suffix = unique_suffix())),
        supplier_id: Set(1),
        receipt_date: Set(date(2026, 9, 1)),
        warehouse_id: Set(1),
        // 词表同源：检验状态取权威常量，不手写第二套 token
        inspection_status: Set(inspection_status.to_string()),
        receipt_status: Set(receipt_status::DRAFT.to_string()),
        total_quantity: Set(Decimal::ZERO),
        total_quantity_alt: Set(Decimal::ZERO),
        total_amount: Set(Decimal::ZERO),
        created_by: Set(1),
        created_at: Set(Utc::now()),
        updated_at: Set(Utc::now()),
        ..Default::default()
    }
    .insert(db)
    .await
    .expect("播种入库单失败")
}

async fn seed_receipt_item(
    db: &DatabaseConnection,
    receipt_id: i32,
    order_item_id: Option<i32>,
    quantity: Decimal,
    batch_no: &str,
) -> purchase_receipt_item::Model {
    purchase_receipt_item::ActiveModel {
        receipt_id: Set(receipt_id),
        order_item_id: Set(order_item_id),
        line_no: Set(1),
        product_id: Set(1),
        material_code: Set("FAB-W6".to_string()),
        material_name: Set("门控契约测试坯布".to_string()),
        batch_no: Set(Some(batch_no.to_string())),
        quantity: Set(quantity),
        quantity_alt: Set(Some(dec("5.0000"))),
        unit_master: Set("米".to_string()),
        ..Default::default()
    }
    .insert(db)
    .await
    .expect("播种入库明细失败")
}

// =========================================================
// 测试 1（sqlite）：结算入口四向判定 = 门控同一口径（真实调用 auto_generate_from_receipt）
// =========================================================

#[tokio::test]
async fn ap_entry_applies_same_inspection_gate_on_sqlite() {
    let db = sqlite_without_ap_invoice().await;
    let svc = ApInvoiceService::new(Arc::new(db.clone()));

    // ① REJECTED → 400 + BUSINESS_ERROR + 真实外显文案（displayable，规则可外显、不含单号）
    let rejected = seed_receipt(&db, purchase_receipt_inspection::REJECTED).await;
    let err = svc
        .auto_generate_from_receipt(rejected.id, 1)
        .await
        .expect_err("质检不合格的收货单必须被结算门控拒绝");
    assert!(
        matches!(&err, AppError::BusinessErrorDisplayable(_)),
        "REJECTED 拒绝必须外显族，实际: {err:?}"
    );
    let body = err.to_response();
    assert_eq!(body.code, "BUSINESS_ERROR");
    assert_eq!(
        body.message, "质检不合格的收货单不能生成应付结算，请先处理不合格品",
        "结算入口拒绝文案必须与裁定句式逐字符一致（动作=生成应付结算）"
    );
    let resp: Response = err.into_response();
    assert_eq!(resp.status(), StatusCode::BAD_REQUEST);

    // ② PENDING（每单必经质检裁定 → 拒绝，含可行动路径）
    let pending = seed_receipt(&db, purchase_receipt_inspection::PENDING).await;
    let err = svc
        .auto_generate_from_receipt(pending.id, 1)
        .await
        .expect_err("质检未完成的收货单必须被结算门控拒绝");
    let body = err.to_response();
    assert_eq!(body.code, "BUSINESS_ERROR");
    assert!(
        body.message.contains("质检尚未完成") && body.message.contains("完成质检"),
        "PENDING 拒绝必须说明原因与行动路径，实际: {}",
        body.message
    );

    // ③ 词表外坏值（m0057 归一前的历史中文 token 形态）→ fail-closed 脱敏拒绝，不回显原值
    let bad = seed_receipt(&db, "待检").await;
    let err = svc
        .auto_generate_from_receipt(bad.id, 1)
        .await
        .expect_err("词表外检验状态必须 fail-closed 拒绝");
    assert!(
        matches!(&err, AppError::BusinessError(_)),
        "词表外必须走脱敏 business（数据完整性问题），实际: {err:?}"
    );
    let body = err.to_response();
    assert_eq!(body.code, "BUSINESS_ERROR");
    assert!(
        !body.message.contains("待检"),
        "脱敏出参不得回显坏数据原值，实际: {}",
        body.message
    );

    // ④ PASSED → 门控必须放行：若门控被移除/后移到重复生成检查之后，请求会因本 fixture
    //    刻意缺失的 ap_invoice 表报 DATABASE_ERROR——此处断"不是门控 BUSINESS_ERROR"，
    //    即合格数据没有被门控误伤（正向对照，防判错过严）
    let passed = seed_receipt(&db, purchase_receipt_inspection::PASSED).await;
    let err = svc
        .auto_generate_from_receipt(passed.id, 1)
        .await
        .expect_err("PASSED 之后的链路在缺表 fixture 上必然失败（用于证明门控已放行）");
    let body = err.to_response();
    assert_ne!(
        body.code, "BUSINESS_ERROR",
        "PASSED 收货单不得被门控拒绝（放行后应继续走后续链路），实际: {body:?}"
    );

    // 无痕：四条路径都没有推进任何收货单状态（被拒/放行后未落终态）
    for id in [rejected.id, pending.id, bad.id, passed.id] {
        let row = purchase_receipt::Entity::find_by_id(id)
            .one(&db)
            .await
            .unwrap()
            .expect("收货单行应存在");
        assert_eq!(
            row.receipt_status,
            receipt_status::DRAFT.to_string(),
            "结算入口的任何路径都不得改动收货单状态"
        );
        assert_eq!(row.confirmed_at, None);
    }
}

// =========================================================
// 测试 2（sqlite）：门控放行后的真实写入口链——PO 进度与库存/流水真实落库（回读断言）
// =========================================================

/// 真实调用 confirm_receipt 事务内的两个写入口（update_order_received_quantity +
/// update_inventory_txn，链路无 lock_exclusive，sqlite 可承载；confirm 对它们的调用
/// 顺序与"门控先于二者"由源码扫描锁①钉住；完整 confirm 链路由活库用例承担）
#[tokio::test]
async fn passed_receipt_advances_po_progress_and_stock_via_confirm_write_entries() {
    let db = sqlite_without_ap_invoice().await;
    let now = Utc::now();
    user::ActiveModel {
        id: Set(100),
        username: Set("w6_gate_tester".to_string()),
        password_hash: Set("x".to_string()),
        is_active: Set(true),
        is_totp_enabled: Set(false),
        created_at: Set(now),
        updated_at: Set(now),
        ..Default::default()
    }
    .insert(&db)
    .await
    .expect("播种操作人失败");

    purchase_order::ActiveModel {
        id: Set(1),
        order_no: Set("PO-W6-A".to_string()),
        supplier_id: Set(1),
        order_date: Set(date(2026, 3, 1)),
        warehouse_id: Set(1),
        department_id: Set(1),
        purchaser_id: Set(100),
        currency: Set("CNY".to_string()),
        exchange_rate: Set(Decimal::ONE),
        total_amount: Set(dec("100.00")),
        total_amount_foreign: Set(dec("100.00")),
        total_quantity: Set(dec("10.0000")),
        total_quantity_alt: Set(Decimal::ZERO),
        order_status: Set(po_status::APPROVED.to_string()),
        created_by: Set(100),
        created_at: Set(now),
        updated_at: Set(now),
        ..Default::default()
    }
    .insert(&db)
    .await
    .expect("播种采购订单失败");

    purchase_order_item::ActiveModel {
        id: Set(11),
        order_id: Set(1),
        line_no: Set(1),
        product_id: Set(1),
        quantity: Set(dec("10.0000")),
        quantity_alt: Set(Decimal::ZERO),
        unit_price: Set(Decimal::ONE),
        unit_price_foreign: Set(Decimal::ONE),
        discount_percent: Set(Decimal::ZERO),
        tax_percent: Set(Decimal::ZERO),
        subtotal: Set(dec("10.00")),
        tax_amount: Set(Decimal::ZERO),
        discount_amount: Set(Decimal::ZERO),
        total_amount: Set(dec("10.00")),
        received_quantity: Set(Decimal::ZERO),
        received_quantity_alt: Set(Decimal::ZERO),
        created_at: Set(now),
        updated_at: Set(now),
        ..Default::default()
    }
    .insert(&db)
    .await
    .expect("播种采购订单明细失败");

    let receipt = seed_receipt(&db, purchase_receipt_inspection::PASSED).await;
    let batch = format!("W6B{}", unique_suffix());
    let item = seed_receipt_item(&db, receipt.id, Some(11), dec("10.0000"), &batch).await;
    let receipt_no = receipt.receipt_no.clone();

    let svc = PurchaseReceiptService::new(Arc::new(db.clone()));
    let receipt_model = purchase_receipt::Entity::find_by_id(receipt.id)
        .one(&db)
        .await
        .unwrap()
        .expect("收货单应可回读");
    let txn = db.begin().await.unwrap();
    svc.update_order_received_quantity(1, receipt.id, receipt_model.receipt_date, &txn, 100)
        .await
        .expect("门控放行后的真实进度入口必须成功");
    let pending_events = svc
        .update_inventory_txn(&receipt_model, &txn)
        .await
        .expect("门控放行后的真实库存入口必须成功");
    assert_eq!(pending_events.len(), 1, "一行入库应产生一条待发布库存事件");
    drop(pending_events);
    txn.commit().await.unwrap();

    // 回读 PO 进度（不是只断调用成功）
    let oi = purchase_order_item::Entity::find_by_id(11)
        .one(&db)
        .await
        .unwrap()
        .unwrap();
    assert_eq!(oi.received_quantity, dec("10.0000"), "已收数量必须真实累加");
    let po = purchase_order::Entity::find_by_id(1)
        .one(&db)
        .await
        .unwrap()
        .unwrap();
    assert_eq!(
        po.order_status,
        po_status::COMPLETED.to_string(),
        "全部收货后订单状态必须真实推进"
    );

    // 回读库存行与流水
    let stock = inventory_stock::Entity::find()
        .filter(inventory_stock::Column::BatchNo.eq(&batch))
        .one(&db)
        .await
        .unwrap()
        .expect("合格收货必须落库存行");
    assert_eq!(stock.quantity_meters, dec("10.0000"));
    assert_eq!(stock.quantity_on_hand, dec("10.0000"));
    assert_eq!(stock.quantity_available, dec("10.0000"));
    let flow_count = inventory_transaction::Entity::find()
        .filter(inventory_transaction::Column::SourceBillNo.eq(&receipt_no))
        .count(&db)
        .await
        .unwrap();
    assert_eq!(
        flow_count, 1,
        "库存行变动必须有同口径流水（追溯不因门控改动而缺失）"
    );
    assert_eq!(item.quantity, dec("10.0000"), "明细数量回读（播种自证）");
}

// =========================================================
// 活库夹具：TEST_DATABASE_URL→已迁移 PG；缺变量时显式失败，禁止条件跳过
// =========================================================

async fn require_postgres(db: &DatabaseConnection) {
    let url = std::env::var("TEST_DATABASE_URL");
    assert!(
        matches!(&url, Ok(u) if u.starts_with("postgres")),
        "本用例覆盖 confirm_receipt 完整链路（lock_exclusive 为 sqlite 不支持的行锁方言），\
         必须跑在 TEST_DATABASE_URL 指向的已迁移 PostgreSQL 上（ci-test-rust-ignored 注入）；\
         本地直接跑需显式 export TEST_DATABASE_URL=postgres://...，禁止 sqlite 回退假绿。\
         当前 TEST_DATABASE_URL={url:?}"
    );
    assert_eq!(
        db.get_database_backend(),
        DbBackend::Postgres,
        "夹具解析出的后端不是 PostgreSQL，活库锁不可信（setup_test_db 无变量时静默回退 sqlite）"
    );
}

// =========================================================
// 测试 3（活库）：PASSED → confirm 成功，库存/PO 关联与状态真实推进（回读）
// =========================================================

#[tokio::test]
#[ignore = "需要 TEST_DATABASE_URL 已迁移 PostgreSQL（confirm_receipt 走 lock_exclusive + 黑名单表 + 审计；前置主数据 supplier/warehouse/product/user id=1 由 CI 种子提供）"]
async fn live_confirm_passed_completes_receipt_and_writes_stock() {
    let db = test_common::setup_test_db().await;
    require_postgres(&db).await;
    let batch = format!("W6LB{}", unique_suffix());
    let receipt = seed_receipt(&db, purchase_receipt_inspection::PASSED).await;
    let receipt_no = receipt.receipt_no.clone();
    let item = seed_receipt_item(&db, receipt.id, None, dec("10.0000"), &batch).await;

    let svc = PurchaseReceiptService::new(Arc::new(db.clone()));
    let done = svc
        .confirm_receipt(receipt.id, 1)
        .await
        .expect("质检 PASSED 的收货单 confirm 必须成功（门控只拦非合格单）");
    assert_eq!(done.receipt_status, receipt_status::COMPLETED.to_string());
    assert!(done.confirmed_at.is_some(), "确认时间必须真实写入");
    assert_eq!(done.confirmed_by, Some(1));

    let stock = inventory_stock::Entity::find()
        .filter(inventory_stock::Column::BatchNo.eq(&batch))
        .one(&db)
        .await
        .unwrap()
        .expect("PASSED 确认后库存行必须真实落库");
    assert_eq!(
        stock.quantity_meters,
        dec("10.0000"),
        "库存数量必须等于明细数量"
    );
    let flow_count = inventory_transaction::Entity::find()
        .filter(inventory_transaction::Column::SourceBillNo.eq(&receipt_no))
        .count(&db)
        .await
        .unwrap();
    assert_eq!(flow_count, 1, "库存流水必须同事务落库");

    // 清理（应付为 commit 后 best-effort 生成，按来源定位一并清理）
    let ap = ap_invoice::Entity::find()
        .filter(ap_invoice::Column::SourceType.eq("PURCHASE_RECEIPT"))
        .filter(ap_invoice::Column::SourceId.eq(receipt.id))
        .all(&db)
        .await
        .unwrap();
    for invoice in ap {
        ap_invoice::Entity::delete_by_id(invoice.id)
            .exec(&db)
            .await
            .expect("清理应付单失败");
    }
    inventory_transaction::Entity::delete_many()
        .filter(inventory_transaction::Column::SourceBillNo.eq(&receipt_no))
        .exec(&db)
        .await
        .expect("清理库存流水失败");
    inventory_stock::Entity::delete_many()
        .filter(inventory_stock::Column::BatchNo.eq(&batch))
        .exec(&db)
        .await
        .expect("清理库存行失败");
    purchase_receipt_item::Entity::delete_by_id(item.id)
        .exec(&db)
        .await
        .expect("清理入库明细失败");
    purchase_receipt::Entity::delete_by_id(receipt.id)
        .exec(&db)
        .await
        .expect("清理入库单失败");
}

// =========================================================
// 测试 4（活库）：REJECTED → 400+BUSINESS_ERROR+真实文案，且零漂移
// =========================================================

#[tokio::test]
#[ignore = "需要 TEST_DATABASE_URL 已迁移 PostgreSQL（confirm_receipt 走 lock_exclusive）"]
async fn live_confirm_rejected_is_refused_with_zero_drift() {
    let db = test_common::setup_test_db().await;
    require_postgres(&db).await;
    let batch = format!("W6LR{}", unique_suffix());
    let receipt = seed_receipt(&db, purchase_receipt_inspection::REJECTED).await;
    let receipt_no = receipt.receipt_no.clone();
    let item = seed_receipt_item(&db, receipt.id, None, dec("10.0000"), &batch).await;

    let svc = PurchaseReceiptService::new(Arc::new(db.clone()));
    let err = svc
        .confirm_receipt(receipt.id, 1)
        .await
        .expect_err("质检不合格的收货单绝对不能确认入库");
    let body = err.to_response();
    assert_eq!(body.code, "BUSINESS_ERROR");
    assert_eq!(
        body.message, "质检不合格的收货单不能确认入库，请先处理不合格品",
        "确认入库入口 REJECTED 文案必须与裁定原文逐字符一致"
    );
    let resp: Response = err.into_response();
    assert_eq!(resp.status(), StatusCode::BAD_REQUEST);

    // 零漂移回读：状态未推进、库存行数与数量不变、无流水/应付落库
    let row = purchase_receipt::Entity::find_by_id(receipt.id)
        .one(&db)
        .await
        .unwrap()
        .expect("收货单必须原样存在");
    assert_eq!(row.receipt_status, receipt_status::DRAFT.to_string());
    assert_eq!(
        row.inspection_status,
        purchase_receipt_inspection::REJECTED.to_string(),
        "门控拒绝不得改动检验状态（REJECTED 仍是事实）"
    );
    assert_eq!(row.confirmed_at, None);
    assert_eq!(row.total_quantity, dec("0"), "总额/总量列不得被半写");
    assert_eq!(
        inventory_stock::Entity::find()
            .filter(inventory_stock::Column::BatchNo.eq(&batch))
            .count(&db)
            .await
            .unwrap(),
        0,
        "被拒收货单不得新增任何库存行"
    );
    assert_eq!(
        inventory_transaction::Entity::find()
            .filter(inventory_transaction::Column::SourceBillNo.eq(&receipt_no))
            .count(&db)
            .await
            .unwrap(),
        0,
        "被拒收货单不得落任何库存流水"
    );
    assert_eq!(
        ap_invoice::Entity::find()
            .filter(ap_invoice::Column::SourceType.eq("PURCHASE_RECEIPT"))
            .filter(ap_invoice::Column::SourceId.eq(receipt.id))
            .count(&db)
            .await
            .unwrap(),
        0,
        "被拒收货单不得生成应付单/凭证"
    );

    purchase_receipt_item::Entity::delete_by_id(item.id)
        .exec(&db)
        .await
        .expect("清理入库明细失败");
    purchase_receipt::Entity::delete_by_id(receipt.id)
        .exec(&db)
        .await
        .expect("清理入库单失败");
}

// =========================================================
// 测试 5（活库）：PENDING（每单必经质检裁定 → 拒绝）
// =========================================================

#[tokio::test]
#[ignore = "需要 TEST_DATABASE_URL 已迁移 PostgreSQL（confirm_receipt 走 lock_exclusive）"]
async fn live_confirm_pending_is_refused_and_stays_draft() {
    let db = test_common::setup_test_db().await;
    require_postgres(&db).await;
    let batch = format!("W6LP{}", unique_suffix());
    let receipt = seed_receipt(&db, purchase_receipt_inspection::PENDING).await;
    let item = seed_receipt_item(&db, receipt.id, None, dec("10.0000"), &batch).await;

    let svc = PurchaseReceiptService::new(Arc::new(db.clone()));
    let err = svc
        .confirm_receipt(receipt.id, 1)
        .await
        .expect_err("质检未完成（PENDING）的收货单必须被拒绝——放行 PENDING 等于门控形同虚设");
    let body = err.to_response();
    assert_eq!(body.code, "BUSINESS_ERROR");
    assert!(
        body.message.contains("质检尚未完成") && body.message.contains("请先完成质检并录入结论"),
        "PENDING 拒绝必须外显原因与可行动路径，实际: {}",
        body.message
    );

    let row = purchase_receipt::Entity::find_by_id(receipt.id)
        .one(&db)
        .await
        .unwrap()
        .expect("收货单必须原样存在");
    assert_eq!(row.receipt_status, receipt_status::DRAFT.to_string());
    assert_eq!(
        inventory_stock::Entity::find()
            .filter(inventory_stock::Column::BatchNo.eq(&batch))
            .count(&db)
            .await
            .unwrap(),
        0,
        "被拒收货单不得新增库存行"
    );

    purchase_receipt_item::Entity::delete_by_id(item.id)
        .exec(&db)
        .await
        .expect("清理入库明细失败");
    purchase_receipt::Entity::delete_by_id(receipt.id)
        .exec(&db)
        .await
        .expect("清理入库单失败");
}

// =========================================================
// 源码扫描防回潮锁
// =========================================================

/// 从源码截取一个 impl 内方法块：anchor 起，到首个以 4 空格缩进的 `}` 行
/// （先例 contract_wave5 同式；先剔 \r 防 Windows 工作树 CRLF 漏检）
fn extract_method(src: &str, anchor: &str) -> String {
    let src = src.replace('\r', "");
    let i = src
        .find(anchor)
        .unwrap_or_else(|| panic!("源码锚点丢失: {anchor}"));
    let j = src[i..]
        .find("\n    }")
        .unwrap_or_else(|| panic!("方法块结束定位失败: {anchor}"));
    src[i..i + j + 6].to_string()
}

/// 锁①：confirm_receipt 内门控必须先于 PO 进度、库存写入与 COMPLETED 状态写入，
/// 且既有 DRAFT 状态门/明细数门原样共存（语义不变）
#[test]
fn source_scan_confirm_gate_before_all_writes_and_state_machine_intact() {
    let src = include_str!("../src/services/purchase_receipt_ops/state.rs").replace('\r', "");
    let block = extract_method(&src, "pub async fn confirm_receipt");
    let gate = block
        .find("Self::ensure_receipt_inspection_allows_flow(&receipt, \"确认入库\")?;")
        .expect("confirm_receipt 必须调用唯一门控入口并以 ? 整单拒绝");
    let order_call = block
        .find("self.update_order_received_quantity(")
        .expect("confirm 必须调用 PO 进度入口");
    let stock_call = block
        .find("self.update_inventory_txn(")
        .expect("confirm 必须调用库存写入入口");
    let completed = block
        .find("build_completed_receipt_active_model")
        .expect("confirm 必须写 COMPLETED 终态");
    let commit = block
        .find("txn.commit().await?")
        .expect("confirm 必须显式提交");
    assert!(
        gate < order_call && gate < stock_call && gate < completed && gate < commit,
        "质检门控必须先于任何库存写入/进度推进/状态推进/提交:\n{block}"
    );

    // 既有状态机门共存且语义未变（DRAFT 判定 + 明细数门不得被动过）
    let lock_block = extract_method(&src, "async fn lock_and_validate_receipt_txn");
    assert!(
        lock_block.contains("receipt.receipt_status != status::purchase_receipt::DRAFT")
            && lock_block.contains("lock_exclusive()"),
        "原 DRAFT 状态门（含行锁）不得被改动:\n{lock_block}"
    );
    assert!(
        lock_block.contains("item_count == 0"),
        "明细数门不得被改动:\n{lock_block}"
    );

    // 门控判定只此一处：state.rs 不得出现第二套 inspection_status 比较
    assert!(
        !src.contains("inspection_status ==") && !src.contains("inspection_status !="),
        "禁止在 state.rs 另写第二套检验状态比较（判定唯一实现见 facade）"
    );
}

/// 锁②：判定唯一实现只按权威词表常量比较、只有一条放行分支（PASSED），
/// 且门控链路上**不存在任何配置开关旁路**（"配置未开即放行"分支必须不存在）
#[test]
fn source_scan_gate_compares_vocabularies_only_and_has_no_config_bypass() {
    let src = include_str!("../src/services/purchase_receipt_service.rs").replace('\r', "");
    let block = extract_method(&src, "pub(crate) fn ensure_receipt_inspection_allows_flow");

    for token_const in [
        "status::purchase_receipt_inspection::PASSED",
        "status::purchase_receipt_inspection::REJECTED",
        "status::purchase_receipt_inspection::PENDING",
    ] {
        assert!(
            block.contains(token_const),
            "门控必须逐字符引用权威词表常量 {token_const}，不得手写第二套 token:\n{block}"
        );
    }
    for literal in ["\"PASSED\"", "\"REJECTED\"", "\"PENDING\""] {
        assert!(
            !block.contains(literal),
            "门控代码（非注释区之外）不得出现词表字符串字面量 {literal}"
        );
    }
    // 放行分支只有一条：Ok(()) 唯一出现在词表常量 PASSED 判定之后
    assert_eq!(
        block.matches("return Ok(())").count(),
        1,
        "放行分支必须唯一（PASSED）:\n{block}"
    );
    let passed_pos = block
        .find("status::purchase_receipt_inspection::PASSED")
        .unwrap();
    let ok_pos = block.find("return Ok(())").unwrap();
    assert!(passed_pos < ok_pos, "唯一放行必须位于 PASSED 判定分支内");

    // 不存在"配置未开即放行"旁路：门控函数与 confirm 事务模块全文不得出现
    // config/feature/setting 形态的开关判定（if config.xxx { return Ok(()) } 式跳过）
    let state_src = include_str!("../src/services/purchase_receipt_ops/state.rs").replace('\r', "");
    for (name, s) in [("gate", &block), ("state.rs", &state_src)] {
        let lowered = s.to_lowercase();
        for forbidden in ["config", "feature_flag", "settings", "开关"] {
            assert!(
                !lowered.contains(forbidden),
                "{name} 出现配置开关标识 {forbidden:?}——门控不允许任何'配置未开即放行'旁路"
            );
        }
    }
    // 拒绝路径不得被吞错兜底
    assert!(
        !block.contains("let _ =") && !block.contains(".ok();"),
        "门控拒绝禁止吞错兜底:\n{block}"
    );
}

/// 锁③：结算入口复用同一判定（不重复实现），且门控先于重复生成检查与任何应付写入
#[test]
fn source_scan_ap_entry_reuses_gate_before_any_ap_access() {
    let src = include_str!("../src/services/ap_invoice_ops/receipt.rs").replace('\r', "");
    let block = extract_method(&src, "async fn find_receipt_and_check_exists");
    let gate = block
        .find("ensure_receipt_inspection_allows_flow(")
        .expect("结算入口必须复用同一门控判定，禁止缺失或另写第二套");
    assert!(
        block.contains("\"生成应付结算\""),
        "结算入口必须携带自身动作口径:\n{block}"
    );
    let dup_check = block
        .find("ap_invoice::Entity::find()")
        .expect("重复生成检查必须存在");
    assert!(
        gate < dup_check,
        "门控必须先于重复生成检查与任何应付表访问（被拒时零应付写入）:\n{block}"
    );
    assert!(
        !src.contains("inspection_status =="),
        "禁止在应付模块另写第二套检验状态比较"
    );
}

/// 锁④：词表常量与裁定逐字符同源（本列合法取值全集只有三态；NULL 在类型层不可达）
#[test]
fn receipt_inspection_vocabulary_and_null_unreachability_are_locked() {
    assert_eq!(
        purchase_receipt_inspection::ALL,
        &["PENDING", "PASSED", "REJECTED"],
        "入库单检验状态词表（写入方 models/status/purchase_inventory.rs）大写三态，\
         不存在第四态/免检 token——每单必经质检裁定的词表级证据"
    );
    assert_eq!(purchase_receipt_inspection::PASSED, "PASSED");
    // 生产 DDL（migration m0009）该列 NOT NULL DEFAULT 'PENDING'：NULL 形态不可达的实证锁
    let mig = include_str!("../migration/src/domain/business/m0009_add_purchase_extensions.rs")
        .replace('\r', "");
    let col = mig
        .lines()
        .find(|l| l.contains("\"inspection_status\"") && l.contains("NOT NULL DEFAULT 'PENDING'"))
        .expect("purchase_receipt.inspection_status 的 NOT NULL DEFAULT 'PENDING' DDL 必须存在");
    assert!(
        !col.contains("NULL,") && col.contains("NOT NULL"),
        "该列必须保持 NOT NULL（NULL 不可达是门控 PENDING 裁定的前提），实际: {col}"
    );
}
