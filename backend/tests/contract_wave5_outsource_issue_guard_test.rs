//! 任务 #169 第 5 波（委外发料匹号门控）：空明细整单放行缺陷的契约锁
//!
//! 缺陷证据链（修复前状态）：
//! - `services/piece_domain_service.rs::validate_pieces_for_issue` 的门控是逐条明细
//!   校验（`for it in items`），0 条明细 = 0 次校验 = 静默 `Ok(())`；
//! - 唯一调用方 `services/outsourcing_ops/order.rs::issue_order`（L370-384）先
//!   `list_by_order` 再直接进校验，中间**没有任何空明细前置拦截**
//!   （`outsourcing_ops/order_item.rs::list_by_order` L220-227 仅按订单 ID 过滤）；
//! - handler `POST /outsourcing-orders/:id/issue`（`outsourcing_handler.rs:185-191`）
//!   是薄透传，同样无明细数校验；
//! - 于是空明细订单会被推进 `issued` 并在事务内生成 OVIS 发料凭证
//!   （order.rs L388-437），与「发料精确到匹」的域规则相悖，属静默数据完整性漏洞。
//!
//! 修复：`validate_pieces_for_issue` 入口显式拒绝空明细
//! （`AppError::business_displayable`，公开规则文案、无内部标识/记录 ID）；
//! 既有逐条拒绝分支（未填匹号/匹不存在/非可用）文案携带查询所得缸号，
//! 按 `utils/error.rs` 模块文档保持脱敏 `business` 形态不变。
//!
//! 覆盖策略（无 mock、真实 service/handler 路径）：
//! - 两条拒绝路径均发生在 `issue_order` 开启事务/取号**之前**
//!   （validate 在 order.rs L384，begin 在 L388），因此 sqlite::memory:
//!   同构表即可真实跑通 handler→service→domain 全链；
//! - 零漂移不只看错误码：拒绝后回查订单**整行逐列相等**（Model PartialEq）、
//!   该订单凭证行数=0、OVIS 前缀凭证数=0；
//! - 正向对照（合规明细发料成功）必经 `generate_no_with_txn`
//!   （`utils/number_generator.rs::lock_prefix` L257-270 为
//!   `pg_advisory_xact_lock`，sqlite 方言不支持，不得伪装成 sqlite 用例——
//!   先例 contract_wave1/wave5 同口径）：`#[ignore]` 活库
//!   （TEST_DATABASE_URL→已迁移 PG，CI `ci-test-rust-ignored` 以 --include-ignored 执行；
//!   本地未设变量时 setup_test_db 回退 sqlite，用例行首的方言断言将其**显式失败并说明**，
//!   非条件跳过）。

mod test_common;

use axum::{
    Router,
    body::Body,
    http::{Method, Request, StatusCode},
    response::Response,
};
use bingxi_backend::container::AppState;
use bingxi_backend::handlers::outsourcing_handler;
use bingxi_backend::models::inventory_piece;
use bingxi_backend::models::outsourcing_order;
use bingxi_backend::models::outsourcing_order_item;
use bingxi_backend::models::outsourcing_voucher;
use bingxi_backend::models::status::outsourcing_order_status;
use bingxi_backend::models::status::outsourcing_order_type;
use bingxi_backend::models::status::outsourcing_voucher_type;
use bingxi_backend::models::status::purchase_inventory::inventory_piece as piece_status;
use bingxi_backend::services::outsourcing_service::OutsourcingOrderService;
use bingxi_backend::services::piece_domain_service::PIECE_TYPE_GREIGE;
use bingxi_backend::utils::error::AppError;
use bingxi_backend::utils::messages::err_msg;
use chrono::Utc;
use rust_decimal::Decimal;
use sea_orm::QueryFilter;
use sea_orm::{
    ActiveModelTrait, ColumnTrait, ConnectionTrait, DatabaseConnection, DbBackend, EntityTrait,
    PaginatorTrait, QuerySelect, Set, Statement,
};
use serde_json::Value;
use std::str::FromStr;
use std::sync::Arc;
use tower::ServiceExt;

