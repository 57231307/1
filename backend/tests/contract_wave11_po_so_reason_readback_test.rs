//! 采购/销售订单三理由出参可读回契约锁（后端线）
//!
//! 已定案口径（本文件逐条钉死）：
//! - 审批 = 通过/拒绝两条动作，各自给理由；理由必须**写进库也能被业务方从出参
//!   逐字读回**。落点列（DDL 与实体取证见各用例注释）：
//!   - `purchase_orders`：`rejected_reason`（system/mod.rs:344，VARCHAR(255) 可空）
//!     / `approval_reason`、`cancel_reason`（m0079 加列，TEXT 可空），
//!     实体列 models/purchase_order.rs:108/:111/:114；
//!   - `sales_orders`：`rejected_reason` / `approval_reason`（m0079 加列，TEXT 可空），
//!     实体列 models/sales_order.rs:49/:51；**该表无 cancel_reason 列**（m0079
//!     该域仅建两列），出参不存在该键——用例 3 为负向锁。
//! - 出参 DTO：`PurchaseOrderDto`（services/po/order.rs）与 `SalesOrderDetail`
//!   （services/so/mod.rs）补齐与列逐字同名 snake_case 键；无 `skip_serializing_if`，
//!   键恒存在、无值即 null（前端据此区分「未审批」与「字段缺失」）。
//!   详情/列表共用同一读取链（Entity::find() 全列 SELECT + into_model::<Dto>()，
//!   SO 详情另有唯一手工构造点 order_query.rs::build_order_detail），本文件用
//!   端点出参断言即覆盖真实链路，不允许也不需要逐行再查。
//! - 两动作两列：approve 只写 approval_reason、reject 只写 rejected_reason、
//!   PO cancel 只写 cancel_reason；对方列必须保持 NULL（防污染），notes 保持
//!   种子原文（防挪用）。
//! - 断言口径：只断 HTTP 码 + 机器 code + 列值，**不断错误文案原文**
//!   （本仓业务拒绝/校验文案永久脱敏）。
//!
//! 夹具形态照 `contract_wave11_order_return_approval_test.rs` 与
//! `contract_wave11_quotation_reason_readback_test.rs`：真库 `setup_test_db`
//! （已迁移 PostgreSQL、nextest 串行 db-integration 组），suppliers 属封存参照表
//! 自增回读，users/customers/业务表逐例 TRUNCATE 后可显式固定 id；路由仅在测试
//! Router 内注册，路径形态与真实路由（routes/purchase.rs、routes/sales.rs）
//! 逐字符一致。

mod test_common;

use bingxi_backend::container::AppState;
use bingxi_backend::handlers::{purchase_order_handler, sales_order_handler};
use bingxi_backend::models::status::{purchase_order as po_status, sales_order as so_status};
use bingxi_backend::models::{customer, purchase_order, sales_order, supplier, user, warehouse};
use bingxi_backend::services::event_notification_service::EventNotificationService;
use chrono::{DateTime, Utc};
use sea_orm::{ActiveModelTrait, DatabaseConnection, EntityTrait, Set};
use serde_json::{Value, json};
use std::sync::Arc;
use test_common::setup_test_db;

/// 注入到 AuthContext 并落 created_by/purchaser_id 的操作人主键
/// （users 逐例被夹具清空，可显式固定）
const OPERATOR_ID: i32 = 9841;

fn now() -> DateTime<Utc> {
    Utc::now()
}

