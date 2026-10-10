//! 委外发料匹号门控契约锁：空明细必须显式拒绝、不得整单放行
//!
//! 门控背景（为什么必须入口拦截）：
//! - `services/piece_domain_service.rs::validate_pieces_for_issue` 的门控是逐条明细
//!   校验（`for it in items`），入口不拦时 0 条明细 = 0 次校验 = 静默 `Ok(())`；
//! - 唯一调用方 `services/outsourcing_ops/order.rs::issue_order` 装载明细后
//!   随即进校验，中间**没有空明细前置拦截**
//!   （`outsourcing_ops/order_item.rs::list_by_order` 仅按订单 ID 过滤）；
//! - handler `POST /outsourcing-orders/:id/issue`（`outsourcing_handler.rs`）
//!   是薄透传，同样无明细数校验；
//! - 于是空明细订单会被推进 `issued` 并在事务内生成 OVIS 发料凭证，
//!   与「发料精确到匹」的域规则相悖，属静默数据完整性漏洞。
//!
//! 现门控：`validate_pieces_for_issue` 入口显式拒绝空明细
//! （`AppError::business_displayable`，公开规则文案、无内部标识/记录 ID）；
//! 既有逐条拒绝分支（未填匹号/匹不存在/非可用）文案携带查询所得缸号，
//! 按 `utils/error.rs` 模块文档保持脱敏 `business` 形态。
//!
//! 覆盖策略（无 mock、真实 service/handler 路径；表结构唯一来源 = backend/migration，
//! 不自建 sqlite 同构表——自建 DDL 与模型 Decimal(14,4) 方言矛盾会引发成片
//! ColumnDecode 失败）：
//! - 两条拒绝路径均发生在 `issue_order` 取号生成凭证之前（拒绝即事务回滚、零副作用），
//!   真 PG（test_common::setup_test_db，
//!   连接已迁移库并清空业务表）上 handler→service→domain 全链真实跑通；
//! - 零漂移不只看错误码：拒绝后回查订单**整行逐列相等**（Model PartialEq）、
//!   该订单凭证行数=0、OVIS 前缀凭证数=0；
//! - FK 前置自种子：inventory_piece.product_id → products、
//!   warehouse_id → warehouses 均为真表外键，会被清空且不播种——夹具自种父行，
//!   不指望环境已有数据；
//! - 正向对照（合规明细发料成功）必经 `generate_no_with_txn`
//!   （`utils/number_generator.rs::lock_prefix` 为 `pg_advisory_xact_lock`，
//!   PG 专有能力，只能在活库真跑——与既有 contract_wave1_* 活库用例同口径）：
//!   `#[ignore]` 活库（TEST_DATABASE_URL→已迁移 PG，CI `ci-test-rust-ignored`
//!   以 --include-ignored 执行；夹具缺 TEST_DATABASE_URL 或指向 sqlite 直接 panic，
//!   不存在静默回退）。

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
    ActiveModelTrait, ColumnTrait, DatabaseConnection, DbBackend, EntityTrait, PaginatorTrait, Set,
};
use serde_json::Value;
use std::str::FromStr;
use std::sync::Arc;
use tower::ServiceExt;

// =========================================================
// 公共夹具（真 PostgreSQL，表结构唯一来源 = backend/migration；
// outsourcing_order/outsourcing_order_item/outsourcing_voucher/inventory_piece
// 由迁移建齐，本文件不再自建 CREATE TABLE）
// =========================================================

fn dec(s: &str) -> Decimal {
    Decimal::from_str(s).unwrap()
}

async fn live_db() -> DatabaseConnection {
    test_common::setup_test_db().await
}

fn unique_tag() -> i64 {
    Utc::now()
        .timestamp_nanos_opt()
        .expect("测试环境时间戳必须可用")
}

