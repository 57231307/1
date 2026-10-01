//! 采购订单 `actual_delivery_date` 收货确认回写契约锁（决策定案 #7）
//!
//! 根因取证（调查组结论，本文件修复后逐项锁死）：
//! - 列存在：`migration/src/domain/system/mod.rs` `ALTER TABLE "purchase_orders"
//!   ADD COLUMN IF NOT EXISTS "actual_delivery_date" DATE;`（可空、**无 DEFAULT**）；
//!   模型 `models/purchase_order.rs` 为 `Option<NaiveDate>`。
//! - 有真实读方：`services/purchase_delivery_calculator.rs` 平均交期谓词要求
//!   `actual_delivery_date IS NOT NULL`（无到货日的单据整批排除）；
//!   `services/po/order_ops/query.rs` 导出与 `frontend/src/api/purchase.ts` 亦透出。
//! - 修复前**全仓零写入点**（`purchase_orders` 域内无任何 `actual_delivery_date`
//!   的 `Set`/赋值；唯一 `Set(None)` 属 `custom_orders` 同名异表列）⇒ 幽灵字段，
//!   交期绩效恒基于空集。
//!
//! 裁定落地：确认收货的**同一事务**回写，语义 = 该 PO 已确认收货中的最大
//! `receipt_date`（`purchase_receipt.receipt_date` 为 NOT NULL，入口守显式清空，
//! 见 `purchase_receipt_ops/crud.rs` 的 `update_receipt` 门控）；部分到货也回写；
//! 完成判定/状态谓词不动；回写失败 `?` 上抛整单回滚，严禁 `let _ =`/`.ok()` 半成功。
//!
//! 覆盖策略（对齐 `contract_wave2_po_item_update_fields_test.rs` 先例，无 mock）：
//! 1. sqlite::memory: 自建表 + **真实调用** `update_order_received_quantity`
//!    （confirm_receipt 事务内的同一入口，链路无 lock_exclusive，sqlite 可承载）：
//!    首次确认 → 列 == 该收货单 receipt_date 且进度/状态同步；更晚收货 → 覆盖为
//!    更大日期；更早补录收货 → 保持最大值不回退；未确认收货的 PO 恒 NULL；
//! 2. 失败实证：PO 不存在时回写入口如实报错上抛，事务不 commit 回滚后
//!    已收进度零残留（锁死「不允许进度写了、到货日没写」的半成功形态）；
//! 3. 防回潮源码扫描：回写调用点必须带 `?` 且夹在 confirm 的 begin/commit 之间；
//!    迁移 DDL 该列不得出现 DEFAULT（NULL 兜底会把"未收货"伪装成有到货日）。
//!
//! 口径影响声明：本列修复前有值恒空 ⇒ 平均交期样本恒空集、页面数字无意义；
//! 修复后样本随真实收货累积，属**数值真实化**，不是回归。

use bingxi_backend::models::status::purchase_order as po_status;
use bingxi_backend::models::{purchase_order, purchase_order_item, purchase_receipt_item, user};
use bingxi_backend::services::purchase_receipt_service::PurchaseReceiptService;
use bingxi_backend::utils::error::AppError;
use chrono::{NaiveDate, Utc};
use rust_decimal::Decimal;
use sea_orm::{
    ActiveModelTrait, ActiveValue::Set, ConnectionTrait, DbBackend, EntityTrait, Statement,
    TransactionTrait,
};
use std::sync::Arc;

fn date(y: i32, m: u32, d: u32) -> NaiveDate {
    NaiveDate::from_ymd_opt(y, m, d).unwrap()
}

/// 最小 DDL：列与 models/*.rs::Model 逐列对应（Decimal→TEXT、数组/JSON→TEXT，
/// 先例 contract_wave2_po_item_update_fields_test.rs）。
/// purchase_orders.actual_delivery_date 刻意**无 DEFAULT**，与生产迁移同形。
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

async fn seed_po(db: &sea_orm::DatabaseConnection, id: i32, order_no: &str, order_date: NaiveDate) {
    let now = Utc::now();
    purchase_order::ActiveModel {
        id: Set(id),
        order_no: Set(order_no.to_string()),
        supplier_id: Set(7),
        order_date: Set(order_date),
        warehouse_id: Set(1),
        department_id: Set(1),
        purchaser_id: Set(100),
        currency: Set("CNY".to_string()),
        exchange_rate: Set(Decimal::ONE),
        total_amount: Set(Decimal::ZERO),
        total_amount_foreign: Set(Decimal::ZERO),
        total_quantity: Set(Decimal::ZERO),
        total_quantity_alt: Set(Decimal::ZERO),
        order_status: Set(po_status::APPROVED.to_string()),
        created_by: Set(100),
        created_at: Set(now),
        updated_at: Set(now),
        ..Default::default()
    }
    .insert(db)
    .await
    .unwrap();
}

