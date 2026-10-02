//! 任务 #169 第 5 波（仓储/调拨/采购收货/退货域）：更新端点可空列三态清空语义收口
//! （对齐 RFC 7386 JSON Merge Patch，范式先例 `department_handler.rs` 与提交 ab257ea7；
//! 与 wave3 互补：wave3 侧重库存域回环与本域 service 级拒清，本文件锁定
//! 收货/退货/仓储/调拨四子域的 DTO 三态形状、真实 sqlite service 回环、
//! 拒清零部分副作用、以及 handler `map_err(internal/bad_request)`→`?` 透传后
//! 错误按真正责任模块定性（业务拒绝/404 不再被拍平成 500/400））
//!
//! 覆盖端点与可空列（DDL/模型证据见各 `backend/src/models/*.rs`，普查表见 PR 描述）：
//! - PUT /warehouses/{id}：address/manager→manager_id/phone/contact_person/capacity/
//!   warehouse_type 可空；name/is_default/status(→is_active) NOT NULL 拒清
//! - PUT /warehouse/locations/{id}：location_type/max_weight/max_height/
//!   is_batch_managed/is_color_managed 可空；location_code NOT NULL 拒清（入口）
//! - PUT /purchase/receipts/{id}：department_id/inspector_id/notes/attachment_urls 可空；
//!   supplier_id/receipt_date NOT NULL 拒清
//! - PUT /purchase/receipts/{id}/items/{item_id}：batch_no/color_code/lot_no/grade/
//!   gram_weight/width/quantity_alt/unit_price/location_code/notes/piece_no 可空；
//!   line_no/product_id/material_code/material_name/quantity NOT NULL 拒清
//! - PUT /purchase/returns/{id}：reason_type/reason_detail/notes 全可空
//! - PUT /purchase/returns/{id}/items/{item_id}：notes 可空；追溯三列
//!   color_no/dye_lot_no/batch_no 为 NOT NULL DEFAULT ''（"改回白坯"提交空串而非 null）、
//!   数量/单价/税率/折扣率 NOT NULL 拒清（明细为原位 update，非整表重插——
//!   历史缺陷"更新重复插行洗掉四维/辅量"由行数与原值双断言锁死）
//! - PUT /sales/returns/{id}：order_id/notes 可空；customer_id/return_date/warehouse_id/
//!   reason_type NOT NULL 拒清；reason_detail 虚拟入参单独清空显式拒绝
//! - PUT /sales/returns/{id}/items/{item_id}：reason→notes 可空；quantity/unit_price 拒清
//! - PUT /inventory/transfers/{id}、items/{item_id}：notes/unit_cost/dye_lot_no 可空
//!
//! 覆盖策略（无 mock；路线一 #4669 判责：表结构唯一来源 = backend/migration，
//! 不再自建 sqlite 同构表——quantity/total_quantity 等 DECIMAL 列被写成 TEXT
//! 即本文件连坐解码红的根因）：
//! - 纯 serde：DTO 三态形状锁（缺席=None / null=Some(None) / 有值=Some(Some(v))），
//!   并反向锁 serde_json 默认行为（裸双层 Option 会把显式 null 折成外层 None——
//!   "只声明双层 Option 不挂适配器 = 清空静默失效"的根因形状）；
//! - 真 PostgreSQL（test_common::setup_test_db）+ 真实 service/handler 回环：
//!   仓储 update、采购退货单头/明细、调拨明细 handler 信封
//!   （以上链路均不加行锁，常规分片可跑真实写读）；
//!   FK 父行自种子（裁定 R1）：purchase_return_item.product_id → products、
//!   inventory_transfers.from/to_warehouse_id → warehouses、
//!   sales_return.customer_id → customers（customers.owner_id → users）、
//!   purchase_receipt.supplier_id → suppliers / warehouse_id → warehouses、
//!   明细 product_id → products；users/audit_logs 由真表提供，不再自建。
//! - `#[ignore]` 活库（TEST_DATABASE_URL→PG，ci-test-rust-ignored 执行）：
//!   销退明细与收货明细（服务链 `lock_exclusive()`，端到端真跑只在已迁移 PG，
//!   夹具缺变量/指向 sqlite 直接 panic，无静默回退）。

mod test_common;

use axum::{
    Router,
    body::Body,
    http::{Method, Request, StatusCode},
    response::Response,
};
use bingxi_backend::container::AppState;
use bingxi_backend::handlers::{
    inventory_transfer_handler, sales_return_handler, warehouse_handler,
};
use bingxi_backend::models::inventory_transfer;
use bingxi_backend::models::inventory_transfer_item;
use bingxi_backend::models::purchase_receipt;
use bingxi_backend::models::purchase_receipt_item;
use bingxi_backend::models::purchase_return;
use bingxi_backend::models::purchase_return_item;
use bingxi_backend::models::sales_return;
use bingxi_backend::models::sales_return_item;
use bingxi_backend::models::status::purchase_inventory::{
    inventory_transfer as transfer_status, purchase_receipt as receipt_status,
    purchase_return as pr_status,
};
use bingxi_backend::models::status::sales_return as sr_status;
use bingxi_backend::models::warehouse;
use bingxi_backend::services::inv::{
    UpdateInventoryTransferItemRequest, UpdateInventoryTransferRequest,
};
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
    PaginatorTrait, QueryFilter, Set, Statement,
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

async fn call_put(app: &Router, uri: &str, body: Value) -> (StatusCode, Value) {
    let req = Request::builder()
        .method(Method::PUT)
        .uri(uri)
        .header("content-type", "application/json")
        .body(Body::from(body.to_string()))
        .expect("构造 PUT 请求失败");
    let resp: Response = app.clone().oneshot(req).await.unwrap();
    let status = resp.status();
    let bytes = axum::body::to_bytes(resp.into_body(), 4 * 1024 * 1024)
        .await
        .unwrap();
    (status, serde_json::from_slice(&bytes).unwrap())
}

async fn sqlite_db() -> DatabaseConnection {
    sea_orm::Database::connect("sqlite::memory:")
        .await
        .expect("sqlite::memory: 连接失败")
}

async fn exec(db: &DatabaseConnection, sql: &str) {
    db.execute_raw(Statement::from_sql_and_values(
        DbBackend::Sqlite,
        sql,
        Vec::<sea_orm::Value>::new(),
    ))
    .await
    .unwrap_or_else(|e| panic!("DDL 执行失败: {e}\nSQL: {sql}"));
}