// =========================================================
// 公共夹具（sqlite::memory: 同构表，列集与 models/*.rs 逐列对应；
// Decimal→TEXT、DateTime→TEXT、bool→INTEGER，先例 contract_wave5
// receipt/transfer 同形态）
// =========================================================

fn dec(s: &str) -> Decimal {
    Decimal::from_str(s).unwrap()
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

const OUTSOURCING_ORDER_DDL: &str = r#"CREATE TABLE outsourcing_order (
    id INTEGER PRIMARY KEY,
    order_no TEXT NOT NULL, order_type TEXT NOT NULL, supplier_id INTEGER NOT NULL,
    production_order_id INTEGER, dye_batch_id INTEGER, color_no TEXT, dye_lot_no TEXT,
    issue_date TEXT NOT NULL, expected_return_date TEXT, actual_return_date TEXT,
    issue_quantity TEXT NOT NULL, issue_unit TEXT NOT NULL, return_quantity TEXT NOT NULL,
    loss_quantity TEXT NOT NULL, loss_type TEXT, loss_rate TEXT, standard_loss_rate TEXT,
    material_cost TEXT NOT NULL, processing_fee TEXT NOT NULL, freight_fee TEXT NOT NULL,
    tax_amount TEXT NOT NULL, abnormal_loss_amount TEXT NOT NULL, total_cost TEXT NOT NULL,
    unit_cost TEXT NOT NULL, status TEXT NOT NULL,
    voucher_no_issue TEXT, voucher_no_fee TEXT, voucher_no_receipt TEXT,
    remarks TEXT, is_deleted INTEGER NOT NULL,
    created_by INTEGER, created_at TEXT NOT NULL, updated_at TEXT NOT NULL
)"#;

const OUTSOURCING_ORDER_ITEM_DDL: &str = r#"CREATE TABLE outsourcing_order_item (
    id INTEGER PRIMARY KEY,
    outsourcing_order_id INTEGER NOT NULL, product_id INTEGER NOT NULL,
    color_no TEXT, dye_lot_no TEXT, batch_no TEXT, warehouse_id INTEGER,
    quantity TEXT NOT NULL, unit TEXT NOT NULL,
    unit_cost TEXT NOT NULL, total_cost TEXT NOT NULL,
    processing_fee TEXT NOT NULL, freight_fee TEXT NOT NULL,
    inventory_transaction_id INTEGER, greige_fabric_id INTEGER,
    piece_no TEXT, remarks TEXT,
    created_at TEXT NOT NULL, updated_at TEXT NOT NULL
)"#;

const OUTSOURCING_VOUCHER_DDL: &str = r#"CREATE TABLE outsourcing_voucher (
    id INTEGER PRIMARY KEY,
    voucher_no TEXT NOT NULL, outsourcing_order_id INTEGER NOT NULL,
    voucher_type TEXT NOT NULL, debit_account TEXT NOT NULL, credit_account TEXT NOT NULL,
    amount TEXT NOT NULL, tax_amount TEXT NOT NULL, tax_transfer_amount TEXT NOT NULL,
    voucher_date TEXT NOT NULL, is_posted INTEGER NOT NULL, posted_at TEXT,
    remarks TEXT, created_by INTEGER, created_at TEXT NOT NULL, updated_at TEXT NOT NULL
)"#;

const INVENTORY_PIECE_DDL: &str = r#"CREATE TABLE inventory_piece (
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

async fn seed_issue_domain_tables(db: &DatabaseConnection) {
    exec(db, OUTSOURCING_ORDER_DDL).await;
    exec(db, OUTSOURCING_ORDER_ITEM_DDL).await;
    exec(db, OUTSOURCING_VOUCHER_DDL).await;
    exec(db, INVENTORY_PIECE_DDL).await;
}

fn unique_tag() -> i64 {
    Utc::now()
        .timestamp_nanos_opt()
        .expect("测试环境时间戳必须可用")
}

