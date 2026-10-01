//! 契约波次 6 · 任务 #151 三缺陷防回潮锁（销售发货 500 漏网站点 / BI DELETE 键推导 / AP 付款契约）
//!
//! 覆盖：
//! 1. **销售发货提交/审批链 internal 重包漏网站点**（取证：handler 对已返回
//!    `Result<_, AppError>` 的 `get_stock_by_product`（services/inventory_stock_query.rs:308）
//!    再 `.map_err(|e| AppError::internal(format!("查询产品库存失败: {}", e)))`，把分类错误
//!    伪装成 500 且把错误原文拼进出参文案；该端点 `GET /inventory/stock/product/{id}`
//!    正是销售发货建单弹窗取可发库存的前置调用）。修复 = `?` 透传（同 AR 收款
//!    ar_payment_handler 先例）。防线：调用名 × map_err(internal) 零命中 + shrink-only ratchet
//!    + sqlite 真跑 HTTP 映射（200 形状 / 缺表归 DATABASE_ERROR 而非 INTERNAL_ERROR、不外泄原文）。
//! 2. **BI DELETE 403 判责取证锁**：权限键由 URL 段推导（middleware/permission.rs:259
//!    extract_resource_info + method_to_action），`/api/v1/erp/bi/**` 的 seg4（sales/dashboards/...）
//!    直接成为资源键名，DELETE 动作推导出 `sales:delete` / `dashboards:delete` 等键——
//!    逐一钉死推导结果作为「权限键未登记（根因 a）」的静态证据；同时锁死 403 拒绝文案
//!    恒为通用脱敏常量，禁止任何人把真实权限键回显到出参。
//! 3. **AP 付款创建契约**：后端 `CreateApPaymentRequest`（services/ap_payment_service.rs:752）
//!    是契约真值（request_id/payment_date 对应 NOT NULL 列，非 Option）；前端 payload 同构
//!    由 serde 真跑锁死；Decimal 出参（payment_amount/request_amount/exchange_rate/currency）
//!    在前端 ap.ts 必须是 `string`（rust_decimal 序列化为十进制字符串，先例 commit
//!    83b8028b/42f67500），NOT NULL 列不得标 `?` —— 源码扫描锁防回潮。

use axum::{
    Router,
    body::Body,
    extract::State,
    http::{Method, Request, StatusCode},
    middleware::{Next, from_fn_with_state},
    response::IntoResponse,
    routing::get,
};
use bingxi_backend::container::AppState;
use bingxi_backend::handlers::inventory_stock_handler_query;
use bingxi_backend::middleware::auth_context::AuthContext;
use bingxi_backend::middleware::permission::{
    extract_action_from_query, extract_resource_info, method_to_action,
};
use bingxi_backend::models::inventory_stock;
use bingxi_backend::services::ap_payment_service::CreateApPaymentRequest;
use bingxi_backend::utils::error::AppError;
use rust_decimal::Decimal;
use sea_orm::{ActiveModelTrait, ActiveValue::Set, ConnectionTrait, DbBackend, Statement};
use serde_json::{Value, json};
use std::str::FromStr;
use std::sync::Arc;
use tower::ServiceExt;

// ---------------------------------------------------------------------------
// 公共脚手架（先例：contract_wave1_ar_payment_error_mapping_test.rs）
// ---------------------------------------------------------------------------