/// update_with_audit 依赖 users（fetch_username）与 audit_logs（落审计行）——
/// 列集与先例 contract_wave3_explicit_null_clear_test.rs 逐列一致
const USERS_DDL: &str = r#"CREATE TABLE users (
    id INTEGER PRIMARY KEY,
    username TEXT NOT NULL, password_hash TEXT NOT NULL,
    real_name TEXT, avatar TEXT, email TEXT, phone TEXT,
    role_id INTEGER, department_id INTEGER, is_active INTEGER NOT NULL,
    totp_secret TEXT, is_totp_enabled INTEGER NOT NULL, totp_recovery_codes TEXT,
    last_login_at TEXT, password_changed_at TEXT, agreed_to_terms_at TEXT,
    created_at TEXT NOT NULL, updated_at TEXT NOT NULL,
    gender TEXT, birth_date TEXT
)"#;

const AUDIT_LOGS_DDL: &str = r#"CREATE TABLE audit_logs (
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

/// warehouses 全列最小 DDL（列与 `models/warehouse.rs::Model` 逐列对应，
/// bool 用 INTEGER、DateTime 用 TEXT——先例形态；
/// NOT NULL：warehouse_code/name/is_default/is_active/created_at/updated_at）
const WAREHOUSES_DDL: &str = r#"CREATE TABLE warehouses (
    id INTEGER PRIMARY KEY,
    warehouse_code TEXT NOT NULL, name TEXT NOT NULL,
    address TEXT, city TEXT, province TEXT, country TEXT, postal_code TEXT,
    phone TEXT, contact_person TEXT,
    is_default INTEGER NOT NULL, email TEXT, manager_id INTEGER,
    is_active INTEGER NOT NULL, notes TEXT, warehouse_type TEXT, capacity INTEGER,
    created_at TEXT NOT NULL, updated_at TEXT NOT NULL
)"#;

async fn seed_warehouse(db: &DatabaseConnection) -> warehouse::Model {
    warehouse::ActiveModel {
        warehouse_code: Set("WH-W5-001".to_string()),
        name: Set("胚布一仓".to_string()),
        address: Set(Some("老地址".to_string())),
        phone: Set(Some("13800000000".to_string())),
        contact_person: Set(Some("张三".to_string())),
        is_default: Set(false),
        manager_id: Set(Some(77)),
        is_active: Set(true),
        notes: Set(Some("原备注".to_string())),
        warehouse_type: Set(Some("greige".to_string())),
        capacity: Set(Some(50)),
        created_at: Set(Utc::now()),
        updated_at: Set(Utc::now()),
        ..Default::default()
    }
    .insert(db)
    .await
    .unwrap()
}

/// purchase_return DDL 与 wave3 逐列一致；purchase_return_item 列与
/// `models/purchase_return_item.rs::Model` 逐列对应（追溯三列 NOT NULL，见模型头注）
const PURCHASE_RETURN_DDL: &str = r#"CREATE TABLE purchase_return (
    id INTEGER PRIMARY KEY,
    return_no TEXT NOT NULL, receipt_id INTEGER, order_id INTEGER,
    supplier_id INTEGER NOT NULL, return_date TEXT NOT NULL,
    warehouse_id INTEGER, department_id INTEGER,
    reason_type TEXT, reason_detail TEXT, return_status TEXT,
    total_quantity TEXT, total_quantity_alt TEXT, total_amount TEXT,
    notes TEXT, created_by INTEGER, created_at TEXT NOT NULL,
    updated_by INTEGER, updated_at TEXT NOT NULL,
    approved_by INTEGER, approved_at TEXT, rejected_reason TEXT
)"#;

const PURCHASE_RETURN_ITEM_DDL: &str = r#"CREATE TABLE purchase_return_item (
    id INTEGER PRIMARY KEY,
    return_id INTEGER NOT NULL, line_no INTEGER NOT NULL, product_id INTEGER NOT NULL,
    quantity TEXT NOT NULL, quantity_alt TEXT NOT NULL,
    unit_price TEXT NOT NULL, unit_price_foreign TEXT NOT NULL,
    discount_percent TEXT NOT NULL, tax_percent TEXT NOT NULL,
    subtotal TEXT NOT NULL, tax_amount TEXT NOT NULL,
    discount_amount TEXT NOT NULL, total_amount TEXT NOT NULL,
    notes TEXT, created_at TEXT NOT NULL, updated_at TEXT NOT NULL,
    color_no TEXT NOT NULL, dye_lot_no TEXT NOT NULL, batch_no TEXT NOT NULL
)"#;

async fn seed_purchase_return_with_item(
    db: &DatabaseConnection,
) -> (purchase_return::Model, purchase_return_item::Model) {
    let r = purchase_return::ActiveModel {
        return_no: Set("PRT-W5-001".to_string()),
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
    .insert(db)
    .await
    .unwrap();
    let item = purchase_return_item::ActiveModel {
        return_id: Set(r.id),
        line_no: Set(1),
        product_id: Set(9),
        quantity: Set(dec("10.00")),
        quantity_alt: Set(dec("5.50")),
        unit_price: Set(dec("5.00")),
        unit_price_foreign: Set(Decimal::ZERO),
        discount_percent: Set(Decimal::ZERO),
        tax_percent: Set(Decimal::ZERO),
        subtotal: Set(dec("50.00")),
        tax_amount: Set(Decimal::ZERO),
        discount_amount: Set(Decimal::ZERO),
        total_amount: Set(dec("50.00")),
        notes: Set(Some("原明细备注".to_string())),
        created_at: Set(Utc::now()),
        updated_at: Set(Utc::now()),
        // 四维追溯列（NOT NULL DEFAULT ''）：色号/缸号/批次预置非空值
        color_no: Set("C001".to_string()),
        dye_lot_no: Set("DL001".to_string()),
        batch_no: Set("B5".to_string()),
        ..Default::default()
    }
    .insert(db)
    .await
    .unwrap();
    (r, item)
}

const INVENTORY_TRANSFERS_DDL: &str = r#"CREATE TABLE inventory_transfers (
    id INTEGER PRIMARY KEY,
    transfer_no TEXT NOT NULL, from_warehouse_id INTEGER NOT NULL,
    to_warehouse_id INTEGER NOT NULL, transfer_date TEXT NOT NULL,
    status TEXT NOT NULL, total_quantity TEXT, notes TEXT,
    created_by INTEGER, approved_by INTEGER, approved_at TEXT,
    shipped_at TEXT, received_at TEXT,
    created_at TEXT NOT NULL, updated_at TEXT NOT NULL,
    approval_level TEXT, approved_by_role TEXT, total_amount TEXT
)"#;