/// 种一张 draft 委外订单（全部 NOT NULL 列显式赋值，活库/ sqlite 同构两用）
async fn seed_draft_order(db: &DatabaseConnection) -> outsourcing_order::Model {
    // outsourcing_order 时间列为 DateTimeWithTimeZone（= DateTime<FixedOffset>），
    // 仓内惯用 `Utc::now().into()`
    let now: chrono::DateTime<chrono::FixedOffset> = Utc::now().into();
    outsourcing_order::ActiveModel {
        order_no: Set(format!("OW-W5G-{}", unique_tag())),
        order_type: Set(outsourcing_order_type::DYEING.to_string()),
        supplier_id: Set(1),
        issue_date: Set(now.date_naive()),
        issue_quantity: Set(dec("100.00")),
        issue_unit: Set("米".to_string()),
        return_quantity: Set(Decimal::ZERO),
        loss_quantity: Set(Decimal::ZERO),
        material_cost: Set(dec("1000.00")),
        processing_fee: Set(Decimal::ZERO),
        freight_fee: Set(Decimal::ZERO),
        tax_amount: Set(Decimal::ZERO),
        abnormal_loss_amount: Set(Decimal::ZERO),
        total_cost: Set(Decimal::ZERO),
        unit_cost: Set(Decimal::ZERO),
        status: Set(outsourcing_order_status::DRAFT.to_string()),
        is_deleted: Set(false),
        created_by: Set(Some(9101)),
        created_at: Set(now),
        updated_at: Set(now),
        ..Default::default()
    }
    .insert(db)
    .await
    .expect("夹具：种 draft 委外订单失败")
}

/// 种一条发料明细（piece_no 三态由入参决定：None=未填、Some("")=空串、Some(值)=引用）
async fn seed_item(
    db: &DatabaseConnection,
    order_id: i32,
    piece_no: Option<&str>,
) -> outsourcing_order_item::Model {
    // outsourcing_order_item 时间列为 DateTimeWithTimeZone（= DateTime<FixedOffset>）
    let now: chrono::DateTime<chrono::FixedOffset> = Utc::now().into();
    outsourcing_order_item::ActiveModel {
        outsourcing_order_id: Set(order_id),
        product_id: Set(9001),
        dye_lot_no: Set(Some("DL-W5G-001".to_string())),
        quantity: Set(dec("50.00")),
        unit: Set("米".to_string()),
        unit_cost: Set(dec("10.00")),
        total_cost: Set(dec("500.00")),
        processing_fee: Set(Decimal::ZERO),
        freight_fee: Set(Decimal::ZERO),
        piece_no: Set(piece_no.map(|s| s.to_string())),
        created_at: Set(now),
        updated_at: Set(now),
        ..Default::default()
    }
    .insert(db)
    .await
    .expect("夹具：种发料明细失败")
}

/// 种一条生产匹（字段全量显式，与 create_greige_pieces_from_report 落库形态一致）
async fn seed_piece(
    db: &DatabaseConnection,
    piece_no: &str,
    status: &str,
    product_id: i32,
    warehouse_id: i32,
) -> inventory_piece::Model {
    let now = Utc::now();
    inventory_piece::ActiveModel {
        id: sea_orm::ActiveValue::NotSet,
        piece_no: Set(piece_no.to_string()),
        piece_type: Set(PIECE_TYPE_GREIGE.to_string()),
        machine_no: Set(Some("M-01".to_string())),
        machine_operator: Set(Some("张师傅".to_string())),
        warehouse_in_at: Set(Some(now)),
        dye_lot_id: Set(None),
        dye_lot_no: Set(String::new()),
        batch_no: Set(format!("OW-LOT-{piece_no}")),
        product_id: Set(product_id),
        warehouse_id: Set(warehouse_id),
        length: Set(dec("50.00")),
        weight: Set(Some(dec("12.50"))),
        width: Set(None),
        gram_weight: Set(None),
        production_date: Set(None),
        quality_status: Set(None),
        inventory_status: Set(Some(piece_status::AVAILABLE.to_string())),
        supplier_piece_no: Set(None),
        position_no: Set(None),
        package_no: Set(None),
        shelf_life: Set(None),
        barcode: Set(Some(piece_no.to_string())),
        parent_piece_id: Set(None),
        inspection_id: Set(None),
        piece_seq: Set(Some(1)),
        location_id: Set(None),
        scan_type: Set(None),
        status: Set(status.to_string()),
        remarks: Set(Some("契约测试夹具匹".to_string())),
        created_at: Set(now),
        updated_at: Set(now),
        created_by: Set(Some(9101)),
        updated_by: Set(None),
        color_no: Set(String::new()),
        original_length: Set(None),
        original_weight: Set(None),
    }
    .insert(db)
    .await
    .expect("夹具：种生产匹失败")
}

