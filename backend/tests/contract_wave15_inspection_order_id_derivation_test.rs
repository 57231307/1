//! 采购质检单 order_id「自入库单派生」契约锁（连真库 PostgreSQL）。
//!
//! UI 建质检单只带 receipt_id（handleReceiptChange 仅回填 supplier_id），
//! 后端 `PurchaseInspectionService::create_inspection` 须从入库单派生 order_id
//! （`purchase_receipt.order_id` 是该列的权威来源），不得让派生字段依赖前端补传。
//!
//! 本锁在真库上钉死派生契约（`setup_test_db` + 真实回读）：
//! 1. 只给 receipt_id ⇒ order_id 从入库单派生并真实落库；
//! 2. 显式上送 order_id ⇒ 以调用方值为准，不被派生覆盖；
//! 3. 入库单 order_id 为 NULL ⇒ 质检单保持 NULL，不伪造订单；
//! 4. 无 receipt_id ⇒ 无派生源，order_id 保持调用方所给值。
//!
//! supplier_id 维持「必须由 req 提供、缺失即 400」的既有门。
//! FK 父行口径同 `contract_wave6_receipt_gate_test`：
//! suppliers id=1 为迁移种子参照表，users/warehouses 自种，departments id=1 恒在。

mod test_common;

use bingxi_backend::models::status::purchase_inventory::purchase_receipt as pr_status;
use bingxi_backend::models::status::purchase_order as po_status;
use bingxi_backend::models::{purchase_inspection, purchase_order, purchase_receipt};
use bingxi_backend::services::purchase_inspection_service::{
    CreatePurchaseInspectionRequest, PurchaseInspectionService,
};
use chrono::{NaiveDate, Utc};
use rust_decimal::Decimal;
use sea_orm::{
    ActiveModelTrait, ActiveValue::Set, ConnectionTrait, DatabaseConnection, DbBackend,
    EntityTrait, Statement,
};
use std::sync::Arc;
use test_common::setup_test_db;

fn date(y: i32, m: u32, d: u32) -> NaiveDate {
    NaiveDate::from_ymd_opt(y, m, d).unwrap()
}

fn unique_suffix() -> i64 {
    Utc::now().timestamp_nanos_opt().unwrap()
}

/// 原生执行种子 SQL（口径同 contract_wave11_concession_receiving_flow_test::exec）。
async fn exec(db: &DatabaseConnection, sql: &str) {
    db.execute_raw(Statement::from_sql_and_values(
        DbBackend::Postgres,
        sql,
        Vec::<sea_orm::Value>::new(),
    ))
    .await
    .unwrap_or_else(|e| panic!("种子执行失败: {e}\nSQL: {sql}"));
}

/// 已迁移真库 + 本域 FK 父行（users 1 / warehouses 1；suppliers 1、departments 1 为种子参照）。
async fn seeded_db() -> DatabaseConnection {
    let db = setup_test_db().await;
    // 用原生 INSERT 落固定 id 的父行，ON CONFLICT 兜幂等（setup_test_db 参照表不清空时可能已存在）。
    exec(
        &db,
        r#"INSERT INTO users (id,username,password_hash,is_active,is_totp_enabled,department_id,created_at,updated_at) VALUES
             (1,'w15_operator','x',TRUE,FALSE,1,'2026-01-01T00:00:00Z','2026-01-01T00:00:00Z')
           ON CONFLICT (id) DO NOTHING"#,
    )
    .await;
    exec(
        &db,
        "INSERT INTO warehouses (id, name, warehouse_code, is_active) VALUES (1, '波15派生主仓', 'W15-W1', true) ON CONFLICT (id) DO NOTHING",
    )
    .await;
    db
}

async fn seed_po(db: &DatabaseConnection) -> i32 {
    let now = Utc::now();
    let po = purchase_order::ActiveModel {
        order_no: Set(format!("PO-W15-{}", unique_suffix())),
        supplier_id: Set(1),
        order_date: Set(date(2026, 9, 1)),
        warehouse_id: Set(1),
        department_id: Set(1),
        purchaser_id: Set(1),
        currency: Set("CNY".to_string()),
        exchange_rate: Set(Decimal::ONE),
        total_amount: Set(Decimal::ZERO),
        total_amount_foreign: Set(Decimal::ZERO),
        total_quantity: Set(Decimal::ZERO),
        total_quantity_alt: Set(Decimal::ZERO),
        order_status: Set(po_status::APPROVED.to_string()),
        created_by: Set(1),
        created_at: Set(now),
        updated_at: Set(now),
        ..Default::default()
    }
    .insert(db)
    .await
    .expect("播种采购订单失败");
    po.id
}

