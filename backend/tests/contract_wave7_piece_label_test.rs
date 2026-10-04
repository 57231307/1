//! #220 成品布入库打印标签 — 真库契约锁（后端线，用户 2026-10-02 点名功能）
//!
//! 表结构唯一来源 = backend/migration（不自建 DDL，遵 #4669 判责路线一）；
//! 夹具 = `test_common::setup_test_db()`（缺 TEST_DATABASE_URL 直接 panic，禁 sqlite 回退）；
//! FK 父行自种子（warehouses/products/batch_dye_lot 等被清空且不播种）。
//!
//! 钉死行为（各条由本文件断言自证）：
//! ① 齐维 dyed 匹 GET /inventory/pieces/{id}/print → 2xx docx，标签正文含该匹**真实字段值**
//!    （缸号/色号/批次/匹号/米数/重量/幅宽/克重/条码文本 + JOIN products 的款号/品名），
//!    且**绝不出现** supplier_piece_no（保密口径）；
//! ② 8 必填字段任一为空（幅宽 NULL / 条码 NULL / 色号空串）→ 400 VALIDATION_ERROR，
//!    message 逐列点名（含 API 列名与匹号）且 ≠ 脱敏常量「请求参数验证失败」，不外显表名/内部 ID；
//! ③ 非 dyed（greige 生产匹）/ 样布（SAMPLE）→ BUSINESS_ERROR 族拒绝（门控先于字段校验）；
//! ④ 打卷入库缺 幅宽/克重/重量 → 400 VALIDATION_ERROR 且**不产生匹行**；补齐后 200，
//!    落库匹行 width/gram_weight/weight 三列非 NULL（实测值直落）。

mod test_common;

use axum::{
    Router,
    body::Body,
    http::{Method, Request, StatusCode, header},
    response::Response,
    routing::{get, post},
};
use bingxi_backend::container::AppState;
use bingxi_backend::handlers::{fabric_inspection_handler, print_handler};
use bingxi_backend::middleware::auth_context::AuthContext;
use bingxi_backend::models::batch_dye_lot;
use bingxi_backend::models::fabric_inspection_record;
use bingxi_backend::models::inventory_piece;
use bingxi_backend::models::product;
use bingxi_backend::models::status::fabric_inspection as inspection_status;
use bingxi_backend::models::status::purchase_inventory::inventory_piece as piece_status;
use bingxi_backend::models::warehouse;
use bingxi_backend::services::piece_domain_service::{PIECE_TYPE_DYED, PIECE_TYPE_GREIGE};
use sea_orm::{ActiveModelTrait, ActiveValue::Set, ColumnTrait, EntityTrait, QueryFilter};
use serde_json::{Value, json};
use std::io::Read;
use std::str::FromStr;
use std::sync::Arc;
use tower::ServiceExt;