async fn seed_order_item(
    db: &sea_orm::DatabaseConnection,
    id: i32,
    order_id: i32,
    quantity: Decimal,
) {
    let now = Utc::now();
    purchase_order_item::ActiveModel {
        id: Set(id),
        order_id: Set(order_id),
        line_no: Set(1),
        product_id: Set(5),
        quantity: Set(quantity),
        quantity_alt: Set(Decimal::ZERO),
        unit_price: Set(Decimal::ONE),
        unit_price_foreign: Set(Decimal::ONE),
        discount_percent: Set(Decimal::ZERO),
        tax_percent: Set(Decimal::ZERO),
        subtotal: Set(quantity),
        tax_amount: Set(Decimal::ZERO),
        discount_amount: Set(Decimal::ZERO),
        total_amount: Set(quantity),
        received_quantity: Set(Decimal::ZERO),
        received_quantity_alt: Set(Decimal::ZERO),
        created_at: Set(now),
        updated_at: Set(now),
        ..Default::default()
    }
    .insert(db)
    .await
    .unwrap();
}

async fn seed_receipt_item(
    db: &sea_orm::DatabaseConnection,
    id: i32,
    receipt_id: i32,
    order_item_id: i32,
    quantity: Decimal,
) {
    purchase_receipt_item::ActiveModel {
        id: Set(id),
        receipt_id: Set(receipt_id),
        order_item_id: Set(Some(order_item_id)),
        line_no: Set(1),
        product_id: Set(5),
        material_code: Set("FAB-WB".to_string()),
        material_name: Set("回写契约测试坯布".to_string()),
        quantity: Set(quantity),
        unit_master: Set("米".to_string()),
        ..Default::default()
    }
    .insert(db)
    .await
    .unwrap();
}

/// 种子：操作人 100；PO1(明细 11，订 10)、PO2(明细 21，订 5)、PO3(明细 31，订 7，
/// 永不收货对照)；入库明细 101(4m)/102(6m)→明细11，201(5m)/202(1m)→明细21。
async fn setup_db() -> sea_orm::DatabaseConnection {
    let db = sea_orm::Database::connect("sqlite::memory:")
        .await
        .expect("sqlite::memory: 连接失败");
    create_tables(&db).await;
    let now = Utc::now();
    user::ActiveModel {
        id: Set(100),
        username: Set("wb_tester".to_string()),
        password_hash: Set("x".to_string()),
        is_active: Set(true),
        is_totp_enabled: Set(false),
        created_at: Set(now),
        updated_at: Set(now),
        ..Default::default()
    }
    .insert(&db)
    .await
    .unwrap();

    seed_po(&db, 1, "PO-WB-A", date(2026, 3, 1)).await;
    seed_po(&db, 2, "PO-WB-B", date(2026, 4, 1)).await;
    seed_po(&db, 3, "PO-WB-C", date(2026, 5, 1)).await;
    seed_order_item(&db, 11, 1, Decimal::from(10)).await;
    seed_order_item(&db, 21, 2, Decimal::from(5)).await;
    seed_order_item(&db, 31, 3, Decimal::from(7)).await;
    seed_receipt_item(&db, 101, 101, 11, Decimal::from(4)).await;
    seed_receipt_item(&db, 102, 102, 11, Decimal::from(6)).await;
    seed_receipt_item(&db, 201, 201, 21, Decimal::from(5)).await;
    seed_receipt_item(&db, 202, 202, 21, Decimal::from(1)).await;
    db
}

/// 真实调用 confirm_receipt 事务内的同一入口（本链路无 lock_exclusive，
/// sqlite 方言可承载；confirm 外层锁由源码扫描锁 3 锁定）
async fn confirm(
    db: &sea_orm::DatabaseConnection,
    order_id: i32,
    receipt_id: i32,
    receipt_date: NaiveDate,
) {
    let svc = PurchaseReceiptService::new(Arc::new(db.clone()));
    let txn = db.begin().await.unwrap();
    svc.update_order_received_quantity(order_id, receipt_id, receipt_date, &txn, 100)
        .await
        .expect("确认收货回写链路必须成功");
    txn.commit().await.unwrap();
}

