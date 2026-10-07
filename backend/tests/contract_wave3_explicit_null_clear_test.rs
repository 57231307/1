//! 第 1 波（库存/仓储/收货/退货域）：更新端点可空列三态清空语义收口
//! （对齐 RFC 7386 JSON Merge Patch，先例 `contract_wave2_explicit_null_clear_test.rs`）
//!
//! 锁定的真实行为（对应本轮修复）：
//! 1. 三态语义：键缺席=保持原值、显式 `null`=清空为 NULL、有值=覆盖。
//!    - DTO 层：`Option<Option<T>>` + `double_option` 适配器区分缺席与显式 null；
//!    - service 层：Some(None) → `Set(None)` 真实落 NULL；None 不 Set（列不进 UPDATE）；
//!    - NOT NULL 列不开 null 清空：显式 null 在任何 DB 访问前被
//!      `AppError::business_displayable` 拒绝（400 + 外显文案，不脱敏、无副作用）。
//! 2. 覆盖端点与可空列（DDL 证据见各 `backend/src/models/*.rs` 与 migration 行号，
//!    详见 PR 描述普查表）：
//!    - PUT /inventory/adjustments/{id}（reason_description/notes 可空）
//!    - PUT /inventory/batches/{id}（dye_lot_no/gram_weight/width/expiry_date 可空）
//!    - PUT /inventory/stock/{id}（bin_location 可空）
//!    - PUT /warehouses/{id}（address/manager/phone/contact_person/capacity/warehouse_type 可空）
//!    - PUT /warehouse/locations/{id}（location_type/max_weight/max_height/is_batch_managed/is_color_managed 可空）
//!    - PUT /inventory/counts/{id}（notes 可空）
//!    - PUT /inventory/counts/items/{item_id}（notes 可空；quantity_actual NOT NULL 拒清）
//!    - PUT /inventory/transfers/{id}（notes 可空）
//!    - PUT /inventory/transfers/items/{item_id}（notes/unit_cost/piece_no 可空置 NULL；
//!      dye_lot_no 模型虽为 Option 但列 DDL 是 NOT NULL DEFAULT ''，"清空"落 ''）
//!    - PUT /sales/returns/{id}（order_id/notes 可空；reason_detail 虚拟入参）
//!    - PUT /sales/returns/{id}/items/{item_id}（reason→notes 可空）
//!    - PUT /purchase/receipts/{id}（department_id/inspector_id/notes/attachment_urls 可空）
//!    - PUT /purchase/receipts/{id}/items/{item_id}（batch_no/color_code/lot_no/grade/gram_weight/width/quantity_alt/unit_price/location_code/notes/piece_no 可空）
//!    - PUT /purchase/returns/{id}（reason_type/reason_detail/notes 全部可空）
//!    - PUT /purchase/returns/{id}/items/{item_id}（notes 可空）
//!
//! 覆盖策略（路线一：统一真库 PostgreSQL，对齐 wave2 / po_item_update_fields 先例，无 mock）：
//! - 纯 serde：DTO 三态形状锁（缺席=None / null=Some(None) / 有值=Some(Some(v))）；
//! - 已迁移真库（`test_common::setup_test_db()`，业务表 TRUNCATE 后为空）：
//!   NOT NULL 显式 null 的 service 级拒绝（拒绝在任何 DB 访问前返回，空表也得到业务错误）、
//!   handler 400 信封与文案外显、**真实表 + 真实 service/handler 的三态回环**
//!   （库存 update_batch_fields、仓储 update_location、退货 update_return、
//!   调拨明细 update_item——FK 前置由用例自种子 warehouses/products 提供，
//! 不再自建 sqlite 同构 DDL——同构 DDL 与真表列型漂移正是 CI 约 130 例
//!   ColumnDecode 红的根因）；
//! - 收货 update_receipt 全链（服务链对 purchase_receipt 加 `lock_exclusive()`）
//!   与其余回环同通道执行：真库即生产方言，行锁/审计链全部真实触发，
//!   不再需要 `#[ignore]` 活库分桶。

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
use bingxi_backend::handlers::{
    inventory_batch_handler, inventory_count_handler, inventory_stock_handler,
    inventory_stock_handler_dto, sales_return_handler, warehouse_handler,
};
use bingxi_backend::middleware::auth_context::AuthContext;
use bingxi_backend::models::audit_log;
use bingxi_backend::models::inventory_stock;
use bingxi_backend::models::inventory_transfer;
use bingxi_backend::models::inventory_transfer_item;
use bingxi_backend::models::location;
use bingxi_backend::models::purchase_return;
use bingxi_backend::models::status::purchase_inventory::inventory_stock_quality_status;
use bingxi_backend::models::status::purchase_inventory::{
    inventory_stock_status, inventory_transfer as transfer_status,
    purchase_receipt as receipt_status, purchase_return as pr_status,
};
use bingxi_backend::services::inv::{
    InventoryTransferService, UpdateInventoryTransferItemRequest, UpdateInventoryTransferRequest,
};
use bingxi_backend::services::inventory_adjustment_service::{
    InventoryAdjustmentService, UpdateAdjustmentRequest,
};
use bingxi_backend::services::inventory_count_service::{
    InventoryCountService, UpdateCountRequest,
};
use bingxi_backend::services::inventory_stock_service::InventoryStockService;
use bingxi_backend::services::purchase_receipt_dto;
use bingxi_backend::services::purchase_receipt_service::PurchaseReceiptService;
use bingxi_backend::services::purchase_return_service::{
    PurchaseReturnService, UpdatePurchaseReturnRequest, UpdateReturnItemRequest,
};
use bingxi_backend::services::sales_return_service::{
    SalesReturnService, UpdateSalesReturnRequest,
};
use bingxi_backend::services::warehouse_service::WarehouseService;
use bingxi_backend::utils::error::AppError;
use chrono::{TimeZone, Utc};
use rust_decimal::Decimal;
use sea_orm::{
    ActiveModelTrait, ColumnTrait, ConnectionTrait, DatabaseConnection, DbBackend, EntityTrait,
    Order, QueryFilter, QueryOrder, Set, Statement,
};
use serde_json::{Value, json};
use std::str::FromStr;
use std::sync::Arc;
use tower::ServiceExt;

// =========================================================
// 公共夹具
// =========================================================

fn dec(s: &str) -> Decimal {
    Decimal::from_str(s).unwrap()
}