async fn seed_receipt(db: &DatabaseConnection, order_id: Option<i32>) -> purchase_receipt::Model {
    let now = Utc::now();
    purchase_receipt::ActiveModel {
        receipt_no: Set(format!("GR-W15-{}", unique_suffix())),
        order_id: Set(order_id),
        supplier_id: Set(1),
        receipt_date: Set(date(2026, 9, 1)),
        warehouse_id: Set(1),
        inspection_status: Set(pr_status::DRAFT.to_string()),
        receipt_status: Set(pr_status::DRAFT.to_string()),
        total_quantity: Set(Decimal::ZERO),
        total_quantity_alt: Set(Decimal::ZERO),
        total_amount: Set(Decimal::ZERO),
        created_by: Set(1),
        created_at: Set(now),
        updated_at: Set(now),
        ..Default::default()
    }
    .insert(db)
    .await
    .expect("播种入库单失败")
}

/// 真实回读质检单（不是断调用成功，而是断落库后从库取回的值）。
async fn reread_inspection(db: &DatabaseConnection, id: i32) -> purchase_inspection::Model {
    purchase_inspection::Entity::find_by_id(id)
        .one(db)
        .await
        .expect("回读质检单失败")
        .expect("质检单必须存在")
}

fn svc(db: &DatabaseConnection) -> PurchaseInspectionService {
    PurchaseInspectionService::new(Arc::new(db.clone()))
}

/// 组装「UI 建单」形态请求：只带 receipt_id + supplier_id，order_id 缺席。
fn ui_like_req(
    receipt_id: Option<i32>,
    supplier_id: Option<i32>,
) -> CreatePurchaseInspectionRequest {
    CreatePurchaseInspectionRequest {
        receipt_id,
        order_id: None,
        supplier_id,
        inspection_date: Some(date(2026, 9, 1)),
        inspector_id: None,
        inspection_type: None,
        sample_size: None,
        notes: Some(format!("E2E-W15-{}", unique_suffix())),
    }
}

// =========================================================
// 锁1：只给 receipt_id ⇒ order_id 从入库单派生并真实落库可回读
// =========================================================

#[tokio::test]
async fn lock1_order_id_derived_from_receipt_when_absent_in_request() {
    let db = seeded_db().await;
    let po_id = seed_po(&db).await;
    let receipt = seed_receipt(&db, Some(po_id)).await;

    let created = svc(&db)
        .create_inspection(ui_like_req(Some(receipt.id), Some(1)), 1)
        .await
        .expect("建质检单应成功");

    assert_eq!(
        created.order_id,
        Some(po_id),
        "入库单携带 order_id 时，缺席请求体的质检单必须从入库单派生 order_id"
    );
    // 真实回读，排除内存值假绿
    let row = reread_inspection(&db, created.id).await;
    assert_eq!(
        row.order_id,
        Some(po_id),
        "派生的 order_id 必须真实落库（回读值逐字段相等）"
    );
    assert_eq!(
        row.receipt_id,
        Some(receipt.id),
        "receipt_id 按原样落库，不受派生逻辑影响"
    );
}

// =========================================================
// 锁2：显式上送 order_id ⇒ 以调用方值为准（不被派生覆盖）
// =========================================================

#[tokio::test]
async fn lock2_explicit_order_id_wins_over_receipt_derivation() {
    let db = seeded_db().await;
    let po_receipt = seed_po(&db).await; // 入库单实际所属订单
    let po_explicit = seed_po(&db).await; // 调用方显式指定的另一订单
    let receipt = seed_receipt(&db, Some(po_receipt)).await;

    let mut req = ui_like_req(Some(receipt.id), Some(1));
    req.order_id = Some(po_explicit);

    let created = svc(&db)
        .create_inspection(req, 1)
        .await
        .expect("显式 order_id 建单应成功");
    let row = reread_inspection(&db, created.id).await;
    assert_eq!(
        row.order_id,
        Some(po_explicit),
        "调用方显式上送 order_id 时必须以其为准，派生只在缺省时生效，不得覆盖明确意图"
    );
}

// =========================================================
// 锁3：入库单自身 order_id 为 NULL ⇒ 质检单保持 NULL（不伪造订单）
// =========================================================

#[tokio::test]
async fn lock3_null_order_id_on_receipt_does_not_fabricate_order() {
    let db = seeded_db().await;
    // 无源采购订单的入库单（order_id NULL）
    let receipt = seed_receipt(&db, None).await;

    let created = svc(&db)
        .create_inspection(ui_like_req(Some(receipt.id), Some(1)), 1)
        .await
        .expect("建质检单应成功");
    let row = reread_inspection(&db, created.id).await;
    assert_eq!(
        row.order_id, None,
        "入库单自身无 order_id 时不得凭空造出采购订单 ID（保持 NULL）"
    );
}

// =========================================================
// 锁4：无 receipt_id（独立建单）⇒ 无派生源，order_id 保持调用方所给（None）
// =========================================================

#[tokio::test]
async fn lock4_without_receipt_id_order_id_stays_as_provided() {
    let db = seeded_db().await;
    let created = svc(&db)
        .create_inspection(ui_like_req(None, Some(1)), 1)
        .await
        .expect("独立建质检单应成功");
    let row = reread_inspection(&db, created.id).await;
    assert_eq!(
        row.order_id, None,
        "无入库单来源时 order_id 不被派生，保持请求所给（此处 None）"
    );
}