fn issue_router(db: DatabaseConnection) -> Router {
    let mut state = AppState::default();
    state.db = Arc::new(db);
    Router::new()
        .route(
            "/outsourcing-orders/{id}/issue",
            axum::routing::post(outsourcing_handler::issue_outsourcing_order),
        )
        .with_state(state)
}

async fn post_issue(app: &Router, id: i32) -> (StatusCode, Value) {
    let req = Request::builder()
        .method(Method::POST)
        .uri(format!("/outsourcing-orders/{}/issue", id))
        .body(Body::empty())
        .expect("构造 POST 请求失败");
    let resp: Response = app.clone().oneshot(req).await.unwrap();
    let status = resp.status();
    let bytes = axum::body::to_bytes(resp.into_body(), 4 * 1024 * 1024)
        .await
        .unwrap();
    (status, serde_json::from_slice(&bytes).unwrap())
}

/// 零漂移回查：拒绝后订单**整行逐列相等**（含 status/voucher_no_issue/updated_at）、
/// 该订单凭证行数=0、全库 OVIS 前缀凭证=0——不只断错误码。
async fn assert_zero_drift(db: &DatabaseConnection, before: &outsourcing_order::Model) {
    let after = outsourcing_order::Entity::find_by_id(before.id)
        .one(db)
        .await
        .unwrap()
        .expect("零漂移回查：订单必须仍存在");
    assert_eq!(
        &after, before,
        "发料被拒后订单整行必须逐列零漂移（状态推进/凭证号回写/时间戳变动都算漂移）"
    );
    assert_eq!(after.status, outsourcing_order_status::DRAFT);
    assert!(after.voucher_no_issue.is_none(), "拒绝后不得回写发料凭证号");
    let vouchers = outsourcing_voucher::Entity::find()
        .filter(outsourcing_voucher::Column::OutsourcingOrderId.eq(before.id))
        .count(db)
        .await
        .unwrap();
    assert_eq!(vouchers, 0, "发料被拒后该订单不得存在任何凭证");
    let ovis = outsourcing_voucher::Entity::find()
        .filter(outsourcing_voucher::Column::VoucherNo.starts_with("OVIS"))
        .count(db)
        .await
        .unwrap();
    assert_eq!(ovis, 0, "不得生成 OVIS 发料凭证");
}

/// 空明细的公开规则文案（与 validate_pieces_for_issue 构造点逐字一致——
/// 文案即契约，改动必须双侧同步）
const EMPTY_ITEMS_REJECT_MSG: &str =
    "委外订单没有发料明细，无法发料；发料必须精确到匹，请先登记发料明细";

// =========================================================
// A) 空明细发料：真实 service 路径显式拒绝（business_displayable），
//    且订单与凭证零漂移（修复前该场景 0 次校验整单放行）
// =========================================================

#[tokio::test]
async fn issue_without_items_rejected_with_displayable_error_and_zero_drift() {
    let db = sqlite_db().await;
    seed_issue_domain_tables(&db).await;
    let order = seed_draft_order(&db).await;

    let service = OutsourcingOrderService::new(Arc::new(db.clone()));
    let err = service
        .issue_order(order.id)
        .await
        .expect_err("空明细发料必须被拒绝（修复前：0 条明细=0 次校验=整单放行）");
    assert!(
        matches!(&err, AppError::BusinessErrorDisplayable(m) if m == EMPTY_ITEMS_REJECT_MSG),
        "空明细必须是可外显业务拒绝且文案逐字一致，实际: {err:?}"
    );
    assert_zero_drift(&db, &order).await;
}