const INVENTORY_TRANSFER_ITEMS_DDL: &str = r#"CREATE TABLE inventory_transfer_items (
    id INTEGER PRIMARY KEY, transfer_id INTEGER NOT NULL, product_id INTEGER NOT NULL,
    quantity TEXT NOT NULL, shipped_quantity TEXT, received_quantity TEXT,
    unit_cost TEXT, notes TEXT, created_at TEXT NOT NULL, updated_at TEXT NOT NULL,
    color_no TEXT NOT NULL, dye_lot_no TEXT, batch_no TEXT NOT NULL
)"#;

async fn seed_transfer(
    db: &DatabaseConnection,
    status: &str,
) -> (inventory_transfer::Model, inventory_transfer_item::Model) {
    let transfer = inventory_transfer::ActiveModel {
        transfer_no: Set(format!(
            "TR-W5-{}",
            Utc::now().timestamp_nanos_opt().unwrap()
        )),
        from_warehouse_id: Set(1),
        to_warehouse_id: Set(2),
        transfer_date: Set(Utc.with_ymd_and_hms(2026, 9, 1, 0, 0, 0).unwrap()),
        status: Set(status.to_string()),
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
        batch_no: Set("B5".to_string()),
        ..Default::default()
    }
    .insert(db)
    .await
    .unwrap();
    (transfer, item)
}

/// PUT /inventory/transfers/items/{item_id}：handler 无 AuthContext 入参，裸 Router 即可
fn transfer_item_router(db: DatabaseConnection) -> Router {
    let state = AppState {
        db: Arc::new(db),
        ..Default::default()
    };
    Router::new()
        .route(
            "/inventory/transfers/items/{item_id}",
            axum::routing::put(inventory_transfer_handler::update_item),
        )
        .with_state(state)
}

fn expect_cleared_business_error(err: AppError, keyword: &str, field: &str) {
    assert!(
        matches!(&err, AppError::BusinessErrorDisplayable(m) if m.contains(keyword)),
        "字段 {field} 显式 null 应被业务错误拒绝且文案含「{keyword}」，实际: {err:?}"
    );
}

// =========================================================
// A) DTO 三态形状锁（纯 serde，无 DB）：
//    缺席=None（保持）、null=Some(None)（清空）、有值=Some(Some(v))（覆盖）。
//    只声明双层 Option 不挂 double_option 适配器 = 显式 null 被 serde_json
//    折成外层 None（清空静默失效），本组用例即该缺陷的形状锁。
// =========================================================

/// 反向锁：serde_json 对裸 `Option<Option<T>>` 的默认行为把 JSON null 折成外层 None
/// ——这正是"只声明双层 Option 不挂适配器 = 清空静默失效"的根因形状
#[test]
fn bare_double_option_folds_explicit_null_root_cause_shape_lock() {
    let plain: Option<Option<String>> =
        serde_json::from_value(json!(null)).expect("裸双层 Option 可解 null");
    assert_eq!(
        plain, None,
        "serde_json 默认把显式 null 折成外层 None——必须挂 double_option 适配器才可区分"
    );
}

#[test]
fn warehouse_dto_shape_lock() {
    let req: warehouse_handler::UpdateWarehouseRequest = serde_json::from_value(json!({
        "name": null,
        "address": null,
        "manager": null,
        "capacity": 120,
        "is_default": null,
        "status": null,
        "warehouse_type": "finished",
    }))
    .expect("仓库 update DTO 三态反序列化失败");
    assert!(
        matches!(req.name, Some(None)),
        "NOT NULL 列 name 显式 null 必须可辨（交 service 拒清），不得塌成保持"
    );
    assert!(
        matches!(req.address, Some(None)),
        "可空列 address null=清空"
    );
    assert!(
        matches!(req.manager, Some(None)),
        "manager→manager_id null=清除经理"
    );
    assert!(matches!(req.capacity, Some(Some(120))));
    assert!(
        matches!(req.is_default, Some(None)),
        "非 Option bool 列显式 null 可辨"
    );
    assert!(
        matches!(req.status, Some(None)),
        "status→is_active 显式 null 可辨"
    );
    assert!(matches!(&req.warehouse_type, Some(Some(v)) if v == "finished"));
    assert!(req.phone.is_none(), "缺席键必须是 None（保持原值）");
    assert!(req.contact_person.is_none());
}

#[test]
fn location_dto_shape_lock() {
    let req: warehouse_handler::UpdateLocationRequest = serde_json::from_value(json!({
        "location_code": null,
        "location_type": null,
        "max_height": null,
        "is_batch_managed": true,
    }))
    .expect("库位 update DTO 三态反序列化失败");
    assert!(
        matches!(req.location_code, Some(None)),
        "NOT NULL location_code 显式 null 可辨（入口拒清）"
    );
    assert!(matches!(req.location_type, Some(None)));
    assert!(matches!(req.max_height, Some(None)));
    assert!(matches!(req.is_batch_managed, Some(Some(true))));
    assert!(req.max_weight.is_none());
    assert!(req.is_color_managed.is_none());
}

#[test]
fn receipt_header_and_item_dto_shape_lock() {
    let header: purchase_receipt_dto::UpdatePurchaseReceiptRequest =
        serde_json::from_value(json!({
            "supplier_id": null,
            "department_id": null,
            "inspector_id": 7,
            "notes": null,
            "attachment_urls": null,
        }))
        .expect("入库单头 update DTO 三态反序列化失败");
    assert!(
        matches!(header.supplier_id, Some(None)),
        "NOT NULL supplier_id 显式 null 可辨"
    );
    assert!(
        matches!(header.department_id, Some(None)),
        "可空 department_id null=清空"
    );
    assert!(matches!(header.inspector_id, Some(Some(7))));
    assert!(matches!(header.notes, Some(None)));
    assert!(matches!(header.attachment_urls, Some(None)));
    assert!(header.receipt_date.is_none(), "缺席键=保持原值");

    let item: purchase_receipt_dto::UpdateReceiptItemRequest = serde_json::from_value(json!({
        "quantity": null,
        "material_name": null,
        "grade": null,
        "quantity_alt": null,
        "unit_price": null,
        "location_code": "A-02",
        "piece_no": null,
    }))
    .expect("入库明细 update DTO 三态反序列化失败");
    assert!(
        matches!(item.quantity, Some(None)),
        "NOT NULL quantity 显式 null 可辨"
    );
    assert!(
        matches!(item.material_name, Some(None)),
        "NOT NULL material_name 显式 null 可辨"
    );
    assert!(matches!(item.grade, Some(None)), "四维·等级 null=清空");
    assert!(
        matches!(item.quantity_alt, Some(None)),
        "辅量 quantity_alt null=清空"
    );
    assert!(matches!(item.unit_price, Some(None)));
    assert!(matches!(&item.location_code, Some(Some(v)) if v == "A-02"));
    assert!(matches!(item.piece_no, Some(None)));
    assert!(item.line_no.is_none());
    assert!(item.batch_no.is_none());
    assert!(item.color_code.is_none());
    assert!(item.lot_no.is_none());
}