/// FK 前置自种子：inventory_piece.product_id → products、
/// warehouse_id → warehouses 为真表外键（business/m0010），两表会被清空且不播种，
/// 缺父行就造父行。返回 (product_id, warehouse_id)。
async fn seed_piece_parents(db: &DatabaseConnection) -> (i32, i32) {
    let now = Utc::now();
    let suffix = unique_tag();
    let p = bingxi_backend::models::product::ActiveModel {
        name: Set(format!("委外发料契约坯布-{suffix}")),
        code: Set(format!("FAB-W5G-{suffix}")),
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
    .expect("夹具：种 products 父行失败");
    let wh = bingxi_backend::models::warehouse::ActiveModel {
        warehouse_code: Set(format!("WH-W5G-{suffix}")),
        name: Set("委外发料契约仓".to_string()),
        is_default: Set(false),
        is_active: Set(true),
        created_at: Set(now),
        updated_at: Set(now),
        ..Default::default()
    }
    .insert(db)
    .await
    .expect("夹具：种 warehouses 父行失败");
    (p.id, wh.id)
}

/// 种一张 draft 委外订单（全部 NOT NULL 列显式赋值；outsourcing_order 无 DB 外键，
/// supplier_id 为业务引用值。发料主单量口径=issue_quantity DECIMAL(14,4)，
/// 与模型 src/models/outsourcing_order.rs 逐列同源）
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

/// 种一条发料明细（piece_no 三态由入参决定：None=未填、Some("")=空串、Some(值)=引用；
/// outsourcing_order_item.product_id 真表无外键约束，沿用业务引用值即可）
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
    let state = AppState {
        db: Arc::new(db),
        ..Default::default()
    };
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
//    且订单与凭证零漂移（空明细若整单放行，正是"0 次校验推进 issued"的漏洞形态）
// =========================================================

#[tokio::test]
async fn issue_without_items_rejected_with_displayable_error_and_zero_drift() {
    let db = live_db().await;
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
    let db = live_db().await;
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
        let db = live_db().await;
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
    let db = live_db().await;
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
    let db = live_db().await;
    let order = seed_draft_order(&db).await;
    let (pid, wid) = seed_piece_parents(&db).await;
    seed_piece(&db, "PX-RESV-W5G", piece_status::RESERVED, pid, wid).await;
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
//    （number_generator.rs，PG 专有的事务级咨询锁），只能在活库真跑；
//    夹具缺 TEST_DATABASE_URL 或指向 sqlite 直接 panic，绝不静默回退。
//    CI 由 ci-test-rust-ignored 以 --include-ignored 真实执行。
// =========================================================

#[tokio::test]
#[ignore = "需要 TEST_DATABASE_URL 指向已迁移 PostgreSQL：发料成功路径的单号生成用 pg_advisory_xact_lock（PG 专有），无法在非活库通道执行"]
async fn live_issue_with_compliant_piece_succeeds_on_postgres() {
    let db = test_common::setup_test_db().await;
    assert_eq!(
        db.get_database_backend(),
        DbBackend::Postgres,
        "本用例必须跑在 TEST_DATABASE_URL 指向的已迁移 PostgreSQL 上；\
         夹具缺该变量或指向 sqlite 时 setup_test_db 已直接 panic（非跳过）"
    );

    // FK 前置自种子：products/warehouses 会被清空且不播种，本用例自造父行，
    // 不指望环境已有数据（直接取环境首行在活库清空后必然取空）。
    let (product_id, warehouse_id) = seed_piece_parents(&db).await;

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

    // 占用闭环：发料事务提交后，被引用匹必须已被 CAS 置 RESERVED。
    // 发料成功路径必经 generate_no_with_txn 的 pg_advisory_xact_lock（见本用例
    // #[ignore] 说明），故"发料成功→RESERVED"整链只能活库真跑；CAS 占用本身
    // （不需要行锁）由下方 D1 用例直接真跑域服务覆盖。
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
// 结算零费用门 + 收回零数量门（委外三条业务裁量中的第 2、3 条）
//
// 覆盖边界（不假装全绿）：
// - settle 的拒绝发生在**取号与 begin() 之前**，故真 PG 常规分片即可真跑全链、真回读零漂移；
// - 收回单 confirm 的 0 量门位于 receipt 行 lock_exclusive 之后（锁路径本文件不端到端真跑），
//   因此 create/update/confirm 三处门以**源码扫描锁**钉住族、文案与位置，
// 活库端到端真跑不在本文件覆盖范围。
// =========================================================

/// 种一张已收回(received)态委外订单，费用两列由入参决定
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
    let state = AppState {
        db: Arc::new(db),
        ..Default::default()
    };
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
    let db = live_db().await;
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

// =========================================================
// 源码扫描锁公共工具（本文件两把扫描锁共用）
//
// 为什么需要这三件套（三形态的"测的写法脆"，而非源码脆）
// 1. `&src[at..at + 1200]` 这类**固定字节窗**在含中文的源码里会在多字节字符中间
//    劈开 → `byte index is not a char boundary` panic（被锁源码一旦增删注释，
//    字节偏移即整体移动，固定窗必炸）。
//    正解：窗的边界一律由**符号**定位（`fn_body`），不做任何硬字节长度假设。
// 2. 单行字面 needle 会因 rustfmt 折行/尾逗号假失败（`cas_piece_status` 已泛型化为
//    `async fn cas_piece_status<C: ConnectionTrait>(`）。正解：正向 contains 之前
//    双侧同一套 `canon` 规范化（剔空白 + 消尾逗号），排版与判定解耦。
// 3. 禁词/needle 命中源码里的**说明性注释**（"这里以前是 X，现已改为 Y"）属假判。
//    正解：禁项只看执行体 —— 先 `code_only` 剥整行注释，再判。
// =========================================================

/// 只保留"代码 + 字符串字面量"：整行注释（`//`、`///`、`//!`）逐行剔除。
/// 按行而非按字符扫描的理由：被锁源码大量使用**跨行 raw string**（SQL），单行做
/// 引号配平会把字符串里的 `"` 当成字符串起止、进而把真代码文本当注释吃掉；
/// 宁少剥（块注释与行尾尾注释不处理）不可错剥——误吃代码会让"必备项"断言假失败，
/// 而漏吃注释只会让"禁项"断言偏保守，两者中后者才是可接受的偏差方向。
fn code_only(src: &str) -> String {
    src.lines()
        .filter(|line| !line.trim_start().starts_with("//"))
        .collect::<Vec<_>>()
        .join("\n")
}

/// 规范形（**仅用于正向 contains**）：在 `code_only` 文本上剔除全部空白，并消掉闭合
/// 定界符前的尾逗号（rustfmt 把 `f(a, b)` 拆成多行后末实参必带 `,`，是折行的必然
/// 产物而非语义）。负向 `!contains` 一律只用 `code_only`，不套本函数——去空白会
/// 扩大匹配面（与本文件既有 `strip_ws` 纪律、`sku_mapping_contract_test.rs:138` 同口径）。
fn canon(src: &str) -> String {
    let mut out: String = code_only(src)
        .chars()
        .filter(|c| !c.is_whitespace())
        .collect();
    loop {
        let next = out.replace(",)", ")").replace(",]", "]").replace(",}", "}");
        if next == out {
            break;
        }
        out = next;
    }
    out
}

/// 符号定位取函数体：从 `signature`（**跳过注释里的同名词**，只在 `code_only` 文本上找）
/// 起，到下一个函数/条目符号之前为止。替代"行号常量"与"符号 + 固定字节窗"两种脆写法：
/// 本仓重构会移动行号与字节偏移，而符号名是测试与被锁源码之间唯一稳定的锚点
/// （文件头与 `contract_wave5_delete_precheck_and_status_literal_lock_test.rs:23` 同式先例）。
fn fn_body(code_src: &str, signature: &str) -> String {
    let anchor = code_src.find(signature).unwrap_or_else(|| {
        panic!("待锁符号不存在: {signature}（若该符号确已被改名/删除，请连同本锁一起更新判据）")
    });
    let tail = &code_src[anchor..];
    let next = [
        "\npub async fn ",
        "\npub(crate) async fn ",
        "\npub fn ",
        "\npub(crate) fn ",
        "\nasync fn ",
        "\nfn ",
        "\n    pub async fn ",
        "\n    pub(crate) async fn ",
        "\n    pub fn ",
        "\n    pub(crate) fn ",
        "\n    async fn ",
        "\n    fn ",
    ]
    .iter()
    .filter_map(|marker| tail.find(marker))
    .min()
    .unwrap_or(tail.len());
    tail[..next].to_string()
}

/// 源码扫描锁：收回单三处 0 量门（create/update/confirm）的族、文案与"先于写入"位置。
/// 这三处走 outsourcing_receipt 表 + 行锁，sqlite 无法真跑，故以扫描锁防漂移，
/// 并在测试文件头声明其覆盖边界（不伪装成端到端）。
#[tokio::test]
async fn receipt_zero_quantity_gates_are_wired_in_all_three_paths() {
    let src = include_str!("../src/services/outsourcing_ops/receipt.rs").replace('\r', "");
    // 禁项与"必备项"都只在剥注释后的文本上判定：源码里的说明注释（"旧口径是只拦负数，
    // 现改为 <=0"之类）不是执行体，命中它既可能假判违例、也可能假判已接线。
    let code = code_only(&src);
    // 正向 needle 走 canon（rustfmt 折行/尾逗号不参与判定）
    let flat = canon(&src);

    // create：0/负数一律 VALIDATION_ERROR 族 + 可外显公开规则（字段取值域），
    // 且必须是函数体第一条校验（先于 validate_create_request 与 insert）
    let create_body = fn_body(&code, "pub async fn create(");
    assert!(
        canon(&create_body).contains(&canon("if req.return_quantity <= Decimal::ZERO")),
        "create 必须把收回数量下界从「负数」收紧到「<=0」，否则 0 量单照样能建"
    );
    assert!(
        flat.contains(&canon(
            "AppError::validation_displayable(\"收回数量必须大于零\")"
        )),
        "create 的 0 量拒绝必须走 VALIDATION 族且外显公开规则文案"
    );
    let create_flat = canon(&create_body);
    let gate_idx = create_flat
        .find(&canon("if req.return_quantity <= Decimal::ZERO"))
        .unwrap();
    let validate_idx = create_flat
        .find(&canon("Self::validate_create_request"))
        .expect("create 应调用建单前置校验");
    let insert_idx = create_flat
        .find(&canon(".insert(&*self.db)"))
        .expect("create 的落库点必须在函数体内");
    assert!(
        gate_idx < validate_idx && gate_idx < insert_idx,
        "数量取值域门必须先于建单前置校验与任何写入"
    );

    // update：三态分支内同一族同一文案（禁 create/update 两套口径）
    assert_eq!(
        code.matches("if v <= Decimal::ZERO").count(),
        1,
        "update 的 return_quantity 分支必须有同一 <=0 门"
    );
    assert_eq!(
        flat.matches(&canon(
            "AppError::validation_displayable(\"收回数量必须大于零\")"
        ))
        .count(),
        2,
        "create 与 update 两处必须同源同文案（不得一个外显一个脱敏）"
    );

    // confirm：0 量草稿（门控上线前既有数据）也必须被拦，且先于凭证/库存/订单写入
    let confirm_body = fn_body(&code, "pub async fn confirm(");
    let confirm_flat = canon(&confirm_body);
    assert!(
        confirm_flat.contains(&canon("if receipt_model.return_quantity <= Decimal::ZERO")),
        "confirm 必须对存量 0 量草稿做兜底门控，否则 0 量单仍可确认并污染成本链"
    );
    assert!(
        confirm_flat.contains(&canon("收回数量为 0，无法确认回仓；请先录入实际收回数量")),
        "confirm 的 0 量拒绝必须外显行动路径"
    );
    let confirm_gate = confirm_flat
        .find(&canon("if receipt_model.return_quantity <= Decimal::ZERO"))
        .unwrap();
    let eligibility = confirm_flat
        .find(&canon("validate_receipt_eligibility"))
        .expect("confirm 应调用超发/资格校验");
    let first_write = confirm_flat
        .find(&canon(".insert(&txn)"))
        .expect("confirm 的事务内写入点必须在函数体内");
    assert!(
        confirm_gate < eligibility && confirm_gate < first_write,
        "0 量门必须先于资格校验与任何凭证/库存写入"
    );
    assert!(
        !confirm_body.contains("return_quantity < Decimal::ZERO"),
        "confirm 不得残留「只拦负数」的旧口径"
    );
}

// =========================================================
// D) 委外发料匹状态占用与释放闭环（CAS 条件更新）
//
// 可测性边界（不假装全绿；本文件统一真 PG）：
// - CAS 占用/释放不需要行锁/取号：D1 直接真跑域服务 reserve_pieces_for_issue
//   （AVAILABLE→RESERVED 落库回查），D2/D3 走完整 issue_order 服务路径
//   （拒绝分支全部发生在凭证取号 pg_advisory_xact_lock 之前，真跑归因与零漂移）；
//   D4 cancel 释放路径无行锁/无取号，真跑整链。
// - "发料成功后 RESERVED" 的端到端正向必经 OVIS 取号（pg_advisory_xact_lock），
//   由上方 #[ignore] 活库用例 C 覆盖（走 ci-test-rust-ignored 通道）；
// - confirm 收回转 SHIPPED 位于 lock_exclusive 之后，本文件以源码扫描锁
//   钉住"事务内、commit 前"的接线，活库端到端真跑不在本文件覆盖范围。
// - 审计溯源 updated_by：流转与操作者写入在同一条 CAS update_many 内，
//   故凡能真跑 CAS 的路径都能真回读 updated_by——D1（占用 AVAILABLE→RESERVED）、
//   D4（释放 RESERVED→AVAILABLE）真断言 updated_by=操作者；
//   "发料成功链/收回转出链"的 updated_by 分别由活库用例 C（真断言）与 D5 的
//   cas_piece_status 源码扫描锁（updated_by 必须位于 set 与 exec 之间，且迁移回填
//   SET 段不得含 updated_by）覆盖——D5 两条链的行锁路径不真跑，不假装全绿。
// =========================================================

/// D1 正向（真 PG CAS 占用，无需行锁/取号，可在常规分片真跑）：域服务 CAS
/// 占用把明细引用的 AVAILABLE 匹置 RESERVED
#[tokio::test]
async fn reserve_pieces_for_issue_marks_piece_reserved_on_postgres() {
    let db = live_db().await;
    let (pid, wid) = seed_piece_parents(&db).await;
    let order = seed_draft_order(&db).await;
    seed_piece(&db, "PX-OCCUPY-D1", piece_status::AVAILABLE, pid, wid).await;
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
    let db = live_db().await;
    let (pid, wid) = seed_piece_parents(&db).await;
    // 第一张单：走域服务真实 CAS 占用（等价于其发料事务提交的库存效果）
    let first = seed_draft_order(&db).await;
    seed_piece(&db, "PX-DOUBLE-D2", piece_status::AVAILABLE, pid, wid).await;
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
/// 同单重复在 `reserve_pieces_for_issue` 入口按「一匹一条明细」显式拦截——
/// 若放行，CAS 循环会把第二条误判为自占用；
/// 拒绝必须整单零副作用（匹不得被部分占用）。
#[tokio::test]
async fn issue_same_order_duplicate_piece_no_rejected_with_zero_drift() {
    let db = live_db().await;
    let (pid, wid) = seed_piece_parents(&db).await;
    let order = seed_draft_order(&db).await;
    seed_piece(&db, "PX-SAMEDUP-D3", piece_status::AVAILABLE, pid, wid).await;
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

/// D4 释放闭环（真 PG 整链，取消无行锁/取号）：issued 单取消后，被占用匹 CAS 回 AVAILABLE。
#[tokio::test]
async fn cancel_issued_order_releases_reserved_piece_to_available() {
    let db = live_db().await;
    let (pid, wid) = seed_piece_parents(&db).await;
    let order = seed_draft_order(&db).await;
    seed_piece(&db, "PX-RELEASE-D4", piece_status::AVAILABLE, pid, wid).await;
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
    // 边界一律由符号定位（fn_body），不再用"本函数符号 + 下一个函数符号的行号/字节偏移"
    // 这种随重构漂移的写法；顺序判定在 canon 文本上做，rustfmt 折行不参与判定。
    let order_src = include_str!("../src/services/outsourcing_ops/order.rs").replace('\r', "");
    let order_code = code_only(&order_src);
    let issue_body = fn_body(&order_code, "pub async fn issue_order(");
    let issue_flat = canon(&issue_body);
    let begin = issue_flat
        .find(&canon("(*self.db).begin()"))
        .expect("issue_order 必须开启事务");
    let reserve = issue_flat
        .find(&canon("reserve_pieces_for_issue(&txn"))
        .expect("占用调用必须传发料事务（&txn），不得传 &self.db");
    let commit = issue_flat
        .find(&canon("txn.commit()"))
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
    let cancel_body = fn_body(&order_code, "pub async fn cancel(");
    let cancel_flat = canon(&cancel_body);
    let cancel_begin = cancel_flat
        .find(&canon("(*self.db).begin()"))
        .expect("cancel 的占用释放分支必须在事务内执行");
    let release = cancel_flat
        .find(&canon("release_reserved_pieces_on_cancel("))
        .expect("cancel 必须接线 RESERVED→AVAILABLE 释放");
    let cancel_commit = cancel_flat
        .find(&canon("txn.commit()"))
        .expect("cancel 释放分支必须显式提交");
    assert!(
        cancel_begin < release && release < cancel_commit,
        "释放必须位于 begin() 之后、commit() 之前（与主单状态推进原子提交）"
    );

    // confirm：转出调用位于其事务内（confirm 首步即 begin）
    let receipt_src = include_str!("../src/services/outsourcing_ops/receipt.rs").replace('\r', "");
    let receipt_code = code_only(&receipt_src);
    let confirm_body = fn_body(&receipt_code, "pub async fn confirm(");
    let confirm_flat = canon(&confirm_body);
    let confirm_begin = confirm_flat
        .find(&canon("(*self.db).begin()"))
        .expect("confirm 首步即开事务");
    let shipped = confirm_flat
        .find(&canon("mark_reserved_pieces_shipped_on_receipt("))
        .expect("confirm 必须接线 RESERVED→SHIPPED 转出");
    let confirm_commit = confirm_flat
        .find(&canon("txn.commit()"))
        .expect("confirm 必须有显式提交");
    assert!(
        confirm_begin < shipped && shipped < confirm_commit,
        "收回转出必须位于事务内（与收回单/凭证/订单原子提交）"
    );

    // 审计溯源落点（源码扫描锁）：cas_piece_status 必须在**同一条 update_many 的
    // .set(...ActiveModel) 与 .exec(...) 之间**写 updated_by——把「状态流转」与
    // 「操作者」压进同一条条件更新，是 CAS 原子性/零 N+1 与本闭环审计溯源的立身点。
    // 补第二次 UPDATE 会引入「状态已改、主体未写」中间态。发料成功链/收回转出链
    // 必经 advisory lock / lock_exclusive，端到端只能由活库用例覆盖，故以扫描锁
    // 钉住（真跑 updated_by 由 D1 占用、D4 释放两条真 PG 用例覆盖）。
    let piece_src = include_str!("../src/services/piece_domain_service.rs").replace('\r', "");
    let piece_code = code_only(&piece_src);
    // 符号只锚到函数名（不带 `(`）：该函数已泛型化为
    // `async fn cas_piece_status<C: ConnectionTrait>(`，把泛型参数写进 needle 会随
    // 签名排版漂移而假失败。
    let cas_body = fn_body(&piece_code, "async fn cas_piece_status");
    let cas_flat = canon(&cas_body);
    assert!(
        cas_flat.contains(&canon("Ok(result.rows_affected)")),
        "cas_piece_status 必须以 rows_affected 收尾（0 命中交调用方归因，绝不静默）"
    );
    assert!(
        cas_flat.contains(&canon("operator_id: Option<i32>")),
        "cas_piece_status 必须显式接收操作者（Option<i32>：None=系统/回填路径写 NULL，不伪造）"
    );
    let set_at = cas_flat
        .find(&canon(".set(inventory_piece::ActiveModel {"))
        .expect("CAS 走 update_many 的 .set(ActiveModel)");
    let upd_at = cas_flat
        .find(&canon("updated_by: Set(operator_id)"))
        .expect("CAS 必须在同一 ActiveModel 内 Set updated_by");
    let exec_at = cas_flat
        .find(&canon(".exec(conn)"))
        .expect("CAS 必须单条 exec，不得拆成两次 UPDATE");
    assert!(
        set_at < upd_at && upd_at < exec_at,
        "updated_by 必须位于 set(ActiveModel) 与 exec 之间（同一 update_many 原子写入）"
    );
    // 三处调用点必须真的接线，且逐点钉死「用自身收到的连接 + 权威状态词表 +
    // 把操作者透传给 CAS」三件事：只锁定义存在等于没锁（丢 operator_id = 审计断链、
    // 改状态词表 = 越出写入方权威表、改回 &self.db = 脱离调用方事务）。
    // needle 用 canon 比对：泛型化 + rustfmt 拆行后调用点是跨行形态。
    for (caller, wired_call) in [
        (
            "reserve_pieces_for_issue",
            "cas_piece_status(conn, pn, piece_status::AVAILABLE, piece_status::RESERVED, operator_id)",
        ),
        (
            "release_reserved_pieces_on_cancel",
            "cas_piece_status(conn, &pn, piece_status::RESERVED, piece_status::AVAILABLE, operator_id)",
        ),
        (
            "mark_reserved_pieces_shipped_on_receipt",
            "cas_piece_status(conn, &pn, piece_status::RESERVED, piece_status::SHIPPED, operator_id)",
        ),
    ] {
        let body = fn_body(&piece_code, &format!("async fn {caller}"));
        assert!(
            canon(&body).contains(&canon(wired_call)),
            "{caller} 的 CAS 接线形态缺失或被改动（应为 `{wired_call}`）：\
             占用/释放/转出三处任一处丢连接、丢操作者或换状态词表都属回潮"
        );
    }

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
