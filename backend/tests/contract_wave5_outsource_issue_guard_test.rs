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
    extract::State,
    http::{Method, Request, StatusCode},
    middleware::{Next, from_fn_with_state},
    response::Response,
};
use bingxi_backend::container::AppState;
use bingxi_backend::handlers::outsourcing_handler;
use bingxi_backend::middleware::auth_context::AuthContext;
use bingxi_backend::models::inventory_piece;
use bingxi_backend::models::outsourcing_order;
use bingxi_backend::models::outsourcing_order_item;
use bingxi_backend::models::outsourcing_voucher;
use bingxi_backend::models::status::outsourcing_order_status;
use bingxi_backend::models::status::outsourcing_order_type;
use bingxi_backend::models::status::outsourcing_voucher_type;
use bingxi_backend::models::status::purchase_inventory::inventory_piece as piece_status;
use bingxi_backend::services::outsourcing_service::OutsourcingOrderService;
use bingxi_backend::services::piece_domain_service;
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

/// 发料/取消/收回三个端点现在经 auth_middleware 注入的 AuthContext 取操作者，
/// 匹状态流转把该 user_id 写入 inventory_piece.updated_by（审计溯源）。
/// 测试路由不挂生产中间件，故以 inject_auth 注入固定操作者，与生产语义等价。
const OPERATOR: i32 = 7788;

