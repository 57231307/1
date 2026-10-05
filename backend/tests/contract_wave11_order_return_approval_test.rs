//! 交易域三单（采购订单/销售订单/采购退货）审批「两动作两理由」真库契约锁（后端线）
//!
//! 已定案口径（本文件逐条钉死，锁口径见各用例头注释）：
//! - 审批 = 通过/拒绝两条动作；PO/SO/退货三域的**通过理由选填**（缺 body/缺键/
//!   空串/纯空白一律落 NULL，禁止实现成"根本不采集"，也禁止伪造成必填）；
//!   **拒绝理由服务端必填**（trim 非空）并落各自专列（三域 reject 端点均已存在）。
//! - 专列落点（m0079 + 既有列）：purchase_orders.rejected_reason(VARCHAR 255)/
//!   approval_reason/cancel_reason；sales_orders.rejected_reason/approval_reason
//!   （reject 停写 notes）；purchase_return.rejected_reason(m0009)/approval_reason
//!   （reject 停写 reason_detail——止毁锁：必须断"另一列没被写"）。
//! - PO cancel 落 cancel_reason，不得再挪用 rejected_reason。
//! - rejected 为审批结论终态：退货单终态后再 approve/reject 均 400+BUSINESS_ERROR。
//! - 可外显校验文案定性、不含记录 ID（`utils/error.rs` 安全边界）。
//!
//! 夹具形态照 `contract_wave11_price_reject_test.rs`：真库 `setup_test_db`
//! （已迁移 PG、nextest 串行 db-integration 组）；suppliers 属封存参照表自增回读，
//! products/users 等业务表逐例 TRUNCATE 后可显式固定 id。
//! 路由仅在测试 Router 内注册（src routes 为枢纽文件，注册由主智能体落地），
//! 路径形态与真实路由逐字符一致（`/purchase/orders/{id}/…`、`/sales/orders/{id}/…`、
//! `/purchase/returns/{id}/…`）。列值断言一律走实体直读 DB（权威回读）。

mod test_common;

use bingxi_backend::container::AppState;
use bingxi_backend::handlers::{
    purchase_order_handler, purchase_return_handler, sales_order_handler,
};
use bingxi_backend::models::status::{
    purchase_order as po_status, purchase_return as pr_status, sales_order as so_status,
};
use bingxi_backend::models::{
    customer, product, purchase_order, purchase_return, purchase_return_item, sales_order,
    supplier, user, warehouse,
};
use chrono::{DateTime, Utc};
use rust_decimal::Decimal;
use sea_orm::{ActiveModelTrait, DatabaseConnection, EntityTrait, Set};
use serde_json::{json, Value};
use std::sync::Arc;
use test_common::setup_test_db;

const OPERATOR_ID: i32 = 9501;
/// 逐例种入的真实产品父行主键（products 属业务表、夹具逐例 TRUNCATE 后可显式固定 id）
const SEED_PRODUCT_ID: i32 = 9511;

fn now() -> DateTime<Utc> {
    Utc::now()
}

async fn seed_operator(db: &Arc<DatabaseConnection>) {
    user::ActiveModel {
        id: Set(OPERATOR_ID),
        username: Set("w11_order_approval".to_string()),
        password_hash: Set("test-only-not-a-real-hash".to_string()),
        real_name: Set(Some("订单退货审批契约锁操作人".to_string())),
        is_active: Set(true),
        is_totp_enabled: Set(false),
        created_at: Set(now()),
        updated_at: Set(now()),
        ..Default::default()
    }
    .insert(db.as_ref())
    .await
    .unwrap_or_else(|e| panic!("种子操作人插入失败: {e}"));
}

/// 种子真实供应商父行：suppliers 属封存参照表不被清空，显式 id 会跨例撞主键，
/// 自增插入后回读真实主键（形状照 `contract_wave11_price_reject_test::seed_supplier`）
async fn seed_supplier(db: &Arc<DatabaseConnection>, tag: &str) -> i32 {
    let s = supplier::ActiveModel {
        supplier_code: Set(format!("SUP-W11OA-{tag}")),
        supplier_name: Set("订单审批契约锁供应商".to_string()),
        supplier_short_name: Set("审供".to_string()),
        supplier_type: Set("面料供应商".to_string()),
        credit_code: Set("91330000TEST00000X".to_string()),
        registered_address: Set("测试注册地址".to_string()),
        legal_representative: Set("测试法人".to_string()),
        registered_capital: Set(Decimal::ZERO),
        establishment_date: Set(chrono::NaiveDate::from_ymd_opt(2020, 1, 1).expect("夹具日期常量")),
        taxpayer_type: Set("一般纳税人".to_string()),
        bank_name: Set("测试银行".to_string()),
        bank_account: Set("6222000000000000".to_string()),
        contact_phone: Set("13800000000".to_string()),
        created_at: Set(now().into()),
        updated_at: Set(now().into()),
        is_processor: Set(false),
        ..Default::default()
    }
    .insert(db.as_ref())
    .await
    .unwrap_or_else(|e| panic!("种子供应商插入失败: {e}"));
    s.id
}