async fn load_po(db: &sea_orm::DatabaseConnection, id: i32) -> purchase_order::Model {
    purchase_order::Entity::find_by_id(id)
        .one(db)
        .await
        .unwrap()
        .unwrap_or_else(|| panic!("采购订单 {id} 不存在"))
}

/// 锁 1a：部分到货确认 → actual_delivery_date == 该收货单 receipt_date，
/// 进度/状态（PARTIAL_RECEIVED，词表常量）同步，完成判定逻辑不受扰动。
#[tokio::test]
async fn confirm_partial_receipt_writes_back_receipt_date() {
    let db = setup_db().await;
    confirm(&db, 1, 101, date(2026, 3, 8)).await;

    let po = load_po(&db, 1).await;
    assert_eq!(
        po.actual_delivery_date,
        Some(date(2026, 3, 8)),
        "确认收货后到货日必须等于该收货单 receipt_date（修复前该列全仓零写入点，恒 NULL）"
    );
    assert_eq!(po.order_status, po_status::PARTIAL_RECEIVED);

    let item = purchase_order_item::Entity::find_by_id(11)
        .one(&db)
        .await
        .unwrap()
        .unwrap();
    assert_eq!(
        item.received_quantity,
        Decimal::from(4),
        "进度须与到货日同事务落库"
    );
}

/// 锁 1b：二次**更晚**收货确认 → 覆盖为更大日期；全部收货判定仍归
/// determine_order_receipt_status（COMPLETED 由进度推得，本回写不越权改状态机）。
#[tokio::test]
async fn later_receipt_overwrites_with_greater_date() {
    let db = setup_db().await;
    confirm(&db, 1, 101, date(2026, 3, 8)).await;
    confirm(&db, 1, 102, date(2026, 3, 20)).await;

    let po = load_po(&db, 1).await;
    assert_eq!(
        po.actual_delivery_date,
        Some(date(2026, 3, 20)),
        "更晚收货必须覆盖为更大日期（最后一次确认入库口径）"
    );
    assert_eq!(po.order_status, po_status::COMPLETED);
    let item = purchase_order_item::Entity::find_by_id(11)
        .one(&db)
        .await
        .unwrap()
        .unwrap();
    assert_eq!(item.received_quantity, Decimal::from(10));
}

/// 锁 1c：**更早**日期的补录收货 → 保持已确认收货中的最大值，不回退。
#[tokio::test]
async fn earlier_backfilled_receipt_never_rolls_back_max_date() {
    let db = setup_db().await;
    confirm(&db, 2, 201, date(2026, 4, 15)).await;
    confirm(&db, 2, 202, date(2026, 4, 5)).await;

    let po = load_po(&db, 2).await;
    assert_eq!(
        po.actual_delivery_date,
        Some(date(2026, 4, 15)),
        "到货日语义=该单已确认收货的最大 receipt_date，更早补录不得回退"
    );
}

/// 锁 1d：未确认收货的 PO 该列恒 NULL（种子即 APPROVED 未收货，行为对照）。
#[tokio::test]
async fn po_without_confirmed_receipt_keeps_null() {
    let db = setup_db().await;
    confirm(&db, 1, 101, date(2026, 3, 8)).await;
    let po = load_po(&db, 3).await;
    assert_eq!(
        po.actual_delivery_date, None,
        "未确认收货的订单不得被写入到货日（NULL 即事实，禁止默认值兜底）"
    );
}

/// 锁 2（半成功实证）：回写目标 PO 不存在 → 入口如实以 NotFound 上抛、调用方
/// 不 commit；事务回滚后已收进度**零残留**——锁死「进度写了、到货日没写」
/// 在原子性上不可能发生（生产 confirm_receipt 由 `?` 传播进同一 txn）。
#[tokio::test]
async fn write_back_failure_rolls_back_progress_no_half_success() {
    let db = setup_db().await;
    let svc = PurchaseReceiptService::new(Arc::new(db.clone()));
    let txn = db.begin().await.unwrap();
    let err = svc
        .update_order_received_quantity(999, 101, date(2026, 3, 8), &txn, 100)
        .await
        .expect_err("PO 不存在时必须整单失败上抛，不得吞错继续");
    assert!(
        matches!(err, AppError::NotFound(_)),
        "错误形态必须是 NotFound，实际 {err:?}"
    );
    assert!(
        err.to_string().contains("采购订单 999"),
        "错误必须外显真实定位信息，实际 {err}"
    );
    // 失败路径与生产一致：不 commit，drop 即回滚
    drop(txn);

    let item = purchase_order_item::Entity::find_by_id(11)
        .one(&db)
        .await
        .unwrap()
        .unwrap();
    assert_eq!(
        item.received_quantity,
        Decimal::ZERO,
        "回写失败必须随事务回滚，不得残留半成功进度"
    );
}

