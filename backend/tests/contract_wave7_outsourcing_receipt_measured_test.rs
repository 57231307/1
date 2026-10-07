//! 委外收回匹实测值补录（m0075）—— 真库契约锁（后端+DB 线，用户裁定"立项并完成"）
//!
//! 表结构唯一来源 = backend/migration（不自建 DDL）；夹具 = `test_common::setup_test_db()`
//! （缺 TEST_DATABASE_URL 直接 panic，禁 sqlite 回退）；FK 父行自种子。
//!
//! 钉死行为（与 已锁口径逐条对齐：标签字段全取匹行实测值、不回落主数据、缺值 fail-closed 点名）
//! ① 收回单创建带三列实测值 ⇒ 逐列落收回单行、confirm 后逐列落匹行（**不取 products 标称值**：
//!    种子里刻意把 products.width/gram_weight 设成与实测值不同的哨兵数，回落即红）；
//! ② 该匹可打出成品布入库标签，标签正文是实测值（打通"收回匹根本打不出标签"的断链）；
//! ③ 收回单不填三列 ⇒ 匹行三列保持 NULL（不塞 0、不兜底），标签继续 400 逐列点名这三列；
//! ④ 更新口三态：键缺席=保持、显式 null=清空回"未补录"、有值=覆盖；0/负数属伪造实测值 ⇒
//!    400 VALIDATION_ERROR 点名该列且库内零变化；
//! ⑤ 拆匹：子卷 width/gram_weight 继承母卷实测值（同一卷布横向剪开，物理同源，非兜底），
//!    weight 维持按 cut_weight 的既有语义；母卷为 NULL 时子卷同样 NULL（不猜值），标签点名
//!    幅宽/克重但**不**点名重量（逐列点名的精度锁）；
//! ⑥ 回读闭环：GET /inventory/pieces 的 PieceResponse 必须回传 weight/width/gram_weight/barcode，
//!    键名与后端输出同为 snake_case 同源，NULL 如实回 null（不得省略键、不得洗成 0）；
//!    收回单端点（出参 = outsourcing_receipt::Model 直接序列化）同样必须带三键——
//!    前端补录表单的回读依赖它，缺键会被 `??`/`||` 兜底掩盖成"已实测"；
//! ⑦ 活库结构锁 + 约束活性锁：三列存在且可空无默认、三条逐列正值 CHECK 在 pg_constraint 里，
//!    且真库直写 0 幅宽必须被数据库拒绝（约束不是注释）。

mod test_common;

use axum::{
    Router,
    body::Body,
    http::{Method, Request, StatusCode, header},
    response::Response,
    routing::{get, post, put},
};
use bingxi_backend::container::AppState;
use bingxi_backend::handlers::{
    inventory_piece_handler, outsourcing_handler, piece_split_handler, print_handler,
};
use bingxi_backend::middleware::auth_context::AuthContext;
use bingxi_backend::models::status::{
    outsourcing_order_status, outsourcing_receipt_quality_status,
    purchase_inventory::inventory_piece as piece_status,
};
use bingxi_backend::models::{
    inventory_piece, outsourcing_order, outsourcing_receipt, product, warehouse,
};
use bingxi_backend::services::piece_domain_service::PIECE_TYPE_DYED;
use rust_decimal::Decimal;
use sea_orm::{
    ActiveModelTrait, ActiveValue::Set, ColumnTrait, ConnectionTrait, DbBackend, EntityTrait,
    QueryFilter, Statement, Value,
};
use serde_json::{Value as Json, json};
use std::io::Read;
use std::str::FromStr;
use std::sync::Arc;
use tower::ServiceExt;

// ---------------------------------------------------------------------------
// 常量（夹具坐标，全部为本用例自建行，不复用迁移种子）
// ---------------------------------------------------------------------------

const WH_ID: i32 = 1;
const PRODUCT_ID: i32 = 5;
const ORDER_ID: i32 = 9;
/// 委外订单缸号（染色外发 ⇒ 收回必须带缸号；匹行缸号取**收回单**的缸号）
const ORDER_DYE_LOT: &str = "OS220ORDERLOT";
/// 带实测值收回用的缸号
const LOT_MEASURED: &str = "OS220LOTHIT";
/// 未补录收回用的缸号
const LOT_UNMEASURED: &str = "OS220LOTMISS";
/// products 标称值哨兵（与实测值刻意不同）：一旦链路回落主数据，①/② 必红
const NOMINAL_WIDTH: &str = "150.00";
const NOMINAL_GRAM: &str = "300.00";

fn dec(s: &str) -> Decimal {
    Decimal::from_str(s).unwrap()
}

fn now() -> chrono::DateTime<chrono::Utc> {
    chrono::Utc::now()
}

fn today() -> chrono::NaiveDate {
    chrono::Utc::now().date_naive()
}

// ---------------------------------------------------------------------------
// 脚手架
// ---------------------------------------------------------------------------