/// 种子真实客户父行（customers 非封存参照表、随业务表清空，仍按 wave11 合同报价
/// 夹具范式自增回读，避免对各表清册口径的额外假设）
async fn seed_customer(db: &Arc<DatabaseConnection>, tag: &str) -> i32 {
    let c = customer::ActiveModel {
        customer_code: Set(format!("CUS-W11OA-{tag}")),
        customer_name: Set("订单审批契约锁客户".to_string()),
        credit_limit: Set(Decimal::ZERO),
        payment_terms: Set(30),
        status: Set("active".to_string()),
        customer_type: Set("retail".to_string()),
        owner_id: Set(OPERATOR_ID),
        created_at: Set(now()),
        updated_at: Set(now()),
        ..Default::default()
    }
    .insert(db.as_ref())
    .await
    .unwrap_or_else(|e| panic!("种子客户 {tag} 插入失败: {e}"));
    c.id
}

/// 种子真实仓库父行（warehouse_id 在 purchase_orders 为 NOT NULL + FK，
/// 形状照 `contract_wave2_po_item_update_fields_test.rs`）
async fn seed_warehouse(db: &Arc<DatabaseConnection>, tag: &str) -> i32 {
    let w = warehouse::ActiveModel {
        warehouse_code: Set(format!("WH-W11OA-{tag}")),
        name: Set("订单审批契约锁仓".to_string()),
        is_default: Set(false),
        is_active: Set(true),
        created_at: Set(now()),
        updated_at: Set(now()),
        ..Default::default()
    }
    .insert(db.as_ref())
    .await
    .unwrap_or_else(|e| panic!("种子仓库插入失败: {e}"));
    w.id
}

/// 种子真实产品父行（退货明细 FK）
async fn seed_product(db: &Arc<DatabaseConnection>) {
    product::ActiveModel {
        id: Set(SEED_PRODUCT_ID),
        name: Set("审批锁面料甲".to_string()),
        code: Set("PRD-W11OA".to_string()),
        unit: Set("米".to_string()),
        status: Set("active".to_string()),
        is_deleted: Set(false),
        product_type: Set("fabric".to_string()),
        created_at: Set(now()),
        updated_at: Set(now()),
        ..Default::default()
    }
    .insert(db.as_ref())
    .await
    .unwrap_or_else(|e| panic!("种子产品插入失败: {e}"));
}

/// 种一行采购订单：notes 预置"原始备注"作为 reject/cancel 挪用防护的断言锚点
async fn seed_po(
    db: &Arc<DatabaseConnection>,
    status: &str,
    supplier_id: i32,
    warehouse_id: i32,
    tag: &str,
) -> i32 {
    let po = purchase_order::ActiveModel {
        order_no: Set(format!("PO-W11OA-{tag}")),
        supplier_id: Set(supplier_id),
        order_date: Set(Utc::now().date_naive()),
        warehouse_id: Set(warehouse_id),
        department_id: Set(1),
        purchaser_id: Set(OPERATOR_ID),
        currency: Set("CNY".to_string()),
        exchange_rate: Set(Decimal::ONE),
        total_amount: Set(Decimal::ZERO),
        total_amount_foreign: Set(Decimal::ZERO),
        total_quantity: Set(Decimal::ZERO),
        total_quantity_alt: Set(Decimal::ZERO),
        order_status: Set(status.to_string()),
        notes: Set(Some("原始备注".to_string())),
        created_by: Set(OPERATOR_ID),
        created_at: Set(now()),
        updated_at: Set(now()),
        ..Default::default()
    }
    .insert(db.as_ref())
    .await
    .unwrap_or_else(|e| panic!("种子采购订单 {tag} 失败: {e}"));
    po.id
}

/// 种一行销售订单（无明细：approve 后置 MRP 循环空转，reject 释放预留空转）
async fn seed_so(db: &Arc<DatabaseConnection>, status: &str, customer_id: i32, tag: &str) -> i32 {
    let order = sales_order::ActiveModel {
        order_no: Set(format!("SO-W11OA-{tag}")),
        customer_id: Set(customer_id),
        order_date: Set(now()),
        status: Set(status.to_string()),
        subtotal: Set(Decimal::ZERO),
        tax_amount: Set(Decimal::ZERO),
        discount_amount: Set(Decimal::ZERO),
        shipping_cost: Set(Decimal::ZERO),
        total_amount: Set(Decimal::ZERO),
        paid_amount: Set(Decimal::ZERO),
        balance_amount: Set(Decimal::ZERO),
        notes: Set(Some("原始备注".to_string())),
        created_by: Set(Some(OPERATOR_ID)),
        created_at: Set(now()),
        updated_at: Set(now()),
        ..Default::default()
    }
    .insert(db.as_ref())
    .await
    .unwrap_or_else(|e| panic!("种子销售订单 {tag} 失败: {e}"));
    order.id
}