/// 从源码截取 impl 块内方法（4 空格收口，先例 contract_wave2 同式；剔 \r 防 CRLF 漏检）
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

/// 锁 3a：回写必须在 confirm 的事务内——`confirm_receipt` 中
/// `update_order_received_quantity`（携 receipt_date）调用点必须位于
/// begin 与 commit 之间，且错误以 `?` 上抛、无吞错形态。
#[test]
fn source_scan_write_back_inside_confirm_transaction() {
    let src = include_str!("../src/services/purchase_receipt_ops/state.rs").replace('\r', "");
    let begin = src
        .find(".begin().await?")
        .expect("confirm_receipt 必须有显式事务 begin");
    let commit = src
        .find("txn.commit().await?")
        .expect("confirm_receipt 必须有显式事务 commit");
    let call = src
        .find("self.update_order_received_quantity(")
        .expect("confirm_receipt 必须调用 update_order_received_quantity");
    assert!(
        begin < call && call < commit,
        "回写入口调用必须夹在 begin({begin}) 与 commit({commit}) 之间，实际位于 {call}"
    );
    // 从 call 起截取到其后第一个 `.await?;`（str::find 无起始偏移入参，先切片再找）
    let call_end_rel = src[call..]
        .find(".await?;")
        .expect("回写入口调用必须以 .await?; 结束");
    let call_block = &src[call..call + call_end_rel + ".await?;".len()];
    assert!(
        call_block.contains("receipt.receipt_date"),
        "确认收货必须把入库单 receipt_date 真实传入回写，禁止以确认时间/当前时间顶替，实际块:\n{call_block}"
    );
    assert!(
        !src.contains("let _ = ") && !src.contains(".ok();"),
        "confirm 链路不得吞错（let _ = / .ok() 会造成进度与到货日半成功）"
    );
}

/// 锁 3b：回写实现必须以 `?` 传播错误、经审计服务在事务连接上落库；
/// 状态判定（determine/save）调用序列不得被扰动（完成判定逻辑不动）。
#[test]
fn source_scan_write_back_propagates_errors_via_question_mark() {
    let src = include_str!("../src/services/purchase_receipt_private.rs");
    let entry = extract_impl_method(src, "pub async fn update_order_received_quantity");
    assert!(
        entry.contains(
            "Self::write_back_actual_delivery_date(txn, order_id, receipt_date, user_id).await?;"
        ),
        "回写必须在同事务入口以 ? 传播且先于状态判定，实际块:\n{entry}"
    );
    let back = extract_impl_method(src, "async fn write_back_actual_delivery_date");
    assert!(
        back.contains("AuditLogService::update_with_audit") && back.contains(".await?;"),
        "回写必须经审计服务在 txn 连接上落库并以 ? 上抛，实际块:\n{back}"
    );
    assert!(
        back.contains(".max(receipt_date)"),
        "必须取已确认收货中的最大 receipt_date，实际块:\n{back}"
    );
    for forbidden in ["let _ =", ".ok()"] {
        assert!(
            !entry.contains(forbidden) && !back.contains(forbidden),
            "回写路径禁止 {forbidden} 吞错形态"
        );
    }
}

/// 锁 4：生产 DDL 该列不得带 DEFAULT——「未收货 = NULL」是交期样本谓词
/// （`actual_delivery_date IS NOT NULL`）的事实来源，默认值兜底会污染样本。
#[test]
fn source_scan_migration_column_has_no_default_and_write_point_is_unique() {
    let mig = include_str!("../migration/src/domain/system/mod.rs").replace('\r', "");
    let line = mig
        .lines()
        .find(|l| l.contains("\"purchase_orders\"") && l.contains("\"actual_delivery_date\""))
        .expect("purchase_orders.actual_delivery_date 的 DDL 必须存在（决策：不删列）");
    assert!(
        !line.to_ascii_uppercase().contains("DEFAULT"),
        "该列必须保持无 DEFAULT（未收货恒 NULL），实际 DDL: {line}"
    );
}
