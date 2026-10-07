//! 出库四维（缸/色/批/匹）真实 PostgreSQL 契约锁：调拨发运消耗染色匹
//!
//! 表结构唯一来源 = backend/migration：本文件不再自建任何 DDL，
//! 全部读写打真实迁移表（正确的 DECIMAL/CHECK/触发器/索引由迁移提供）。
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
//! 第四维口径（当前权威，`services/inv/fabric_class.rs` 与 `models/status/*` 核对）：
//! 出库四维 = 缸号(dye_lot_no) / 色号(color_no) / 批次(batch_no) / 匹号(piece_no)，
//! 款号由 product_id 承载；幅宽(width)/旧四维写法不参与四维判定。
//!
//! 脚手架：`test_common::setup_test_db()`（已迁移 PG + TRUNCATE RESTART IDENTITY，
//! 显式 id 断言稳定）+ build_app + 注入 auth + 真发 HTTP 请求；
//! 行状态一律回读真库比对。状态字面量全部引自 models::status 词表，不手写第二套。

mod test_common;

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
use bingxi_backend::models::inventory_transfer;
use bingxi_backend::models::inventory_transfer_item;
use bingxi_backend::models::product;
use bingxi_backend::models::status::inventory_transfer as transfer_status;
use bingxi_backend::models::status::purchase_inventory::inventory_piece as piece_status;
use bingxi_backend::models::warehouse;
use sea_orm::{ActiveModelTrait, ActiveValue::Set, EntityTrait};
use serde_json::Value;
use std::str::FromStr;
use std::sync::Arc;
use tower::ServiceExt;

fn dec(s: &str) -> rust_decimal::Decimal {
    rust_decimal::Decimal::from_str(s).unwrap()
}

fn now() -> chrono::DateTime<chrono::Utc> {
    chrono::Utc::now()
}

// ---------------------------------------------------------------------------
// 脚手架
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