async fn seed_operator(db: &Arc<DatabaseConnection>) {
    user::ActiveModel {
        id: Set(OPERATOR_ID),
        username: Set("w11_po_so_reason_readback".to_string()),
        password_hash: Set("test-only-not-a-real-hash".to_string()),
        real_name: Set(Some("PO/SO理由回读契约锁操作人".to_string())),
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

/// suppliers 属封存参照表不被清空，显式 id 会跨例撞主键，自增插入后回读真实主键
async fn seed_supplier(db: &Arc<DatabaseConnection>, tag: &str) -> i32 {
    let s = supplier::ActiveModel {
        supplier_code: Set(format!("SUP-W11RR-{tag}")),
        supplier_name: Set("理由回读契约锁供应商".to_string()),
        supplier_short_name: Set("回供".to_string()),
        supplier_type: Set("面料供应商".to_string()),
        credit_code: Set("91330000TEST00000X".to_string()),
        registered_address: Set("测试注册地址".to_string()),
        legal_representative: Set("测试法人".to_string()),
        registered_capital: Set(rust_decimal::Decimal::ZERO),
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

async fn seed_customer(db: &Arc<DatabaseConnection>, tag: &str) -> i32 {
    let c = customer::ActiveModel {
        customer_code: Set(format!("CUS-W11RR-{tag}")),
        customer_name: Set("理由回读契约锁客户".to_string()),
        credit_limit: Set(rust_decimal::Decimal::ZERO),
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

/// warehouse_id 在 purchase_orders 为 NOT NULL + FK（形状照 wave11 三单审批锁夹具）
async fn seed_warehouse(db: &Arc<DatabaseConnection>, tag: &str) -> i32 {
    let w = warehouse::ActiveModel {
        warehouse_code: Set(format!("WH-W11RR-{tag}")),
        name: Set("理由回读契约锁仓".to_string()),
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

/// 种一行采购订单：notes 预置「原始备注」作为 reject/cancel/approve 挪用防护的
/// 断言锚点；三理由列由 ..Default::default() 保持 NULL（未采集态）
async fn seed_po(
    db: &Arc<DatabaseConnection>,
    status: &str,
    supplier_id: i32,
    warehouse_id: i32,
    tag: &str,
) -> i32 {
    let po = purchase_order::ActiveModel {
        order_no: Set(format!("PO-W11RR-{tag}")),
        supplier_id: Set(supplier_id),
        order_date: Set(Utc::now().date_naive()),
        warehouse_id: Set(warehouse_id),
        department_id: Set(1),
        purchaser_id: Set(OPERATOR_ID),
        currency: Set("CNY".to_string()),
        exchange_rate: Set(rust_decimal::Decimal::ONE),
        total_amount: Set(rust_decimal::Decimal::ZERO),
        total_amount_foreign: Set(rust_decimal::Decimal::ZERO),
        total_quantity: Set(rust_decimal::Decimal::ZERO),
        total_quantity_alt: Set(rust_decimal::Decimal::ZERO),
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

/// 种一行销售订单（无明细：approve 后置 MRP 循环空转，reject 释放预留空转；
/// notes 同 PO 作挪用防护锚点——SO reject 历史挪用 notes，m0079 起停写）
async fn seed_so(db: &Arc<DatabaseConnection>, status: &str, customer_id: i32, tag: &str) -> i32 {
    let order = sales_order::ActiveModel {
        order_no: Set(format!("SO-W11RR-{tag}")),
        customer_id: Set(customer_id),
        order_date: Set(now()),
        status: Set(status.to_string()),
        subtotal: Set(rust_decimal::Decimal::ZERO),
        tax_amount: Set(rust_decimal::Decimal::ZERO),
        discount_amount: Set(rust_decimal::Decimal::ZERO),
        shipping_cost: Set(rust_decimal::Decimal::ZERO),
        total_amount: Set(rust_decimal::Decimal::ZERO),
        paid_amount: Set(rust_decimal::Decimal::ZERO),
        balance_amount: Set(rust_decimal::Decimal::ZERO),
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

/// PO 行权威回读（出参断言之外，落库列值以数据库行做交叉取证）
async fn read_po(db: &Arc<DatabaseConnection>, id: i32) -> purchase_order::Model {
    purchase_order::Entity::find_by_id(id)
        .one(db.as_ref())
        .await
        .unwrap_or_else(|e| panic!("采购订单 {id} 直读失败: {e}"))
        .unwrap_or_else(|| panic!("采购订单 {id} 必须存在"))
}

/// SO 行权威回读
async fn read_so(db: &Arc<DatabaseConnection>, id: i32) -> sales_order::Model {
    sales_order::Entity::find_by_id(id)
        .one(db.as_ref())
        .await
        .unwrap_or_else(|e| panic!("销售订单 {id} 直读失败: {e}"))
        .unwrap_or_else(|| panic!("销售订单 {id} 必须存在"))
}

/// HTTP 形态夹具：两域 approve/reject（+PO cancel、SO cancel）动作端点与
/// 详情/列表读取端点共九个；auth 注入与空串剔除中间件照 wave11 三单审批锁先例。
/// 路径形态与真实路由（routes/purchase.rs / routes/sales.rs）逐字符一致。
async fn reason_readback_app() -> (Arc<DatabaseConnection>, axum::Router) {
    let db = Arc::new(setup_test_db().await);
    seed_operator(&db).await;
    let mut state = AppState {
        db: db.clone(),
        ..Default::default()
    };
    // `AppState::default()` 的全部子服务绑在 `DatabaseConnection::default()`
    // （= sea-orm 的 `Disconnected` 变体，`src/container/mod.rs:334`）上，只覆盖 `db`
    // 字段不会把它们搬到真库。本锁用例 1 经真实 handler 走 PO reject
    // （`src/handlers/purchase_order_handler.rs:377` → `notify_approval_result` `:383`）
    // 与 SO approve（`src/handlers/sales_order_handler.rs:433` → `notify_order_approved`
    // `:437`）两条成功路径，落库后必走站内通知；通知服务对 `Disconnected` 连接取
    // backend 直接 panic（sea-orm-2.0.2 `src/database/db_connection.rs:727`）。
    // 详情/列表的 `data_permission_service` 触达点在 `if let Some(role_id)` 门内
    // （`purchase_order_handler.rs:57`、`sales_order_handler.rs:200`），本夹具 AuthContext
    // 固定 `role_id: None` 不经该路径，故只补被测路径真正触达的事件通知服务。
    // 生产装配无条件构造该服务并与 `state.db` 同池（`src/container/mod.rs:343`），
    // 禁止用 `None` 绕过真实链路（那会把已装配的通知通道变成不可达分支）。
    state.event_notification_service = Some(Arc::new(EventNotificationService::new(db.clone())));
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
        username: "w11_po_so_reason_readback".to_string(),
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
            "/purchase/orders/{id}",
            axum::routing::get(purchase_order_handler::get_order),
        )
        .route(
            "/purchase/orders",
            axum::routing::get(purchase_order_handler::list_orders),
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
            "/sales/orders/{id}/cancel",
            axum::routing::post(sales_order_handler::cancel_order),
        )
        .route(
            "/sales/orders/{id}",
            axum::routing::get(sales_order_handler::get_order),
        )
        .route(
            "/sales/orders",
            axum::routing::get(sales_order_handler::list_orders),
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

async fn get_json(app: &axum::Router, uri: &str) -> (axum::http::StatusCode, Value) {
    use tower::ServiceExt;
    let resp = app
        .clone()
        .oneshot(
            axum::http::Request::builder()
                .method(axum::http::Method::GET)
                .uri(uri)
                .body(axum::body::Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    let status = resp.status();
    let bytes = axum::body::to_bytes(resp.into_body(), 4 * 1024 * 1024)
        .await
        .unwrap()
        .to_vec();
    let v: Value = serde_json::from_slice(&bytes).expect("响应必须是 JSON 统一信封");
    (status, v)
}

/// 逐字回读断言（详情/列表共用口径）：键必须存在（无 skip_serializing_if ⇒
/// 未采集为 null 而非缺键），且值为落库原文
fn assert_reason_verbatim(row: &Value, key: &str, expected: &str, where_: &str) {
    assert!(
        row.get(key).is_some(),
        "{where_}出参必须含与列名逐字同名的 {key} 键，实得 {row}"
    );
    assert_eq!(
        row[key].as_str(),
        Some(expected),
        "{where_} {key} 必须逐字回读落库原文（禁截断/禁格式化），实得 {row}"
    );
}

fn assert_reason_null(row: &Value, key: &str, where_: &str) {
    assert!(
        row.get(key).is_some(),
        "{where_}出参必须含 {key} 键（键恒存在），实得 {row}"
    );
    assert_eq!(
        row[key],
        Value::Null,
        "{where_} 两动作两列防污染：{key} 必须保持 null，实得 {row}"
    );
}

// ---------------------------------------------------------------------------
// 1. PO 三理由列经详情+列表端点逐字可读回：approve/reject/cancel 三行动作各
//    写各列（approval_reason / rejected_reason / cancel_reason），对方两列
//    保持 null，notes 保持种子原文；端点回读与真库直读同源。
//    改坏什么必红：DTO 缺任一键 ⇒ 键存在性红；From/into_model 列映射漏列 ⇒
//    值 null 红；动作挪用他列 ⇒ 防污染 null 红；列表与详情分列集 ⇒ 列表值红。
// ---------------------------------------------------------------------------
#[tokio::test]
async fn po_reason_columns_readback_via_detail_and_list_verbatim() {
    let (db, app) = reason_readback_app().await;
    let supplier_id = seed_supplier(&db, "P1").await;
    let warehouse_id = seed_warehouse(&db, "P1").await;

    let approved_id = seed_po(
        &db,
        po_status::PENDING_APPROVAL,
        supplier_id,
        warehouse_id,
        "P1a",
    )
    .await;
    let raw_approval = "  价格与交期均在授权区间内，同意通过  ";
    let (status, v) = post_json(
        &app,
        &format!("/purchase/orders/{approved_id}/approve"),
        json!({ "approval_reason": raw_approval }),
    )
    .await;
    assert_eq!(
        status,
        axum::http::StatusCode::OK,
        "pending_approval→approved 权威路径必须 200，实得 {v}"
    );
    assert_eq!(v["code"], 200, "成功信封 code 必须 200，实得 {v}");

    let rejected_id = seed_po(
        &db,
        po_status::PENDING_APPROVAL,
        supplier_id,
        warehouse_id,
        "P1b",
    )
    .await;
    let reject_reason = "超出年度框架协议单价上限，不予通过";
    let (status, v) = post_json(
        &app,
        &format!("/purchase/orders/{rejected_id}/reject"),
        json!({ "reason": reject_reason }),
    )
    .await;
    assert_eq!(
        status,
        axum::http::StatusCode::OK,
        "pending_approval→rejected 权威路径必须 200，实得 {v}"
    );

    let cancelled_id = seed_po(&db, po_status::DRAFT, supplier_id, warehouse_id, "P1c").await;
    let cancel_reason = "重复下单，作废本单";
    let (status, v) = post_json(
        &app,
        &format!("/purchase/orders/{cancelled_id}/cancel"),
        json!({ "reason": cancel_reason }),
    )
    .await;
    assert_eq!(
        status,
        axum::http::StatusCode::OK,
        "draft→cancelled 取消路径必须 200，实得 {v}"
    );

    // 详情端点逐字回读 + 双列防污染 + notes 防挪用
    let (status, detail) = get_json(&app, &format!("/purchase/orders/{approved_id}")).await;
    assert_eq!(status, axum::http::StatusCode::OK, "详情必须 200：{detail}");
    let data = &detail["data"];
    assert_eq!(
        data["status"],
        po_status::APPROVED,
        "批准行详情状态必须 approved"
    );
    assert_reason_verbatim(
        data,
        "approval_reason",
        raw_approval.trim(),
        "PO 批准行详情",
    );
    assert_reason_null(data, "rejected_reason", "PO 批准行详情");
    assert_reason_null(data, "cancel_reason", "PO 批准行详情");
    assert_eq!(
        data["notes"], "原始备注",
        "approve 不得挪用 notes 列承载理由"
    );

    let (status, detail) = get_json(&app, &format!("/purchase/orders/{rejected_id}")).await;
    assert_eq!(status, axum::http::StatusCode::OK, "详情必须 200：{detail}");
    let data = &detail["data"];
    assert_eq!(
        data["status"],
        po_status::REJECTED,
        "拒绝行详情状态必须 rejected"
    );
    assert_reason_verbatim(data, "rejected_reason", reject_reason, "PO 拒绝行详情");
    assert_reason_null(data, "approval_reason", "PO 拒绝行详情");
    assert_reason_null(data, "cancel_reason", "PO 拒绝行详情");
    assert_eq!(
        data["notes"], "原始备注",
        "reject 不得挪用 notes 列承载理由"
    );

    let (status, detail) = get_json(&app, &format!("/purchase/orders/{cancelled_id}")).await;
    assert_eq!(status, axum::http::StatusCode::OK, "详情必须 200：{detail}");
    let data = &detail["data"];
    assert_eq!(
        data["status"],
        po_status::CANCELLED,
        "取消行详情状态必须 cancelled"
    );
    assert_reason_verbatim(data, "cancel_reason", cancel_reason, "PO 取消行详情");
    assert_reason_null(data, "approval_reason", "PO 取消行详情");
    assert_reason_null(data, "rejected_reason", "PO 取消行详情");
    assert_eq!(
        data["notes"], "原始备注",
        "cancel 不得挪用 notes 列承载理由"
    );

    // 列表端点同一链（Entity::find 全列 + into_model）逐字回读三列
    let (status, body) = get_json(
        &app,
        &format!("/purchase/orders?page=1&page_size=50&supplier_id={supplier_id}"),
    )
    .await;
    assert_eq!(status, axum::http::StatusCode::OK, "列表必须 200：{body}");
    let items = body["data"]["items"]
        .as_array()
        .expect("分页信封 data.items 必须是数组");
    let approved_row = items
        .iter()
        .find(|d| d["id"] == json!(approved_id))
        .expect("列表必须含刚批准的行");
    assert_reason_verbatim(
        approved_row,
        "approval_reason",
        raw_approval.trim(),
        "PO 批准行列表",
    );
    assert_reason_null(approved_row, "rejected_reason", "PO 批准行列表");
    let rejected_row = items
        .iter()
        .find(|d| d["id"] == json!(rejected_id))
        .expect("列表必须含刚拒绝的行");
    assert_reason_verbatim(
        rejected_row,
        "rejected_reason",
        reject_reason,
        "PO 拒绝行列表",
    );
    let cancelled_row = items
        .iter()
        .find(|d| d["id"] == json!(cancelled_id))
        .expect("列表必须含刚取消的行");
    assert_reason_verbatim(
        cancelled_row,
        "cancel_reason",
        cancel_reason,
        "PO 取消行列表",
    );

    // 交叉取证：端点回读值与真库列值同源
    let row = read_po(&db, approved_id).await;
    assert_eq!(
        row.approval_reason.as_deref(),
        Some(raw_approval.trim()),
        "落库列值必须与端点回读一致"
    );
    let row = read_po(&db, rejected_id).await;
    assert_eq!(
        row.rejected_reason.as_deref(),
        Some(reject_reason),
        "落库列值必须与端点回读一致"
    );
    let row = read_po(&db, cancelled_id).await;
    assert_eq!(
        row.cancel_reason.as_deref(),
        Some(cancel_reason),
        "落库列值必须与端点回读一致"
    );
}

// ---------------------------------------------------------------------------
// 2. SO 两理由列经详情+列表端点逐字可读回：approve 写 approval_reason、
//    reject 写 rejected_reason，对方列保持 null，notes 保持种子原文（reject
//    历史挪用 notes，m0079 起停写——本锁同时钉住不回潮）。
//    改坏什么必红：SalesOrderDetail 缺键 ⇒ 键存在性红；详情手工构造点
//    build_order_detail 或列表 into_model 任一漏列 ⇒ 对应值红；reject 挪用
//    notes ⇒ notes 断言红。
// ---------------------------------------------------------------------------
#[tokio::test]
async fn so_reason_columns_readback_via_detail_and_list_verbatim() {
    let (db, app) = reason_readback_app().await;
    let customer_id = seed_customer(&db, "S2").await;

    let approved_id = seed_so(&db, so_status::PENDING, customer_id, "S2a").await;
    let raw_approval = "  客户信用额度与交期均已复核，同意通过  ";
    let (status, v) = post_json(
        &app,
        &format!("/sales/orders/{approved_id}/approve"),
        json!({ "approval_reason": raw_approval }),
    )
    .await;
    assert_eq!(
        status,
        axum::http::StatusCode::OK,
        "pending→approved 权威路径必须 200，实得 {v}"
    );
    assert_eq!(v["code"], 200, "成功信封 code 必须 200，实得 {v}");

    let rejected_id = seed_so(&db, so_status::PENDING, customer_id, "S2b").await;
    let reject_reason = "低于成本价且无授信支撑，不予通过";
    let (status, v) = post_json(
        &app,
        &format!("/sales/orders/{rejected_id}/reject"),
        json!({ "reason": reject_reason }),
    )
    .await;
    assert_eq!(
        status,
        axum::http::StatusCode::OK,
        "pending→rejected 权威路径必须 200，实得 {v}"
    );

    let (status, detail) = get_json(&app, &format!("/sales/orders/{approved_id}")).await;
    assert_eq!(status, axum::http::StatusCode::OK, "详情必须 200：{detail}");
    let data = &detail["data"];
    assert_eq!(
        data["status"],
        so_status::APPROVED,
        "批准行详情状态必须 approved"
    );
    assert_reason_verbatim(
        data,
        "approval_reason",
        raw_approval.trim(),
        "SO 批准行详情",
    );
    assert_reason_null(data, "rejected_reason", "SO 批准行详情");
    assert_eq!(
        data["notes"], "原始备注",
        "approve 不得挪用 notes 列承载理由"
    );

    let (status, detail) = get_json(&app, &format!("/sales/orders/{rejected_id}")).await;
    assert_eq!(status, axum::http::StatusCode::OK, "详情必须 200：{detail}");
    let data = &detail["data"];
    assert_eq!(
        data["status"],
        so_status::REJECTED,
        "拒绝行详情状态必须 rejected"
    );
    assert_reason_verbatim(data, "rejected_reason", reject_reason, "SO 拒绝行详情");
    assert_reason_null(data, "approval_reason", "SO 拒绝行详情");
    assert_eq!(
        data["notes"], "原始备注",
        "reject 不得再挪用 notes（m0079 起理由只落 rejected_reason 专列）"
    );

    let (status, body) = get_json(
        &app,
        &format!("/sales/orders?page=1&page_size=50&customer_id={customer_id}"),
    )
    .await;
    assert_eq!(status, axum::http::StatusCode::OK, "列表必须 200：{body}");
    let items = body["data"]["items"]
        .as_array()
        .expect("分页信封 data.items 必须是数组");
    let approved_row = items
        .iter()
        .find(|d| d["id"] == json!(approved_id))
        .expect("列表必须含刚批准的行");
    assert_reason_verbatim(
        approved_row,
        "approval_reason",
        raw_approval.trim(),
        "SO 批准行列表",
    );
    assert_reason_null(approved_row, "rejected_reason", "SO 批准行列表");
    let rejected_row = items
        .iter()
        .find(|d| d["id"] == json!(rejected_id))
        .expect("列表必须含刚拒绝的行");
    assert_reason_verbatim(
        rejected_row,
        "rejected_reason",
        reject_reason,
        "SO 拒绝行列表",
    );
    assert_reason_null(rejected_row, "approval_reason", "SO 拒绝行列表");

    let row = read_so(&db, approved_id).await;
    assert_eq!(
        row.approval_reason.as_deref(),
        Some(raw_approval.trim()),
        "落库列值必须与端点回读一致"
    );
    let row = read_so(&db, rejected_id).await;
    assert_eq!(
        row.rejected_reason.as_deref(),
        Some(reject_reason),
        "落库列值必须与端点回读一致"
    );
}

// ---------------------------------------------------------------------------
// 3. SO 出参无 cancel_reason 键（负向锁）：sales_orders 无 cancel_reason 列
//    （m0079 该域仅建 approval_reason/rejected_reason 两列），故 SO 详情/列表
//    出参 JSON 键集里**禁止**出现 cancel_reason——防止将来有人塞假键或未经
//    迁移裁定硬造列。同时正锁两理由键恒存在。
//    改坏什么必红：DTO 被塞 cancel_reason: None 假键 ⇒ 键不存在断言红。
// ---------------------------------------------------------------------------
#[tokio::test]
async fn so_output_has_no_cancel_reason_key_negative_lock() {
    let (db, app) = reason_readback_app().await;
    let customer_id = seed_customer(&db, "S3").await;

    // 走真实 cancel 动作后再读回：取消动作本身在该域无任何理由列可落
    let cancelled_id = seed_so(&db, so_status::PENDING, customer_id, "S3a").await;
    let (status, v) = post_empty_body(&app, &format!("/sales/orders/{cancelled_id}/cancel")).await;
    assert_eq!(
        status,
        axum::http::StatusCode::OK,
        "pending→cancelled 取消路径必须 200，实得 {v}"
    );

    let (status, detail) = get_json(&app, &format!("/sales/orders/{cancelled_id}")).await;
    assert_eq!(status, axum::http::StatusCode::OK, "详情必须 200：{detail}");
    let data = &detail["data"];
    assert_eq!(
        data["status"],
        so_status::CANCELLED,
        "取消行详情状态必须 cancelled"
    );
    assert!(
        data.get("cancel_reason").is_none(),
        "SO 出参禁止出现 cancel_reason 键——sales_orders 无该列（m0079 仅建两列），\
         该键即假键；实得键集 {:?}",
        data.as_object().map(|o| o.keys().collect::<Vec<_>>())
    );
    assert!(
        data.get("approval_reason").is_some() && data.get("rejected_reason").is_some(),
        "两理由键必须恒存在（未采集为 null），实得 {data}"
    );
    assert_eq!(
        data["approval_reason"],
        Value::Null,
        "cancel 动作不得写 approval_reason（该域 cancel 无专列且不挪用两理由列）"
    );
    assert_eq!(
        data["rejected_reason"],
        Value::Null,
        "cancel 动作不得挪用 rejected_reason"
    );

    let (status, body) = get_json(
        &app,
        &format!("/sales/orders?page=1&page_size=50&customer_id={customer_id}"),
    )
    .await;
    assert_eq!(status, axum::http::StatusCode::OK, "列表必须 200：{body}");
    let items = body["data"]["items"]
        .as_array()
        .expect("分页信封 data.items 必须是数组");
    let cancelled_row = items
        .iter()
        .find(|d| d["id"] == json!(cancelled_id))
        .expect("列表必须含刚取消的行");
    assert!(
        cancelled_row.get("cancel_reason").is_none(),
        "SO 列表项同样禁止 cancel_reason 键，实得 {cancelled_row}"
    );
}

/// 无 body 的 POST（SO cancel 端点无请求体提取器；形态照 wave11 三单审批锁先例）
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
// 4. 拒绝空/纯空白理由 fail-closed：两域 reject 均 400 + 机器 code
//    VALIDATION_ERROR（只断状态码与 code，**不断文案原文**——权限/校验文案
//    永久脱敏），且行零污染：状态不变、rejected_reason 保持 NULL、approval_reason
//    不受牵连；失败后详情出参两键仍恒存在且为 null（缺键兜底假绿不可达）。
//    改坏什么必红：必填门被拆 ⇒ 200 红；错误改吞 ⇒ code 断言红；
//    校验失败前已先写列 ⇒ 直读 NULL 红；DTO 键被 skip_serializing_if 吞 ⇒
//    assert_reason_null 的键存在性红。
// ---------------------------------------------------------------------------
#[tokio::test]
async fn reject_blank_reason_fails_closed_and_row_untouched() {
    let (db, app) = reason_readback_app().await;
    let supplier_id = seed_supplier(&db, "P4").await;
    let warehouse_id = seed_warehouse(&db, "P4").await;
    let po_id = seed_po(
        &db,
        po_status::PENDING_APPROVAL,
        supplier_id,
        warehouse_id,
        "P4",
    )
    .await;

    let (status, v) = post_json(
        &app,
        &format!("/purchase/orders/{po_id}/reject"),
        json!({ "reason": "   " }),
    )
    .await;
    assert_eq!(
        status,
        axum::http::StatusCode::BAD_REQUEST,
        "PO 纯空白拒绝理由必须 400 fail-closed，实得 {v}"
    );
    assert_eq!(
        v["code"], "VALIDATION_ERROR",
        "失败信封机器 code 必须是 VALIDATION_ERROR（不断文案原文），实得 {v}"
    );
    let row = read_po(&db, po_id).await;
    assert_eq!(
        row.order_status,
        po_status::PENDING_APPROVAL,
        "校验失败行状态必须原样保留"
    );
    assert_eq!(
        row.rejected_reason, None,
        "校验失败不得先写 rejected_reason"
    );
    assert_eq!(
        row.approval_reason, None,
        "校验失败不得牵连 approval_reason"
    );

    let customer_id = seed_customer(&db, "S4").await;
    let so_id = seed_so(&db, so_status::PENDING, customer_id, "S4").await;
    let (status, v) = post_json(
        &app,
        &format!("/sales/orders/{so_id}/reject"),
        json!({ "reason": "   " }),
    )
    .await;
    assert_eq!(
        status,
        axum::http::StatusCode::BAD_REQUEST,
        "SO 纯空白拒绝理由必须 400 fail-closed，实得 {v}"
    );
    assert_eq!(
        v["code"], "VALIDATION_ERROR",
        "失败信封机器 code 必须是 VALIDATION_ERROR（不断文案原文），实得 {v}"
    );
    let row = read_so(&db, so_id).await;
    assert_eq!(row.status, so_status::PENDING, "校验失败行状态必须原样保留");
    assert_eq!(
        row.rejected_reason, None,
        "校验失败不得先写 rejected_reason"
    );
    assert_eq!(
        row.approval_reason, None,
        "校验失败不得牵连 approval_reason"
    );

    // 失败路径之后详情出参：两键恒存在且为 null（未采集态可读、键不被吞）
    let (status, detail) = get_json(&app, &format!("/purchase/orders/{po_id}")).await;
    assert_eq!(status, axum::http::StatusCode::OK, "详情必须 200：{detail}");
    let data = &detail["data"];
    assert_reason_null(data, "rejected_reason", "PO 未决行详情");
    assert_reason_null(data, "approval_reason", "PO 未决行详情");
    assert_reason_null(data, "cancel_reason", "PO 未决行详情");

    let (status, detail) = get_json(&app, &format!("/sales/orders/{so_id}")).await;
    assert_eq!(status, axum::http::StatusCode::OK, "详情必须 200：{detail}");
    let data = &detail["data"];
    assert_reason_null(data, "rejected_reason", "SO 未决行详情");
    assert_reason_null(data, "approval_reason", "SO 未决行详情");
}