/// SO 行权威回读（列值断言一律直读 DB）
async fn sales_order_read(db: &Arc<DatabaseConnection>, id: i32) -> sales_order::Model {
    sales_order::Entity::find_by_id(id)
        .one(db.as_ref())
        .await
        .unwrap()
        .expect("SO 行必须存在")
}

/// 种一行采购退货单（无来源订单/无仓库：approve 的库存扣减与回写均显式跳过，
/// 拒绝只需主表行）
async fn seed_return(
    db: &Arc<DatabaseConnection>,
    status: &str,
    supplier_id: i32,
    tag: &str,
) -> i32 {
    let r = purchase_return::ActiveModel {
        return_no: Set(format!("RT-W11OA-{tag}")),
        supplier_id: Set(supplier_id),
        return_date: Set(Utc::now().date_naive()),
        return_status: Set(Some(status.to_string())),
        reason_detail: Set(Some("退货原因明细".to_string())),
        created_by: Set(Some(OPERATOR_ID)),
        created_at: Set(now()),
        updated_at: Set(now()),
        ..Default::default()
    }
    .insert(db.as_ref())
    .await
    .unwrap_or_else(|e| panic!("种子采购退货单 {tag} 失败: {e}"));
    r.id
}

/// 退货 approve 前置：至少一行明细（服务层 item_count==0 即业务拒绝）
async fn seed_return_item(db: &Arc<DatabaseConnection>, return_id: i32, tag: &str) {
    purchase_return_item::ActiveModel {
        return_id: Set(return_id),
        line_no: Set(1),
        product_id: Set(SEED_PRODUCT_ID),
        quantity: Set(Decimal::ONE),
        quantity_alt: Set(Decimal::ZERO),
        unit_price: Set(Decimal::ONE),
        unit_price_foreign: Set(Decimal::ONE),
        discount_percent: Set(Decimal::ZERO),
        tax_percent: Set(Decimal::ZERO),
        subtotal: Set(Decimal::ONE),
        tax_amount: Set(Decimal::ZERO),
        discount_amount: Set(Decimal::ZERO),
        total_amount: Set(Decimal::ONE),
        color_no: Set(String::new()),
        dye_lot_no: Set(String::new()),
        batch_no: Set(String::new()),
        created_at: Set(now()),
        updated_at: Set(now()),
        ..Default::default()
    }
    .insert(db.as_ref())
    .await
    .unwrap_or_else(|e| panic!("种子退货明细 {tag} 失败: {e}"));
}

/// HTTP 形态夹具：三域 approve/reject（+PO cancel）七端点；auth 注入与空串剔除
/// 中间件照 `contract_wave11_price_reject_test.rs` 先例。
async fn order_approval_app() -> (Arc<DatabaseConnection>, axum::Router) {
    let db = Arc::new(setup_test_db().await);
    seed_operator(&db).await;
    let state = AppState {
        db: db.clone(),
        ..Default::default()
    };
    async fn inject_auth(
        auth: axum::extract::State<bingxi_backend::middleware::auth_context::AuthContext>,
        mut request: axum::http::Request<axum::body::Body>,
        next: axum::middleware::Next,
    ) -> axum::response::Response {
        request.extensions_mut().insert(auth.0);
        next.run(request).await
    }
    let auth = bingxi_backend::middleware::auth_context::AuthContext {
        user_id: OPERATOR_ID,
        username: "w11_order_approval".to_string(),
        role_id: None,
        department_id: None,
        data_scope: Some("all".to_string()),
        dept_ids: None,
        dept_member_user_ids: None,
    };
    let app = axum::Router::new()
        .route(
            "/purchase/orders/{id}/approve",
            axum::routing::post(purchase_order_handler::approve_order),
        )
        .route(
            "/purchase/orders/{id}/reject",
            axum::routing::post(purchase_order_handler::reject_order),
        )
        .route(
            "/purchase/orders/{id}/cancel",
            axum::routing::post(purchase_order_handler::cancel_order),
        )
        .route(
            "/sales/orders/{id}/approve",
            axum::routing::post(sales_order_handler::approve_order),
        )
        .route(
            "/sales/orders/{id}/reject",
            axum::routing::post(sales_order_handler::reject_order),
        )
        .route(
            "/purchase/returns/{id}/approve",
            axum::routing::post(purchase_return_handler::approve_purchase_return),
        )
        .route(
            "/purchase/returns/{id}/reject",
            axum::routing::post(purchase_return_handler::reject_purchase_return),
        )
        .with_state(state)
        .layer(axum::middleware::from_fn_with_state(auth, inject_auth))
        .layer(axum::middleware::from_fn(
            bingxi_backend::utils::query_params::normalize_empty_query_params,
        ));
    (db, app)
}