#[test]
fn purchase_return_header_and_item_dto_shape_lock() {
    let header: UpdatePurchaseReturnRequest = serde_json::from_value(json!({
        "reason_type": null,
        "reason_detail": "缩水超标",
    }))
    .expect("采购退货单头 update DTO 三态反序列化失败");
    assert!(
        matches!(header.reason_type, Some(None)),
        "可空 reason_type null=清空"
    );
    assert!(matches!(&header.reason_detail, Some(Some(v)) if v == "缩水超标"));
    assert!(header.notes.is_none(), "缺席键=保持原值");

    let item: UpdateReturnItemRequest = serde_json::from_value(json!({
        "quantity_returned": null,
        "unit_price": null,
        "tax_rate": null,
        "discount_percent": null,
        "color_no": null,
        "dye_lot_no": null,
        "batch_no": null,
        "material_id": 3,
    }))
    .expect("采购退货明细 update DTO 三态反序列化失败");
    assert!(
        matches!(item.quantity_returned, Some(None)),
        "NOT NULL 数量显式 null 可辨"
    );
    assert!(
        matches!(item.unit_price, Some(None)),
        "NOT NULL 单价显式 null 可辨"
    );
    assert!(matches!(item.tax_rate, Some(None)));
    assert!(matches!(item.discount_percent, Some(None)));
    assert!(
        matches!(item.color_no, Some(None)),
        "四维·色号（NOT NULL DEFAULT ''）null 可辨并被拒"
    );
    assert!(matches!(item.dye_lot_no, Some(None)), "四维·缸号 null 可辨");
    assert!(matches!(item.batch_no, Some(None)), "四维·批次 null 可辨");
    assert!(matches!(item.material_id, Some(Some(3))));
    assert!(item.line_no.is_none());
    assert!(item.notes.is_none());
}

#[test]
fn sales_return_header_and_item_dto_shape_lock() {
    let header: UpdateSalesReturnRequest = serde_json::from_value(json!({
        "order_id": null,
        "customer_id": null,
        "return_date": null,
        "reason_detail": null,
        "notes": null,
    }))
    .expect("销售退货单头 update DTO 三态反序列化失败");
    assert!(
        matches!(header.order_id, Some(None)),
        "可空 sales_order_id null=解除关联"
    );
    assert!(
        matches!(header.customer_id, Some(None)),
        "NOT NULL customer_id null 可辨"
    );
    assert!(matches!(header.return_date, Some(None)));
    assert!(
        matches!(header.reason_detail, Some(None)),
        "虚拟入参 null 可辨（单独清空将被显式拒绝）"
    );
    assert!(
        matches!(header.notes, Some(None)),
        "notes→remarks 可空 null=清空"
    );
    assert!(header.warehouse_id.is_none());
    assert!(header.reason_type.is_none());

    let item: sales_return_handler::UpdateReturnItemRequest = serde_json::from_value(json!({
        "quantity": null,
        "reason": null,
    }))
    .expect("销售退货明细 update DTO 三态反序列化失败");
    assert!(
        matches!(item.quantity, Some(None)),
        "NOT NULL quantity null 可辨"
    );
    assert!(
        matches!(item.reason, Some(None)),
        "reason→notes 可空 null=清空"
    );
    assert!(item.unit_price.is_none());
}

#[test]
fn transfer_header_and_item_dto_shape_lock() {
    let header: UpdateInventoryTransferRequest = serde_json::from_value(json!({
        "notes": null,
        "status": null,
    }))
    .expect("调拨单头 update DTO 三态反序列化失败");
    assert!(matches!(header.notes, Some(None)), "可空 notes null=清空");
    assert!(
        matches!(header.status, Some(None)),
        "非 Option status 列 null 可辨并被拒"
    );
    assert!(header.items.is_none());

    let item: UpdateInventoryTransferItemRequest = serde_json::from_value(json!({
        "quantity": null,
        "unit_cost": null,
        "dye_lot_no": null,
        "notes": null,
        "color_no": "RED-7",
    }))
    .expect("调拨明细 update DTO 三态反序列化失败");
    assert!(
        matches!(item.quantity, Some(None)),
        "NOT NULL quantity null 可辨"
    );
    assert!(
        matches!(item.unit_cost, Some(None)),
        "可空 unit_cost null=清空（旧单层 Option 形态会塌成保持）"
    );
    assert!(matches!(item.dye_lot_no, Some(None)));
    assert!(matches!(item.notes, Some(None)));
    assert!(matches!(&item.color_no, Some(Some(v)) if v == "RED-7"));
    assert!(item.product_id.is_none());
    assert!(item.batch_no.is_none());
}

// =========================================================
// B) 仓储子域：真实 sqlite 三态回环 + NOT NULL 拒清零部分副作用
// =========================================================