/// 种子布局（真表逐列按 models/*.rs + backend/migration 迁移核对；夹具 TRUNCATE 后
/// 显式 id 稳定）：
/// - 调拨单 1（approved，item piece_no='P-9' 指向真实可用染色匹）→ 发运成功组；
/// - 调拨单 2（approved，item piece_no IS NULL 的染色布存量行）→ 缺匹号拒绝组；
/// - 调出仓(1)库存 50、调入仓(2)库存 0；染色匹 P-9 AVAILABLE。
/// FK 父行自种子：warehouses(1,2) / products(5)。
async fn seeded_db() -> Arc<sea_orm::DatabaseConnection> {
    let db = Arc::new(test_common::setup_test_db().await);
    for (wid, name, code) in [
        (1i32, "胚布成品调出仓", "WH-P4D-1"),
        (2i32, "分仓", "WH-P4D-2"),
    ] {
        warehouse::ActiveModel {
            id: Set(wid),
            warehouse_code: Set(code.to_string()),
            name: Set(name.to_string()),
            is_default: Set(false),
            is_active: Set(true),
            created_at: Set(now()),
            updated_at: Set(now()),
            ..Default::default()
        }
        .insert(&*db)
        .await
        .unwrap_or_else(|e| panic!("种子仓库 {wid} 插入失败: {e}"));
    }
    product::ActiveModel {
        id: Set(5),
        code: Set("FAB-RED".to_string()),
        name: Set("红色染色布".to_string()),
        unit: Set("米".to_string()),
        status: Set("active".to_string()),
        is_deleted: Set(false),
        product_type: Set("fabric".to_string()),
        product_grade: Set(Some("一等品".to_string())),
        created_at: Set(now()),
        updated_at: Set(now()),
        ..Default::default()
    }
    .insert(&*db)
    .await
    .expect("种子产品 5 插入失败");

    for tid in [1i32, 2] {
        inventory_transfer::ActiveModel {
            id: Set(tid),
            transfer_no: Set(format!("TRF-P4D-{tid:03}")),
            from_warehouse_id: Set(1),
            to_warehouse_id: Set(2),
            transfer_date: Set(now()),
            status: Set(transfer_status::APPROVED.to_string()),
            total_quantity: Set(dec("10.00")),
            total_amount: Set(rust_decimal::Decimal::ZERO),
            created_at: Set(now()),
            updated_at: Set(now()),
            ..Default::default()
        }
        .insert(&*db)
        .await
        .unwrap_or_else(|e| panic!("种子调拨单 {tid} 插入失败: {e}"));
    }

    // item1：四维齐全（缸号 DL-A/色号 RED/批次 B1/匹号 P-9）；item2：染色布缺匹号（存量 NULL 行）
    inventory_transfer_item::ActiveModel {
        id: Set(1),
        transfer_id: Set(1),
        product_id: Set(5),
        quantity: Set(dec("10.00")),
        shipped_quantity: Set(rust_decimal::Decimal::ZERO),
        received_quantity: Set(rust_decimal::Decimal::ZERO),
        created_at: Set(now()),
        updated_at: Set(now()),
        color_no: Set("RED".to_string()),
        dye_lot_no: Set(Some("DL-A".to_string())),
        batch_no: Set("B1".to_string()),
        piece_no: Set(Some("P-9".to_string())),
        ..Default::default()
    }
    .insert(&*db)
    .await
    .expect("种子调拨明细 1 插入失败");
    inventory_transfer_item::ActiveModel {
        id: Set(2),
        transfer_id: Set(2),
        product_id: Set(5),
        quantity: Set(dec("10.00")),
        shipped_quantity: Set(rust_decimal::Decimal::ZERO),
        received_quantity: Set(rust_decimal::Decimal::ZERO),
        created_at: Set(now()),
        updated_at: Set(now()),
        color_no: Set("RED".to_string()),
        dye_lot_no: Set(Some("DL-A".to_string())),
        batch_no: Set("B1".to_string()),
        piece_no: Set(None),
        ..Default::default()
    }
    .insert(&*db)
    .await
    .expect("种子调拨明细 2 插入失败");

    for (sid, wh) in [(1i32, 1i32), (2, 2)] {
        let on_hand = if sid == 1 {
            dec("50.00")
        } else {
            rust_decimal::Decimal::ZERO
        };
        inventory_stock::ActiveModel {
            id: Set(sid),
            warehouse_id: Set(wh),
            product_id: Set(5),
            quantity_on_hand: Set(on_hand),
            quantity_available: Set(on_hand),
            quantity_reserved: Set(rust_decimal::Decimal::ZERO),
            quantity_shipped: Set(rust_decimal::Decimal::ZERO),
            quantity_incoming: Set(rust_decimal::Decimal::ZERO),
            reorder_point: Set(rust_decimal::Decimal::ZERO),
            max_stock_point: Set(rust_decimal::Decimal::ZERO),
            reorder_quantity: Set(rust_decimal::Decimal::ZERO),
            created_at: Set(now()),
            updated_at: Set(now()),
            batch_no: Set("B1".to_string()),
            color_no: Set("RED".to_string()),
            dye_lot_no: Set(Some("DL-A".to_string())),
            grade: Set("一等品".to_string()),
            quantity_meters: Set(on_hand),
            quantity_kg: Set(if sid == 1 {
                dec("10.00")
            } else {
                rust_decimal::Decimal::ZERO
            }),
            stock_status: Set("正常".to_string()),
            quality_status: Set("合格".to_string()),
            version: Set(0),
            replenishment_strategy: Set("reorder_point".to_string()),
            ..Default::default()
        }
        .insert(&*db)
        .await
        .unwrap_or_else(|e| panic!("种子库存行 {sid} 插入失败: {e}"));
    }

    inventory_piece::ActiveModel {
        id: Set(1),
        piece_no: Set("P-9".to_string()),
        piece_type: Set("dyed".to_string()),
        warehouse_id: Set(1),
        product_id: Set(5),
        batch_no: Set("B1".to_string()),
        color_no: Set("RED".to_string()),
        dye_lot_no: Set("DL-A".to_string()),
        length: Set(dec("30.00")),
        status: Set(piece_status::AVAILABLE.to_string()),
        created_at: Set(now()),
        updated_at: Set(now()),
        ..Default::default()
    }
    .insert(&*db)
    .await
    .expect("种子染色匹 P-9 插入失败");
    db
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