fn make_scope_auth(user_id: i32) -> AuthContext {
    AuthContext {
        user_id,
        username: format!("e2e_user_{user_id}"),
        role_id: None,
        department_id: Some(1),
        data_scope: Some("all".to_string()),
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

async fn exec_pg(db: &DatabaseConnection, sql: &str) {
    db.execute_raw(Statement::from_string(DbBackend::Postgres, sql.to_string()))
        .await
        .unwrap_or_else(|e| panic!("真库种子执行失败: {e}\nSQL: {sql}"));
}

/// FK 前置种子：warehouses（inventory_stocks/inventory_transfers/warehouse_locations/
/// purchase_receipt 多条真表 FK 的指向表）与 products（inventory_stocks FK）。
/// warehouses/products 都是业务表、夹具 TRUNCATE 后为空，必须用例自建。
async fn seed_warehouses_and_products(db: &DatabaseConnection) {
    exec_pg(
        db,
        r#"INSERT INTO warehouses (id, name, warehouse_code, is_active)
           VALUES (1, '波3三态锁主仓', 'W3-EPC-W1', true),
                  (2, '波3三态锁目标仓', 'W3-EPC-W2', true)"#,
    )
    .await;
    exec_pg(
        db,
        r#"INSERT INTO products (id, code, name)
           VALUES (2, 'W3-EPC-P2', '波3三态锁库存产品'),
                  (5, 'W3-EPC-P5', '波3三态锁调拨产品')"#,
    )
    .await;
}

fn batch_service_router(db: DatabaseConnection) -> Router {
    let state = AppState {
        db: Arc::new(db),
        ..Default::default()
    };
    Router::new()
        .route(
            "/inventory/batches/{id}",
            axum::routing::put(inventory_batch_handler::update_batch),
        )
        .with_state(state)
        .layer(from_fn_with_state(make_scope_auth(9101), inject_auth))
}

fn stock_router(db: DatabaseConnection) -> Router {
    let state = AppState {
        db: Arc::new(db),
        ..Default::default()
    };
    Router::new()
        .route(
            "/inventory/stock/{id}",
            axum::routing::put(inventory_stock_handler::update_stock),
        )
        .with_state(state)
        .layer(from_fn_with_state(make_scope_auth(9101), inject_auth))
}

fn location_router(db: DatabaseConnection) -> Router {
    let state = AppState {
        db: Arc::new(db),
        ..Default::default()
    };
    Router::new()
        .route(
            "/warehouse/locations/{id}",
            axum::routing::put(warehouse_handler::update_location),
        )
        .with_state(state)
        .layer(from_fn_with_state(make_scope_auth(9101), inject_auth))
}

// =========================================================
// A) DTO 三态形状锁（纯 serde，无 DB）：
//    缺席=None（保持）、null=Some(None)（清空）、有值=Some(Some(v))（覆盖）
// =========================================================

#[test]
fn adjustment_dto_distinguishes_absent_null_and_value() {
    let req: bingxi_backend::handlers::inventory_adjustment_handler::UpdateAdjustmentRequestPayload =
        serde_json::from_value(json!({
            "reason_description": null,
            "notes": "新说明",
            "warehouse_id": null,
        }))
        .expect("调整单 update DTO 三态反序列化失败");
    assert!(
        matches!(req.reason_description, Some(None)),
        "显式 null 的 reason_description 必须反序列化为 Some(None)（清空），不得与缺席塌同"
    );
    assert!(matches!(&req.notes, Some(Some(v)) if v == "新说明"));
    assert!(
        matches!(req.warehouse_id, Some(None)),
        "NOT NULL 列的显式 null 也必须可辨，才能交 service 判业务错误拒绝"
    );
    assert!(
        req.adjustment_date.is_none(),
        "缺席键必须是 None（保持原值）"
    );
    assert!(req.adjustment_type.is_none());
    assert!(req.reason_type.is_none());
}

#[test]
fn batch_dto_distinguishes_absent_null_and_value() {
    let req: inventory_batch_handler::UpdateBatchRequest = serde_json::from_value(json!({
        "dye_lot_no": null,
        "gram_weight": 210.5,
        "color_no": null,
    }))
    .expect("批次 update DTO 三态反序列化失败");
    assert!(matches!(req.dye_lot_no, Some(None)));
    assert!(matches!(req.gram_weight, Some(Some(v)) if (v - 210.5).abs() < f64::EPSILON));
    assert!(
        matches!(req.color_no, Some(None)),
        "NOT NULL 列显式 null 可辨"
    );
    assert!(req.width.is_none(), "缺席键必须是 None（保持原值）");
    assert!(req.expiry_date.is_none());
    assert!(req.quality_status.is_none());
}

#[test]
fn stock_dto_distinguishes_absent_null_and_value() {
    let req: inventory_stock_handler_dto::UpdateStockWithVersionRequest =
        serde_json::from_value(json!({
            "bin_location": null,
            "reorder_point": "20.00",
            "quantity_on_hand": null,
            "version": 3,
        }))
        .expect("库存 update DTO 三态反序列化失败");
    assert!(matches!(req.bin_location, Some(None)));
    assert!(matches!(&req.reorder_point, Some(Some(v)) if *v == dec("20.00")));
    assert!(
        matches!(req.quantity_on_hand, Some(None)),
        "NOT NULL 列显式 null 可辨，不得塌成缺席"
    );
    assert!(req.quantity_available.is_none());
    assert!(req.max_stock_point.is_none());
    assert_eq!(req.version, 3, "version 保持必填裸值（乐观锁比对键）");
}

#[test]
fn warehouse_and_location_dto_distinguish_absent_null_and_value() {
    let wh: warehouse_handler::UpdateWarehouseRequest = serde_json::from_value(json!({
        "address": null,
        "manager": null,
        "capacity": 120,
        "name": null,
    }))
    .expect("仓库 update DTO 三态反序列化失败");
    assert!(matches!(wh.address, Some(None)));
    assert!(
        matches!(wh.manager, Some(None)),
        "manager→manager_id 可空列，显式 null=清除经理"
    );
    assert!(matches!(wh.capacity, Some(Some(120))));
    assert!(matches!(wh.name, Some(None)), "NOT NULL 列显式 null 可辨");
    assert!(wh.phone.is_none());
    assert!(wh.is_default.is_none());

    let loc: warehouse_handler::UpdateLocationRequest = serde_json::from_value(json!({
        "location_type": null,
        "max_weight": 500.5,
        "location_code": null,
    }))
    .expect("库位 update DTO 三态反序列化失败");
    assert!(matches!(loc.location_type, Some(None)));
    assert!(matches!(loc.max_weight, Some(Some(v)) if (v - 500.5).abs() < f64::EPSILON));
    assert!(
        matches!(loc.location_code, Some(None)),
        "NOT NULL 列 location_code 显式 null 可辨"
    );
    assert!(loc.max_height.is_none());
    assert!(loc.is_batch_managed.is_none());
}

#[test]
fn count_and_transfer_dto_distinguish_absent_null_and_value() {
    let count: inventory_count_handler::UpdateCountPayload = serde_json::from_value(json!({
        "notes": null,
        "count_date": null,
    }))
    .expect("盘点 update DTO 三态反序列化失败");
    assert!(matches!(count.notes, Some(None)));
    assert!(
        matches!(count.count_date, Some(None)),
        "NOT NULL 列 count_date 显式 null 可辨"
    );

    let transfer: UpdateInventoryTransferRequest = serde_json::from_value(json!({
        "notes": null,
        "status": null,
    }))
    .expect("调拨 update DTO 三态反序列化失败");
    assert!(matches!(transfer.notes, Some(None)));
    assert!(
        matches!(transfer.status, Some(None)),
        "status（非 Option 模型列）显式 null 可辨并被入口拒绝，不得塌成保持"
    );
    assert!(transfer.items.is_none(), "items 缺席=不动明细");

    // 盘点明细（PUT /inventory/counts/items/{item_id}）：前端 api 出参已声明
    // notes?: string | null，后端 DTO 必须挂适配器兑现 null=清空，否则清空静默失效
    let count_item: inventory_count_handler::UpdateCountItemPayload =
        serde_json::from_value(json!({
            "notes": null,
            "quantity_actual": null,
        }))
        .expect("盘点明细 update DTO 三态反序列化失败");
    assert!(
        matches!(count_item.notes, Some(None)),
        "可空列 notes 显式 null=Some(None)（清空），不得与缺席塌同"
    );
    assert!(
        matches!(count_item.quantity_actual, Some(None)),
        "NOT NULL 列 quantity_actual 显式 null 可辨，交 service 拒绝"
    );
    let count_item_absent: inventory_count_handler::UpdateCountItemPayload =
        serde_json::from_value(json!({ "notes": "补录" }))
            .expect("盘点明细 update DTO 三态反序列化失败");
    assert!(
        count_item_absent.quantity_actual.is_none(),
        "缺席键必须是 None（保持原值）"
    );
    assert!(matches!(
        &count_item_absent.notes,
        Some(Some(v)) if v == "补录"
    ));
}

#[test]
fn transfer_item_dto_distinguishes_absent_null_and_value() {
    let item: UpdateInventoryTransferItemRequest = serde_json::from_value(json!({
        "notes": null,
        "unit_cost": null,
        "dye_lot_no": null,
        "quantity": null,
        "color_no": "RED-7",
    }))
    .expect("调拨明细 update DTO 三态反序列化失败");
    assert!(
        matches!(item.notes, Some(None)),
        "可空列 notes 显式 null=清空"
    );
    assert!(
        matches!(item.unit_cost, Some(None)),
        "可空列 unit_cost 显式 null=清空（旧单 Option 形态会塌成保持）"
    );
    assert!(matches!(item.dye_lot_no, Some(None)));
    assert!(
        matches!(item.quantity, Some(None)),
        "NOT NULL 列 quantity 显式 null 可辨，交 service 拒绝"
    );
    assert!(matches!(&item.color_no, Some(Some(v)) if v == "RED-7"));
    assert!(item.product_id.is_none(), "缺席键必须是 None（保持原值）");
    assert!(item.batch_no.is_none());
}

#[test]
fn sales_return_and_item_dto_distinguish_absent_null_and_value() {
    let req: UpdateSalesReturnRequest = serde_json::from_value(json!({
        "order_id": null,
        "notes": "补录备注",
        "customer_id": null,
        "reason_detail": null,
    }))
    .expect("销售退货 update DTO 三态反序列化失败");
    assert!(
        matches!(req.order_id, Some(None)),
        "sales_order_id 可空列显式 null=解除关联"
    );
    assert!(matches!(&req.notes, Some(Some(v)) if v == "补录备注"));
    assert!(
        matches!(req.customer_id, Some(None)),
        "NOT NULL 列显式 null 可辨"
    );
    assert!(matches!(req.reason_detail, Some(None)));
    assert!(req.return_date.is_none());
    assert!(req.warehouse_id.is_none());
    assert!(req.reason_type.is_none());

    let item: sales_return_handler::UpdateReturnItemRequest = serde_json::from_value(json!({
        "reason": null,
        "quantity": null,
    }))
    .expect("销售退货明细 update DTO 三态反序列化失败");
    assert!(
        matches!(item.reason, Some(None)),
        "reason→notes 可空列显式 null 清空"
    );
    assert!(
        matches!(item.quantity, Some(None)),
        "NOT NULL 列显式 null 可辨"
    );
    assert!(item.unit_price.is_none());
}

#[test]
fn receipt_and_purchase_return_dto_distinguish_absent_null_and_value() {
    let header: purchase_receipt_dto::UpdatePurchaseReceiptRequest =
        serde_json::from_value(json!({
            "notes": null,
            "attachment_urls": null,
            "inspector_id": null,
            "supplier_id": null,
        }))
        .expect("入库单头 update DTO 三态反序列化失败");
    assert!(matches!(header.notes, Some(None)));
    assert!(matches!(header.attachment_urls, Some(None)));
    assert!(matches!(header.inspector_id, Some(None)));
    assert!(
        matches!(header.supplier_id, Some(None)),
        "NOT NULL 列 supplier_id 显式 null 可辨"
    );
    assert!(header.receipt_date.is_none());
    assert!(header.department_id.is_none());

    let item: purchase_receipt_dto::UpdateReceiptItemRequest = serde_json::from_value(json!({
        "batch_no": null,
        "unit_price": null,
        "quantity": null,
        "color_code": "RED-01",
    }))
    .expect("入库明细 update DTO 三态反序列化失败");
    assert!(matches!(item.batch_no, Some(None)));
    assert!(matches!(item.unit_price, Some(None)));
    assert!(
        matches!(item.quantity, Some(None)),
        "NOT NULL 列显式 null 可辨"
    );
    assert!(matches!(&item.color_code, Some(Some(v)) if v == "RED-01"));
    assert!(item.line_no.is_none());

    let pr: UpdatePurchaseReturnRequest = serde_json::from_value(json!({
        "reason_type": null,
        "reason_detail": null,
        "notes": null,
    }))
    .expect("采购退货 update DTO 三态反序列化失败");
    assert!(matches!(pr.reason_type, Some(None)));
    assert!(matches!(pr.reason_detail, Some(None)));
    assert!(matches!(pr.notes, Some(None)));

    let pr_item: UpdateReturnItemRequest = serde_json::from_value(json!({
        "notes": null,
        "color_no": null,
    }))
    .expect("采购退货明细 update DTO 三态反序列化失败");
    assert!(
        matches!(pr_item.notes, Some(None)),
        "notes 可空列显式 null 清空"
    );
    assert!(
        matches!(pr_item.color_no, Some(None)),
        "追溯列 color_no 为 NOT NULL DEFAULT ''——显式 null 可辨并被拒绝（改回白坯须提交空串）"
    );
    assert!(pr_item.line_no.is_none());
    assert!(pr_item.quantity_returned.is_none());
}

// =========================================================
// B) service 层 NOT NULL 显式 null 拒绝（真库空业务表即可：拒绝在任何 DB 访问前返回）
// =========================================================

fn expect_cleared_business_error(err: AppError, keyword: &str, field: &str) {
    assert!(
        matches!(&err, AppError::BusinessErrorDisplayable(m) if m.contains(keyword)),
        "字段 {field} 显式 null 应被业务错误拒绝且文案含「{keyword}」，实际: {err:?}"
    );
}

#[tokio::test]
async fn adjustment_explicit_null_on_not_null_columns_rejected_before_db() {
    let db = setup_test_db().await;
    let service = InventoryAdjustmentService::new(Arc::new(db));
    let cases = [
        (
            "warehouse_id",
            UpdateAdjustmentRequest {
                warehouse_id: Some(None),
                ..Default::default()
            },
            "仓库不能清空",
        ),
        (
            "adjustment_date",
            UpdateAdjustmentRequest {
                adjustment_date: Some(None),
                ..Default::default()
            },
            "调整日期不能清空",
        ),
        (
            "adjustment_type",
            UpdateAdjustmentRequest {
                adjustment_type: Some(None),
                ..Default::default()
            },
            "调整类型不能清空",
        ),
        (
            "reason_type",
            UpdateAdjustmentRequest {
                reason_type: Some(None),
                ..Default::default()
            },
            "原因类型不能清空",
        ),
    ];
    for (field, req, keyword) in cases {
        let err = service
            .update_adjustment(999_999, req)
            .await
            .expect_err("NOT NULL 列显式 null 必须被拒绝");
        expect_cleared_business_error(err, keyword, field);
    }
}

#[tokio::test]
async fn batch_explicit_null_on_not_null_columns_rejected_before_db() {
    let db = setup_test_db().await;
    let service = InventoryStockService::new(Arc::new(db));
    let err = service
        .update_batch_fields(
            999_999,
            Some(None),
            None,
            None,
            None,
            None,
            None,
            None,
            None,
        )
        .await
        .expect_err("NOT NULL 列 color_no 显式 null 必须被拒绝");
    expect_cleared_business_error(err, "色号不能清空", "color_no");

    let err2 = service
        .update_batch_fields(
            999_999,
            None,
            None,
            None,
            None,
            None,
            None,
            Some(None),
            Some(None),
        )
        .await
        .expect_err("NOT NULL 列 stock_status/quality_status 显式 null 必须被拒绝");
    expect_cleared_business_error(err2, "库存状态不能清空", "stock_status");
}

#[tokio::test]
async fn stock_handler_explicit_null_on_not_null_columns_rejected_before_db() {
    let app = stock_router(setup_test_db().await);
    let (status, v) = call(
        &app,
        Method::PUT,
        "/inventory/stock/999999",
        Some(json!({ "quantity_on_hand": null, "version": 1 })),
    )
    .await;
    assert_eq!(status, StatusCode::BAD_REQUEST, "实际: {v}");
    assert_eq!(v["code"], "BUSINESS_ERROR");
    assert_eq!(v["message"], "在库数量不能清空：该字段为必填项");
    assert_ne!(
        v["message"], "业务处理失败",
        "business_displayable 不得被脱敏"
    );
}

#[tokio::test]
async fn warehouse_explicit_null_on_not_null_columns_rejected_before_db() {
    let db = setup_test_db().await;
    let service = WarehouseService::new(Arc::new(db));
    let mk = |name, is_default, status| warehouse_handler::UpdateWarehouseRequest {
        name,
        address: None,
        manager: None,
        phone: None,
        contact_person: None,
        is_default,
        capacity: None,
        status,
        warehouse_type: None,
    };
    let cases = [
        (mk(Some(None), None, None), "仓库名称不能清空", "name"),
        (
            mk(None, Some(None), None),
            "默认仓库标志不能清空",
            "is_default",
        ),
        (mk(None, None, Some(None)), "启用状态不能清空", "status"),
    ];
    for (req, keyword, field) in cases {
        let err = service
            .update(999_999, 9101, req)
            .await
            .expect_err("NOT NULL 列显式 null 必须被拒绝");
        expect_cleared_business_error(err, keyword, field);
    }
}

#[tokio::test]
async fn location_handler_explicit_null_on_not_null_columns_rejected_before_db() {
    let app = location_router(setup_test_db().await);
    let (status, v) = call(
        &app,
        Method::PUT,
        "/warehouse/locations/999999",
        Some(json!({ "location_code": null })),
    )
    .await;
    assert_eq!(status, StatusCode::BAD_REQUEST, "实际: {v}");
    assert_eq!(v["code"], "BUSINESS_ERROR");
    assert_eq!(v["message"], "库位编码不能清空：该字段为必填项");
}

#[tokio::test]
async fn count_and_transfer_explicit_null_on_not_null_columns_rejected_before_db() {
    let db = setup_test_db().await;
    let count_service = InventoryCountService::new(Arc::new(db.clone()));
    let err = count_service
        .update_count(
            999_999,
            UpdateCountRequest {
                count_date: Some(None),
                notes: None,
            },
            Some(9101),
        )
        .await
        .expect_err("NOT NULL 列 count_date 显式 null 必须被拒绝");
    expect_cleared_business_error(err, "盘点日期不能清空", "count_date");

    let transfer_service = InventoryTransferService::new(Arc::new(db));
    let err2 = transfer_service
        .update_transfer(
            999_999,
            UpdateInventoryTransferRequest {
                status: Some(None),
                notes: None,
                items: None,
            },
            9101,
        )
        .await
        .expect_err("status（非 Option 模型列）显式 null 必须被拒绝");
    expect_cleared_business_error(err2, "调拨状态不能清空", "status");
}

#[tokio::test]
async fn count_item_explicit_null_on_not_null_quantity_rejected_before_db() {
    // 盘点明细 quantity_actual 为 NOT NULL 列（inventory_count_item 模型非 Option Decimal）：
    // 显式 null 必须在任何 DB 访问前（含 begin 事务）被拒——空业务表亦得业务错误
    let db = setup_test_db().await;
    let service = InventoryCountService::new(Arc::new(db));
    let err = service
        .update_count_item(999_999, Some(None), None)
        .await
        .expect_err("NOT NULL 列 quantity_actual 显式 null 必须被拒绝");
    expect_cleared_business_error(err, "实盘数量不能清空", "quantity_actual");
}

#[tokio::test]
async fn sales_return_header_and_item_explicit_null_rejected_before_db() {
    let db = setup_test_db().await;
    let service = SalesReturnService::new(Arc::new(db));
    let mk = |customer_id, return_date, warehouse_id, reason_type| UpdateSalesReturnRequest {
        order_id: None,
        customer_id,
        return_date,
        warehouse_id,
        reason_type,
        reason_detail: None,
        notes: None,
    };
    let cases = [
        (
            mk(Some(None), None, None, None),
            "客户不能清空",
            "customer_id",
        ),
        (
            mk(None, Some(None), None, None),
            "退货日期不能清空",
            "return_date",
        ),
        (
            mk(None, None, Some(None), None),
            "仓库不能清空",
            "warehouse_id",
        ),
        (
            mk(None, None, None, Some(None)),
            "退货原因类型不能清空",
            "reason_type",
        ),
    ];
    for (req, keyword, field) in cases {
        let err = service
            .update_return(999_999, req, 9101)
            .await
            .expect_err("NOT NULL 列显式 null 必须被拒绝");
        expect_cleared_business_error(err, keyword, field);
    }

    // reason_detail 是虚拟入参：脱离 reason_type 的单独清空显式拒绝（不静默忽略）
    let err = service
        .update_return(
            999_999,
            UpdateSalesReturnRequest {
                order_id: None,
                customer_id: None,
                return_date: None,
                warehouse_id: None,
                reason_type: None,
                reason_detail: Some(None),
                notes: None,
            },
            9101,
        )
        .await
        .expect_err("reason_detail 单独显式 null 必须被显式拒绝");
    expect_cleared_business_error(err, "退货原因详情不能单独清空", "reason_detail");

    let err2 = service
        .update_return_item(999_999, Some(None), None, None, 9101)
        .await
        .expect_err("NOT NULL 列 quantity 显式 null 必须被拒绝");
    expect_cleared_business_error(err2, "退货数量不能清空", "quantity");
    let err3 = service
        .update_return_item(999_999, None, Some(None), None, 9101)
        .await
        .expect_err("NOT NULL 列 unit_price 显式 null 必须被拒绝");
    expect_cleared_business_error(err3, "退货单价不能清空", "unit_price");
}

#[tokio::test]
async fn receipt_header_and_item_explicit_null_rejected_before_db() {
    let db = setup_test_db().await;
    let service = PurchaseReceiptService::new(Arc::new(db));
    let err = service
        .update_receipt(
            999_999,
            purchase_receipt_dto::UpdatePurchaseReceiptRequest {
                supplier_id: Some(None),
                ..Default::default()
            },
            9101,
        )
        .await
        .expect_err("NOT NULL 列 supplier_id 显式 null 必须被拒绝");
    expect_cleared_business_error(err, "供应商不能清空", "supplier_id");
    let err2 = service
        .update_receipt(
            999_999,
            purchase_receipt_dto::UpdatePurchaseReceiptRequest {
                receipt_date: Some(None),
                ..Default::default()
            },
            9101,
        )
        .await
        .expect_err("NOT NULL 列 receipt_date 显式 null 必须被拒绝");
    expect_cleared_business_error(err2, "入库日期不能清空", "receipt_date");

    let err3 = service
        .update_receipt_item(
            999_999,
            purchase_receipt_dto::UpdateReceiptItemRequest {
                quantity: Some(None),
                ..Default::default()
            },
            9101,
        )
        .await
        .expect_err("NOT NULL 列 quantity 显式 null 必须被拒绝");
    expect_cleared_business_error(err3, "入库数量不能清空", "quantity");
    let err4 = service
        .update_receipt_item(
            999_999,
            purchase_receipt_dto::UpdateReceiptItemRequest {
                material_code: Some(None),
                ..Default::default()
            },
            9101,
        )
        .await
        .expect_err("NOT NULL 列 material_code 显式 null 必须被拒绝");
    expect_cleared_business_error(err4, "物料编码不能清空", "material_code");
}

#[tokio::test]
async fn purchase_return_item_explicit_null_on_not_null_columns_rejected_before_db() {
    let db = setup_test_db().await;
    let service = PurchaseReturnService::new(Arc::new(db));
    let mk = |line_no,
              material_id,
              quantity_returned,
              unit_price,
              tax_rate,
              discount_percent,
              color_no,
              dye_lot_no,
              batch_no| UpdateReturnItemRequest {
        line_no,
        material_id,
        quantity_returned,
        unit_price,
        tax_rate,
        discount_percent,
        notes: None,
        color_no,
        dye_lot_no,
        batch_no,
    };
    let cases = [
        (
            "line_no",
            mk(Some(None), None, None, None, None, None, None, None, None),
            "行号不能清空",
        ),
        (
            "material_id",
            mk(None, Some(None), None, None, None, None, None, None, None),
            "物料不能清空",
        ),
        (
            "quantity_returned",
            mk(None, None, Some(None), None, None, None, None, None, None),
            "退货数量不能清空",
        ),
        (
            "unit_price",
            mk(None, None, None, Some(None), None, None, None, None, None),
            "单价不能清空",
        ),
        (
            "tax_rate",
            mk(None, None, None, None, Some(None), None, None, None, None),
            "税率不能清空",
        ),
        (
            "discount_percent",
            mk(None, None, None, None, None, Some(None), None, None, None),
            "折扣率不能清空",
        ),
        (
            "color_no",
            mk(None, None, None, None, None, None, Some(None), None, None),
            "色号不能清空",
        ),
        (
            "dye_lot_no",
            mk(None, None, None, None, None, None, None, Some(None), None),
            "缸号不能清空",
        ),
        (
            "batch_no",
            mk(None, None, None, None, None, None, None, None, Some(None)),
            "批次不能清空",
        ),
    ];
    for (field, req, keyword) in cases {
        let err = service
            .update_item(999_999, req, 9101)
            .await
            .expect_err("NOT NULL 列显式 null 必须被拒绝（任何 DB 访问前）");
        expect_cleared_business_error(err, keyword, field);
    }
}

#[tokio::test]
async fn batch_handler_put_null_color_no_400_visible_message() {
    let app = batch_service_router(setup_test_db().await);
    let (status, v) = call(
        &app,
        Method::PUT,
        "/inventory/batches/999999",
        Some(json!({ "color_no": null })),
    )
    .await;
    assert_eq!(status, StatusCode::BAD_REQUEST, "实际: {v}");
    assert_eq!(v["code"], "BUSINESS_ERROR");
    assert_eq!(v["message"], "色号不能清空：该字段为必填项");
    assert_ne!(
        v["message"], "业务处理失败",
        "business_displayable 不得被脱敏"
    );
}

#[tokio::test]
async fn transfer_item_explicit_null_on_not_null_columns_rejected_before_db() {
    let db = setup_test_db().await;
    let service = InventoryTransferService::new(Arc::new(db));
    let mk = |product_id, quantity, color_no, batch_no| UpdateInventoryTransferItemRequest {
        product_id,
        quantity,
        notes: None,
        unit_cost: None,
        color_no,
        dye_lot_no: None,
        batch_no,
        // 匹号缺席=保持原值；匹号的三态（白坯可清空/染色布清空被拒）另需专列用例覆盖
        piece_no: None,
    };
    let cases = [
        (
            "product_id",
            mk(Some(None), None, None, None),
            "产品不能清空",
        ),
        (
            "quantity",
            mk(None, Some(None), None, None),
            "调拨数量不能清空",
        ),
        ("color_no", mk(None, None, Some(None), None), "色号不能清空"),
        ("batch_no", mk(None, None, None, Some(None)), "批次不能清空"),
    ];
    for (field, req, keyword) in cases {
        let err = service
            .update_item(999_999, req)
            .await
            .expect_err("NOT NULL 列显式 null 必须被拒绝（任何 DB 访问前）");
        expect_cleared_business_error(err, keyword, field);
    }
}

// =========================================================
// C) 真库真实表 + 真实 service/handler 的三态回环
//    （同一列三态各一断言：省略=保持 / null=落 NULL / 有值=覆盖）
// =========================================================

async fn seed_stock(db: &DatabaseConnection) -> inventory_stock::Model {
    inventory_stock::ActiveModel {
        warehouse_id: Set(1),
        product_id: Set(2),
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
        batch_no: Set("B-W3-001".to_string()),
        color_no: Set("C001".to_string()),
        dye_lot_no: Set(Some("DL001".to_string())),
        grade: Set("一等品".to_string()),
        expiry_date: Set(Some(Utc.with_ymd_and_hms(2026, 12, 31, 0, 0, 0).unwrap())),
        quantity_meters: Set(dec("100.00")),
        quantity_kg: Set(dec("50.00")),
        gram_weight: Set(Some(dec("180.00"))),
        stock_status: Set(inventory_stock_status::NORMAL.to_string()),
        quality_status: Set(inventory_stock_quality_status::PASS.to_string()),
        version: Set(0),
        replenishment_strategy: Set("reorder_point".to_string()),
        ..Default::default()
    }
    .insert(db)
    .await
    .unwrap()
}

/// 库存域（批次编辑）：dye_lot_no 同一列三态各一断言 + gram_weight/expiry_date 覆盖与清空
#[tokio::test]
async fn inventory_batch_update_tri_state_roundtrip_on_postgres() {
    let db = setup_test_db().await;
    seed_warehouses_and_products(&db).await;
    let stock = seed_stock(&db).await;
    let service = InventoryStockService::new(Arc::new(db.clone()));

    // 1) 键缺席=保持：只覆盖 gram_weight，其余参数 None → 对应列不进 UPDATE
    let after = service
        .update_batch_fields(
            stock.id,
            None,
            None,
            None,
            Some(Some(210.5)),
            None,
            None,
            None,
            None,
        )
        .await
        .unwrap();
    assert_eq!(after.gram_weight, Some(dec("210.5")), "有值=覆盖");
    assert_eq!(
        after.dye_lot_no.as_deref(),
        Some("DL001"),
        "缺席键必须保持原值"
    );
    assert_eq!(after.color_no, "C001");

    // 2) 显式 null=落 NULL：dye_lot_no Some(None) / expiry_date Some(None)
    let after = service
        .update_batch_fields(
            stock.id,
            None,
            Some(None),
            None,
            None,
            None,
            Some(None),
            None,
            None,
        )
        .await
        .unwrap();
    assert!(
        after.dye_lot_no.is_none(),
        "发 null 后 dye_lot_no 必须为 NULL"
    );
    assert!(
        after.expiry_date.is_none(),
        "发 null 后 expiry_date 必须为 NULL"
    );
    assert_eq!(
        after.gram_weight,
        Some(dec("210.5")),
        "未再提交的 gram_weight 保持上一次结果"
    );

    // 3) 有值=覆盖：dye_lot_no 重新给值（同一列三态闭环），color_no 保持
    let after = service
        .update_batch_fields(
            stock.id,
            None,
            Some(Some("DL009".to_string())),
            None,
            None,
            None,
            None,
            None,
            None,
        )
        .await
        .unwrap();
    assert_eq!(after.dye_lot_no.as_deref(), Some("DL009"), "有值=覆盖");
    let reread = inventory_stock::Entity::find_by_id(stock.id)
        .one(&db)
        .await
        .unwrap()
        .unwrap();
    assert_eq!(reread.dye_lot_no.as_deref(), Some("DL009"));
    assert_eq!(reread.color_no, "C001", "缺席的 NOT NULL 列全程未动");
}

/// warehouse_locations 由迁移 m0010 建表（FK→warehouses），不再自建 DDL
async fn seed_location(db: &DatabaseConnection) -> location::Model {
    location::ActiveModel {
        warehouse_id: Set(1),
        location_code: Set("LOC-A01".to_string()),
        location_type: Set(Some("货架".to_string())),
        max_weight: Set(Some(dec("300.00"))),
        max_height: Set(Some(dec("250.00"))),
        is_batch_managed: Set(Some(true)),
        is_color_managed: Set(Some(false)),
        created_at: Set(Utc::now()),
        updated_at: Set(Utc::now()),
        ..Default::default()
    }
    .insert(db)
    .await
    .unwrap()
}

/// 仓储域（库位编辑）：location_type 同一列三态各一断言（真实 handler PUT）
#[tokio::test]
async fn warehouse_location_update_tri_state_roundtrip_on_postgres() {
    let db = setup_test_db().await;
    seed_warehouses_and_products(&db).await;
    let created = seed_location(&db).await;
    let id = created.id;
    let app = location_router(db.clone());
    let uri = format!("/warehouse/locations/{id}");

    // 1) 键缺席=保持：只提交 max_weight 覆盖，location_type/location_code 缺席不动
    let (status, v) = call(
        &app,
        Method::PUT,
        &uri,
        Some(json!({ "max_weight": 500.5 })),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "库位三态 update 失败: {v}");
    let row = location::Entity::find_by_id(id)
        .one(&db)
        .await
        .unwrap()
        .unwrap();
    assert_eq!(row.max_weight, Some(dec("500.5")), "有值=覆盖");
    assert_eq!(
        row.location_type.as_deref(),
        Some("货架"),
        "缺席键必须保持原值"
    );
    assert_eq!(row.location_code, "LOC-A01");
    assert_eq!(row.max_height, Some(dec("250.00")));

    // 2) 显式 null=落 NULL：location_type/is_color_managed 清空
    let (status, v) = call(
        &app,
        Method::PUT,
        &uri,
        Some(json!({ "location_type": null, "is_color_managed": null })),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "库位清空 update 失败: {v}");
    let row = location::Entity::find_by_id(id)
        .one(&db)
        .await
        .unwrap()
        .unwrap();
    assert!(
        row.location_type.is_none(),
        "发 null 后 location_type 必须为 NULL"
    );
    assert!(
        row.is_color_managed.is_none(),
        "发 null 后 is_color_managed 必须为 NULL"
    );
    assert_eq!(row.is_batch_managed, Some(true), "缺席键保持");

    // 3) 有值=覆盖：location_type 重新给值（同一列三态闭环）
    let (status, v) = call(
        &app,
        Method::PUT,
        &uri,
        Some(json!({ "location_type": "地面堆垛" })),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "库位覆盖 update 失败: {v}");
    let row = location::Entity::find_by_id(id)
        .one(&db)
        .await
        .unwrap()
        .unwrap();
    assert_eq!(row.location_type.as_deref(), Some("地面堆垛"), "有值=覆盖");
}

/// 退货域（采购退货单头）：reason_detail 同一列三态各一断言（真实 service 调用；
/// update_return 不加行锁，真库全链可跑）
#[tokio::test]
async fn purchase_return_header_update_tri_state_roundtrip_on_postgres() {
    let db = setup_test_db().await;
    // purchase_return / users / audit_logs 均为迁移产出的真表（update_with_audit 依赖
    // users（fetch_username）与 audit_logs（落审计行），夹具 TRUNCATE 后为空表即可）
    let created = purchase_return::ActiveModel {
        return_no: Set("PR-W3-001".to_string()),
        supplier_id: Set(1),
        return_date: Set(chrono::NaiveDate::from_ymd_opt(2026, 9, 1).unwrap()),
        reason_type: Set(Some("QUALITY".to_string())),
        reason_detail: Set(Some("染色疵点".to_string())),
        return_status: Set(Some(pr_status::DRAFT.to_string())),
        notes: Set(Some("原备注".to_string())),
        created_at: Set(Utc::now()),
        updated_at: Set(Utc::now()),
        ..Default::default()
    }
    .insert(&db)
    .await
    .unwrap();
    let id = created.id;
    let service = PurchaseReturnService::new(Arc::new(db.clone()));

    // 1) 键缺席=保持：全 None 提交，三列原值不动
    let after = service
        .update_return(id, UpdatePurchaseReturnRequest::default(), 9101)
        .await
        .unwrap();
    assert_eq!(after.reason_detail.as_deref(), Some("染色疵点"));
    assert_eq!(after.reason_type.as_deref(), Some("QUALITY"));
    assert_eq!(after.notes.as_deref(), Some("原备注"));

    // 2) 有值=覆盖
    let after = service
        .update_return(
            id,
            UpdatePurchaseReturnRequest {
                reason_type: None,
                reason_detail: Some(Some("缩水超标".to_string())),
                notes: None,
            },
            9101,
        )
        .await
        .unwrap();
    assert_eq!(
        after.reason_detail.as_deref(),
        Some("缩水超标"),
        "有值=覆盖"
    );

    // 3) 显式 null=落 NULL（同一列三态闭环）；notes 缺席保持
    let after = service
        .update_return(
            id,
            UpdatePurchaseReturnRequest {
                reason_type: None,
                reason_detail: Some(None),
                notes: None,
            },
            9101,
        )
        .await
        .unwrap();
    assert!(
        after.reason_detail.is_none(),
        "发 null 后 reason_detail 列必须为 NULL"
    );
    assert_eq!(after.reason_type.as_deref(), Some("QUALITY"), "缺席键保持");
    assert_eq!(after.notes.as_deref(), Some("原备注"), "缺席键保持");

    let reread = purchase_return::Entity::find_by_id(id)
        .one(&db)
        .await
        .unwrap()
        .unwrap();
    assert!(reread.reason_detail.is_none(), "DB 回读必须为 NULL");

    // 审计：after_snapshot 反映清空后的真实 NULL（不留假旧值）
    let log = audit_log::Entity::find()
        .filter(audit_log::Column::ResourceType.eq("auto_audit"))
        .filter(audit_log::Column::ResourceId.eq(id.to_string()))
        .order_by(audit_log::Column::Id, Order::Desc)
        .one(&db)
        .await
        .unwrap()
        .expect("update_with_audit 必须落审计行");
    let after_snapshot = log
        .after_snapshot
        .as_ref()
        .expect("after_snapshot 必须存在");
    assert!(
        after_snapshot
            .0
            .get("reason_detail")
            .map(|x| x.is_null())
            .unwrap_or(false),
        "审计 after_snapshot 必须反映清空后的真实 NULL: {}",
        after_snapshot.0
    );
}

// =========================================================
// C2) 拒清必须"零部分副作用"：同请求其他字段不得先落库
// =========================================================

/// NOT NULL 列显式 null 被拒时，同一请求里其他可空/有值字段必须一律不落库
/// （拒绝发生在任何 DB 访问前；若实现只挡列不改请求，就会出现"改了一半"的脏写）
#[tokio::test]
async fn explicit_null_on_not_null_rejected_and_sibling_fields_not_persisted() {
    // 批次编辑（service 直调）：color_no(null) + gram_weight(999.0) 同请求 → 整体拒绝
    let db = setup_test_db().await;
    seed_warehouses_and_products(&db).await;
    let stock = seed_stock(&db).await;
    let service = InventoryStockService::new(Arc::new(db.clone()));
    let err = service
        .update_batch_fields(
            stock.id,
            Some(None),
            None,
            None,
            Some(Some(999.0)),
            None,
            None,
            None,
            None,
        )
        .await
        .expect_err("NOT NULL 列 color_no 显式 null 必须被拒绝");
    expect_cleared_business_error(err, "色号不能清空", "color_no");
    let reread = inventory_stock::Entity::find_by_id(stock.id)
        .one(&db)
        .await
        .unwrap()
        .unwrap();
    assert_eq!(
        reread.gram_weight,
        Some(dec("180.00")),
        "同请求的 gram_weight 不得部分落库"
    );
    assert_eq!(reread.color_no, "C001");

    // 库位编辑（真实 handler PUT）：location_code(null) + location_type 同请求 → 400 且不动列
    let db2 = setup_test_db().await;
    seed_warehouses_and_products(&db2).await;
    let row = seed_location(&db2).await;
    let app = location_router(db2.clone());
    let (status, v) = call(
        &app,
        Method::PUT,
        &format!("/warehouse/locations/{}", row.id),
        Some(json!({ "location_code": null, "location_type": "不应落库" })),
    )
    .await;
    assert_eq!(status, StatusCode::BAD_REQUEST, "实际: {v}");
    assert_eq!(v["code"], "BUSINESS_ERROR");
    assert_eq!(v["message"], "库位编码不能清空：该字段为必填项");
    let after = location::Entity::find_by_id(row.id)
        .one(&db2)
        .await
        .unwrap()
        .unwrap();
    assert_eq!(
        after.location_type.as_deref(),
        Some("货架"),
        "同请求的 location_type 不得落库"
    );
    assert_eq!(after.location_code, "LOC-A01");
}

// =========================================================
// C3) 调拨明细（PUT /inventory/transfers/items/{item_id}）三态回环
// =========================================================

/// inventory_transfers / inventory_transfer_items 由迁移 m0001 建表
/// （FK：from/to_warehouse→warehouses；items→transfers），不再自建 DDL。
async fn seed_transfer_with_item(
    db: &DatabaseConnection,
) -> (inventory_transfer::Model, inventory_transfer_item::Model) {
    let transfer = inventory_transfer::ActiveModel {
        transfer_no: Set("TR-W3-001".to_string()),
        from_warehouse_id: Set(1),
        to_warehouse_id: Set(2),
        transfer_date: Set(Utc.with_ymd_and_hms(2026, 9, 1, 0, 0, 0).unwrap()),
        status: Set(transfer_status::PENDING.to_string()),
        total_quantity: Set(dec("10.00")),
        notes: Set(None),
        created_at: Set(Utc::now()),
        updated_at: Set(Utc::now()),
        total_amount: Set(Decimal::ZERO),
        ..Default::default()
    }
    .insert(db)
    .await
    .unwrap();
    let item = inventory_transfer_item::ActiveModel {
        transfer_id: Set(transfer.id),
        product_id: Set(5),
        quantity: Set(dec("10.00")),
        shipped_quantity: Set(Decimal::ZERO),
        received_quantity: Set(Decimal::ZERO),
        unit_cost: Set(Some(dec("3.50"))),
        notes: Set(Some("原备注".to_string())),
        created_at: Set(Utc::now()),
        updated_at: Set(Utc::now()),
        color_no: Set("C001".to_string()),
        dye_lot_no: Set(Some("DL001".to_string())),
        batch_no: Set("B7".to_string()),
        // 染色布行必须四维齐全（2026-10-02 裁定：出库第四维=匹号）：
        // 缺匹号的行在建 service 更新（追溯列进判定）时被正当拒绝
        // （p1 实证 "染色布必须提供匹号（color_no=C001 但 piece_no 为空）"），
        // 三态回环根本跑不到。正解是补齐夹具第四维，不是放宽源码门控或删步骤。
        piece_no: Set(Some("P001".to_string())),
        ..Default::default()
    }
    .insert(db)
    .await
    .unwrap();
    (transfer, item)
}

/// 调拨明细 update 三态回环：可空列（notes/unit_cost/dye_lot_no）缺席保持 / null 清空 /
/// 有值覆盖全部经真实 service + 真库回读；染色布行的缸号清空受四维追溯不变量拒绝
/// 且不产生部分写（DTO 声明的 unit_cost/追溯列在旧实现里被整体丢弃，属"假保存"缺陷）。
/// 种子染色布行自带匹号 P001（第四维，2026-10-02 裁定）：否则任何追溯列更新都会被
/// "染色布必须提供匹号"正当拒绝、三态回环不可达（p1 判责：夹具缺维非语义错）。
#[tokio::test]
async fn transfer_item_update_tri_state_roundtrip_on_postgres() {
    let db = setup_test_db().await;
    seed_warehouses_and_products(&db).await;
    let (_, seeded) = seed_transfer_with_item(&db).await;
    let item_id = seeded.id;
    let service = InventoryTransferService::new(Arc::new(db.clone()));
    let reload = || {
        let db = db.clone();
        async move {
            inventory_transfer_item::Entity::find_by_id(item_id)
                .one(&db)
                .await
                .unwrap()
                .unwrap()
        }
    };

    // 1) 键缺席=保持：只改 notes，unit_cost/dye_lot_no 原值不动
    service
        .update_item(
            item_id,
            UpdateInventoryTransferItemRequest {
                product_id: None,
                quantity: None,
                notes: Some(Some("三态新备注".to_string())),
                unit_cost: None,
                color_no: None,
                dye_lot_no: None,
                batch_no: None,
                piece_no: None,
            },
        )
        .await
        .unwrap();
    let row = reload().await;
    assert_eq!(row.notes.as_deref(), Some("三态新备注"), "有值=覆盖");
    assert_eq!(row.unit_cost, Some(dec("3.50")), "缺席键必须保持原值");
    assert_eq!(row.dye_lot_no.as_deref(), Some("DL001"));
    assert_eq!(
        row.piece_no.as_deref(),
        Some("P001"),
        "匹号（第四维）缺席键同样必须保持，不得被隐式清空"
    );

    // 2) 显式 null=清空：notes/unit_cost 同请求置 NULL，追溯列缺席不动
    service
        .update_item(
            item_id,
            UpdateInventoryTransferItemRequest {
                product_id: None,
                quantity: None,
                notes: Some(None),
                unit_cost: Some(None),
                color_no: None,
                dye_lot_no: None,
                batch_no: None,
                piece_no: None,
            },
        )
        .await
        .unwrap();
    let row = reload().await;
    assert!(row.notes.is_none(), "发 null 后 notes 必须为 NULL");
    assert!(row.unit_cost.is_none(), "发 null 后 unit_cost 必须为 NULL");
    assert_eq!(row.dye_lot_no.as_deref(), Some("DL001"), "缺席键保持");

    // 3) 有值=覆盖：dye_lot_no 换缸号（同一可空列三态闭环）
    service
        .update_item(
            item_id,
            UpdateInventoryTransferItemRequest {
                product_id: None,
                quantity: None,
                notes: None,
                unit_cost: None,
                color_no: None,
                dye_lot_no: Some(Some("DL009".to_string())),
                batch_no: None,
                piece_no: None,
            },
        )
        .await
        .unwrap();
    let row = reload().await;
    assert_eq!(row.dye_lot_no.as_deref(), Some("DL009"), "有值=覆盖");
    assert_eq!(
        row.piece_no.as_deref(),
        Some("P001"),
        "改缸号时匹号缺席必须保持（四维逐列独立三态，不得连带清空）"
    );

    // 4) 染色布（生效色号非空）清空缸号：违反四维追溯不变量，如实拒绝且明细零写
    let err = service
        .update_item(
            item_id,
            UpdateInventoryTransferItemRequest {
                product_id: None,
                quantity: None,
                notes: Some(Some("不该落库".to_string())),
                unit_cost: None,
                color_no: None,
                dye_lot_no: Some(None),
                batch_no: None,
                piece_no: None,
            },
        )
        .await
        .expect_err("染色布行清空缸号必须被拒绝（不是静默放行也不是塌成保持）");
    assert!(
        matches!(&err, AppError::ValidationErrorDisplayable(m) if m.contains("染色布必须提供缸号")),
        "应为四维追溯校验错误且外显真实原因，实际: {err:?}"
    );
    let row = reload().await;
    assert_eq!(row.dye_lot_no.as_deref(), Some("DL009"), "拒绝后缸号保持");
    assert!(row.notes.is_none(), "同请求的 notes 不得部分落库");
}

/// 白坯调拨明细（色号空串）更新的真库回读锁（复审 阻断项 B-1 的回归锁）。
///
/// 缸号列 DDL 是 `NOT NULL DEFAULT ''`（migration/src/domain/system/mod.rs:292，同迁移
/// 已把历史 NULL 回填 ''），所以 DB 里"无缸号"的合法表示是空串。原 update 实现
/// `active.dye_lot_no = Set(dims.dye_lot_no)` 在白坯归一为 None 时生成
/// `SET dye_lot_no = NULL` → PG 23502 → 被拍平成脱敏 DATABASE_ERROR 500，
/// 即"白坯明细只改个批次也必 500"（本文件 C3 只播染色布行，抓不到这条路径）。
/// 建单路径用 NotSet 让 DEFAULT '' 生效，update 路径不能照抄——NotSet 会让
/// "染色改回白坯"残留旧缸号（步骤 3 钉死），故正确写法是显式落 ''。
#[tokio::test]
async fn transfer_item_update_greige_dye_lot_lands_empty_string_on_postgres() {
    let db = setup_test_db().await;
    seed_warehouses_and_products(&db).await;
    let transfer = inventory_transfer::ActiveModel {
        transfer_no: Set("TR-W3-GREIGE".to_string()),
        from_warehouse_id: Set(1),
        to_warehouse_id: Set(2),
        transfer_date: Set(Utc.with_ymd_and_hms(2026, 9, 2, 0, 0, 0).unwrap()),
        status: Set(transfer_status::PENDING.to_string()),
        total_quantity: Set(dec("8.00")),
        notes: Set(None),
        total_amount: Set(Decimal::ZERO),
        created_at: Set(Utc::now()),
        updated_at: Set(Utc::now()),
        ..Default::default()
    }
    .insert(&db)
    .await
    .unwrap();
    // 白坯行的真实形态：色号空串 + 缸号空串（建单路径 NotSet 让 DEFAULT '' 生效）
    let item = inventory_transfer_item::ActiveModel {
        transfer_id: Set(transfer.id),
        product_id: Set(5),
        quantity: Set(dec("8.00")),
        shipped_quantity: Set(Decimal::ZERO),
        received_quantity: Set(Decimal::ZERO),
        unit_cost: Set(None),
        notes: Set(None),
        created_at: Set(Utc::now()),
        updated_at: Set(Utc::now()),
        color_no: Set(String::new()),
        dye_lot_no: Set(Some(String::new())),
        batch_no: Set("B7".to_string()),
        piece_no: Set(None),
        ..Default::default()
    }
    .insert(&db)
    .await
    .unwrap();
    let item_id = item.id;
    let service = InventoryTransferService::new(Arc::new(db.clone()));
    let reload = || {
        let db = db.clone();
        async move {
            inventory_transfer_item::Entity::find_by_id(item_id)
                .one(&db)
                .await
                .unwrap()
                .unwrap()
        }
    };

    // 1) 白坯行只改批次（追溯键被提交 → 进四维判定，缸号归一 None）：必须成功且落 ''
    service
        .update_item(
            item_id,
            UpdateInventoryTransferItemRequest {
                product_id: None,
                quantity: None,
                notes: None,
                unit_cost: None,
                color_no: None,
                dye_lot_no: None,
                batch_no: Some(Some("B8".to_string())),
                piece_no: None,
            },
        )
        .await
        .expect("白坯明细改批次必须成功（修复前此处 500：SET dye_lot_no = NULL 撞 NOT NULL）");
    let row = reload().await;
    assert_eq!(row.batch_no, "B8", "有值=覆盖");
    assert_eq!(
        row.dye_lot_no.as_deref(),
        Some(""),
        "白坯缸号必须落空串（DB 的\"无缸号\"表示），既不得为 NULL 也不得报错"
    );

    // 2) 白坯改染色：四维（色/缸/批/匹）齐全如实覆盖
    service
        .update_item(
            item_id,
            UpdateInventoryTransferItemRequest {
                product_id: None,
                quantity: None,
                notes: None,
                unit_cost: None,
                color_no: Some(Some("C77".to_string())),
                dye_lot_no: Some(Some("DL77".to_string())),
                batch_no: Some(Some("B8".to_string())),
                piece_no: Some(Some("P-77".to_string())),
            },
        )
        .await
        .expect("染色布四维齐全必须放行");
    let row = reload().await;
    assert_eq!(row.color_no, "C77");
    assert_eq!(row.dye_lot_no.as_deref(), Some("DL77"));
    assert_eq!(row.piece_no.as_deref(), Some("P-77"));

    // 3) 染色改回白坯（显式清空缸号+匹号、色号清空）：缸号必须真的变成 ''，
    //    不得像建单路径那样靠 NotSet 保持——残留 DL77 就是数据说谎
    service
        .update_item(
            item_id,
            UpdateInventoryTransferItemRequest {
                product_id: None,
                quantity: None,
                notes: None,
                unit_cost: None,
                color_no: Some(Some(String::new())),
                dye_lot_no: Some(None),
                batch_no: None,
                piece_no: Some(None),
            },
        )
        .await
        .expect("色号清空为白坯后清空缸号/匹号必须放行");
    let row = reload().await;
    assert_eq!(row.color_no, "", "色号清空落库");
    assert_eq!(
        row.dye_lot_no.as_deref(),
        Some(""),
        "改回白坯后缸号必须清空为 ''，不得残留 DL77"
    );
    assert!(
        row.piece_no.is_none(),
        "匹号是 m0066 可空列，显式 null 落 NULL"
    );
    assert_eq!(row.batch_no, "B8", "缺席键保持原批次");
}

// =========================================================
// D) 收货链路三态全链（update_receipt 服务链 lock_exclusive + update_with_audit，
//    路线一真库通道上与其余回环同桶执行，生产方言真实触发）
// =========================================================

/// 真库硬前提自检：夹具契约要求 TEST_DATABASE_URL → 已迁移 PostgreSQL，
/// 缺变量/指 sqlite 由 setup_test_db 直接 panic；此处再显式锁一次方言，
/// 防止未来有人把夹具改回静默回退（条件跳过/静默 pass 是本仓定性的假绿根因）。
async fn require_postgres(db: &DatabaseConnection) {
    assert_eq!(
        db.get_database_backend(),
        DbBackend::Postgres,
        "本用例必须跑在已迁移的 PostgreSQL（TEST_DATABASE_URL）上；禁止 sqlite 回退假绿"
    );
}

#[tokio::test]
async fn live_receipt_update_null_clears_nullable_and_absent_keeps() {
    use bingxi_backend::models::purchase_receipt;
    let db = test_common::setup_test_db().await;
    require_postgres(&db).await;
    // FK 前置：purchase_receipt.warehouse_id → warehouses（业务表，TRUNCATE 后需自种）；
    // supplier_id=1 由迁移 m0015 播种（sealed 参照表，不清空）
    seed_warehouses_and_products(&db).await;
    // 操作人前置：生产 update_receipt 在 owner 检查前先调 is_admin_user(user_id)
    // （purchase_receipt_ops/auth.rs，查不到用户即 NotFound("用户不存在")），随后
    // update_with_audit 的 fetch_username 同样按 user_id 回查 users。users 不在夹具的
    // SEALED_REFERENCE_TABLES（迁移不播种，test_common.rs 口径），夹具 TRUNCATE 后为空，
    // 用例必须自插操作人——配齐口径同
    // contract_wave5_receipt_return_three_state_test.rs::seeded_db。
    exec_pg(
        &db,
        r#"INSERT INTO users (id,username,password_hash,is_active,is_totp_enabled,department_id,created_at,updated_at) VALUES
             (9101,'w3_receipt_op','x',TRUE,FALSE,1,'2026-01-01T00:00:00Z','2026-01-01T00:00:00Z')"#,
    )
    .await;
    let suffix = Utc::now().timestamp_nanos_opt().unwrap();

    let created = purchase_receipt::ActiveModel {
        receipt_no: Set(format!("GR-W3-{suffix}")),
        supplier_id: Set(1),
        receipt_date: Set(chrono::NaiveDate::from_ymd_opt(2026, 9, 1).unwrap()),
        warehouse_id: Set(1),
        department_id: Set(Some(1)),
        inspector_id: Set(Some(2)),
        inspection_status: Set("PENDING".to_string()),
        receipt_status: Set(receipt_status::DRAFT.to_string()),
        total_quantity: Set(Decimal::ZERO),
        total_quantity_alt: Set(Decimal::ZERO),
        total_amount: Set(Decimal::ZERO),
        notes: Set(Some("待清空的备注".to_string())),
        attachment_urls: Set(Some(vec!["a.pdf".to_string()])),
        created_by: Set(9101),
        created_at: Set(Utc::now()),
        updated_at: Set(Utc::now()),
        ..Default::default()
    }
    .insert(&db)
    .await
    .unwrap();

    let service = PurchaseReceiptService::new(Arc::new(db.clone()));
    // 三态：notes/inspector_id/attachment_urls 显式 null 清空；receipt_date/supplier_id/
    // department_id 键缺席保持
    let after = service
        .update_receipt(
            created.id,
            purchase_receipt_dto::UpdatePurchaseReceiptRequest {
                notes: Some(None),
                inspector_id: Some(None),
                attachment_urls: Some(None),
                ..Default::default()
            },
            9101,
        )
        .await
        .expect("入库单三态 update 失败");
    assert!(after.notes.is_none(), "发 null 后 notes 必须为 NULL");
    assert!(
        after.inspector_id.is_none(),
        "发 null 后 inspector_id 必须为 NULL"
    );
    assert!(
        after.attachment_urls.is_none(),
        "发 null 后 attachment_urls 必须为 NULL"
    );
    assert_eq!(after.department_id, Some(1), "缺席键必须保持原值");

    let reread = purchase_receipt::Entity::find_by_id(created.id)
        .one(&db)
        .await
        .unwrap()
        .unwrap();
    assert!(reread.notes.is_none(), "DB 回读 notes 必须为 NULL");
    assert_eq!(reread.supplier_id, 1);

    // 同一列再覆盖：notes 有值=覆盖
    let after = service
        .update_receipt(
            created.id,
            purchase_receipt_dto::UpdatePurchaseReceiptRequest {
                notes: Some(Some("三态回环新备注".to_string())),
                ..Default::default()
            },
            9101,
        )
        .await
        .unwrap();
    assert_eq!(after.notes.as_deref(), Some("三态回环新备注"), "有值=覆盖");
}