#[tokio::test]
async fn warehouse_update_tri_state_roundtrip_on_sqlite() {
    let db = sqlite_db().await;
    exec(&db, WAREHOUSES_DDL).await;
    exec(&db, USERS_DDL).await;
    exec(&db, AUDIT_LOGS_DDL).await;
    let seeded = seed_warehouse(&db).await;
    let id = seeded.id;
    let service = WarehouseService::new(Arc::new(db.clone()));

    // 1) 键缺席=保持：只覆盖 capacity，address/manager/phone 原值不动
    service
        .update(
            id,
            9101,
            warehouse_handler::UpdateWarehouseRequest {
                name: None,
                address: None,
                manager: None,
                phone: None,
                contact_person: None,
                is_default: None,
                capacity: Some(Some(80)),
                status: None,
                warehouse_type: None,
            },
        )
        .await
        .unwrap();
    let row = warehouse::Entity::find_by_id(id)
        .one(&db)
        .await
        .unwrap()
        .unwrap();
    assert_eq!(row.capacity, Some(80), "有值=覆盖");
    assert_eq!(row.address.as_deref(), Some("老地址"), "缺席键必须保持原值");
    assert_eq!(row.manager_id, Some(77));
    assert_eq!(row.phone.as_deref(), Some("13800000000"));

    // 2) 显式 null=落 NULL：address/manager/contact_person/capacity 清空，phone 缺席保持
    service
        .update(
            id,
            9101,
            warehouse_handler::UpdateWarehouseRequest {
                name: None,
                address: Some(None),
                manager: Some(None),
                phone: None,
                contact_person: Some(None),
                is_default: None,
                capacity: Some(None),
                status: None,
                warehouse_type: None,
            },
        )
        .await
        .unwrap();
    let row = warehouse::Entity::find_by_id(id)
        .one(&db)
        .await
        .unwrap()
        .unwrap();
    assert!(row.address.is_none(), "发 null 后 address 必须为 NULL");
    assert!(
        row.manager_id.is_none(),
        "发 null 后 manager_id 必须为 NULL（清除经理）"
    );
    assert!(row.capacity.is_none(), "发 null 后 capacity 必须为 NULL");
    assert!(
        row.contact_person.is_none(),
        "发 null 后 contact_person 必须为 NULL"
    );
    assert_eq!(
        row.phone.as_deref(),
        Some("13800000000"),
        "缺席键保持（与清空两种不同结果）"
    );

    // 3) 有值=覆盖：同一列 address 回环；manager 字符串按 i32 解析；
    //    status=inactive 落 is_active=false；缺席 is_default 保持
    service
        .update(
            id,
            9101,
            warehouse_handler::UpdateWarehouseRequest {
                name: None,
                address: Some(Some("新地址".to_string())),
                manager: Some(Some("88".to_string())),
                phone: None,
                contact_person: None,
                is_default: None,
                capacity: None,
                status: Some(Some("inactive".to_string())),
                warehouse_type: Some(Some("finished".to_string())),
            },
        )
        .await
        .unwrap();
    let row = warehouse::Entity::find_by_id(id)
        .one(&db)
        .await
        .unwrap()
        .unwrap();
    assert_eq!(
        row.address.as_deref(),
        Some("新地址"),
        "同一可空列三态闭环：有值=覆盖"
    );
    assert_eq!(row.manager_id, Some(88), "manager 字符串按 i32 解析覆盖");
    assert_eq!(row.warehouse_type.as_deref(), Some("finished"));
    assert!(!row.is_active, "status=inactive 必须落 is_active=false");
    assert!(!row.is_default, "缺席的 is_default 保持原值（seed=false）");
}

#[tokio::test]
async fn warehouse_not_null_reject_has_no_partial_side_effect() {
    let db = sqlite_db().await;
    exec(&db, WAREHOUSES_DDL).await;
    exec(&db, USERS_DDL).await;
    exec(&db, AUDIT_LOGS_DDL).await;
    let seeded = seed_warehouse(&db).await;
    let service = WarehouseService::new(Arc::new(db.clone()));

    // 同请求：NOT NULL name 显式 null + address 有值 + capacity/status 清空 —— 必须整体拒绝，
    // 任何其他列不得先落库（"改了一半"即脏写）
    let err = service
        .update(
            seeded.id,
            9101,
            warehouse_handler::UpdateWarehouseRequest {
                name: Some(None),
                address: Some(Some("不应落库".to_string())),
                manager: Some(None),
                phone: None,
                contact_person: None,
                is_default: None,
                capacity: Some(None),
                status: Some(None),
                warehouse_type: None,
            },
        )
        .await
        .expect_err("NOT NULL 列显式 null 必须被拒绝");
    expect_cleared_business_error(err, "仓库名称不能清空", "name");
    let row = warehouse::Entity::find_by_id(seeded.id)
        .one(&db)
        .await
        .unwrap()
        .unwrap();
    assert_eq!(
        row.address.as_deref(),
        Some("老地址"),
        "同请求 address 不得落库"
    );
    assert_eq!(row.manager_id, Some(77), "同请求 manager 清空不得落库");
    assert_eq!(row.capacity, Some(50), "同请求 capacity 清空不得落库");
    assert!(row.is_active, "同请求 status 清空不得落库");

    // 三列拒清逐列验证（业务文案逐字外显、不脱敏、非 internal）
    let cases = [
        (
            warehouse_handler::UpdateWarehouseRequest {
                name: Some(None),
                address: None,
                manager: None,
                phone: None,
                contact_person: None,
                is_default: None,
                capacity: None,
                status: None,
                warehouse_type: None,
            },
            "仓库名称不能清空：该字段为必填项",
        ),
        (
            warehouse_handler::UpdateWarehouseRequest {
                name: None,
                address: None,
                manager: None,
                phone: None,
                contact_person: None,
                is_default: Some(None),
                capacity: None,
                status: None,
                warehouse_type: None,
            },
            "默认仓库标志不能清空：该字段为必填项",
        ),
        (
            warehouse_handler::UpdateWarehouseRequest {
                name: None,
                address: None,
                manager: None,
                phone: None,
                contact_person: None,
                is_default: None,
                capacity: None,
                status: Some(None),
                warehouse_type: None,
            },
            "启用状态不能清空：该字段为必填项",
        ),
    ];
    for (req, expected_msg) in cases {
        let err = service
            .update(seeded.id, 9101, req)
            .await
            .expect_err("NOT NULL 列显式 null 必须被拒绝");
        assert!(
            matches!(&err, AppError::BusinessErrorDisplayable(m) if m == expected_msg),
            "文案必须逐字外显（不脱敏），期望「{expected_msg}」，实际: {err:?}"
        );
    }
}

// =========================================================
// C) 退货子域：采购退货单头/明细真实 sqlite 三态回环
//    （明细原位 update、四维+辅量不得被整表重插洗掉——行数与原值双断言）
// =========================================================