async fn post_json(app: &axum::Router, uri: &str, body: Value) -> (axum::http::StatusCode, Value) {
    use tower::ServiceExt;
    let resp = app
        .clone()
        .oneshot(
            axum::http::Request::builder()
                .method(axum::http::Method::POST)
                .uri(uri)
                .header(axum::http::header::CONTENT_TYPE, "application/json")
                .body(axum::body::Body::from(body.to_string()))
                .unwrap(),
        )
        .await
        .unwrap();
    let status = resp.status();
    let bytes = axum::body::to_bytes(resp.into_body(), 4 * 1024 * 1024)
        .await
        .unwrap();
    let text = String::from_utf8_lossy(&bytes).to_string();
    (
        status,
        serde_json::from_str(&text).unwrap_or(Value::String(text)),
    )
}

/// 无 body 的 POST（Option<Json<T>> 形态专用：不带 content-type、零长度 body）
async fn post_empty_body(app: &axum::Router, uri: &str) -> (axum::http::StatusCode, Value) {
    use tower::ServiceExt;
    let resp = app
        .clone()
        .oneshot(
            axum::http::Request::builder()
                .method(axum::http::Method::POST)
                .uri(uri)
                .body(axum::body::Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    let status = resp.status();
    let bytes = axum::body::to_bytes(resp.into_body(), 4 * 1024 * 1024)
        .await
        .unwrap();
    let text = String::from_utf8_lossy(&bytes).to_string();
    (
        status,
        serde_json::from_str(&text).unwrap_or(Value::String(text)),
    )
}

// ---------------------------------------------------------------------------
// 1) 三域 reject 空串/纯空白理由 ⇒ 400 + VALIDATION_ERROR，且行零变化。
//    改坏什么必红：摘掉 handler 的 validate()/trim 必填门 ⇒ 空白理由直达服务层
//    落库，本例红；把校验降成日志放行（静默兜底）⇒ 400 断言红。
// ---------------------------------------------------------------------------

#[tokio::test]
async fn po_reject_blank_reason_is_400_validation_and_row_untouched() {
    let (db, app) = order_approval_app().await;
    let supplier_id = seed_supplier(&db, "T1PO").await;
    let warehouse_id = seed_warehouse(&db, "T1PO").await;
    let id = seed_po(
        &db,
        po_status::PENDING_APPROVAL,
        supplier_id,
        warehouse_id,
        "T1PO",
    )
    .await;

    for blank in ["", "   ", "\t  "] {
        let (status, v) = post_json(
            &app,
            &format!("/purchase/orders/{id}/reject"),
            json!({ "reason": blank }),
        )
        .await;
        assert_eq!(
            status,
            axum::http::StatusCode::BAD_REQUEST,
            "PO 拒绝理由 {blank:?} 必须 400，实得 {v}"
        );
        assert_eq!(
            v["code"], "VALIDATION_ERROR",
            "PO 拒绝理由必填必须归 VALIDATION_ERROR 信封，实得 {v}"
        );
    }

    let row = purchase_order::Entity::find_by_id(id)
        .one(db.as_ref())
        .await
        .unwrap()
        .expect("PO 行必须存在");
    assert_eq!(
        row.order_status,
        po_status::PENDING_APPROVAL,
        "被拒的拒绝不得改状态"
    );
    assert_eq!(
        row.rejected_reason, None,
        "被拒的拒绝不得落 rejected_reason"
    );
}

#[tokio::test]
async fn so_reject_blank_reason_is_400_validation_and_row_untouched() {
    let (db, app) = order_approval_app().await;
    let customer_id = seed_customer(&db, "T1SO").await;
    let id = seed_so(&db, so_status::PENDING, customer_id, "T1SO").await;

    for blank in ["", "   ", "\t  "] {
        let (status, v) = post_json(
            &app,
            &format!("/sales/orders/{id}/reject"),
            json!({ "reason": blank }),
        )
        .await;
        assert_eq!(
            status,
            axum::http::StatusCode::BAD_REQUEST,
            "SO 拒绝理由 {blank:?} 必须 400，实得 {v}"
        );
        assert_eq!(
            v["code"], "VALIDATION_ERROR",
            "SO 拒绝理由必填必须归 VALIDATION_ERROR 信封，实得 {v}"
        );
    }

    let row = sales_order_read(&db, id).await;
    assert_eq!(row.status, so_status::PENDING, "被拒的拒绝不得改状态");
    assert_eq!(
        row.rejected_reason, None,
        "被拒的拒绝不得落 rejected_reason"
    );
    assert_eq!(
        row.notes.as_deref(),
        Some("原始备注"),
        "被拒的拒绝不得改写 notes"
    );
}

#[tokio::test]
async fn return_reject_blank_reason_is_400_validation_and_row_untouched() {
    let (db, app) = order_approval_app().await;
    let supplier_id = seed_supplier(&db, "T1RT").await;
    let id = seed_return(&db, pr_status::SUBMITTED, supplier_id, "T1RT").await;

    for blank in ["", "   ", "\t  "] {
        let (status, v) = post_json(
            &app,
            &format!("/purchase/returns/{id}/reject"),
            json!({ "reason": blank }),
        )
        .await;
        assert_eq!(
            status,
            axum::http::StatusCode::BAD_REQUEST,
            "退货拒绝理由 {blank:?} 必须 400，实得 {v}"
        );
        assert_eq!(
            v["code"], "VALIDATION_ERROR",
            "退货拒绝理由必填必须归 VALIDATION_ERROR 信封，实得 {v}"
        );
    }

    let row = purchase_return::Entity::find_by_id(id)
        .one(db.as_ref())
        .await
        .unwrap()
        .expect("退货行必须存在");
    assert_eq!(
        row.return_status.as_deref(),
        Some(pr_status::SUBMITTED),
        "被拒的拒绝不得改退货状态"
    );
    assert_eq!(
        row.rejected_reason, None,
        "被拒的拒绝不得落 rejected_reason"
    );
    assert_eq!(
        row.reason_detail.as_deref(),
        Some("退货原因明细"),
        "被拒的拒绝不得改写 reason_detail"
    );
}

// ---------------------------------------------------------------------------
// 2) PO/SO/退货 reject 权威路径：rejected_reason 逐字回读（trim 后原样），
//    且**另一列未被改写**（notes / reason_detail）——止毁的行为级锁。
//    改坏什么必红：so/contract.rs 回到写 notes ⇒ notes 断言红；
//    purchase_return_service.rs 回到写 reason_detail ⇒ reason_detail 断言红。
// ---------------------------------------------------------------------------

#[tokio::test]
async fn po_reject_persists_rejected_reason_verbatim_and_notes_untouched() {
    let (db, app) = order_approval_app().await;
    let supplier_id = seed_supplier(&db, "T2PO").await;
    let warehouse_id = seed_warehouse(&db, "T2PO").await;
    let id = seed_po(
        &db,
        po_status::PENDING_APPROVAL,
        supplier_id,
        warehouse_id,
        "T2PO",
    )
    .await;

    let raw = "  价格条款与资信复核不通过，拒绝  ";
    let (status, v) = post_json(
        &app,
        &format!("/purchase/orders/{id}/reject"),
        json!({ "reason": raw }),
    )
    .await;
    assert_eq!(
        status,
        axum::http::StatusCode::OK,
        "PO pending→rejected 权威路径必须 200，实得 {v}"
    );

    let row = purchase_order::Entity::find_by_id(id)
        .one(db.as_ref())
        .await
        .unwrap()
        .expect("PO 行必须存在");
    assert_eq!(
        row.order_status,
        po_status::REJECTED,
        "PO 拒绝须落 REJECTED 态"
    );
    assert_eq!(
        row.rejected_reason.as_deref(),
        Some(raw.trim()),
        "PO rejected_reason 必须逐字回读（trim 后原样，禁止只进日志）"
    );
    assert_eq!(
        row.notes.as_deref(),
        Some("原始备注"),
        "PO reject 不得改写 notes（专列已就位，挪用即止毁失效）"
    );
    assert_eq!(
        row.approval_reason, None,
        "两动作两理由：拒绝不得写 approval_reason"
    );
    assert_eq!(row.cancel_reason, None, "拒绝不得挪用 cancel_reason");
}

#[tokio::test]
async fn so_reject_persists_rejected_reason_verbatim_and_notes_untouched() {
    let (db, app) = order_approval_app().await;
    let customer_id = seed_customer(&db, "T2SO").await;
    let id = seed_so(&db, so_status::PENDING, customer_id, "T2SO").await;

    let raw = "  交期无法满足客户合同约束，拒绝  ";
    let (status, v) = post_json(
        &app,
        &format!("/sales/orders/{id}/reject"),
        json!({ "reason": raw }),
    )
    .await;
    assert_eq!(
        status,
        axum::http::StatusCode::OK,
        "SO pending→rejected 权威路径必须 200，实得 {v}"
    );

    let row = sales_order_read(&db, id).await;
    assert_eq!(row.status, so_status::REJECTED, "SO 拒绝须落 rejected 态");
    assert_eq!(
        row.rejected_reason.as_deref(),
        Some(raw.trim()),
        "SO rejected_reason 必须逐字回读（此前覆盖写 notes 的挪用必须收口）"
    );
    assert_eq!(
        row.notes.as_deref(),
        Some("原始备注"),
        "SO reject 不得改写 notes——notes 回归订单备注语义（止毁锁）"
    );
    assert_eq!(
        row.approval_reason, None,
        "两动作两理由：拒绝不得写 approval_reason"
    );
}

#[tokio::test]
async fn return_reject_persists_rejected_reason_and_reason_detail_untouched() {
    let (db, app) = order_approval_app().await;
    let supplier_id = seed_supplier(&db, "T2RT").await;
    let id = seed_return(&db, pr_status::SUBMITTED, supplier_id, "T2RT").await;

    let raw = "  质检判不合格，整批退回  ";
    let (status, v) = post_json(
        &app,
        &format!("/purchase/returns/{id}/reject"),
        json!({ "reason": raw }),
    )
    .await;
    assert_eq!(
        status,
        axum::http::StatusCode::OK,
        "退货 submitted→rejected 权威路径必须 200，实得 {v}"
    );

    let row = purchase_return::Entity::find_by_id(id)
        .one(db.as_ref())
        .await
        .unwrap()
        .expect("退货行必须存在");
    assert_eq!(
        row.return_status.as_deref(),
        Some(pr_status::REJECTED),
        "退货拒绝须落 rejected 态"
    );
    assert_eq!(
        row.rejected_reason.as_deref(),
        Some(raw.trim()),
        "退货 rejected_reason 专列必须逐字回读（此前写入方缺位）"
    );
    assert_eq!(
        row.reason_detail.as_deref(),
        Some("退货原因明细"),
        "退货 reject 不得改写 reason_detail——建单时的退货原因明细归位（止毁锁）"
    );
    assert_eq!(
        row.approval_reason, None,
        "两动作两理由：拒绝不得写 approval_reason"
    );
}

// ---------------------------------------------------------------------------
// 3) PO cancel 落 cancel_reason 专列，rejected_reason 不被挪用覆写。
//    改坏什么必红：po/contract.rs cancel 回到写 rejected_reason ⇒ cancel_reason 断言
//    与 rejected_reason 保持断言双双变红。
// ---------------------------------------------------------------------------
#[tokio::test]
async fn po_cancel_writes_cancel_reason_not_rejected_reason() {
    let (db, app) = order_approval_app().await;
    let supplier_id = seed_supplier(&db, "T3PO").await;
    let warehouse_id = seed_warehouse(&db, "T3PO").await;
    let id = seed_po(&db, po_status::DRAFT, supplier_id, warehouse_id, "T3PO").await;
    // 预置历史拒绝理由锚点：cancel 覆写它即挪用回潮
    let anchor_row = purchase_order::Entity::find_by_id(id)
        .one(db.as_ref())
        .await
        .unwrap()
        .expect("PO 行必须存在");
    let mut anchor_active: purchase_order::ActiveModel = anchor_row.into();
    anchor_active.rejected_reason = Set(Some("历史拒绝理由不得被覆写".to_string()));
    anchor_active.update(db.as_ref()).await.unwrap();

    // cancel 通道本批只改落列不改语义（原样落库、不 trim——trim 必填收口只裁决了
    // reject/approve 两动作，cancel 档不在本批口径内），故提交不带前后空白的理由
    let reason = "供应商产能不足，商务侧取消";
    let (status, v) = post_json(
        &app,
        &format!("/purchase/orders/{id}/cancel"),
        json!({ "reason": reason }),
    )
    .await;
    assert_eq!(
        status,
        axum::http::StatusCode::OK,
        "PO cancel 权威路径必须 200，实得 {v}"
    );

    let row = purchase_order::Entity::find_by_id(id)
        .one(db.as_ref())
        .await
        .unwrap()
        .expect("PO 行必须存在");
    assert_eq!(
        row.order_status,
        po_status::CANCELLED,
        "PO 取消须落 CANCELLED 态"
    );
    assert_eq!(
        row.cancel_reason.as_deref(),
        Some(reason),
        "cancel 理由必须逐字落 cancel_reason 专列"
    );
    assert_eq!(
        row.rejected_reason.as_deref(),
        Some("历史拒绝理由不得被覆写"),
        "cancel 不得再挪用 rejected_reason（两动作两列）"
    );
    assert_eq!(
        row.notes.as_deref(),
        Some("原始备注"),
        "cancel 不得改写 notes"
    );
}

// ---------------------------------------------------------------------------
// 4) approve 选填档三域：不带 body 放行且 approval_reason IS NULL；
//    纯空白同样落 NULL（选填不得伪造成必填）；带非空理由逐字落列。
//    改坏什么必红：入参退化成强类型 Json<T> ⇒ 无 body 调用解码层裸 400，形态①红；
//    把选填实现成必填 ⇒ 形态①/②红；理由只进日志不落列 ⇒ 形态③红。
// ---------------------------------------------------------------------------

#[tokio::test]
async fn po_approve_optional_reason_absent_blank_and_verbatim() {
    let (db, app) = order_approval_app().await;
    let supplier_id = seed_supplier(&db, "T4PO").await;
    let warehouse_id = seed_warehouse(&db, "T4PO").await;

    // 形态①：完全不带 body——Option<Json<T>> 放行且列保持 NULL
    let id1 = seed_po(
        &db,
        po_status::PENDING_APPROVAL,
        supplier_id,
        warehouse_id,
        "T4PO-A",
    )
    .await;
    let (status, v) = post_empty_body(&app, &format!("/purchase/orders/{id1}/approve")).await;
    assert_eq!(
        status,
        axum::http::StatusCode::OK,
        "PO 无 body 批准必须放行（选填档不得伪造成必填），实得 {v}"
    );
    let row = purchase_order::Entity::find_by_id(id1)
        .one(db.as_ref())
        .await
        .unwrap()
        .expect("PO 行必须存在");
    assert_eq!(
        row.order_status,
        po_status::APPROVED,
        "PO 批准须落 APPROVED 态"
    );
    assert_eq!(
        row.approval_reason, None,
        "未提供通过理由时列必须为 NULL（可空即 NULL，禁止落空串/伪值）"
    );

    // 形态②：纯空白理由——归一为 NULL
    let id2 = seed_po(
        &db,
        po_status::PENDING_APPROVAL,
        supplier_id,
        warehouse_id,
        "T4PO-B",
    )
    .await;
    let (status, v) = post_json(
        &app,
        &format!("/purchase/orders/{id2}/approve"),
        json!({ "approval_reason": "   " }),
    )
    .await;
    assert_eq!(
        status,
        axum::http::StatusCode::OK,
        "PO 空白理由批准必须放行，实得 {v}"
    );
    let row = purchase_order::Entity::find_by_id(id2)
        .one(db.as_ref())
        .await
        .unwrap()
        .expect("PO 行必须存在");
    assert_eq!(row.approval_reason, None, "纯空白理由必须落 NULL");

    // 形态③：非空理由逐字落列
    let id3 = seed_po(
        &db,
        po_status::PENDING_APPROVAL,
        supplier_id,
        warehouse_id,
        "T4PO-C",
    )
    .await;
    let raw = "  比价合规，予以通过  ";
    let (status, v) = post_json(
        &app,
        &format!("/purchase/orders/{id3}/approve"),
        json!({ "approval_reason": raw }),
    )
    .await;
    assert_eq!(
        status,
        axum::http::StatusCode::OK,
        "PO 带理由批准必须 200，实得 {v}"
    );
    let row = purchase_order::Entity::find_by_id(id3)
        .one(db.as_ref())
        .await
        .unwrap()
        .expect("PO 行必须存在");
    assert_eq!(
        row.approval_reason.as_deref(),
        Some(raw.trim()),
        "PO approval_reason 必须逐字落列回读（不再只进日志）"
    );
    assert_eq!(row.rejected_reason, None, "批准不得写 rejected_reason");
}

#[tokio::test]
async fn so_approve_optional_reason_absent_blank_and_verbatim() {
    let (db, app) = order_approval_app().await;
    let customer_id = seed_customer(&db, "T4SO").await;

    // 形态①：不带 body 放行且 NULL
    let id1 = seed_so(&db, so_status::PENDING, customer_id, "T4SO-A").await;
    let (status, v) = post_empty_body(&app, &format!("/sales/orders/{id1}/approve")).await;
    assert_eq!(
        status,
        axum::http::StatusCode::OK,
        "SO 无 body 批准必须放行（选填档不得伪造成必填），实得 {v}"
    );
    let row = sales_order_read(&db, id1).await;
    assert_eq!(row.status, so_status::APPROVED, "SO 批准须落 approved 态");
    assert_eq!(row.approval_reason, None, "未提供通过理由时列必须为 NULL");

    // 形态②：纯空白落 NULL
    let id2 = seed_so(&db, so_status::PENDING, customer_id, "T4SO-B").await;
    let (status, v) = post_json(
        &app,
        &format!("/sales/orders/{id2}/approve"),
        json!({ "approval_reason": "  \t" }),
    )
    .await;
    assert_eq!(
        status,
        axum::http::StatusCode::OK,
        "SO 空白理由批准必须放行，实得 {v}"
    );
    let row = sales_order_read(&db, id2).await;
    assert_eq!(row.approval_reason, None, "纯空白理由必须落 NULL");

    // 形态③：非空理由逐字落列，notes 不被覆写
    let id3 = seed_so(&db, so_status::PENDING, customer_id, "T4SO-C").await;
    let raw = "  客户信用与交期复核通过  ";
    let (status, v) = post_json(
        &app,
        &format!("/sales/orders/{id3}/approve"),
        json!({ "approval_reason": raw }),
    )
    .await;
    assert_eq!(
        status,
        axum::http::StatusCode::OK,
        "SO 带理由批准必须 200，实得 {v}"
    );
    let row = sales_order_read(&db, id3).await;
    assert_eq!(
        row.approval_reason.as_deref(),
        Some(raw.trim()),
        "SO approval_reason 必须逐字落列回读"
    );
    assert_eq!(
        row.notes.as_deref(),
        Some("原始备注"),
        "SO approve 补写理由不得覆写 notes"
    );
    assert_eq!(row.rejected_reason, None, "批准不得写 rejected_reason");
}

#[tokio::test]
async fn return_approve_optional_reason_absent_and_verbatim() {
    let (db, app) = order_approval_app().await;
    let supplier_id = seed_supplier(&db, "T4RT").await;
    seed_product(&db).await;

    // 形态①：不带 body 放行且 NULL
    let id1 = seed_return(&db, pr_status::SUBMITTED, supplier_id, "T4RT-A").await;
    seed_return_item(&db, id1, "T4RT-A").await;
    let (status, v) = post_empty_body(&app, &format!("/purchase/returns/{id1}/approve")).await;
    assert_eq!(
        status,
        axum::http::StatusCode::OK,
        "退货无 body 批准必须放行（选填档不得伪造成必填），实得 {v}"
    );
    let row = purchase_return::Entity::find_by_id(id1)
        .one(db.as_ref())
        .await
        .unwrap()
        .expect("退货行必须存在");
    assert_eq!(
        row.return_status.as_deref(),
        Some(pr_status::APPROVED),
        "退货批准须落 approved 态"
    );
    assert_eq!(row.approval_reason, None, "未提供通过理由时列必须为 NULL");

    // 形态②：非空理由逐字落列
    let id2 = seed_return(&db, pr_status::SUBMITTED, supplier_id, "T4RT-B").await;
    seed_return_item(&db, id2, "T4RT-B").await;
    let raw = "  退货数量与质检单一致，同意退货  ";
    let (status, v) = post_json(
        &app,
        &format!("/purchase/returns/{id2}/approve"),
        json!({ "approval_reason": raw }),
    )
    .await;
    assert_eq!(
        status,
        axum::http::StatusCode::OK,
        "退货带理由批准必须 200，实得 {v}"
    );
    let row = purchase_return::Entity::find_by_id(id2)
        .one(db.as_ref())
        .await
        .unwrap()
        .expect("退货行必须存在");
    assert_eq!(
        row.approval_reason.as_deref(),
        Some(raw.trim()),
        "退货 approval_reason 必须逐字落列回读（采集通道存在性锁）"
    );
    assert_eq!(
        row.reason_detail.as_deref(),
        Some("退货原因明细"),
        "approve 不得改写 reason_detail"
    );
    assert_eq!(row.rejected_reason, None, "批准不得写 rejected_reason");
}

// ---------------------------------------------------------------------------
// 5) 退货 rejected 终态双向拦截：再 approve、再 reject 均 400+BUSINESS_ERROR，
//    行零变化（终态无回退边；只断机器码与状态码、不断文案原文）。
//    改坏什么必红：门只拦 approve ⇒ reject 段红；门回潮放行 ⇒ 400 断言红。
// ---------------------------------------------------------------------------
#[tokio::test]
async fn return_rejected_row_is_terminal_for_both_approve_and_reject() {
    let (db, app) = order_approval_app().await;
    let supplier_id = seed_supplier(&db, "T5RT").await;
    let id = seed_return(&db, pr_status::SUBMITTED, supplier_id, "T5RT").await;

    let (status, v) = post_json(
        &app,
        &format!("/purchase/returns/{id}/reject"),
        json!({ "reason": "首轮拒绝成立" }),
    )
    .await;
    assert_eq!(
        status,
        axum::http::StatusCode::OK,
        "前置：submitted→rejected 必须成功，实得 {v}"
    );

    // 再批准：状态门拦（理由给满，排除必填档干扰，只测门）
    let (status, v) = post_json(
        &app,
        &format!("/purchase/returns/{id}/approve"),
        json!({ "approval_reason": "终态后重试批准" }),
    )
    .await;
    assert_eq!(
        status,
        axum::http::StatusCode::BAD_REQUEST,
        "退货 rejected 行再批准必须 400（终态无回退边），实得 {v}"
    );
    assert_eq!(
        v["code"], "BUSINESS_ERROR",
        "退货批准状态门必须归 BUSINESS_ERROR，实得 {v}"
    );

    // 再拒绝：同样被门拦
    let (status, v) = post_json(
        &app,
        &format!("/purchase/returns/{id}/reject"),
        json!({ "reason": "终态后重试拒绝" }),
    )
    .await;
    assert_eq!(
        status,
        axum::http::StatusCode::BAD_REQUEST,
        "退货 rejected 行再拒绝必须 400（终态无回退边），实得 {v}"
    );
    assert_eq!(
        v["code"], "BUSINESS_ERROR",
        "退货拒绝状态门必须归 BUSINESS_ERROR，实得 {v}"
    );

    // 零副作用：状态与首轮拒绝理由逐字保持
    let row = purchase_return::Entity::find_by_id(id)
        .one(db.as_ref())
        .await
        .unwrap()
        .expect("退货行必须存在");
    assert_eq!(
        row.return_status.as_deref(),
        Some(pr_status::REJECTED),
        "被拒的终态流转不得改动作后状态"
    );
    assert_eq!(
        row.rejected_reason.as_deref(),
        Some("首轮拒绝成立"),
        "被拒的终态流转不得覆写 rejected_reason"
    );
    assert_eq!(
        row.approval_reason, None,
        "被拒的再批准不得落 approval_reason"
    );
}