#[tokio::test]
async fn issue_without_items_http_envelope_400_business_error_real_msg() {
    let db = sqlite_db().await;
    seed_issue_domain_tables(&db).await;
    let order = seed_draft_order(&db).await;
    let app = issue_router(db.clone());

    let (status, v) = post_issue(&app, order.id).await;
    assert_eq!(status, StatusCode::BAD_REQUEST, "实际响应: {v}");
    assert_eq!(v["code"], "BUSINESS_ERROR", "实际响应: {v}");
    assert_eq!(v["message"], EMPTY_ITEMS_REJECT_MSG, "公开规则文案必须外显");
    assert_ne!(
        v["message"],
        serde_json::json!(err_msg::BUSINESS_PUBLIC),
        "business_displayable 不得被脱敏成固定常量"
    );
    assert_zero_drift(&db, &order).await;
}

// =========================================================
// B) 有明细但匹号非法：逐条校验分支（未填/引用不存在/非可用）仍按既有
//    脱敏 business 定性（文案携带查询所得缸号/匹号，不满足外显安全边界），
//    400 族 + 零漂移
// =========================================================

#[tokio::test]
async fn issue_items_without_piece_no_rejected_and_zero_drift() {
    for (case, piece_no) in [("未填（NULL）", None), ("空串", Some(""))] {
        let db = sqlite_db().await;
        seed_issue_domain_tables(&db).await;
        let order = seed_draft_order(&db).await;
        seed_item(&db, order.id, piece_no).await;

        let service = OutsourcingOrderService::new(Arc::new(db.clone()));
        let err = service
            .issue_order(order.id)
            .await
            .expect_err(&format!("明细缺生产匹号必须被拒绝（case: {case}）"));
        assert!(
            matches!(&err, AppError::BusinessError(m) if m.contains("必须填写生产匹号")),
            "case {case}：应为脱敏 business 且内部文案含规则说明，实际: {err:?}"
        );
        assert_zero_drift(&db, &order).await;
    }
}

#[tokio::test]
async fn issue_item_referencing_nonexistent_piece_rejected_and_zero_drift() {
    let db = sqlite_db().await;
    seed_issue_domain_tables(&db).await;
    let order = seed_draft_order(&db).await;
    seed_item(&db, order.id, Some("PX-GHOST-W5G")).await;

    let service = OutsourcingOrderService::new(Arc::new(db.clone()));
    let err = service
        .issue_order(order.id)
        .await
        .expect_err("引用虚构匹号必须被拒绝");
    assert!(
        matches!(&err, AppError::BusinessError(m) if m.contains("不存在")),
        "应为脱敏 business（匹号是查询所得实体值），实际: {err:?}"
    );
    assert_zero_drift(&db, &order).await;
}

#[tokio::test]
async fn issue_item_referencing_unavailable_piece_rejected_and_zero_drift() {
    let db = sqlite_db().await;
    seed_issue_domain_tables(&db).await;
    let order = seed_draft_order(&db).await;
    seed_piece(&db, "PX-RESV-W5G", piece_status::RESERVED, 9001, 9001).await;
    seed_item(&db, order.id, Some("PX-RESV-W5G")).await;

    // HTTP 全链：脱敏 business 的出参形态（400 + BUSINESS_ERROR + 固定脱敏常量）
    let app = issue_router(db.clone());
    let (status, v) = post_issue(&app, order.id).await;
    assert_eq!(status, StatusCode::BAD_REQUEST, "实际响应: {v}");
    assert_eq!(v["code"], "BUSINESS_ERROR", "实际响应: {v}");
    assert_eq!(
        v["message"],
        serde_json::json!(err_msg::BUSINESS_PUBLIC),
        "文案含匹号/状态等查询所得实体值，出参必须保持脱敏常量"
    );
    assert_zero_drift(&db, &order).await;

    // service 级：内部真实文案可被日志侧检索
    let service = OutsourcingOrderService::new(Arc::new(db.clone()));
    let err = service
        .issue_order(order.id)
        .await
        .expect_err("已预留(RESERVED)匹必须被拒绝");
    assert!(
        matches!(&err, AppError::BusinessError(m) if m.contains("非可用")),
        "实际: {err:?}"
    );
    assert_zero_drift(&db, &order).await;
}