#[tokio::test]
async fn purchase_return_item_update_tri_state_roundtrip_on_sqlite() {
    let db = sqlite_db().await;
    exec(&db, PURCHASE_RETURN_DDL).await;
    exec(&db, PURCHASE_RETURN_ITEM_DDL).await;
    exec(&db, USERS_DDL).await;
    exec(&db, AUDIT_LOGS_DDL).await;
    let (r, seeded) = seed_purchase_return_with_item(&db).await;
    let item_id = seeded.id;
    let service = PurchaseReturnService::new(Arc::new(db.clone()));
    let reload = || async {
        purchase_return_item::Entity::find_by_id(item_id)
            .one(&db)
            .await
            .unwrap()
            .unwrap()
    };

    // 1) 键缺席=保持：只改 notes（有值=覆盖），四维/辅量/单价原值不动
    let after = service
        .update_item(
            item_id,
            UpdateReturnItemRequest {
                notes: Some(Some("三态新备注".to_string())),
                ..Default::default()
            },
            9101,
        )
        .await
        .unwrap();
    assert_eq!(after.notes.as_deref(), Some("三态新备注"), "有值=覆盖");
    let row = reload().await;
    assert_eq!(row.color_no, "C001", "缺席键：四维·色号保持");
    assert_eq!(row.dye_lot_no, "DL001", "缺席键：四维·缸号保持");
    assert_eq!(row.batch_no, "B5", "缺席键：四维·批次保持");
    assert_eq!(
        row.quantity_alt,
        dec("5.50"),
        "缺席键：辅量保持（重插洗数据回归锁）"
    );

    // 2) 显式 null=落 NULL：notes 清空（与"缺席→保持"两种不同结果）；其余缺席
    service
        .update_item(
            item_id,
            UpdateReturnItemRequest {
                notes: Some(None),
                ..Default::default()
            },
            9101,
        )
        .await
        .unwrap();
    let row = reload().await;
    assert!(row.notes.is_none(), "发 null 后 notes 必须为 NULL");

    // 3) 色号改回白坯=空串覆盖（NOT NULL 列不开 null、开空串）；notes 缺席不再改动
    service
        .update_item(
            item_id,
            UpdateReturnItemRequest {
                color_no: Some(Some(String::new())),
                ..Default::default()
            },
            9101,
        )
        .await
        .unwrap();
    let row = reload().await;
    assert_eq!(row.color_no, "", "色号改回白坯=空串覆盖");
    assert!(row.notes.is_none(), "缺席键不再改动");
    assert_eq!(row.dye_lot_no, "DL001");

    // 4) 数量覆盖→金额重算+单据头汇总同步；行数恒为 1（更新重复插行历史缺陷回归锁）
    service
        .update_item(
            item_id,
            UpdateReturnItemRequest {
                quantity_returned: Some(Some(dec("20.00"))),
                ..Default::default()
            },
            9101,
        )
        .await
        .unwrap();
    let row = reload().await;
    assert_eq!(row.quantity, dec("20.00"), "数量覆盖落库");
    assert_eq!(row.subtotal, dec("100.00"), "subtotal 按 qty×price 重算");
    assert_eq!(row.total_amount, dec("100.00"));
    let count = purchase_return_item::Entity::find()
        .filter(purchase_return_item::Column::ReturnId.eq(r.id))
        .count(&db)
        .await
        .unwrap();
    assert_eq!(count, 1, "明细更新必须原位 update，不得整表重插出多行");
    let head = purchase_return::Entity::find_by_id(r.id)
        .one(&db)
        .await
        .unwrap()
        .unwrap();
    assert_eq!(head.total_quantity, Some(dec("20.00")), "单头汇总同步");
    assert_eq!(
        head.total_quantity_alt,
        Some(dec("5.50")),
        "单头辅量汇总不得被洗掉"
    );
    assert_eq!(head.total_amount, Some(dec("100.00")));
}

#[tokio::test]
async fn purchase_return_not_null_reject_has_no_partial_side_effect() {
    let db = sqlite_db().await;
    exec(&db, PURCHASE_RETURN_DDL).await;
    exec(&db, PURCHASE_RETURN_ITEM_DDL).await;
    exec(&db, USERS_DDL).await;
    exec(&db, AUDIT_LOGS_DDL).await;
    let (_, seeded) = seed_purchase_return_with_item(&db).await;
    let item_id = seeded.id;
    let service = PurchaseReturnService::new(Arc::new(db.clone()));

    // 同请求：NOT NULL 缸号显式 null + notes 有值 —— 整体拒绝且 notes 不落库
    let err = service
        .update_item(
            item_id,
            UpdateReturnItemRequest {
                dye_lot_no: Some(None),
                notes: Some(Some("不应落库".to_string())),
                ..Default::default()
            },
            9101,
        )
        .await
        .expect_err("NOT NULL 追溯列显式 null 必须被拒绝");
    expect_cleared_business_error(err, "缸号不能清空", "dye_lot_no");
    let row = purchase_return_item::Entity::find_by_id(item_id)
        .one(&db)
        .await
        .unwrap()
        .unwrap();
    assert_eq!(
        row.notes.as_deref(),
        Some("原明细备注"),
        "同请求 notes 不得落库"
    );
    assert_eq!(row.dye_lot_no, "DL001", "拒绝后缸号保持");
    assert_eq!(row.quantity, dec("10.00"));

    // 单头三列均可空：显式 null=清空、缺席=保持、有值=覆盖（同一列回环）
    service
        .update_return(
            seeded.return_id,
            UpdatePurchaseReturnRequest {
                reason_type: Some(None),
                reason_detail: None,
                notes: Some(Some("新备注".to_string())),
            },
            9101,
        )
        .await
        .unwrap();
    let head = purchase_return::Entity::find_by_id(seeded.return_id)
        .one(&db)
        .await
        .unwrap()
        .unwrap();
    assert!(head.reason_type.is_none(), "可空列发 null 必须落 NULL");
    assert_eq!(
        head.reason_detail.as_deref(),
        Some("染色疵点"),
        "缺席键保持原值"
    );
    assert_eq!(head.notes.as_deref(), Some("新备注"), "有值=覆盖");
}

// =========================================================
// D) 调拨子域：handler `map_err(internal/bad_request)`→`?` 透传后
//    错误按真正责任模块定性（404/业务拒绝不再被拍平）
// =========================================================