fn make_auth(user_id: i32) -> AuthContext {
    AuthContext {
        user_id,
        username: format!("e2e_user_{user_id}"),
        role_id: Some(2),
        department_id: Some(1),
        data_scope: Some("self".to_string()),
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

use axum::response::Response;

fn build_stock_app(state: AppState, auth: AuthContext) -> Router {
    Router::new()
        .route(
            "/inventory/stock/product/{productId}",
            get(inventory_stock_handler_query::get_stock_by_product),
        )
        .with_state(state)
        .layer(from_fn_with_state(auth, inject_auth))
}

async fn request_json(app: &Router, req: Request<Body>) -> (StatusCode, Value) {
    let resp = app.clone().oneshot(req).await.unwrap();
    let status = resp.status();
    let bytes = axum::body::to_bytes(resp.into_body(), 4 * 1024 * 1024)
        .await
        .unwrap();
    (status, serde_json::from_slice(&bytes).unwrap())
}

async fn exec(db: &sea_orm::DatabaseConnection, sql: &str) {
    db.execute_raw(Statement::from_sql_and_values(
        DbBackend::Sqlite,
        sql,
        Vec::new(),
    ))
    .await
    .unwrap_or_else(|e| panic!("DDL 执行失败: {e}\nSQL: {sql}"));
}

/// 与 models/inventory_stock.rs::Model（表 inventory_stocks）逐列对应
const CREATE_INVENTORY_STOCKS: &str = r#"CREATE TABLE inventory_stocks (
    id INTEGER PRIMARY KEY,
    warehouse_id INTEGER NOT NULL, product_id INTEGER NOT NULL,
    quantity_on_hand TEXT NOT NULL, quantity_available TEXT NOT NULL,
    quantity_reserved TEXT NOT NULL, quantity_shipped TEXT NOT NULL,
    quantity_incoming TEXT NOT NULL, reorder_point TEXT NOT NULL,
    max_stock_point TEXT, reorder_quantity TEXT,
    bin_location TEXT, last_count_date TEXT, last_movement_date TEXT,
    created_at TEXT NOT NULL, updated_at TEXT NOT NULL,
    batch_no TEXT NOT NULL, color_no TEXT NOT NULL, dye_lot_no TEXT,
    grade TEXT NOT NULL, production_date TEXT, expiry_date TEXT,
    quantity_meters TEXT NOT NULL, quantity_kg TEXT NOT NULL,
    gram_weight TEXT, width TEXT, location_id INTEGER,
    shelf_no TEXT, layer_no TEXT,
    stock_status TEXT NOT NULL, quality_status TEXT NOT NULL,
    version INTEGER NOT NULL, replenishment_strategy TEXT NOT NULL
)"#;

async fn sqlite_state(with_table: bool) -> AppState {
    let db = sea_orm::Database::connect("sqlite::memory:")
        .await
        .expect("sqlite::memory: 连接失败");
    if with_table {
        exec(&db, CREATE_INVENTORY_STOCKS).await;
        inventory_stock::ActiveModel {
            warehouse_id: Set(1),
            product_id: Set(7),
            quantity_on_hand: Set(Decimal::from_str("100.50").unwrap()),
            quantity_available: Set(Decimal::from_str("80.25").unwrap()),
            quantity_reserved: Set(Decimal::ZERO),
            quantity_shipped: Set(Decimal::ZERO),
            quantity_incoming: Set(Decimal::ZERO),
            reorder_point: Set(Decimal::ZERO),
            reorder_quantity: Set(Decimal::ZERO),
            batch_no: Set("B1".to_string()),
            color_no: Set("RED".to_string()),
            dye_lot_no: Set(Some("DL-A".to_string())),
            grade: Set("一等品".to_string()),
            quantity_meters: Set(Decimal::from_str("80.25").unwrap()),
            quantity_kg: Set(Decimal::ZERO),
            stock_status: Set("normal".to_string()),
            quality_status: Set("待检".to_string()),
            version: Set(0),
            replenishment_strategy: Set("reorder_point".to_string()),
            created_at: Set(chrono::Utc::now()),
            updated_at: Set(chrono::Utc::now()),
            ..Default::default()
        }
        .insert(&db)
        .await
        .expect("种子库存行插入失败");
    }
    AppState {
        db: Arc::new(db),
        ..Default::default()
    }
}

// ===========================================================================
// 1) 销售发货链：internal 重包防回潮
// ===========================================================================

/// 销售发货提交/审批窗口文件（含发货建单弹窗前置取库存端点）。
const SHIPMENT_WINDOW: &[(&str, &str)] = &[
    (
        "src/handlers/inventory_stock_handler_query.rs",
        include_str!("../src/handlers/inventory_stock_handler_query.rs"),
    ),
    (
        "src/handlers/sales_order_handler.rs",
        include_str!("../src/handlers/sales_order_handler.rs"),
    ),
    (
        "src/services/so/delivery.rs",
        include_str!("../src/services/so/delivery.rs"),
    ),
    (
        "src/services/so/delivery_ops/ship.rs",
        include_str!("../src/services/so/delivery_ops/ship.rs"),
    ),
    (
        "src/services/so/delivery_ops/cancel.rs",
        include_str!("../src/services/so/delivery_ops/cancel.rs"),
    ),
    (
        "src/services/so/delivery_ops/inventory.rs",
        include_str!("../src/services/so/delivery_ops/inventory.rs"),
    ),
    (
        "src/services/so/order_workflow.rs",
        include_str!("../src/services/so/order_workflow.rs"),
    ),
];

fn strip_comments_and_compress(src: &str) -> String {
    src.lines()
        .filter(|l| !l.trim_start().starts_with("//"))
        .collect::<Vec<_>>()
        .join(" ")
        .chars()
        .filter(|c| !c.is_whitespace())
        .collect()
}

/// 在压缩文本中检查 call 的每个出现点：从调用名末尾到首个 `?` 之间若出现
/// `.map_err(` 且其后 200 字节内含 `AppError::internal(`，判回潮（先例 g1）。
fn call_is_rewrapped_in_internal(norm: &str, call: &str) -> bool {
    let mut from = 0usize;
    while let Some(rel) = norm[from..].find(call) {
        let at = from + rel + call.len();
        let search_end = match norm[at..].find('?') {
            Some(q) => at + q,
            None => {
                let mut e = (at + 200).min(norm.len());
                while !norm.is_char_boundary(e) {
                    e -= 1;
                }
                e
            }
        };
        if let Some(mp) = norm[at..search_end].find(".map_err(") {
            let abs_m = at + mp;
            let mut seg_end = (abs_m + 200).min(norm.len());
            while !norm.is_char_boundary(seg_end) {
                seg_end -= 1;
            }
            if norm[abs_m..seg_end].contains("AppError::internal(") {
                return true;
            }
        }
        from = at;
    }
    false
}

/// 逐一在服务层源码核实返回 `Result<_, AppError>` 的发货链调用：
/// get_stock_by_product（inventory_stock_query.rs:308）、ship_order（delivery_ops/ship.rs:31）、
/// create_delivery（so/delivery.rs:130）、get_order_deliveries（so/delivery.rs:118）、
/// cancel_delivery（delivery_ops/cancel.rs:33）、submit_order/approve_order（order_workflow.rs:89/267）、
/// get_order_detail（order_query.rs）。任何一处再被 map_err(internal) 重包即根因回潮。
const APP_ERROR_RETURNING_CALLS: &[&str] = &[
    "get_stock_by_product",
    "ship_order",
    "create_delivery",
    "get_order_deliveries",
    "cancel_delivery",
    "submit_order",
    "approve_order",
    "get_order_detail",
    "reduce_inventory_four_dim",
    "check_inventory",
];

#[test]
fn shipment_window_never_rewraps_app_error_calls_as_internal() {
    for (path, src) in SHIPMENT_WINDOW {
        let norm = strip_comments_and_compress(src);
        for call in APP_ERROR_RETURNING_CALLS {
            assert!(
                !call_is_rewrapped_in_internal(&norm, call),
                "{path}: 调用 `{call}(..)` 后紧跟 `.map_err(|..| AppError::internal(..))` 回潮——\
                 该方法返回 Result<_, AppError>，必须 `?` 透传保留真实 status/code\
                 （销售发货 500 漏网站点根因，先例 AR 收款 ar_payment_handler）。"
            );
        }
    }
}

/// shrink-only ratchet：基线 = 修复后逐文件 grep 坐实的真实命中数。
/// 非零条目保留理由：
/// - sales_order_handler.rs=10：全部为 `serde_json::to_value` 序列化失败的
///   真系统错误（非对 AppError 调用的重包）。
/// - delivery_ops/inventory.rs=1：`扣减规划返回了候选之外的库存行`（不可能态
///   内部不变量违例），真系统错误。
const INTERNAL_RATCHET: &[(&str, usize)] = &[
    ("src/handlers/inventory_stock_handler_query.rs", 0),
    ("src/handlers/sales_order_handler.rs", 10),
    ("src/services/so/delivery.rs", 0),
    ("src/services/so/delivery_ops/ship.rs", 0),
    ("src/services/so/delivery_ops/cancel.rs", 0),
    ("src/services/so/delivery_ops/inventory.rs", 1),
    ("src/services/so/order_workflow.rs", 0),
];

#[test]
fn shipment_window_internal_count_is_shrink_only() {
    for (path, cap) in INTERNAL_RATCHET {
        let src = SHIPMENT_WINDOW
            .iter()
            .find(|(p, _)| p == path)
            .unwrap_or_else(|| panic!("SHIPMENT_WINDOW 缺少文件: {path}"))
            .1;
        let hits = src.matches("AppError::internal(").count();
        assert!(
            hits <= *cap,
            "{path}: `AppError::internal(` 命中 {hits} 处，超过基线 {cap} 处——禁止把 4xx 拍平成 500 回潮。"
        );
    }
}

/// 本波修复站点钉死：get_stock_by_product 的错误必须原样透传（无 map_err 重包）。
#[test]
fn get_stock_by_product_handler_passes_errors_through() {
    let src = include_str!("../src/handlers/inventory_stock_handler_query.rs");
    assert!(
        !src.contains("map_err(|e| AppError::internal(format!(\"查询产品库存失败"),
        "inventory_stock_handler_query.rs 重新出现 internal 重包（本波漏网站点修复被回滚）"
    );
}

// ---------------------------------------------------------------------------
// sqlite 真跑：透传后的真实 HTTP 映射
// ---------------------------------------------------------------------------

/// 正常路径：种子库存行 → 200 且形状 {list,total,page,page_size}（Decimal 出参字符串）
#[tokio::test]
async fn stock_by_product_success_returns_200_shape() {
    let app = build_stock_app(sqlite_state(true).await, make_auth(100));
    let (status, v) = request_json(
        &app,
        Request::builder()
            .uri("/inventory/stock/product/7?page=1&page_size=10")
            .body(Body::empty())
            .unwrap(),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "成功路径不得回归 5xx: {v}");
    assert_eq!(v["data"]["total"], 1);
    assert_eq!(v["data"]["list"].as_array().unwrap().len(), 1);
    assert!(
        v["data"]["list"][0]["quantity_available"].is_string(),
        "rust_decimal 出参必须是十进制字符串"
    );
}

/// 失败路径：表不存在 → DbErr 经 From<DbErr> 归 DATABASE_ERROR 族，
/// 不再是修复前的 INTERNAL_ERROR 伪装；且 SQL 原文（no such table）不得外泄。
#[tokio::test]
async fn stock_by_product_db_error_maps_to_database_error_not_internal() {
    let app = build_stock_app(sqlite_state(false).await, make_auth(100));
    let (status, v) = request_json(
        &app,
        Request::builder()
            .uri("/inventory/stock/product/7?page=1&page_size=10")
            .body(Body::empty())
            .unwrap(),
    )
    .await;
    assert_eq!(v["code"], "DATABASE_ERROR");
    assert_ne!(
        v["code"], "INTERNAL_ERROR",
        "修复前 map_err(internal) 把 DbErr 伪装成 INTERNAL_ERROR/500 —— 本波根因，禁止回潮"
    );
    let body = v.to_string();
    assert!(
        !body.contains("no such table"),
        "SQL 错误原文外泄到响应：{body}"
    );
    assert_ne!(v["message"], "服务器内部错误");
    let _ = status; // DATABASE_ERROR 族的具体 status 由 utils/error.rs 映射表锁定（g1 已有信封矩阵）
}

/// 信封矩阵补强（发货链按族透传目标）：状态门→BUSINESS_ERROR、取值→VALIDATION_ERROR、
/// 不存在→NOT_FOUND、越权→FORBIDDEN，且脱敏族文案为固定常量。
#[tokio::test]
async fn shipment_error_family_envelopes_exact() {
    let cases: Vec<(AppError, StatusCode, &str, &str)> = vec![
        (
            AppError::business("只有已审批的订单才能发货"),
            StatusCode::BAD_REQUEST,
            "BUSINESS_ERROR",
            "业务处理失败",
        ),
        (
            AppError::validation("发货必须指定仓库 ID"),
            StatusCode::BAD_REQUEST,
            "VALIDATION_ERROR",
            "请求参数验证失败",
        ),
        (
            AppError::not_found("销售订单 999999 不存在"),
            StatusCode::NOT_FOUND,
            "NOT_FOUND",
            "资源未找到",
        ),
        (
            AppError::permission_denied("越权访问订单数据"),
            StatusCode::FORBIDDEN,
            "FORBIDDEN",
            "无权限",
        ),
    ];
    for (err, want_status, want_code, want_msg) in cases {
        let expect_status = want_status;
        let resp = err.into_response();
        let status = resp.status();
        let bytes = axum::body::to_bytes(resp.into_body(), 1024 * 1024)
            .await
            .unwrap();
        let v: Value = serde_json::from_slice(&bytes).unwrap();
        assert_eq!(status, expect_status);
        assert_eq!(v["code"], want_code);
        assert_eq!(v["message"], want_msg, "脱敏族文案必须是固定常量");
        assert_ne!(status, StatusCode::INTERNAL_SERVER_ERROR);
    }
}

// ===========================================================================
// 2) BI DELETE 403：键推导取证锁（根因 a = 推导键未登记）
// ===========================================================================

/// BI 域 URL 段的权限键推导逐条钉死（middleware/permission.rs:259 extract_resource_info +
/// method_to_action）。/bi 是模块前缀（utils/path_utils.rs:82），seg4 直接成为资源键名：
/// DELETE /bi/sales/... → `sales:delete`；DELETE /bi/dashboards/1 → `dashboards:delete`。
/// 这些键在权限注册/角色种子中不存在 → fail-closed 403（取证：根因 a，登记缺失）。
#[test]
fn bi_delete_permission_key_derivation_is_locked() {
    let delete = method_to_action(&Method::DELETE);
    let read = method_to_action(&Method::GET);
    // (路径, 期望资源键, 期望动作)
    let cases: Vec<(&str, &str, &str)> = vec![
        ("/api/v1/erp/bi/sales/by-time", "sales", delete.as_str()),
        ("/api/v1/erp/bi/dashboards/1", "dashboards", "delete"),
        ("/api/v1/erp/bi/charts/2", "charts", "delete"),
        ("/api/v1/erp/bi/favorites/3", "favorites", "delete"),
        // GET 读键同源（对照：读能过是因为 sales:read 已随销售域登记）
        ("/api/v1/erp/bi/sales/kpi", "sales", read.as_str()),
    ];
    for (path, want_resource, want_action) in cases {
        let (resource, _id) = extract_resource_info(path);
        assert_eq!(resource, want_resource, "BI 键推导漂移: {path}");
        assert!(
            ["read", "create", "update", "delete"].contains(&want_action),
            "动作词表仅 read/create/update/delete"
        );
    }
    assert_eq!(delete, "delete");
    assert_eq!(read, "read");
}

/// `?action=` 白名单不外溢：delete 不在 print/export/download 白名单内，
/// 查询参数无法把动作改成已登记键（防止用 query 绕过推导）。
#[test]
fn bi_action_query_stays_within_whitelist() {
    let uri = "/api/v1/erp/bi/dashboards/1?action=delete"
        .parse::<axum::http::Uri>()
        .unwrap();
    assert_eq!(extract_action_from_query(&uri), None);
    let uri = "/api/v1/erp/bi/dashboards/1?action=export"
        .parse::<axum::http::Uri>()
        .unwrap();
    assert_eq!(extract_action_from_query(&uri), Some("export".to_string()));
}

/// 403 拒绝文案脱敏常量锁：permission.rs 出参只允许通用文案，禁止把
/// resource:action 真实权限键回显（键名本身即权限拓扑情报）。
#[test]
fn permission_denial_message_stays_sanitized() {
    let src = include_str!("../src/middleware/permission.rs");
    let norm = strip_comments_and_compress(src);
    assert!(
        norm.contains("forbidden_response(\"权限不足，无法访问该资源\")"),
        "permission.rs 的 403 出参文案必须保持通用脱敏常量"
    );
    // required_permission（resource:action）只允许进审计事件字段，禁止进 forbidden_response 出参
    assert!(
        !norm.contains("forbidden_response(required_permission"),
        "禁止把真实权限键回显到 403 响应"
    );
    assert!(
        !norm.contains("forbidden_response(format!"),
        "禁止把拼装后的权限细节回显到 403 响应"
    );
}

// ===========================================================================
// 3) AP 付款创建契约：后端 DTO 为真值
// ===========================================================================

/// 后端 `CreateApPaymentRequest`（services/ap_payment_service.rs:752，对应 POST /ap/payments）
/// 接受前端 PaymentTab 的真实 payload；NOT NULL 列对应的键（request_id/payment_date）
/// 缺键必须在反序列化层拒绝（serde missing field），不允许 Option 掩盖。
#[test]
fn ap_payment_create_dto_matches_frontend_payload() {
    // 前端真实形态（views/ap/tabs/PaymentTab.vue submitPayment）
    let ok: CreateApPaymentRequest = serde_json::from_value(json!({
        "request_id": 5,
        "payment_date": "2026-09-20",
        "notes": null,
    }))
    .expect("前端 payload 必须能按后端 DTO 反序列化");
    assert_eq!(ok.request_id, 5);
    assert_eq!(ok.payment_date.to_string(), "2026-09-20");

    // 缺 request_id（NOT NULL 来源键）→ 反序列化层拒绝，走 4xx 参数错，不带病触库
    let missing = serde_json::from_value::<CreateApPaymentRequest>(json!({
        "payment_date": "2026-09-20",
    }));
    let err = missing.expect_err("缺 request_id 必须被 DTO 拒绝（不得 Option 掩盖）");
    assert!(
        err.to_string().contains("request_id"),
        "缺键错误必须点名字段: {err}"
    );
    // 平铺错误形态（把金额/供应商直接拍平发送）不得被误接受为合法结构
    let flat = serde_json::from_value::<CreateApPaymentRequest>(json!({
        "supplier_id": 1,
        "payment_amount": "100.00",
        "payment_date": "2026-09-20",
    }));
    assert!(
        flat.is_err(),
        "后端只认 request_id 派生口径，平铺字段结构必须拒绝"
    );
}

/// 前端契约类型扫描锁（frontend/src/api/ap.ts）：
/// rust_decimal 出参必须声明为 string（十进制字符串，先例 commit 83b8028b/42f67500 ——
/// number 声明会在 .toFixed 运行期崩）；后端 NOT NULL 列在前端不得标 `?`。
#[test]
fn ap_ts_decimal_outputs_are_typed_as_strings() {
    let ts = include_str!("../../frontend/src/api/ap.ts");
    for field in [
        "payment_amount: number",
        "request_amount: number",
        "exchange_rate: number",
    ] {
        assert!(
            !ts.contains(field),
            "ap.ts 出现 `{field}`：rust_decimal 出参被声明成 number（.toFixed 运行期崩溃族）"
        );
    }
    // 逐键钉死为正形
    for field in [
        "payment_amount: string",
        "request_amount: string",
        "exchange_rate: string",
    ] {
        assert!(ts.contains(field), "ap.ts 契约漂移：应存在 `{field}`");
    }
    // APPayment.currency 对应后端 NOT NULL 列（models/ap_payment.rs:49），不得带 `?`
    assert!(
        ts.contains("currency: string;"),
        "currency 为后端 NOT NULL 列，前端不得标可选掩盖缺键"
    );
}