// =========================================================
// C) 正向对照（活库，防"一刀切拒绝"）：合规明细 + 可用匹 → 发料成功、
//    状态推进 issued、OVIS 发料凭证恰生成 1 张。
//    issue 成功路径必经 generate_no_with_txn 的 pg_advisory_xact_lock
//    （number_generator.rs L257-270），sqlite 方言不支持，不得伪装成
//    sqlite 用例（先例 wave1/wave5 同口径）；本地无 TEST_DATABASE_URL 时
//    setup_test_db 回退 sqlite，用例行首方言断言显式失败并说明，
//    CI 由 ci-test-rust-ignored 以 --include-ignored 真实执行。
// =========================================================

#[tokio::test]
#[ignore = "需要 TEST_DATABASE_URL 指向已迁移 PostgreSQL：发料成功路径的单号生成用 pg_advisory_xact_lock，sqlite 方言不支持，无法非活库化"]
async fn live_issue_with_compliant_piece_succeeds_on_postgres() {
    let db = test_common::setup_test_db().await;
    assert_eq!(
        db.get_database_backend(),
        DbBackend::Postgres,
        "本用例必须跑在 TEST_DATABASE_URL 指向的已迁移 PostgreSQL 上；\
         本地未设置该变量时 setup_test_db 回退 sqlite，此处显式失败（非跳过）"
    );

    // 活库基准引用数据：显式取真实存在的物料/仓库 ID，缺失即显式失败（不做静默兜底）
    let product_id: i32 = bingxi_backend::models::product::Entity::find()
        .select_only()
        .column(bingxi_backend::models::product::Column::Id)
        .into_tuple::<(i32,)>()
        .one(&db)
        .await
        .unwrap()
        .map(|(id,)| id)
        .expect("活库夹具：products 表无任何记录，正向对照无法执行（需 seed 基准数据）");
    let warehouse_id: i32 = bingxi_backend::models::warehouse::Entity::find()
        .select_only()
        .column(bingxi_backend::models::warehouse::Column::Id)
        .into_tuple::<(i32,)>()
        .one(&db)
        .await
        .unwrap()
        .map(|(id,)| id)
        .expect("活库夹具：warehouses 表无任何记录，正向对照无法执行（需 seed 基准数据）");

    let tag = unique_tag();
    let piece_no = format!("PW5G-{}-001", tag);
    seed_piece(
        &db,
        &piece_no,
        piece_status::AVAILABLE,
        product_id,
        warehouse_id,
    )
    .await;
    let order = seed_draft_order(&db).await;
    seed_item(&db, order.id, Some(piece_no.as_str())).await;

    let service = OutsourcingOrderService::new(Arc::new(db.clone()));
    let updated = service
        .issue_order(order.id)
        .await
        .expect("合规发料（有明细+真实可用匹）必须成功——空明细拒绝不得扩大化成一刀切");

    assert_eq!(
        updated.status,
        outsourcing_order_status::ISSUED,
        "正向对照：状态必须推进 issued"
    );
    let voucher_no = updated
        .voucher_no_issue
        .clone()
        .expect("正向对照：发料凭证号必须回写");
    assert!(
        voucher_no.starts_with("OVIS"),
        "发料凭证号必须来自 OVIS 号段，实际: {voucher_no}"
    );

    // 回查落库事实：恰 1 张 issue 凭证，且订单列与返回体一致（非内存态自证）
    let vouchers = outsourcing_voucher::Entity::find()
        .filter(outsourcing_voucher::Column::OutsourcingOrderId.eq(order.id))
        .all(&db)
        .await
        .unwrap();
    assert_eq!(vouchers.len(), 1, "正向对照：应恰生成 1 张发料凭证");
    assert_eq!(vouchers[0].voucher_type, outsourcing_voucher_type::ISSUE);
    assert_eq!(vouchers[0].voucher_no, voucher_no);
    let persisted = outsourcing_order::Entity::find_by_id(order.id)
        .one(&db)
        .await
        .unwrap()
        .expect("正向对照：订单必须存在");
    assert_eq!(persisted.status, outsourcing_order_status::ISSUED);
    assert_eq!(
        persisted.voucher_no_issue.as_deref(),
        Some(voucher_no.as_str())
    );
}