#[tokio::test]
async fn transfer_item_update_not_found_propagates_404_not_flattened() {
    let app = transfer_item_router(sqlite_db().await);
    // 明细不存在：service 返回 AppError::not_found —— 原实现 map_err(bad_request)
    // 会拍平成 400，`?` 透传后必须是 404 + NOT_FOUND
    let (status, v) = call_put(
        &app,
        "/inventory/transfers/items/999999",
        json!({ "notes": null }),
    )
    .await;
    assert_eq!(status, StatusCode::NOT_FOUND, "实际: {v}");
    assert_eq!(v["code"], "NOT_FOUND");
    assert_ne!(
        status,
        StatusCode::INTERNAL_SERVER_ERROR,
        "not_found 绝不允许被拍平成 500"
    );
}

#[tokio::test]
async fn transfer_item_update_business_gate_not_500_not_flattened() {
    let db = sqlite_db().await;
    exec(&db, INVENTORY_TRANSFERS_DDL).await;
    exec(&db, INVENTORY_TRANSFER_ITEMS_DDL).await;
    let (_, item) = seed_transfer(&db, transfer_status::SHIPPED).await;
    let app = transfer_item_router(db.clone());
    let uri = format!("/inventory/transfers/items/{}", item.id);

    // 状态门：已发货单改明细 = 业务拒绝。原实现 map_err(internal) 形态会成 500；
    // `?` 透传后按真实责任定性为业务错误，绝不 INTERNAL_ERROR
    let (status, v) = call_put(&app, &uri, json!({ "notes": "已发货还想改" })).await;
    assert_eq!(
        status,
        StatusCode::BAD_REQUEST,
        "业务拒绝不得成 500，实际: {v}"
    );
    assert_eq!(v["code"], "BUSINESS_ERROR");
    assert_ne!(v["code"], "INTERNAL_ERROR", "业务拒绝绝不留 internal");

    // NOT NULL 拒清经真实 handler（门控在任何 DB 访问之前，先于状态门）：
    // quantity null → 400 + business_displayable 文案逐字外显不脱敏
    let (status, v) = call_put(&app, &uri, json!({ "quantity": null })).await;
    assert_eq!(status, StatusCode::BAD_REQUEST, "实际: {v}");
    assert_eq!(v["code"], "BUSINESS_ERROR");
    assert_eq!(v["message"], "调拨数量不能清空：该字段为必填项");
    assert_ne!(
        v["message"], "业务处理失败",
        "business_displayable 不得被脱敏"
    );
}

// =========================================================
// E) 活库（PG，#[ignore]，ci-test-rust-ignored 执行）：
//    销退明细与收货明细三态全链（服务链 lock_exclusive，sqlite 不支持行锁，
//    不得伪装成 sqlite 用例）
// =========================================================

async fn require_postgres(db: &DatabaseConnection) {
    assert_eq!(
        db.get_database_backend(),
        DbBackend::Postgres,
        "本用例必须跑在已迁移的 PostgreSQL（TEST_DATABASE_URL）上；禁止 sqlite 回退假绿"
    );
}

#[tokio::test]
#[ignore = "需要 TEST_DATABASE_URL 已迁移 PostgreSQL（update_return_item 走 lock_exclusive + 事务）"]
async fn live_sales_return_item_update_tri_state_on_postgres() {
    let db = test_common::setup_test_db().await;
    require_postgres(&db).await;
    let suffix = Utc::now().timestamp_nanos_opt().unwrap();

    let r = sales_return::ActiveModel {
        return_no: Set(format!("SR-W5-{suffix}")),
        customer_id: Set(1),
        return_date: Set(chrono::NaiveDate::from_ymd_opt(2026, 9, 1).unwrap()),
        warehouse_id: Set(1),
        reason: Set("质量问题".to_string()),
        // 状态词表唯一来源 models::status::sales::sales_return（大写 DRAFT）——逐字符同源
        status: Set(sr_status::DRAFT.to_string()),
        total_amount: Set(Decimal::ZERO),
        remarks: Set(Some("原备注".to_string())),
        created_by: Set(9101),
        created_at: Set(Utc::now()),
        updated_at: Set(Utc::now()),
        ..Default::default()
    }
    .insert(&db)
    .await
    .unwrap();
    let item = sales_return_item::ActiveModel {
        return_id: Set(r.id),
        line_no: Set(1),
        product_id: Set(1),
        quantity: Set(dec("10.00")),
        quantity_alt: Set(dec("1.00")),
        unit_price: Set(dec("10.00")),
        unit_price_foreign: Set(Decimal::ZERO),
        discount_percent: Set(Decimal::ZERO),
        tax_percent: Set(Decimal::ZERO),
        subtotal: Set(dec("100.00")),
        tax_amount: Set(Decimal::ZERO),
        discount_amount: Set(Decimal::ZERO),
        total_amount: Set(dec("100.00")),
        notes: Set(Some("原明细备注".to_string())),
        created_at: Set(Utc::now()),
        updated_at: Set(Utc::now()),
        color_no: Set(String::new()),
        dye_lot_no: Set(String::new()),
        batch_no: Set(String::new()),
        ..Default::default()
    }
    .insert(&db)
    .await
    .unwrap();
    let item_id = item.id;
    let service = SalesReturnService::new(Arc::new(db.clone()));
    let reload = || async {
        sales_return_item::Entity::find_by_id(item_id)
            .one(&db)
            .await
            .unwrap()
            .unwrap()
    };

    // 1) 键缺席=保持：只改 reason（有值=覆盖），quantity/unit_price 原值不动
    service
        .update_return_item(item_id, None, None, Some(Some("新原因".to_string())), 9101)
        .await
        .unwrap();
    let row = reload().await;
    assert_eq!(row.notes.as_deref(), Some("新原因"), "有值=覆盖");
    assert_eq!(row.quantity, dec("10.00"), "缺席键保持");

    // 2) 显式 null=落 NULL（与"缺席→保持"两种不同结果的同列闭环）
    service
        .update_return_item(item_id, None, None, Some(None), 9101)
        .await
        .unwrap();
    let row = reload().await;
    assert!(row.notes.is_none(), "发 null 后 notes 必须为 NULL");

    // 3) NOT NULL 拒清：quantity null + reason 有值同请求 → 拒绝且 reason 不落库
    let err = service
        .update_return_item(
            item_id,
            Some(None),
            None,
            Some(Some("不应落库".to_string())),
            9101,
        )
        .await
        .expect_err("NOT NULL quantity 显式 null 必须被拒绝");
    expect_cleared_business_error(err, "退货数量不能清空", "quantity");
    let row = reload().await;
    assert!(row.notes.is_none(), "同请求 reason 不得落库");
    assert_eq!(row.quantity, dec("10.00"));

    // 4) 单价覆盖→金额重算（与 add_return_item 同口径）且行数恒为 1（插行回归锁）
    service
        .update_return_item(item_id, None, Some(Some(dec("12.00"))), None, 9101)
        .await
        .unwrap();
    let row = reload().await;
    assert_eq!(
        row.total_amount,
        dec("120.00"),
        "单价覆盖后 total_amount 重算"
    );
    let count = sales_return_item::Entity::find()
        .filter(sales_return_item::Column::ReturnId.eq(r.id))
        .count(&db)
        .await
        .unwrap();
    assert_eq!(count, 1, "明细更新必须原位 update，不得重复插行");
}