fn dec(s: &str) -> rust_decimal::Decimal {
    rust_decimal::Decimal::from_str(s).unwrap()
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
        user_id: 220,
        username: "wave220_piece_label".to_string(),
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

fn build_app(db: &Arc<sea_orm::DatabaseConnection>, auth: AuthContext) -> Router {
    let state = AppState {
        db: db.clone(),
        ..Default::default()
    };
    Router::new()
        .route(
            "/erp/inventory/pieces/{id}/print",
            get(print_handler::inventory_piece_label_print_docx),
        )
        .route(
            "/erp/fabric-inspections/{id}/roll",
            post(fabric_inspection_handler::roll_fabric),
        )
        .with_state(state)
        .layer(axum::middleware::from_fn_with_state(auth, inject_auth))
}

async fn send(
    app: &Router,
    method: Method,
    uri: &str,
    body: Option<Value>,
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

fn json_body(bytes: &[u8]) -> Value {
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

/// 解 docx（zip）取 word/document.xml 全文（内容真伪断言范式，对齐
/// frontend/e2e/traversal/37b-print-content.spec.ts 的 JSZip 解包比对）
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

// ---------------------------------------------------------------------------
// 种子
// ---------------------------------------------------------------------------

/// 种子匹行参数（列逐一对 models/inventory_piece.rs 核对）；结构体入参，避免十余个位置参数
struct PieceSeed {
    id: i32,
    piece_no: &'static str,
    piece_type: &'static str,
    status: &'static str,
    color_no: &'static str,
    width: Option<rust_decimal::Decimal>,
    gram_weight: Option<rust_decimal::Decimal>,
    weight: Option<rust_decimal::Decimal>,
    barcode: Option<&'static str>,
}

impl PieceSeed {
    /// 齐维 dyed 匹模板（8 必填字段 + 条码齐全）；差异用例按列覆写
    fn complete(id: i32, piece_no: &'static str) -> Self {
        Self {
            id,
            piece_no,
            piece_type: PIECE_TYPE_DYED,
            status: piece_status::AVAILABLE,
            color_no: "W220-C01",
            width: Some(dec("180.00")),
            gram_weight: Some(dec("200.00")),
            weight: Some(dec("21.50")),
            barcode: Some(piece_no),
        }
    }
}

async fn seed_piece(db: &Arc<sea_orm::DatabaseConnection>, s: PieceSeed) {
    inventory_piece::ActiveModel {
        id: Set(s.id),
        piece_no: Set(s.piece_no.to_string()),
        piece_type: Set(s.piece_type.to_string()),
        status: Set(s.status.to_string()),
        warehouse_id: Set(1),
        product_id: Set(5),
        batch_no: Set("W220-B01".to_string()),
        color_no: Set(s.color_no.to_string()),
        dye_lot_no: Set("W220-LOT-A".to_string()),
        length: Set(dec("50.00")),
        width: Set(s.width),
        gram_weight: Set(s.gram_weight),
        weight: Set(s.weight),
        quality_status: Set(Some("合格".to_string())),
        inventory_status: Set(Some("available".to_string())),
        barcode: Set(s.barcode.map(|b| b.to_string())),
        warehouse_in_at: Set(Some(now())),
        supplier_piece_no: Set(Some("SUP-SECRET-W220".to_string())),
        created_at: Set(now()),
        updated_at: Set(now()),
        ..Default::default()
    }
    .insert(db.as_ref())
    .await
    .unwrap_or_else(|e| panic!("种子匹 {} 插入失败: {}", s.id, e));
}

async fn seeded_db() -> Arc<sea_orm::DatabaseConnection> {
    let db = Arc::new(test_common::setup_test_db().await);

    warehouse::ActiveModel {
        id: Set(1),
        warehouse_code: Set("WH-W220".to_string()),
        name: Set("成品布仓".to_string()),
        is_default: Set(false),
        is_active: Set(true),
        created_at: Set(now()),
        updated_at: Set(now()),
        ..Default::default()
    }
    .insert(&*db)
    .await
    .expect("种子仓库插入失败");

    product::ActiveModel {
        id: Set(5),
        code: Set("FAB-W220".to_string()),
        name: Set("红色成品布".to_string()),
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
    .expect("种子产品插入失败");

    // ① 齐维 dyed 匹（全字段真实值，supplier_piece_no 为保密反证哨兵）
    seed_piece(&db, PieceSeed::complete(10, "W220-LOT-A-001")).await;
    // ②a 幅宽 NULL（打卷旧数据形态：width 可空遗留）
    seed_piece(
        &db,
        PieceSeed {
            width: None,
            ..PieceSeed::complete(11, "W220-LOT-A-011")
        },
    )
    .await;
    // ②b 条码 NULL
    seed_piece(
        &db,
        PieceSeed {
            barcode: None,
            ..PieceSeed::complete(12, "W220-LOT-A-012")
        },
    )
    .await;
    // ②c 色号空串（NOT NULL 列的空值形态同样是「无据」）
    seed_piece(
        &db,
        PieceSeed {
            color_no: "",
            ..PieceSeed::complete(13, "W220-LOT-A-013")
        },
    )
    .await;
    // ③a 生产匹（greige，字段齐全也必须被类型门控拒绝）
    seed_piece(
        &db,
        PieceSeed {
            piece_type: PIECE_TYPE_GREIGE,
            ..PieceSeed::complete(20, "W220-LOT-A-020")
        },
    )
    .await;
    // ③b 样布（piece_type=dyed + status=SAMPLE，字段齐全也必须被状态门控拒绝）
    seed_piece(
        &db,
        PieceSeed {
            status: piece_status::SAMPLE,
            ..PieceSeed::complete(21, "W220-LOT-A-021")
        },
    )
    .await;

    // 打卷组父行：缸号 + 已评级验布记录
    batch_dye_lot::ActiveModel {
        id: Set(40),
        batch_no: Set("W220-RB1".to_string()),
        product_id: Set(5),
        dye_lot_no: Set("W220-ROLL1".to_string()),
        dye_date: Set(today()),
        quantity: Set(dec("500.00")),
        status: Set("ACTIVE".to_string()),
        created_at: Set(now()),
        updated_at: Set(now()),
        ..Default::default()
    }
    .insert(&*db)
    .await
    .expect("种子缸号插入失败");

    fabric_inspection_record::ActiveModel {
        id: Set(30),
        inspection_no: Set("FIR-W220-030".to_string()),
        dye_lot_no: Set(Some("W220-ROLL1".to_string())),
        product_id: Set(Some(5)),
        color_no: Set(Some("W220-RC1".to_string())),
        inspection_date: Set(today()),
        scoring_system: Set("four_point".to_string()),
        inspected_yards: Set(dec("100.00")),
        total_defect_points: Set(0),
        total_rolls: Set(0),
        total_roll_length: Set(rust_decimal::Decimal::ZERO),
        total_roll_weight: Set(rust_decimal::Decimal::ZERO),
        status: Set(inspection_status::GRADED.to_string()),
        is_deleted: Set(false),
        // 本模型时间列为 DateTimeWithTimeZone(FixedOffset)（models/fabric_inspection_record.rs:92-93）
        created_at: Set(now().into()),
        updated_at: Set(now().into()),
        ..Default::default()
    }
    .insert(&*db)
    .await
    .expect("种子验布记录插入失败");

    db
}

// ---------------------------------------------------------------------------
// ① 齐维 dyed 匹：2xx docx，标签正文为真实字段值，供应商编码零出现
// ---------------------------------------------------------------------------

#[tokio::test]
async fn complete_dyed_piece_returns_docx_with_real_values() {
    let db = seeded_db().await;
    let app = build_app(&db, make_auth());

    let (status, bytes) = send(&app, Method::GET, "/erp/inventory/pieces/10/print", None).await;
    assert!(
        status.is_success(),
        "齐维 dyed 匹打标签必须 2xx，实际 {status}: {}",
        String::from_utf8_lossy(&bytes)
            .chars()
            .take(300)
            .collect::<String>()
    );

    let xml = docx_document_xml(&bytes);
    // 标签标题 + 该匹真实值（逐列对种子核对）
    for expect in [
        "成品布入库标签",
        "W220-LOT-A",     // 缸号
        "W220-C01",       // 色号
        "W220-B01",       // 批次
        "W220-LOT-A-001", // 匹号（条码文本码值同源）
        "50",             // 米数
        "21.5",           // 重量
        "180",            // 幅宽
        "200",            // 克重
        "合格",           // 等级
        "FAB-W220",       // 款号（LEFT JOIN products.code）
        "红色成品布",     // 品名（LEFT JOIN products.name）
    ] {
        assert!(xml.contains(expect), "标签正文必须含真实值 {expect}");
    }
    // 保密口径（决策 §6）：供应商侧编码绝不允许出现在标签
    assert!(
        !xml.contains("SUP-SECRET-W220"),
        "supplier_piece_no 属保密列，绝不得出现在成品布标签"
    );
    // 标签是「该卷实测档案」，不得掺成本口径
    assert!(
        !xml.contains("unit_cost") && !xml.contains("total_cost"),
        "成本列不得进标签"
    );
}

// ---------------------------------------------------------------------------
// ② 缺任一必填字段：400 VALIDATION_ERROR 且 message 逐列点名（≠ 脱敏常量）
// ---------------------------------------------------------------------------

#[tokio::test]
async fn missing_field_fails_closed_naming_the_column() {
    let db = seeded_db().await;
    let app = build_app(&db, make_auth());

    let cases = [
        (11, "W220-LOT-A-011", "幅宽(width)"),
        (12, "W220-LOT-A-012", "条码(barcode)"),
        (13, "W220-LOT-A-013", "色号(color_no)"),
    ];
    for (id, piece_no, named_column) in cases {
        let (status, bytes) = send(
            &app,
            Method::GET,
            &format!("/erp/inventory/pieces/{id}/print"),
            None,
        )
        .await;
        let v = json_body(&bytes);
        assert_eq!(
            status,
            StatusCode::BAD_REQUEST,
            "匹 {id} 缺字段必须 400，信封: {v}"
        );
        assert_eq!(
            v["code"], "VALIDATION_ERROR",
            "字段必填缺失按本仓边界归 VALIDATION_ERROR 族，信封: {v}"
        );
        let msg = v["message"].as_str().unwrap_or_default();
        assert_ne!(
            msg, "请求参数验证失败",
            "缺字段拒绝必须外显点名，不得是脱敏常量（匹 {id}）"
        );
        assert!(
            msg.contains(named_column),
            "message 必须点名缺的是 {named_column}，实际: {msg}"
        );
        assert!(
            msg.contains(piece_no),
            "message 须带用户可见匹号 {piece_no} 以便定位，实际: {msg}"
        );
        // 不外显内部实现：表名/SQL/行 ID
        assert!(
            !msg.contains("inventory_piece") && !msg.contains("SELECT"),
            "文案不得泄露表名/SQL，实际: {msg}"
        );
    }
}

// ---------------------------------------------------------------------------
// ③ 非 dyed / 样布：BUSINESS_ERROR 族拒绝（门控先于字段校验）
// ---------------------------------------------------------------------------

#[tokio::test]
async fn non_dyed_and_sample_pieces_are_business_rejected() {
    let db = seeded_db().await;
    let app = build_app(&db, make_auth());

    // greige 生产匹（字段齐全）→ 必须按业务族拒，而非字段族
    let (status, bytes) = send(&app, Method::GET, "/erp/inventory/pieces/20/print", None).await;
    let v = json_body(&bytes);
    assert_eq!(status, StatusCode::BAD_REQUEST);
    assert_eq!(
        v["code"], "BUSINESS_ERROR",
        "非染色匹拒绝须 BUSINESS_ERROR 族（门控），信封: {v}"
    );

    // 样布（dyed + SAMPLE，字段齐全）→ 同样业务族拒绝（不打签，决策裁定 4）
    let (status, bytes) = send(&app, Method::GET, "/erp/inventory/pieces/21/print", None).await;
    let v = json_body(&bytes);
    assert_eq!(status, StatusCode::BAD_REQUEST);
    assert_eq!(
        v["code"], "BUSINESS_ERROR",
        "样布拒绝须 BUSINESS_ERROR 族，信封: {v}"
    );
}

// ---------------------------------------------------------------------------
// ④ 打卷实测值必填：缺三值 → 400 且零落库；齐全 → 200 且匹行三列非 NULL
// ---------------------------------------------------------------------------

#[tokio::test]
async fn roll_fabric_requires_measured_width_weight_gram() {
    let db = seeded_db().await;
    let app = build_app(&db, make_auth());

    // —— 缺幅宽/克重/重量：400 VALIDATION_ERROR，且不得产生匹行（拒绝先于写）——
    let (status, bytes) = send(
        &app,
        Method::POST,
        "/erp/fabric-inspections/30/roll",
        Some(json!({"warehouse_id": 1, "roll_length": 30.0})),
    )
    .await;
    let v = json_body(&bytes);
    assert!(
        status.is_client_error(),
        "打卷缺实测值必须 4xx（禁裸 500），实际 {status}: {v}"
    );
    assert_eq!(
        v["code"], "VALIDATION_ERROR",
        "字段必填族须 VALIDATION_ERROR，信封: {v}"
    );
    let msg = v["message"].as_str().unwrap_or_default();
    assert_ne!(msg, "请求参数验证失败", "缺字段须外显点名，实际: {msg}");
    for named in ["重量(kg)", "幅宽(cm)", "克重(g/m²)"] {
        assert!(msg.contains(named), "message 应点名 {named}，实际: {msg}");
    }
    let orphan = inventory_piece::Entity::find()
        .filter(inventory_piece::Column::InspectionId.eq(30))
        .all(&*db)
        .await
        .unwrap();
    assert!(
        orphan.is_empty(),
        "缺实测值被拒后不得留下无据匹行（拒绝先于任何写）"
    );

    // —— 三值齐全：200，且落库匹行 width/gram_weight/weight 三列非 NULL ——
    let (status, bytes) = send(
        &app,
        Method::POST,
        "/erp/fabric-inspections/30/roll",
        Some(json!({
            "warehouse_id": 1,
            "roll_length": 30.0,
            "roll_weight": 20.5,
            "roll_width": 185.0,
            "roll_gram_weight": 210.0,
        })),
    )
    .await;
    assert_eq!(
        status,
        StatusCode::OK,
        "三值齐全打卷应成功，body={}",
        String::from_utf8_lossy(&bytes)
            .chars()
            .take(300)
            .collect::<String>()
    );

    let pieces = inventory_piece::Entity::find()
        .filter(inventory_piece::Column::InspectionId.eq(30))
        .all(&*db)
        .await
        .unwrap();
    assert_eq!(pieces.len(), 1, "齐全打卷应恰好落一匹");
    let p = &pieces[0];
    assert_eq!(p.width, Some(dec("185.0")), "width 必须取打卷请求实测值");
    assert_eq!(
        p.gram_weight,
        Some(dec("210.0")),
        "gram_weight 必须取打卷请求实测值"
    );
    assert_eq!(p.weight, Some(dec("20.5")), "weight 必须取打卷请求实测值");
    assert_eq!(p.piece_type, "dyed");
    assert!(p.barcode.is_some(), "打卷产生的匹必须带条码（=piece_no）");

    let insp = fabric_inspection_record::Entity::find_by_id(30)
        .one(&*db)
        .await
        .unwrap()
        .unwrap();
    assert_eq!(
        insp.status,
        inspection_status::ROLLED,
        "打卷成功须流转 rolled"
    );
    assert_eq!(
        insp.total_roll_weight,
        dec("20.5"),
        "汇总重量按实测值累计（不再兜底 0）"
    );
}