fn make_auth(user_id: i32) -> AuthContext {
    AuthContext {
        user_id,
        username: format!("wave5g_user_{user_id}"),
        role_id: Some(2),
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

fn issue_router(db: DatabaseConnection) -> Router {
    let mut state = AppState::default();
    state.db = Arc::new(db);
    Router::new()
        .route(
            "/outsourcing-orders/{id}/issue",
            axum::routing::post(outsourcing_handler::issue_outsourcing_order),
        )
        .with_state(state)
        .layer(from_fn_with_state(make_auth(OPERATOR), inject_auth))
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
        .issue_order(order.id, Some(OPERATOR))
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
            .issue_order(order.id, Some(OPERATOR))
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
        .issue_order(order.id, Some(OPERATOR))
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
        .issue_order(order.id, Some(OPERATOR))
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
    let piece = seed_piece(
        &db,
        &piece_no,
        piece_status::AVAILABLE,
        product_id,
        warehouse_id,
    )
    .await;
    let piece_id = piece.id;
    let order = seed_draft_order(&db).await;
    seed_item(&db, order.id, Some(piece_no.as_str())).await;

    let service = OutsourcingOrderService::new(Arc::new(db.clone()));
    let updated = service
        .issue_order(order.id, Some(OPERATOR))
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

    // 占用闭环（#194）：发料事务提交后，被引用匹必须已被 CAS 置 RESERVED。
    // 发料成功路径必经 generate_no_with_txn 的 pg_advisory_xact_lock（见本用例
    // #[ignore] 说明），故"发料成功→RESERVED"整链只能活库真跑；CAS 占用本身
    // （不需要行锁）由下方 sqlite 段 D1 直接真跑域服务覆盖。
    let reserved = inventory_piece::Entity::find_by_id(piece_id)
        .one(&db)
        .await
        .unwrap()
        .expect("占用闭环：发料后必须能回查被引用匹");
    assert_eq!(
        reserved.status,
        piece_status::RESERVED,
        "正向对照：发料成功后 inventory_piece.status 必须为 RESERVED（占用闭环）"
    );
    assert_eq!(
        reserved.updated_by,
        Some(OPERATOR),
        "审计溯源：发料成功链（AVAILABLE→RESERVED）的 updated_by 必须为操作者，非空非伪造"
    );
}

// =========================================================
// 结算零费用门 + 收回零数量门（决策定案：委外三条业务裁量中的第 2、3 条）
//
// 覆盖边界（不假装全绿）：
// - settle 的拒绝发生在**取号与 begin() 之前**，故 sqlite 能真跑全链、真回读零漂移；
// - 收回单 confirm 的 0 量门位于 receipt 行 lock_exclusive 之后（sqlite 不支持
//   lock_exclusive，仓内先例见本文件 E 段/wave5 receipt_return_three_state），
//   因此 create/update/confirm 三处门以**源码扫描锁**钉住族、文案与位置，
//   活库真跑用例留给 CI 的 --ignored job 与后续 DDL 夹具补齐批次（挂账任务 #194）。
// =========================================================

/// 种一张已收回(received)态委外订单，费用两列由入参决定（sqlite/活库同构两用）
async fn seed_received_order(
    db: &DatabaseConnection,
    processing_fee: Decimal,
    freight_fee: Decimal,
) -> outsourcing_order::Model {
    let now: chrono::DateTime<chrono::FixedOffset> = Utc::now().into();
    outsourcing_order::ActiveModel {
        order_no: Set(format!("OW-W5S-{}", unique_tag())),
        order_type: Set(outsourcing_order_type::DYEING.to_string()),
        supplier_id: Set(1),
        issue_date: Set(now.date_naive()),
        issue_quantity: Set(dec("100.00")),
        issue_unit: Set("米".to_string()),
        return_quantity: Set(dec("100.00")),
        loss_quantity: Set(Decimal::ZERO),
        material_cost: Set(dec("1000.00")),
        processing_fee: Set(processing_fee),
        freight_fee: Set(freight_fee),
        tax_amount: Set(Decimal::ZERO),
        abnormal_loss_amount: Set(Decimal::ZERO),
        total_cost: Set(Decimal::ZERO),
        unit_cost: Set(Decimal::ZERO),
        status: Set(outsourcing_order_status::RECEIVED.to_string()),
        is_deleted: Set(false),
        created_by: Set(Some(9101)),
        created_at: Set(now),
        updated_at: Set(now),
        ..Default::default()
    }
    .insert(db)
    .await
    .expect("夹具：种 received 委外订单失败")
}

fn settle_router(db: DatabaseConnection) -> Router {
    let mut state = AppState::default();
    state.db = Arc::new(db);
    Router::new()
        .route(
            "/outsourcing-orders/{id}/settle",
            axum::routing::post(outsourcing_handler::settle_outsourcing_order),
        )
        .with_state(state)
}

async fn post_settle(app: &Router, id: i32) -> (StatusCode, Value) {
    let req = Request::builder()
        .method(Method::POST)
        .uri(format!("/outsourcing-orders/{}/settle", id))
        .body(Body::empty())
        .expect("构造 POST 请求失败");
    let resp: Response = app.clone().oneshot(req).await.unwrap();
    let status = resp.status();
    let bytes = axum::body::to_bytes(resp.into_body(), 4 * 1024 * 1024)
        .await
        .unwrap();
    (status, serde_json::from_slice(&bytes).unwrap())
}

/// 零费用（加工费+运费<=0）结算必须被拒：400 + BUSINESS_ERROR + 可外显公开规则文案，
/// 且订单整行零漂移、该单 outsourcing_voucher 计数为 0（不许落一张金额为 0 的空壳凭证）。
#[tokio::test]
async fn settle_with_zero_fee_is_rejected_with_displayable_message_and_no_empty_voucher() {
    let db = sqlite_db().await;
    seed_issue_domain_tables(&db).await;
    let order = seed_received_order(&db, Decimal::ZERO, Decimal::ZERO).await;
    let app = settle_router(db.clone());

    let (status, body) = post_settle(&app, order.id).await;
    assert_eq!(
        status,
        StatusCode::BAD_REQUEST,
        "零费用结算应 400，实际 body={body}"
    );
    assert_eq!(
        body["code"], "BUSINESS_ERROR",
        "前置未满足属业务族，实际={body}"
    );
    assert_eq!(
        body["message"], "加工费与运费合计需大于 0 才能结算，请先补录委外加工成本",
        "公开业务规则必须外显真实原因（不是脱敏常量），实际={body}"
    );

    let after = outsourcing_order::Entity::find_by_id(order.id)
        .one(&db)
        .await
        .unwrap()
        .expect("零结算门：订单必须存在");
    assert_eq!(
        after.status,
        outsourcing_order_status::RECEIVED,
        "被拒后订单状态不得推进为 settled"
    );
    assert_eq!(
        after.voucher_no_fee, None,
        "被拒后不得回写加工费凭证号，实际={:?}",
        after.voucher_no_fee
    );
    assert_eq!(
        after.updated_at, order.updated_at,
        "被拒后不得触碰 updated_at（零副作用）"
    );
    let voucher_count = outsourcing_voucher::Entity::find()
        .filter(outsourcing_voucher::Column::OutsourcingOrderId.eq(order.id))
        .count(&db)
        .await
        .unwrap();
    assert_eq!(
        voucher_count, 0,
        "零费用结算被拒时不得生成任何凭证（含金额为 0 的空壳 OVFE 凭证）"
    );
}

/// 源码扫描锁：收回单三处 0 量门（create/update/confirm）的族、文案与"先于写入"位置。
/// 这三处走 outsourcing_receipt 表 + 行锁，sqlite 无法真跑，故以扫描锁防漂移，
/// 并在测试文件头声明其覆盖边界（不伪装成端到端）。
#[tokio::test]
async fn receipt_zero_quantity_gates_are_wired_in_all_three_paths() {
    let src = include_str!("../src/services/outsourcing_ops/receipt.rs").replace('\r', "");

    // create：0/负数一律 VALIDATION_ERROR 族 + 可外显公开规则（字段取值域），
    // 且必须是函数体第一条校验（先于 validate_create_request 与 insert）
    let create_at = src
        .find("pub async fn create(")
        .expect("receipt.rs 必须有 create 入口");
    let create_head = &src[create_at..create_at + 1200];
    assert!(
        create_head.contains("if req.return_quantity <= Decimal::ZERO"),
        "create 必须把收回数量下界从「负数」收紧到「<=0」，否则 0 量单照样能建"
    );
    assert!(
        create_head.contains("AppError::validation_displayable(\"收回数量必须大于零\")"),
        "create 的 0 量拒绝必须走 VALIDATION 族且外显公开规则文案"
    );
    let gate_idx = create_head
        .find("if req.return_quantity <= Decimal::ZERO")
        .unwrap();
    let validate_idx = create_head
        .find("Self::validate_create_request")
        .expect("create 应调用建单前置校验");
    assert!(
        gate_idx < validate_idx,
        "数量取值域门必须先于建单前置校验与任何写入"
    );

    // update：三态分支内同一族同一文案（禁 create/update 两套口径）
    assert_eq!(
        src.matches("if v <= Decimal::ZERO").count(),
        1,
        "update 的 return_quantity 分支必须有同一 <=0 门"
    );
    assert_eq!(
        src.matches("AppError::validation_displayable(\"收回数量必须大于零\")")
            .count(),
        2,
        "create 与 update 两处必须同源同文案（不得一个外显一个脱敏）"
    );

    // confirm：0 量草稿（门控上线前既有数据）也必须被拦，且先于凭证/库存/订单写入
    let confirm_at = src
        .find("pub async fn confirm(")
        .expect("receipt.rs 必须有 confirm 入口");
    let confirm_head = &src[confirm_at..confirm_at + 2600];
    assert!(
        confirm_head.contains("if receipt_model.return_quantity <= Decimal::ZERO"),
        "confirm 必须对存量 0 量草稿做兜底门控，否则 0 量单仍可确认并污染成本链"
    );
    assert!(
        confirm_head.contains("收回数量为 0，无法确认回仓；请先录入实际收回数量"),
        "confirm 的 0 量拒绝必须外显行动路径"
    );
    let confirm_gate = confirm_head
        .find("if receipt_model.return_quantity <= Decimal::ZERO")
        .unwrap();
    let eligibility = confirm_head
        .find("validate_receipt_eligibility")
        .expect("confirm 应调用超发/资格校验");
    assert!(
        confirm_gate < eligibility,
        "0 量门必须先于资格校验与任何凭证/库存写入"
    );
    assert!(
        !confirm_head.contains("return_quantity < Decimal::ZERO"),
        "confirm 不得残留「只拦负数」的旧口径"
    );
}

// =========================================================
// D) 任务 #194：委外发料匹状态占用与释放闭环（CAS 条件更新）
//
// 可测性边界（不假装全绿）：
// - CAS 占用/释放不需要行锁，sqlite 可真实跑：D1 直接真跑域服务
//   reserve_pieces_for_issue（AVAILABLE→RESERVED 落库回查），
//   D2/D3 走完整 issue_order 服务路径（拒绝分支全部发生在凭证取号
//   pg_advisory_xact_lock 之前，可在 sqlite 真跑到拒绝与回滚）；
//   D4 cancel 释放路径无行锁/无取号，sqlite 真跑整链。
// - "发料成功后 RESERVED" 的端到端正向必经 OVIS 取号（advisory lock，
//   sqlite 不支持），由上方 #[ignore] 活库用例 C 覆盖；
// - confirm 收回转 SHIPPED 位于 lock_exclusive 之后（sqlite 不支持），
//   与既有 0 量门同策：以源码扫描锁钉住"事务内、commit 前"的接线，
//   活库真跑留给 CI --ignored 与后续批次（测试专家补点）。
// - 审计溯源 updated_by（#941 本批）：流转与操作者写入在同一条 CAS update_many 内，
//   故凡能真跑 CAS 的路径都能真回读 updated_by——D1（占用 AVAILABLE→RESERVED）、
//   D4（释放 RESERVED→AVAILABLE）在 sqlite 真断言 updated_by=操作者；
//   "发料成功链/收回转出链"的 updated_by 分别由活库用例 C（真断言）与 D5 的
//   cas_piece_status 源码扫描锁（updated_by 必须位于 set 与 exec 之间，且迁移回填
//   SET 段不得含 updated_by）覆盖——sqlite 跑不到这两条链的行锁路径，不假装全绿。
// =========================================================

/// D1 正向（sqlite 真跑）：域服务 CAS 占用把明细引用的 AVAILABLE 匹置 RESERVED
#[tokio::test]
async fn reserve_pieces_for_issue_marks_piece_reserved_on_sqlite() {
    let db = sqlite_db().await;
    seed_issue_domain_tables(&db).await;
    let order = seed_draft_order(&db).await;
    seed_piece(&db, "PX-OCCUPY-D1", piece_status::AVAILABLE, 9001, 9001).await;
    let item = seed_item(&db, order.id, Some("PX-OCCUPY-D1")).await;

    piece_domain_service::reserve_pieces_for_issue(
        &db,
        std::slice::from_ref(&item),
        Some(OPERATOR),
    )
    .await
    .expect("可用匹的 CAS 占用必须成功（AVAILABLE→RESERVED）");

    let after = inventory_piece::Entity::find()
        .filter(inventory_piece::Column::PieceNo.eq("PX-OCCUPY-D1"))
        .one(&db)
        .await
        .unwrap()
        .expect("占用后必须能回查该匹");
    assert_eq!(
        after.status,
        piece_status::RESERVED,
        "发料占用闭环：reserve 成功后 inventory_piece.status 必须为 RESERVED"
    );
    assert_eq!(
        after.updated_by,
        Some(OPERATOR),
        "审计溯源：CAS 占用（AVAILABLE→RESERVED）必须与状态同条 update_many 写入操作者 updated_by"
    );
}

/// D2 负例（跨单互斥）：第一张单已占用 RESERVED 后，第二张 draft 单引用同匹发料
/// → HTTP 400 BUSINESS_ERROR + 第二张单零漂移 + 匹仍归第一张单的占用。
#[tokio::test]
async fn issue_second_order_referencing_reserved_piece_rejected_with_zero_drift() {
    let db = sqlite_db().await;
    seed_issue_domain_tables(&db).await;
    // 第一张单：走域服务真实 CAS 占用（等价于其发料事务提交的库存效果）
    let first = seed_draft_order(&db).await;
    seed_piece(&db, "PX-DOUBLE-D2", piece_status::AVAILABLE, 9001, 9001).await;
    let first_item = seed_item(&db, first.id, Some("PX-DOUBLE-D2")).await;
    piece_domain_service::reserve_pieces_for_issue(
        &db,
        std::slice::from_ref(&first_item),
        Some(OPERATOR),
    )
    .await
    .expect("夹具：第一张单占用必须成功");
    let mut first_active: outsourcing_order::ActiveModel = first.clone().into();
    first_active.status = Set(outsourcing_order_status::ISSUED.to_string());
    first_active
        .update(&db)
        .await
        .expect("夹具：第一张单推进 issued 失败");

    // 第二张单：同一匹号发料，HTTP 全链必须被拒
    let second = seed_draft_order(&db).await;
    seed_item(&db, second.id, Some("PX-DOUBLE-D2")).await;
    let app = issue_router(db.clone());
    let (status, body) = post_issue(&app, second.id).await;
    assert_eq!(
        status,
        StatusCode::BAD_REQUEST,
        "跨单重复发料必须 400，实际 body={body}"
    );
    assert_eq!(body["code"], "BUSINESS_ERROR", "实际响应: {body}");
    assert_eq!(
        body["message"],
        serde_json::json!(err_msg::BUSINESS_PUBLIC),
        "文案含匹号/状态等查询所得实体值，出参必须保持脱敏常量"
    );
    assert_zero_drift(&db, &second).await;

    // 占用不漂移：匹仍是第一张单的 RESERVED（拒绝不得部分改写库存）
    let piece = inventory_piece::Entity::find()
        .filter(inventory_piece::Column::PieceNo.eq("PX-DOUBLE-D2"))
        .one(&db)
        .await
        .unwrap()
        .expect("回查占用匹");
    assert_eq!(
        piece.status,
        piece_status::RESERVED,
        "第二张单被拒不得改变第一张单的 RESERVED 占用"
    );

    // service 级归因（日志侧真实文案可检索）
    let service = OutsourcingOrderService::new(Arc::new(db.clone()));
    let err = service
        .issue_order(second.id, Some(OPERATOR))
        .await
        .expect_err("已预留匹跨单重复发料必须被拒绝");
    assert!(
        matches!(&err, AppError::BusinessError(m) if m.contains("非可用")),
        "应为脱敏 business 且内部文案含归因，实际: {err:?}"
    );
}

/// D3 负例（同单重复引用）：同一订单两条明细引用同一 AVAILABLE 匹 → 显式拒绝。
/// 修复前 validate 的批量 map 双双放行，CAS 循环会把第二条误判为自占用；
/// 拒绝必须整单零副作用（匹不得被部分占用）。
#[tokio::test]
async fn issue_same_order_duplicate_piece_no_rejected_with_zero_drift() {
    let db = sqlite_db().await;
    seed_issue_domain_tables(&db).await;
    let order = seed_draft_order(&db).await;
    seed_piece(&db, "PX-SAMEDUP-D3", piece_status::AVAILABLE, 9001, 9001).await;
    seed_item(&db, order.id, Some("PX-SAMEDUP-D3")).await;
    seed_item(&db, order.id, Some("PX-SAMEDUP-D3")).await;

    let service = OutsourcingOrderService::new(Arc::new(db.clone()));
    let err = service
        .issue_order(order.id, Some(OPERATOR))
        .await
        .expect_err("同单两条明细引用同一匹必须被拒绝");
    assert!(
        matches!(&err, AppError::BusinessError(m) if m.contains("重复引用生产匹")),
        "应为脱敏 business（文案含匹号）且归因为同单重复引用，实际: {err:?}"
    );
    assert_zero_drift(&db, &order).await;

    let piece = inventory_piece::Entity::find()
        .filter(inventory_piece::Column::PieceNo.eq("PX-SAMEDUP-D3"))
        .one(&db)
        .await
        .unwrap()
        .expect("回查被引用匹");
    assert_eq!(
        piece.status,
        piece_status::AVAILABLE,
        "整单拒绝不得留下部分占用（该匹必须仍为 AVAILABLE）"
    );
}

/// D4 释放闭环（sqlite 真跑整链）：issued 单取消后，被占用匹 CAS 回 AVAILABLE。
#[tokio::test]
async fn cancel_issued_order_releases_reserved_piece_to_available() {
    let db = sqlite_db().await;
    seed_issue_domain_tables(&db).await;
    let order = seed_draft_order(&db).await;
    seed_piece(&db, "PX-RELEASE-D4", piece_status::AVAILABLE, 9001, 9001).await;
    let item = seed_item(&db, order.id, Some("PX-RELEASE-D4")).await;
    piece_domain_service::reserve_pieces_for_issue(
        &db,
        std::slice::from_ref(&item),
        Some(OPERATOR),
    )
    .await
    .expect("夹具：占用必须成功");
    let mut issued: outsourcing_order::ActiveModel = order.clone().into();
    issued.status = Set(outsourcing_order_status::ISSUED.to_string());
    let issued = issued.update(&db).await.expect("夹具：推进 issued 失败");

    let service = OutsourcingOrderService::new(Arc::new(db.clone()));
    let cancelled = service
        .cancel(issued.id, Some(OPERATOR))
        .await
        .expect("issued 单取消必须成功（释放路径不得硬失败）");
    assert_eq!(
        cancelled.status,
        outsourcing_order_status::CANCELLED,
        "取消后订单状态必须为 cancelled"
    );

    let piece = inventory_piece::Entity::find()
        .filter(inventory_piece::Column::PieceNo.eq("PX-RELEASE-D4"))
        .one(&db)
        .await
        .unwrap()
        .expect("回查释放匹");
    assert_eq!(
        piece.status,
        piece_status::AVAILABLE,
        "取消闭环：issued 单取消后 RESERVED 匹必须 CAS 回 AVAILABLE"
    );
    assert_eq!(
        piece.updated_by,
        Some(OPERATOR),
        "审计溯源：取消释放（RESERVED→AVAILABLE）必须把操作者写入 updated_by"
    );
}

/// D5 源码扫描锁：占用/释放/转出的调用点必须位于各自事务 begin() 之后、
/// commit() 之前（顺序即原子性契约，勿匹配 message 文案）。
/// sqlite 无法真跑 issue 成功链与 confirm 链（advisory lock / lock_exclusive），
/// 本锁是这两条"事务内接线"的防漂移手段。
#[tokio::test]
async fn piece_occupancy_calls_are_wired_inside_their_transactions() {
    // issue_order：begin < reserve_pieces_for_issue(&txn..) < commit，
    // 且不得残留事务外 validate（TOCTOU 窗口源头）
    let order_src = include_str!("../src/services/outsourcing_ops/order.rs").replace('\r', "");
    let issue_at = order_src
        .find("pub async fn issue_order(")
        .expect("order.rs 必须有 issue_order 入口");
    let issue_end = order_src[issue_at..]
        .find("pub async fn record_processing(")
        .map(|o| issue_at + o)
        .expect("issue_order 与 record_processing 之间即其函数体");
    let issue_body = &order_src[issue_at..issue_end];
    let begin = issue_body
        .find("(*self.db).begin()")
        .expect("issue_order 必须开启事务");
    let reserve = issue_body
        .find("reserve_pieces_for_issue(&txn")
        .expect("占用调用必须传发料事务（&txn），不得传 &self.db");
    let commit = issue_body
        .find("txn.commit()")
        .expect("issue_order 必须有显式提交");
    assert!(
        begin < reserve && reserve < commit,
        "占用必须位于 begin() 之后、commit() 之前（任一行失败整体回滚）"
    );
    assert!(
        !issue_body.contains("validate_pieces_for_issue(&*self.db"),
        "发料校验/占用不得回到事务外（TOCTOU 重复发料窗口）"
    );

    // cancel：占用释放分支的释放调用位于其事务内
    let cancel_at = order_src
        .find("pub async fn cancel(")
        .expect("order.rs 必须有 cancel 入口");
    let cancel_end = order_src[cancel_at..]
        .find("pub async fn get_by_id(")
        .map(|o| cancel_at + o)
        .expect("cancel 与 get_by_id 之间即其函数体");
    let cancel_body = &order_src[cancel_at..cancel_end];
    let cancel_begin = cancel_body
        .find("(*self.db).begin()")
        .expect("cancel 的占用释放分支必须在事务内执行");
    let release = cancel_body
        .find("release_reserved_pieces_on_cancel(")
        .expect("cancel 必须接线 RESERVED→AVAILABLE 释放");
    let cancel_commit = cancel_body
        .find("txn.commit()")
        .expect("cancel 释放分支必须显式提交");
    assert!(
        cancel_begin < release && release < cancel_commit,
        "释放必须位于 begin() 之后、commit() 之前（与主单状态推进原子提交）"
    );

    // confirm：转出调用位于其事务内（confirm 首步即 begin）
    let receipt_src = include_str!("../src/services/outsourcing_ops/receipt.rs").replace('\r', "");
    let confirm_at = receipt_src
        .find("pub async fn confirm(")
        .expect("receipt.rs 必须有 confirm 入口");
    let confirm_end = receipt_src[confirm_at..]
        .find("async fn trigger_quality_inspection(")
        .map(|o| confirm_at + o)
        .expect("confirm 与 trigger_quality_inspection 之间即其函数体");
    let confirm_body = &receipt_src[confirm_at..confirm_end];
    let confirm_begin = confirm_body
        .find("(*self.db).begin()")
        .expect("confirm 首步即开事务");
    let shipped = confirm_body
        .find("mark_reserved_pieces_shipped_on_receipt(")
        .expect("confirm 必须接线 RESERVED→SHIPPED 转出");
    let confirm_commit = confirm_body
        .find("txn.commit()")
        .expect("confirm 必须有显式提交");
    assert!(
        confirm_begin < shipped && shipped < confirm_commit,
        "收回转出必须位于事务内（与收回单/凭证/订单原子提交）"
    );

    // 审计溯源落点（源码扫描锁）：cas_piece_status 必须在**同一条 update_many 的
    // .set(...ActiveModel) 与 .exec(...) 之间**写 updated_by——把「状态流转」与
    // 「操作者」压进同一条条件更新，是 CAS 原子性/零 N+1 与本闭环审计溯源的立身点。
    // 补第二次 UPDATE 会引入「状态已改、主体未写」中间态。发料成功链/收回转出链
    // 必经 advisory lock / lock_exclusive，sqlite 无法真跑到 updated_by，故以扫描锁
    // 钉住（真跑 updated_by 由 D1 占用、D4 释放两条 sqlite 用例覆盖）。
    let piece_src = include_str!("../src/services/piece_domain_service.rs").replace('\r', "");
    let cas_at = piece_src
        .find("async fn cas_piece_status(")
        .expect("piece_domain_service.rs 必须有 cas_piece_status");
    let cas_body = &piece_src[cas_at
        ..cas_at
            + piece_src[cas_at..]
                .find("Ok(result.rows_affected)")
                .expect("cas_piece_status 以 rows_affected 收尾")];
    assert!(
        cas_body.contains("operator_id: Option<i32>"),
        "cas_piece_status 必须显式接收操作者（Option<i32>：None=系统/回填路径写 NULL，不伪造）"
    );
    let set_at = cas_body
        .find(".set(inventory_piece::ActiveModel {")
        .expect("CAS 走 update_many 的 .set(ActiveModel)");
    let upd_at = cas_body
        .find("updated_by: Set(operator_id)")
        .expect("CAS 必须在同一 ActiveModel 内 Set updated_by");
    let exec_at = cas_body
        .find(".exec(conn)")
        .expect("CAS 必须单条 exec，不得拆成两次 UPDATE");
    assert!(
        set_at < upd_at && upd_at < exec_at,
        "updated_by 必须位于 set(ActiveModel) 与 exec 之间（同一 update_many 原子写入）"
    );

    // 口径一致性：迁移回填路径无操作主体，不得伪造 updated_by——回填 SQL 必须只
    // 置 status/updated_at、不含 updated_by 赋值（与运行家人工路径写真实 user_id 对照）。
    let backfill_src = include_str!(
        "../migration/src/domain/production/m0065_backfill_outsourcing_reserved_pieces.rs"
    )
    .replace('\r', "");
    let backfill_update = backfill_src
        .find("UPDATE \"inventory_piece\" p")
        .expect("回填迁移必须有 inventory_piece 置 RESERVED 的 UPDATE");
    // 只截取 SET ... 到首个 WHERE 之间的赋值段（回填的 updated_by 讨论仅在注释里，
    // 不在此段），避免误吃后续 RAISE/子查询文本
    let backfill_where = backfill_update
        + backfill_src[backfill_update..]
            .find("WHERE")
            .expect("回填 UPDATE 必须带 WHERE 条件");
    let backfill_set = &backfill_src[backfill_update..backfill_where];
    assert!(
        backfill_set.contains("\"status\" = 'RESERVED'") && backfill_set.contains("\"updated_at\""),
        "回填 SET 段应只含 status/updated_at（口径与本文件运行时 CAS 的时间戳一致）"
    );
    assert!(
        !backfill_set.contains("updated_by"),
        "回填 UPDATE 的 SET 段不得写 updated_by：系统回填无操作主体，伪造用户 ID 属制造数据"
    );
}