#[tokio::test]
#[ignore = "需要 TEST_DATABASE_URL 已迁移 PostgreSQL（update_receipt_item 走 lock_exclusive）"]
async fn live_receipt_item_update_tri_state_on_postgres() {
    let db = test_common::setup_test_db().await;
    require_postgres(&db).await;
    let suffix = Utc::now().timestamp_nanos_opt().unwrap();

    let receipt = purchase_receipt::ActiveModel {
        receipt_no: Set(format!("GR-W5-{suffix}")),
        supplier_id: Set(1),
        receipt_date: Set(chrono::NaiveDate::from_ymd_opt(2026, 9, 1).unwrap()),
        warehouse_id: Set(1),
        inspection_status: Set("PENDING".to_string()),
        // 状态词表唯一来源 models::status::purchase_inventory::purchase_receipt（大写 DRAFT）
        receipt_status: Set(receipt_status::DRAFT.to_string()),
        total_quantity: Set(Decimal::ZERO),
        total_quantity_alt: Set(Decimal::ZERO),
        total_amount: Set(Decimal::ZERO),
        created_by: Set(9101),
        created_at: Set(Utc::now()),
        updated_at: Set(Utc::now()),
        ..Default::default()
    }
    .insert(&db)
    .await
    .unwrap();
    let item = purchase_receipt_item::ActiveModel {
        receipt_id: Set(receipt.id),
        line_no: Set(1),
        product_id: Set(1),
        material_code: Set("MC-W5-001".to_string()),
        material_name: Set("测试面料".to_string()),
        batch_no: Set(Some("B5".to_string())),
        color_code: Set(Some("C001".to_string())),
        lot_no: Set(Some("DL001".to_string())),
        grade: Set(Some("一等品".to_string())),
        quantity: Set(dec("10.00")),
        quantity_alt: Set(Some(dec("1.00"))),
        unit_master: Set("米".to_string()),
        unit_price: Set(Some(dec("8.00"))),
        created_at: Set(Some(Utc::now())),
        // purchase_receipt_item 实体无 updated_at 列（models/purchase_receipt_item.rs
        // 无该字段），原字面量系臆测字段，删除不影响任何断言
        ..Default::default()
    }
    .insert(&db)
    .await
    .unwrap();
    let item_id = item.id;
    let service = PurchaseReceiptService::new(Arc::new(db.clone()));
    let reload = || async {
        purchase_receipt_item::Entity::find_by_id(item_id)
            .one(&db)
            .await
            .unwrap()
            .unwrap()
    };

    // 1) 键缺席=保持：只覆盖 quantity，四维（batch/color/lot/grade）与单价原值不动
    service
        .update_receipt_item(
            item_id,
            purchase_receipt_dto::UpdateReceiptItemRequest {
                quantity: Some(Some(dec("12.00"))),
                ..Default::default()
            },
            9101,
        )
        .await
        .unwrap();
    let row = reload().await;
    assert_eq!(row.quantity, dec("12.00"), "有值=覆盖");
    assert_eq!(row.batch_no.as_deref(), Some("B5"), "缺席键：批次保持");
    assert_eq!(row.color_code.as_deref(), Some("C001"), "缺席键：色号保持");
    assert_eq!(row.lot_no.as_deref(), Some("DL001"), "缺席键：缸号保持");
    assert_eq!(row.grade.as_deref(), Some("一等品"), "缺席键：等级保持");
    assert_eq!(row.unit_price, Some(dec("8.00")), "缺席键：单价保持");

    // 2) 显式 null=落 NULL：四维三列 + 辅量/单价清空，batch_no 缺席保持（两种不同结果）
    service
        .update_receipt_item(
            item_id,
            purchase_receipt_dto::UpdateReceiptItemRequest {
                color_code: Some(None),
                lot_no: Some(None),
                grade: Some(None),
                quantity_alt: Some(None),
                unit_price: Some(None),
                ..Default::default()
            },
            9101,
        )
        .await
        .unwrap();
    let row = reload().await;
    assert!(
        row.color_code.is_none(),
        "发 null 后 color_code 必须为 NULL"
    );
    assert!(row.lot_no.is_none(), "发 null 后 lot_no 必须为 NULL");
    assert!(row.grade.is_none(), "发 null 后 grade 必须为 NULL");
    assert!(
        row.quantity_alt.is_none(),
        "发 null 后 quantity_alt 必须为 NULL"
    );
    assert!(
        row.unit_price.is_none(),
        "发 null 后 unit_price 必须为 NULL"
    );
    assert_eq!(
        row.batch_no.as_deref(),
        Some("B5"),
        "缺席键保持（与清空结果不同）"
    );

    // 3) 再覆盖闭环：grade 重给值（同一可空列三态闭环）
    service
        .update_receipt_item(
            item_id,
            purchase_receipt_dto::UpdateReceiptItemRequest {
                grade: Some(Some("优等品".to_string())),
                ..Default::default()
            },
            9101,
        )
        .await
        .unwrap();
    let row = reload().await;
    assert_eq!(row.grade.as_deref(), Some("优等品"), "同一可空列三态闭环");

    // 4) NOT NULL 拒清且同请求其他字段不落库
    let err = service
        .update_receipt_item(
            item_id,
            purchase_receipt_dto::UpdateReceiptItemRequest {
                quantity: Some(None),
                color_code: Some(Some("不应落库".to_string())),
                ..Default::default()
            },
            9101,
        )
        .await
        .expect_err("NOT NULL quantity 显式 null 必须被拒绝");
    expect_cleared_business_error(err, "入库数量不能清空", "quantity");
    let row = reload().await;
    assert!(row.color_code.is_none(), "同请求 color_code 不得落库");
    assert_eq!(row.quantity, dec("12.00"));
}