fn make_auth() -> AuthContext {
    AuthContext {
        user_id: 221,
        username: "wave220_receipt_measured".to_string(),
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

/// 被测端点：收回单 CRUD/confirm、拆匹、匹列表、成品布入库标签
fn build_app(db: &Arc<sea_orm::DatabaseConnection>, auth: AuthContext) -> Router {
    let state = AppState {
        db: db.clone(),
        ..Default::default()
    };
    Router::new()
        .route(
            "/erp/production/outsourcing-receipts",
            post(outsourcing_handler::create_outsourcing_receipt),
        )
        .route(
            "/erp/production/outsourcing-receipts/{id}",
            put(outsourcing_handler::update_outsourcing_receipt),
        )
        .route(
            "/erp/production/outsourcing-receipts/{id}/confirm",
            post(outsourcing_handler::confirm_outsourcing_receipt),
        )
        .route(
            "/erp/inventory/piece-split",
            post(piece_split_handler::split_fabric_piece),
        )
        .route(
            "/erp/inventory/pieces",
            get(inventory_piece_handler::list_pieces),
        )
        .route(
            "/erp/inventory/pieces/{id}/print",
            get(print_handler::inventory_piece_label_print_docx),
        )
        .with_state(state)
        .layer(axum::middleware::from_fn_with_state(auth, inject_auth))
}

async fn send(
    app: &Router,
    method: Method,
    uri: &str,
    body: Option<Json>,
) -> (StatusCode, Vec<u8>) {
    let builder = Request::builder().method(method).uri(uri);
    let req = match body {
        Some(b) => builder
            .header(header::CONTENT_TYPE, "application/json")
            .body(Body::from(serde_json::to_vec(&b).unwrap()))
            .unwrap(),
        None => builder.body(Body::empty()).unwrap(),
    };
    let resp = app.clone().oneshot(req).await.unwrap();
    let status = resp.status();
    let bytes = axum::body::to_bytes(resp.into_body(), 8 * 1024 * 1024)
        .await
        .unwrap()
        .to_vec();
    (status, bytes)
}

fn json_body(bytes: &[u8]) -> Json {
    serde_json::from_slice(bytes).unwrap_or_else(|e| {
        panic!(
            "期望 JSON 信封（err={e}），body={}…",
            String::from_utf8_lossy(bytes)
                .chars()
                .take(400)
                .collect::<String>()
        )
    })
}

fn docx_document_xml(bytes: &[u8]) -> String {
    let cursor = std::io::Cursor::new(bytes.to_vec());
    let mut archive = zip::ZipArchive::new(cursor).expect("标签响应应是合法 docx(zip) 容器");
    let mut xml = String::new();
    for i in 0..archive.len() {
        let mut file = archive.by_index(i).expect("docx zip 条目可读");
        if file.name() == "word/document.xml" {
            file.read_to_string(&mut xml)
                .expect("document.xml 应为 UTF-8");
        }
    }
    assert!(!xml.is_empty(), "docx 必须含 word/document.xml 正文");
    xml
}

/// 只取 `<w:t>` 文本跑（text run）内容拼成纯文本。
/// 反兜底断言必须建立在**文本层**上：document.xml 的排版标记本身带大量数字属性
/// （spacing/width/size），按原始 XML 做 `!contains("150")` 会误报，正值断言也会失真。
fn docx_label_text(bytes: &[u8]) -> String {
    let xml = docx_document_xml(bytes);
    let mut text = String::new();
    let mut rest: &str = &xml;
    while let Some(open) = rest.find("<w:t") {
        let after = &rest[open..];
        // 只认真正的文本跑：<w:t> 或 <w:t xml:space="...">；<w:tc>/<w:tab/> 等一律跳过
        let is_text_run = after.starts_with("<w:t>") || after.starts_with("<w:t ");
        let Some(gt) = after.find('>') else { break };
        if !is_text_run {
            rest = &after[gt + 1..];
            continue;
        }
        let inner = &after[gt + 1..];
        let Some(close) = inner.find("</w:t>") else {
            break;
        };
        text.push_str(&inner[..close]);
        rest = &inner[close + "</w:t>".len()..];
    }
    assert!(!text.is_empty(), "标签正文应至少含一个 <w:t> 文本跑");
    text
}

// ---------------------------------------------------------------------------
// 种子
// ---------------------------------------------------------------------------

async fn seed_base(db: &Arc<sea_orm::DatabaseConnection>) {
    warehouse::ActiveModel {
        id: Set(WH_ID),
        warehouse_code: Set("WH-OS220".to_string()),
        name: Set("成品布仓".to_string()),
        // 染色匹入成品仓（piece_domain_service::validate_warehouse_for_piece_type 的门）
        warehouse_type: Set(Some("finished".to_string())),
        is_default: Set(false),
        is_active: Set(true),
        created_at: Set(now()),
        updated_at: Set(now()),
        ..Default::default()
    }
    .insert(db.as_ref())
    .await
    .expect("种子成品仓插入失败");

    product::ActiveModel {
        id: Set(PRODUCT_ID),
        code: Set("FAB-OS220".to_string()),
        name: Set("红色成品布".to_string()),
        unit: Set("米".to_string()),
        status: Set("active".to_string()),
        is_deleted: Set(false),
        product_type: Set("fabric".to_string()),
        // 主数据标称值哨兵： 口径禁止标签/匹行回落这里
        width: Set(Some(dec(NOMINAL_WIDTH))),
        gram_weight: Set(Some(dec(NOMINAL_GRAM))),
        created_at: Set(now()),
        updated_at: Set(now()),
        ..Default::default()
    }
    .insert(db.as_ref())
    .await
    .expect("种子产品插入失败");

    outsourcing_order::ActiveModel {
        id: Set(ORDER_ID),
        order_no: Set("OS-OS220-009".to_string()),
        order_type: Set("dyeing".to_string()),
        supplier_id: Set(1),
        dye_lot_no: Set(Some(ORDER_DYE_LOT.to_string())),
        color_no: Set(Some("OS220ORDERC".to_string())),
        issue_date: Set(today()),
        issue_quantity: Set(dec("50.00")),
        issue_unit: Set("米".to_string()),
        return_quantity: Set(Decimal::ZERO),
        loss_quantity: Set(Decimal::ZERO),
        standard_loss_rate: Set(Some(dec("0.1200"))),
        material_cost: Set(dec("1000.00")),
        processing_fee: Set(dec("200.00")),
        freight_fee: Set(dec("50.00")),
        tax_amount: Set(Decimal::ZERO),
        abnormal_loss_amount: Set(Decimal::ZERO),
        total_cost: Set(Decimal::ZERO),
        unit_cost: Set(Decimal::ZERO),
        // 收回资格门（services/outsourcing_ops/order.rs:48）：仅 issued/processing 可收回
        status: Set(outsourcing_order_status::PROCESSING.to_string()),
        is_deleted: Set(false),
        created_at: Set(now().into()),
        updated_at: Set(now().into()),
        ..Default::default()
    }
    .insert(db.as_ref())
    .await
    .expect("种子委外订单插入失败");
}

async fn seeded_db() -> Arc<sea_orm::DatabaseConnection> {
    let db = Arc::new(test_common::setup_test_db().await);
    seed_base(&db).await;
    db
}

/// 收回单创建体。`measured = None` ⇒ 三个键**整体缺席**（未补录形态）；
/// `Some((w, wd, g))` ⇒ 以字符串写值（Decimal 精确，不经浮点）。
fn receipt_body(
    receipt_no: &str,
    dye_lot_no: &str,
    measured: Option<(Decimal, Decimal, Decimal)>,
) -> Json {
    let mut body = json!({
        "receipt_no": receipt_no,
        "outsourcing_order_id": ORDER_ID,
        "receipt_date": "2026-10-05",
        "product_id": PRODUCT_ID,
        "color_no": "OS220C",
        "dye_lot_no": dye_lot_no,
        "warehouse_id": WH_ID,
        "return_quantity": "45.0000",
        "quality_status": outsourcing_receipt_quality_status::QUALIFIED,
        "grade": "A",
    });
    if let Some((weight, width, gram_weight)) = measured {
        body["weight"] = json!(weight.to_string());
        body["width"] = json!(width.to_string());
        body["gram_weight"] = json!(gram_weight.to_string());
    }
    body
}

/// 创建收回单（走真实 POST 端点，DTO→service→DB 全链）并返回 data（含 id）
async fn create_receipt(
    app: &Router,
    receipt_no: &str,
    dye_lot_no: &str,
    measured: Option<(Decimal, Decimal, Decimal)>,
) -> Json {
    let (status, bytes) = send(
        app,
        Method::POST,
        "/erp/production/outsourcing-receipts",
        Some(receipt_body(receipt_no, dye_lot_no, measured)),
    )
    .await;
    let v = json_body(&bytes);
    assert_eq!(
        status,
        StatusCode::OK,
        "创建收回单应成功（实测值补录不得把既有链路改红），信封: {v}"
    );
    v["data"].clone()
}

/// 确认收回单（draft → confirmed，同事务产匹）
async fn confirm_receipt(app: &Router, id: i32) -> Json {
    let (status, bytes) = send(
        app,
        Method::POST,
        &format!("/erp/production/outsourcing-receipts/{id}/confirm"),
        None,
    )
    .await;
    let v = json_body(&bytes);
    assert_eq!(status, StatusCode::OK, "确认收回单应成功，信封: {v}");
    v["data"].clone()
}

async fn piece_of_lot(
    db: &Arc<sea_orm::DatabaseConnection>,
    dye_lot_no: &str,
) -> inventory_piece::Model {
    let rows = inventory_piece::Entity::find()
        .filter(inventory_piece::Column::DyeLotNo.eq(dye_lot_no))
        .all(db.as_ref())
        .await
        .unwrap();
    assert_eq!(
        rows.len(),
        1,
        "缸号 {dye_lot_no} 下应恰好一匹（确认收回产一条，多余行说明夹具或链路串写）"
    );
    rows.into_iter().next().unwrap()
}

async fn receipt_row(db: &Arc<sea_orm::DatabaseConnection>, id: i32) -> outsourcing_receipt::Model {
    outsourcing_receipt::Entity::find_by_id(id)
        .one(db.as_ref())
        .await
        .unwrap()
        .unwrap_or_else(|| panic!("收回单 {id} 应已落库"))
}

/// PUT 收回单并回传状态码+信封（草稿态才可更新，本用例全程不 confirm）
async fn update_receipt(app: &Router, id: i32, body: Json) -> (StatusCode, Json) {
    let (status, bytes) = send(
        app,
        Method::PUT,
        &format!("/erp/production/outsourcing-receipts/{id}"),
        Some(body),
    )
    .await;
    (status, json_body(&bytes))
}

/// 拆匹母卷种子（列逐一对 models/inventory_piece.rs 核对）
struct ParentSeed {
    id: i32,
    piece_no: &'static str,
    dye_lot_no: &'static str,
    width: Option<Decimal>,
    gram_weight: Option<Decimal>,
    weight: Option<Decimal>,
}

async fn seed_parent(db: &Arc<sea_orm::DatabaseConnection>, s: ParentSeed) {
    inventory_piece::ActiveModel {
        id: Set(s.id),
        piece_no: Set(s.piece_no.to_string()),
        piece_type: Set(PIECE_TYPE_DYED.to_string()),
        status: Set(piece_status::AVAILABLE.to_string()),
        warehouse_id: Set(WH_ID),
        product_id: Set(PRODUCT_ID),
        batch_no: Set(s.dye_lot_no.to_string()),
        color_no: Set("OS220C".to_string()),
        dye_lot_no: Set(s.dye_lot_no.to_string()),
        length: Set(dec("50.00")),
        width: Set(s.width),
        gram_weight: Set(s.gram_weight),
        weight: Set(s.weight),
        quality_status: Set(Some("合格".to_string())),
        inventory_status: Set(Some("available".to_string())),
        barcode: Set(Some(s.piece_no.to_string())),
        warehouse_in_at: Set(Some(now())),
        created_at: Set(now()),
        updated_at: Set(now()),
        ..Default::default()
    }
    .insert(db.as_ref())
    .await
    .unwrap_or_else(|e| panic!("种子布卷 {} 插入失败: {}", s.id, e));
}

/// POST 拆匹
async fn split(app: &Router, body: Json) -> (StatusCode, Json) {
    let (status, bytes) = send(app, Method::POST, "/erp/inventory/piece-split", Some(body)).await;
    (status, json_body(&bytes))
}

// ---------------------------------------------------------------------------
// ① + ② 收回实测值逐列直落匹行（不回落主数据），且该匹能打标签
// ---------------------------------------------------------------------------

#[tokio::test]
async fn receipt_measured_values_land_on_piece_and_enable_label() {
    let db = seeded_db().await;
    let app = build_app(&db, make_auth());

    let created = create_receipt(
        &app,
        "OSR-220-HIT",
        LOT_MEASURED,
        Some((dec("21.5"), dec("185.0"), dec("200.0"))),
    )
    .await;
    let receipt_id = created["id"].as_i64().expect("创建响应应回收回单 id") as i32;

    // 出参契约锁（前端回读口的真实形状）：收回单端点直接序列化 outsourcing_receipt::Model
    // （handlers/outsourcing_handler.rs:303-310 返回类型直书 Model，无 rename ⇒ 键=列名 snake_case）。
    // 三键必须存在且等于提交值：缺键会被前端 `?? / ||` 兜底掩盖成"已实测"，与标签 fail-closed 口径分裂。
    let created_obj = created.as_object().expect("收回单出参应是对象");
    for key in ["weight", "width", "gram_weight"] {
        assert!(
            created_obj.contains_key(key),
            "POST /outsourcing-receipts 出参必须回传 {key} 键（前端补录回读依赖），实得键集: {:?}",
            created_obj.keys().collect::<Vec<_>>()
        );
    }
    assert_eq!(json_dec(&created["weight"]), Some(dec("21.5")));
    assert_eq!(json_dec(&created["width"]), Some(dec("185.0")));
    assert_eq!(json_dec(&created["gram_weight"]), Some(dec("200.0")));

    // 收回单行：三列如实落库（值与提交值逐列相等）
    let receipt = receipt_row(&db, receipt_id).await;
    assert_eq!(
        receipt.weight,
        Some(dec("21.5")),
        "weight 必须取收回单录入实测值"
    );
    assert_eq!(
        receipt.width,
        Some(dec("185.0")),
        "width 必须取收回单录入实测值"
    );
    assert_eq!(
        receipt.gram_weight,
        Some(dec("200.0")),
        "gram_weight 必须取收回单录入实测值"
    );

    confirm_receipt(&app, receipt_id).await;

    // 匹行三列 = 收回单实测值（**不是** products 的 150/300 标称值）
    let piece = piece_of_lot(&db, LOT_MEASURED).await;
    assert_eq!(piece.weight, Some(dec("21.5")));
    assert_eq!(piece.width, Some(dec("185.0")));
    assert_eq!(piece.gram_weight, Some(dec("200.0")));
    assert_eq!(piece.piece_type, PIECE_TYPE_DYED);
    assert!(
        piece.barcode.is_some(),
        "收回产匹必须带条码（=piece_no 口径）"
    );

    // 标签：这条匹现在打得出来，且印的是实测值
    let (status, bytes) = send(
        &app,
        Method::GET,
        &format!("/erp/inventory/pieces/{}/print", piece.id),
        None,
    )
    .await;
    assert!(
        status.is_success(),
        "补齐实测值的委外收回匹必须能打出标签（此前该链路三列恒 NULL ⇒ 断链），实际 {status}: {}",
        String::from_utf8_lossy(&bytes)
            .chars()
            .take(300)
            .collect::<String>()
    );
    let text = docx_label_text(&bytes);
    for expect in [
        "成品布入库标签",
        LOT_MEASURED,            // 缸号（取收回单缸号）
        "OS220C",                // 色号
        "45",                    // 米数（return_quantity）
        "21.5",                  // 实测重量
        "185",                   // 实测幅宽
        "200",                   // 实测克重
        piece.piece_no.as_str(), // 匹号 = 条码文本
        "FAB-OS220",             // 款号（JOIN products.code）
        "红色成品布",            // 品名（JOIN products.name）
    ] {
        assert!(
            text.contains(expect),
            "标签正文必须含真实值 {expect}，实得: {text}"
        );
    }
    // 反兜底：products 标称幅宽/克重绝不得出现在标签上
    assert!(
        !text.contains("150") && !text.contains("300"),
        "标签出现主数据标称值即为回落 products（违背 #220 已锁口径），实得: {text}"
    );
}

// ---------------------------------------------------------------------------
// ③ 未补录：匹行保持 NULL（不塞 0/不兜底），标签继续逐列点名拒绝
// ---------------------------------------------------------------------------

#[tokio::test]
async fn missing_measured_values_keep_piece_null_and_label_names_three_columns() {
    let db = seeded_db().await;
    let app = build_app(&db, make_auth());

    let created = create_receipt(&app, "OSR-220-MISS", LOT_UNMEASURED, None).await;
    let receipt_id = created["id"].as_i64().unwrap() as i32;

    // 未补录形态的出参同样是"键在、值为 null"（省略键会让前端 `?? '-'` 兜底把缺值
    // 与"已实测但恰好为空"混为一谈；本仓禁止响应结构随值形态变化）
    let created_obj = created.as_object().expect("收回单出参应是对象");
    for key in ["weight", "width", "gram_weight"] {
        assert!(
            created_obj.contains_key(key),
            "未补录时 {key} 键仍须在场且值为 null，实得键集: {:?}",
            created_obj.keys().collect::<Vec<_>>()
        );
        assert!(
            created_obj[key].is_null(),
            "未补录不得被写成 0 或其它默认值，实得: {created}"
        );
    }

    let receipt = receipt_row(&db, receipt_id).await;
    assert_eq!(
        (receipt.weight, receipt.width, receipt.gram_weight),
        (None, None, None),
        "未补录必须保持 NULL：DEFAULT 0 会把无据伪装成实测值（m0075 文件头论证）"
    );

    confirm_receipt(&app, receipt_id).await;
    let piece = piece_of_lot(&db, LOT_UNMEASURED).await;
    assert_eq!(
        (piece.weight, piece.width, piece.gram_weight),
        (None, None, None),
        "匹行三列必须原样 NULL，不得由 confirm 侧兜底"
    );

    let (status, bytes) = send(
        &app,
        Method::GET,
        &format!("/erp/inventory/pieces/{}/print", piece.id),
        None,
    )
    .await;
    let v = json_body(&bytes);
    assert_eq!(status, StatusCode::BAD_REQUEST, "缺实测值须 400，信封: {v}");
    assert_eq!(
        v["code"], "VALIDATION_ERROR",
        "缺列属字段必填族 ⇒ VALIDATION_ERROR，信封: {v}"
    );
    let msg = v["message"].as_str().unwrap_or_default();
    assert_ne!(msg, "请求参数验证失败", "缺字段须外显点名，不得是脱敏常量");
    for named in ["重量(weight)", "幅宽(width)", "克重(gram_weight)"] {
        assert!(msg.contains(named), "message 应点名 {named}，实际: {msg}");
    }
    assert!(
        msg.contains(&piece.piece_no),
        "message 须带用户可见匹号，实际: {msg}"
    );
}

// ---------------------------------------------------------------------------
// ④ 更新口三态 + 值域门（0/负数属伪造实测值）
// ---------------------------------------------------------------------------

#[tokio::test]
async fn receipt_update_three_state_and_measured_value_domain() {
    let db = seeded_db().await;
    let app = build_app(&db, make_auth());

    let created = create_receipt(
        &app,
        "OSR-220-UPD",
        "OS220LOTUPD",
        Some((dec("21.5"), dec("185.0"), dec("200.0"))),
    )
    .await;
    let id = created["id"].as_i64().unwrap() as i32;

    // 键全缺席 ⇒ 三列保持原值（RFC 7386 三态，"键缺席"不等于"清空"）
    let (status, v) = update_receipt(&app, id, json!({"remarks": "补录核对中"})).await;
    assert_eq!(status, StatusCode::OK, "仅改备注应成功，信封: {v}");
    let row = receipt_row(&db, id).await;
    assert_eq!(
        (row.weight, row.width, row.gram_weight),
        (Some(dec("21.5")), Some(dec("185.0")), Some(dec("200.0"))),
        "键缺席必须保持原实测值（不得被洗成 NULL 或 0）"
    );

    // 显式 null ⇒ 清空回"未补录"（DB 可空列语义），其余两列不动
    let (status, v) = update_receipt(&app, id, json!({"width": null})).await;
    assert_eq!(
        status,
        StatusCode::OK,
        "清空可空列应成功（未补录是合法形态），信封: {v}"
    );
    let row = receipt_row(&db, id).await;
    assert_eq!(row.width, None, "显式 null 须清空为 NULL");
    assert_eq!(
        row.weight,
        Some(dec("21.5")),
        "清空 width 不得连带改动 weight"
    );
    assert_eq!(
        row.gram_weight,
        Some(dec("200.0")),
        "清空 width 不得连带改动 gram_weight"
    );

    // 0 幅宽 = 伪造实测值 ⇒ 400 点名该列，库内零变化（不得让 0 绕过标签 fail-closed）
    let (status, v) = update_receipt(&app, id, json!({"width": "0"})).await;
    assert_eq!(status, StatusCode::BAD_REQUEST, "0 幅宽须 400，信封: {v}");
    assert_eq!(
        v["code"], "VALIDATION_ERROR",
        "值域越界须 VALIDATION_ERROR，信封: {v}"
    );
    let msg = v["message"].as_str().unwrap_or_default();
    assert!(
        msg.contains("幅宽(width)"),
        "拒绝必须点名幅宽列，实际: {msg}"
    );
    assert_eq!(
        receipt_row(&db, id).await.width,
        None,
        "被拒的更新不得落库（零变化）"
    );

    // 负克重同样被拒并点名
    let (status, v) = update_receipt(&app, id, json!({"gram_weight": "-1"})).await;
    assert_eq!(status, StatusCode::BAD_REQUEST, "负克重须 400，信封: {v}");
    assert!(
        v["message"]
            .as_str()
            .unwrap_or_default()
            .contains("克重(gram_weight)"),
        "拒绝必须点名克重列，实际: {}",
        v["message"]
    );
    assert_eq!(receipt_row(&db, id).await.gram_weight, Some(dec("200.0")));

    // 有值 ⇒ 覆盖（按实补录的正向路径）
    let (status, v) =
        update_receipt(&app, id, json!({"width": "186.5", "gram_weight": "205.0"})).await;
    assert_eq!(status, StatusCode::OK, "补录覆盖应成功，信封: {v}");
    let row = receipt_row(&db, id).await;
    assert_eq!(row.width, Some(dec("186.5")));
    assert_eq!(row.gram_weight, Some(dec("205.0")));
}

// ---------------------------------------------------------------------------
// ⑤a 拆匹继承：同一卷布剪开 ⇒ 幅宽/克重物理同源；weight 仍按 cut_weight
// ---------------------------------------------------------------------------

#[tokio::test]
async fn piece_split_inherits_parent_measured_width_and_gram() {
    let db = seeded_db().await;
    let app = build_app(&db, make_auth());

    seed_parent(
        &db,
        ParentSeed {
            id: 100,
            piece_no: "OS220-P100",
            dye_lot_no: "OS220LOTSPLIT",
            width: Some(dec("185.0")),
            gram_weight: Some(dec("200.0")),
            weight: Some(dec("30.0")),
        },
    )
    .await;

    let (status, v) = split(
        &app,
        json!({
            "parent_piece_id": 100,
            "cut_length": "10.0000",
            "cut_weight": "5.0000",
            "new_barcode": "OS220-CHILD-101",
        }),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "拆匹应成功，信封: {v}");

    let children = inventory_piece::Entity::find()
        .filter(inventory_piece::Column::ParentPieceId.eq(100))
        .all(db.as_ref())
        .await
        .unwrap();
    assert_eq!(children.len(), 1, "拆一刀应恰得一子卷");
    let child = &children[0];
    assert_eq!(
        child.width,
        Some(dec("185.0")),
        "子卷幅宽必须继承母卷实测值（同一卷布横向剪开不改变幅宽）"
    );
    assert_eq!(
        child.gram_weight,
        Some(dec("200.0")),
        "子卷克重必须继承母卷实测值（同布同克重）"
    );
    assert_eq!(
        child.weight,
        Some(dec("5.0")),
        "子卷重量仍是本次剪裁的 cut_weight（不得继承整卷重量）"
    );
    assert_eq!(child.length, dec("10.0"));

    // 母卷剩余重量按剪裁扣减（既有语义不得被改动），实测幅宽/克重不受拆分影响
    let parent = inventory_piece::Entity::find_by_id(100)
        .one(db.as_ref())
        .await
        .unwrap()
        .unwrap();
    assert_eq!(parent.weight, Some(dec("25.0")));
    assert_eq!(parent.width, Some(dec("185.0")), "拆分不得改动母卷实测幅宽");

    // 继承后的子卷具备打标签依据（这就是补录链的业务目的）
    let (status, bytes) = send(
        &app,
        Method::GET,
        &format!("/erp/inventory/pieces/{}/print", child.id),
        None,
    )
    .await;
    assert!(
        status.is_success(),
        "继承到实测幅宽/克重的子卷应可打标签，实际 {status}: {}",
        String::from_utf8_lossy(&bytes)
            .chars()
            .take(300)
            .collect::<String>()
    );
    let text = docx_label_text(&bytes);
    assert!(
        text.contains("OS220-CHILD-101"),
        "标签应是被拆出的子卷自身（匹号/条码），实得: {text}"
    );
    assert!(text.contains("185"), "标签须含继承的幅宽");
    assert!(text.contains("200"), "标签须含继承的克重");
    assert!(text.contains("10"), "标签须含子卷米数（10 米）");
}

// ---------------------------------------------------------------------------
// ⑤b 母卷 NULL ⇒ 子卷 NULL（不猜值），标签仍点名且只点名缺的两列
// ---------------------------------------------------------------------------

#[tokio::test]
async fn piece_split_with_null_parent_keeps_child_null_and_label_still_refuses() {
    let db = seeded_db().await;
    let app = build_app(&db, make_auth());

    seed_parent(
        &db,
        ParentSeed {
            id: 102,
            piece_no: "OS220-P102",
            dye_lot_no: "OS220LOTSPLN",
            width: None,
            gram_weight: None,
            weight: Some(dec("30.0")),
        },
    )
    .await;

    let (status, v) = split(
        &app,
        json!({
            "parent_piece_id": 102,
            "cut_length": "10.0000",
            "cut_weight": "5.0000",
            "new_barcode": "OS220-CHILD-103",
        }),
    )
    .await;
    assert_eq!(
        status,
        StatusCode::OK,
        "母卷缺幅宽/克重不阻止拆匹（缺值由标签环节 fail-closed 拦），信封: {v}"
    );

    let child = inventory_piece::Entity::find()
        .filter(inventory_piece::Column::ParentPieceId.eq(102))
        .one(db.as_ref())
        .await
        .unwrap()
        .expect("子卷应落库");
    assert_eq!(
        child.width, None,
        "母卷无实测幅宽时子卷必须 NULL（禁止猜值/回落主数据）"
    );
    assert_eq!(child.gram_weight, None, "母卷无实测克重时子卷必须 NULL");
    assert_eq!(child.weight, Some(dec("5.0")));

    let (status, bytes) = send(
        &app,
        Method::GET,
        &format!("/erp/inventory/pieces/{}/print", child.id),
        None,
    )
    .await;
    let v = json_body(&bytes);
    assert_eq!(
        status,
        StatusCode::BAD_REQUEST,
        "子卷缺实测值须 400，信封: {v}"
    );
    assert_eq!(v["code"], "VALIDATION_ERROR", "信封: {v}");
    let msg = v["message"].as_str().unwrap_or_default();
    assert!(msg.contains("幅宽(width)"), "应点名幅宽，实际: {msg}");
    assert!(msg.contains("克重(gram_weight)"), "应点名克重，实际: {msg}");
    assert!(
        !msg.contains("重量(weight)"),
        "子卷重量由 cut_weight 提供，属已实测列，不得被一起点名（逐列点名精度锁），实际: {msg}"
    );
}

// ---------------------------------------------------------------------------
// ⑥ 回读闭环：PieceResponse 回传三列 + barcode（三实测列缺值如实回 null；barcode 非缺值
//    考察对象，按生产口径恒为 piece_no 派生值，见用例内族F 注记）
// ---------------------------------------------------------------------------

/// Decimal 出参按本仓口径序列化为字符串，按串解码后数值比较（不做 .toFixed 造数）
fn json_dec(v: &Json) -> Option<Decimal> {
    match v.as_str() {
        Some(s) => {
            Some(Decimal::from_str(s).unwrap_or_else(|e| panic!("Decimal 串 {s} 解码失败: {e}")))
        }
        None => {
            assert!(v.is_null(), "数量列只可能是 Decimal 串或 null，实得 {v}");
            None
        }
    }
}

#[tokio::test]
async fn pieces_list_returns_measured_columns_and_barcode() {
    let db = seeded_db().await;
    let app = build_app(&db, make_auth());

    // 一条三值齐全、一条缺幅宽/克重（同一查询同时锁"有值回传"与"NULL 如实回 null"）
    seed_parent(
        &db,
        ParentSeed {
            id: 110,
            piece_no: "OS220-L110",
            dye_lot_no: "OS220LOTLIST",
            width: Some(dec("185.0")),
            gram_weight: Some(dec("200.0")),
            weight: Some(dec("21.5")),
        },
    )
    .await;
    seed_parent(
        &db,
        ParentSeed {
            id: 111,
            piece_no: "OS220-L111",
            dye_lot_no: "OS220LOTLIST",
            width: None,
            gram_weight: None,
            weight: None,
        },
    )
    .await;

    let (status, bytes) = send(
        &app,
        Method::GET,
        "/erp/inventory/pieces?page=1&page_size=50&dye_lot_no=OS220LOTLIST",
        None,
    )
    .await;
    assert_eq!(
        status,
        StatusCode::OK,
        "匹列表应 200，body={}",
        String::from_utf8_lossy(&bytes)
    );
    let v = json_body(&bytes);
    let items = v["data"]["items"]
        .as_array()
        .unwrap_or_else(|| panic!("分页列表唯一读法是 data.items，信封: {v}"));
    assert_eq!(items.len(), 2, "两条种子都应在结果内");

    let hit = items
        .iter()
        .find(|i| i["id"] == json!(110))
        .expect("id=110 行应存在");
    let obj = hit.as_object().expect("条目应是对象");
    for key in ["weight", "width", "gram_weight", "barcode"] {
        assert!(
            obj.contains_key(key),
            "PieceResponse 必须回传 {key} 键（读键与后端输出同为 snake_case 且同源），实得键集: {:?}",
            obj.keys().collect::<Vec<_>>()
        );
    }
    assert_eq!(json_dec(&hit["weight"]), Some(dec("21.5")));
    assert_eq!(json_dec(&hit["width"]), Some(dec("185.0")));
    assert_eq!(json_dec(&hit["gram_weight"]), Some(dec("200.0")));
    assert_eq!(hit["barcode"], json!("OS220-L110"));

    let miss = items
        .iter()
        .find(|i| i["id"] == json!(111))
        .expect("id=111 行应存在");
    let miss_obj = miss.as_object().expect("条目应是对象");
    // 缺值考察对象仅为三实测列（本例种子有意缺 width/gram_weight/weight）；
    // barcode **不在**本清单：生产全部产匹路径都按"由 piece_no 派生"落库
    // （piece_domain_service.rs:186/:657、fabric_inspection_service.rs:688、
    // piece_split_handler.rs:229），本文件 seed_parent:428 同口径写入 ⇒ 它恒非 null，
    // 旧断言循环把它列进"缺值必须回 null"是测试自身矛盾（CI 族F 实得
    // barcode="OS220-L111" 判红）。双向覆盖：这里改钉"键存在 + 等于生产派生值"。
    for key in ["weight", "width", "gram_weight"] {
        assert!(
            miss_obj.contains_key(key),
            "缺值行的 {key} 键必须仍存在且值为 null（省略键会让前端 ?? 兜底掩盖缺值），实得: {miss}"
        );
        assert!(
            miss_obj[key].is_null(),
            "缺值列必须如实回 null，实得 {miss}"
        );
    }
    assert!(
        miss_obj.contains_key("barcode"),
        "缺值行的 barcode 键必须仍存在（省略键同样会被前端 ?? 兜底掩盖），实得键集: {:?}",
        miss_obj.keys().collect::<Vec<_>>()
    );
    assert_eq!(
        miss["barcode"],
        json!("OS220-L111"),
        "barcode 必须等于生产口径派生值（= 该匹 piece_no），不是随便非空；实得 {miss}"
    );
}

// ---------------------------------------------------------------------------
// ⑦ 活库结构锁 + 约束活性锁（m0075 不是"写在迁移文本里"就算生效）
// ---------------------------------------------------------------------------

/// 结构锁用的收回单裸行（除 receipt_no/width 外全部固定；绕开服务口直插真库）
fn bare_receipt(receipt_no: &str, width: Option<Decimal>) -> outsourcing_receipt::ActiveModel {
    outsourcing_receipt::ActiveModel {
        receipt_no: Set(receipt_no.to_string()),
        outsourcing_order_id: Set(ORDER_ID),
        receipt_date: Set(today()),
        product_id: Set(PRODUCT_ID),
        return_quantity: Set(dec("10.00")),
        loss_quantity: Set(Decimal::ZERO),
        is_loss_normal: Set(true),
        unit_cost: Set(Decimal::ZERO),
        total_cost: Set(Decimal::ZERO),
        abnormal_loss_amount: Set(Decimal::ZERO),
        status: Set("draft".to_string()),
        is_deleted: Set(false),
        width: Set(width),
        weight: Set(None),
        gram_weight: Set(None),
        created_at: Set(now().into()),
        updated_at: Set(now().into()),
        ..Default::default()
    }
}

#[tokio::test]
async fn m0075_column_shape_and_positive_check_are_live() {
    let db = test_common::setup_test_db().await;

    // 结构：三列存在、numeric、可空、**无默认值**（默认值即伪造实测值）
    for column in ["weight", "width", "gram_weight"] {
        let rows = db
            .query_all_raw(Statement::from_sql_and_values(
                DbBackend::Postgres,
                "SELECT data_type, is_nullable, COALESCE(column_default, '<null>') \
                   FROM information_schema.columns \
                  WHERE table_schema = 'public' \
                    AND table_name = 'outsourcing_receipt' \
                    AND column_name = $1"
                    .to_string(),
                vec![Value::String(Some(column.to_string()))],
            ))
            .await
            .unwrap_or_else(|e| panic!("查询 outsourcing_receipt.{column} 结构失败: {e}"));
        assert_eq!(
            rows.len(),
            1,
            "outsourcing_receipt.{column} 应恰好 1 列（0=迁移未生效，>1=schema 泄漏）"
        );
        let data_type: String = rows[0]
            .try_get_by_index(0)
            .unwrap_or_else(|e| panic!("{column} data_type 解码失败: {e}"));
        let is_nullable: String = rows[0]
            .try_get_by_index(1)
            .unwrap_or_else(|e| panic!("{column} is_nullable 解码失败: {e}"));
        let column_default: String = rows[0]
            .try_get_by_index(2)
            .unwrap_or_else(|e| panic!("{column} column_default 解码失败: {e}"));
        assert_eq!(
            data_type, "numeric",
            "{column} 应为 numeric（DECIMAL(18,4)，与 inventory_piece 同型以保证透传无损）"
        );
        assert_eq!(is_nullable, "YES", "{column} 必须可空（NULL=未补录）");
        assert_eq!(
            column_default, "<null>",
            "{column} 不得带 DEFAULT：默认 0 会把无据伪装成实测值并绕过标签 fail-closed"
        );
    }

    // 三条逐列正值 CHECK 必须真实存在（按列命名 ⇒ 23514 可归因到列）
    let chk_rows = db
        .query_all_raw(Statement::from_string(
            DbBackend::Postgres,
            "SELECT conname FROM pg_constraint \
              WHERE conrelid = 'public.outsourcing_receipt'::regclass AND contype = 'c' \
                AND conname LIKE 'chk_outsourcing_receipt_%_positive' ORDER BY conname"
                .to_string(),
        ))
        .await
        .unwrap_or_else(|e| panic!("查询 outsourcing_receipt CHECK 失败: {e}"));
    let mut live: Vec<String> = Vec::new();
    for r in &chk_rows {
        live.push(
            r.try_get_by_index::<String>(0)
                .unwrap_or_else(|e| panic!("conname 解码失败: {e}")),
        );
    }
    for want in [
        "chk_outsourcing_receipt_gram_weight_positive",
        "chk_outsourcing_receipt_weight_positive",
        "chk_outsourcing_receipt_width_positive",
    ] {
        assert!(
            live.contains(&want.to_string()),
            "缺少 CHECK {want}，实得 {live:?}"
        );
    }

    // 活性：真库直写 0 幅宽必须被数据库拒绝（并发/旁路写入的兜底，不是装饰）
    let err = bare_receipt("OSR-220-CHECK-ZERO", Some(Decimal::ZERO))
        .insert(&db)
        .await
        .expect_err("0 幅宽不得落库（m0075 正值 CHECK 必须真实生效）");
    let msg = err.to_string();
    assert!(
        msg.contains("chk_outsourcing_receipt_width_positive") || msg.contains("23514"),
        "拒绝原因必须是该列 CHECK 违例（chk_outsourcing_receipt_width_positive / 23514），实际: {msg}"
    );

    // 对照：三列 NULL（未补录）必须是合法形态
    let null_row = bare_receipt("OSR-220-CHECK-NULL", None)
        .insert(&db)
        .await
        .unwrap_or_else(|e| {
            panic!("三列 NULL 的收回单必须可落库（本批口径=可空但如实透传），实际: {e}")
        });
    assert_eq!(null_row.width, None);
    assert_eq!(null_row.weight, None);
    assert_eq!(null_row.gram_weight, None);

    let leftovers = outsourcing_receipt::Entity::find()
        .filter(outsourcing_receipt::Column::ReceiptNo.eq("OSR-220-CHECK-ZERO"))
        .all(&db)
        .await
        .unwrap();
    assert!(
        leftovers.is_empty(),
        "被 CHECK 拒掉的写入应零落库（不得留下半行）"
    );
}
