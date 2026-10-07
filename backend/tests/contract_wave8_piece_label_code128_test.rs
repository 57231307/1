//! 成品布入库标签 — Code128 条码图形真库契约锁
//!
//! 表结构唯一来源 = backend/migration（不自建 DDL）；
//! 夹具 = `test_common::setup_test_db()`（缺 TEST_DATABASE_URL 直接 panic，禁 sqlite 回退）。
//!
//! 钉死行为（口径来源：memory project-pr942-piece-label-feature「条码只印文本码值 +
//! Code128 图形依赖本轮授权落地」；编码内容 = 匹行 `barcode` 列实测值，三处写入点均 = piece_no）：
//! ① 齐维 dyed 匹 → 2xx docx，`word/media/` 内必须存在 PNG 条目：非空、PNG 魔数、
//!    IHDR 尺寸 = code128 标准总宽（含 10X 静区）× 渲染常量；像素行反向重构的条序
//!    必须与 code128 库对同一码值产出的序列逐条相等，且解码回读值 == 种子条码值；
//!    同时正文仍含文本码值（图/文同源）。
//! ② `barcode IS NULL` 与 `barcode = ''`（空串）都是缺值：400 + 信封机器码
//!    VALIDATION_ERROR（按纪律只断 status+机器码，不断文案原文）。
//! 边界声明：校验位/符号表的算法正确性由 code128 crate（ISO/IEC 15417 实现 + 其
//! encode/decode 往返自测）保证；本锁证明的是「DB 值→渲染→docx 打包→像素→解码」
//! 全链路无篡改，真机 PDA 识读率仍只能部署环境人工点验。

mod test_common;

use axum::{
    Router,
    body::Body,
    http::{Method, Request, StatusCode},
    response::Response,
    routing::get,
};
use bingxi_backend::container::AppState;
use bingxi_backend::handlers::print_handler;
use bingxi_backend::middleware::auth_context::AuthContext;
use bingxi_backend::models::inventory_piece;
use bingxi_backend::models::product;
use bingxi_backend::models::status::purchase_inventory::inventory_piece as piece_status;
use bingxi_backend::models::warehouse;
use bingxi_backend::services::piece_domain_service::PIECE_TYPE_DYED;
use bingxi_backend::utils::barcode::{BARCODE_HEIGHT_PX, MODULE_WIDTH_PX};
use code128::{Bar, Code128};
use sea_orm::{ActiveModelTrait, ActiveValue::Set};
use serde_json::Value;
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

// ---------------------------------------------------------------------------
// 脚手架（与邻近 contract_wave7_piece_label_test.rs 同范式）
// ---------------------------------------------------------------------------

fn make_auth() -> AuthContext {
    AuthContext {
        user_id: 221,
        username: "wave221_code128_label".to_string(),
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
        .with_state(state)
        .layer(axum::middleware::from_fn_with_state(auth, inject_auth))
}

async fn send(app: &Router, uri: &str) -> (StatusCode, Vec<u8>) {
    let resp = app
        .clone()
        .oneshot(
            Request::builder()
                .method(Method::GET)
                .uri(uri)
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
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

/// 解 docx（zip），返回 (条目名, 字节) 中所有 word/media/*.png
fn docx_media_pngs(bytes: &[u8]) -> Vec<(String, Vec<u8>)> {
    let cursor = std::io::Cursor::new(bytes.to_vec());
    let mut archive = zip::ZipArchive::new(cursor).expect("标签响应应是合法 docx(zip) 容器");
    let mut out = Vec::new();
    for i in 0..archive.len() {
        let mut file = archive.by_index(i).expect("docx zip 条目可读");
        if file.name().starts_with("word/media/") && file.name().ends_with(".png") {
            let mut buf = Vec::new();
            file.read_to_end(&mut buf).expect("media 条目可读为字节");
            out.push((file.name().to_string(), buf));
        }
    }
    out
}

/// 解 docx 取 word/document.xml 全文
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

/// 从 PNG 中部像素行反向重构条序（模块单位）。
/// 依赖声明的渲染几何：左静区 = 10 模块白、黑条起始于 x=10×X、右静区同理；
/// 任何一条不满足都直接 panic（fail-closed，不静默放行空白/畸形图形）。
fn bars_from_png_row(png: &[u8]) -> Vec<Bar> {
    let img = image::ImageReader::new(std::io::Cursor::new(png.to_vec()))
        .with_guessed_format()
        .expect("media 字节应可被识别格式")
        .decode()
        .expect("PNG 应可解码")
        .into_luma8();
    let (w, h) = (img.width(), img.height());
    assert!(w > 0 && h > 0, "条码位图尺寸必须 > 0");
    let row = h / 2;
    let black = |x: u32| img.get_pixel(x, row).0[0] < 128;

    let first = (0..w)
        .find(|x| black(*x))
        .expect("条码行必须含黑条（禁空白图）");
    let last = (0..w)
        .rev()
        .find(|x| black(*x))
        .expect("条码行必须含黑条（禁空白图）");
    assert_eq!(
        first,
        10 * MODULE_WIDTH_PX,
        "左静区必须是标准 10X（ISO/IEC 15417）"
    );
    assert_eq!(
        w - (last + 1),
        10 * MODULE_WIDTH_PX,
        "右静区必须是标准 10X（终止符之后无多余黑区）"
    );

    let mut bars = Vec::new();
    let mut x = first;
    while x <= last {
        assert!(black(x), "重构游标必须落在黑条起点");
        let mut bw = 0u32;
        while x + bw <= last && black(x + bw) {
            bw += 1;
        }
        x += bw;
        let mut sp = 0u32;
        while x + sp <= last && !black(x + sp) {
            sp += 1;
        }
        x += sp;
        assert_eq!(bw % MODULE_WIDTH_PX, 0, "黑条宽必须是 X 的整数倍");
        assert_eq!(sp % MODULE_WIDTH_PX, 0, "白区宽必须是 X 的整数倍");
        bars.push(Bar {
            width: (bw / MODULE_WIDTH_PX) as u8,
            space: (sp / MODULE_WIDTH_PX) as u8,
        });
    }
    bars
}

// ---------------------------------------------------------------------------
// 种子
// ---------------------------------------------------------------------------

struct PieceSeed {
    id: i32,
    piece_no: &'static str,
    barcode: Option<&'static str>,
}

async fn seed_piece(db: &Arc<sea_orm::DatabaseConnection>, s: PieceSeed) {
    inventory_piece::ActiveModel {
        id: Set(s.id),
        piece_no: Set(s.piece_no.to_string()),
        piece_type: Set(PIECE_TYPE_DYED.to_string()),
        status: Set(piece_status::AVAILABLE.to_string()),
        warehouse_id: Set(1),
        product_id: Set(5),
        batch_no: Set("W221-B01".to_string()),
        color_no: Set("W221-C01".to_string()),
        dye_lot_no: Set("W221-LOT-B".to_string()),
        length: Set(dec("50.00")),
        width: Set(Some(dec("180.00"))),
        gram_weight: Set(Some(dec("200.00"))),
        weight: Set(Some(dec("21.50"))),
        quality_status: Set(Some("合格".to_string())),
        inventory_status: Set(Some("available".to_string())),
        barcode: Set(s.barcode.map(|b| b.to_string())),
        warehouse_in_at: Set(Some(now())),
        supplier_piece_no: Set(Some("SUP-SECRET-W221".to_string())),
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
        warehouse_code: Set("WH-W221".to_string()),
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
        code: Set("FAB-W221".to_string()),
        name: Set("蓝色成品布".to_string()),
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

    // ① 齐维 dyed 匹（barcode = piece_no，与真实写入点语义一致）
    seed_piece(
        &db,
        PieceSeed {
            id: 10,
            piece_no: "W221-LOT-B-001",
            barcode: Some("W221-LOT-B-001"),
        },
    )
    .await;
    // ②a 条码 NULL
    seed_piece(
        &db,
        PieceSeed {
            id: 11,
            piece_no: "W221-LOT-B-011",
            barcode: None,
        },
    )
    .await;
    // ②b 条码空串（口径「NULL、空串」同属缺值）
    seed_piece(
        &db,
        PieceSeed {
            id: 12,
            piece_no: "W221-LOT-B-012",
            barcode: Some(""),
        },
    )
    .await;

    db
}

// ---------------------------------------------------------------------------
// ① 齐维 dyed 匹：docx 内嵌 Code128 PNG，像素反向解码 == 种子条码值
// ---------------------------------------------------------------------------

#[tokio::test]
async fn complete_dyed_piece_label_embeds_decodable_code128_png() {
    let db = seeded_db().await;
    let app = build_app(&db, make_auth());

    let (status, bytes) = send(&app, "/erp/inventory/pieces/10/print").await;
    assert!(
        status.is_success(),
        "齐维 dyed 匹打标签必须 2xx，实际 {status}: {}",
        String::from_utf8_lossy(&bytes)
            .chars()
            .take(300)
            .collect::<String>()
    );

    // 1) media 里必须恰有一张 PNG 且非空、带 PNG 魔数
    let pngs = docx_media_pngs(&bytes);
    let names: Vec<&str> = pngs.iter().map(|(n, _)| n.as_str()).collect();
    assert!(
        pngs.len() == 1,
        "标签 docx 必须内嵌恰好一张条码 PNG，实际条目: {names:?}"
    );
    let (name, png) = &pngs[0];
    assert!(!png.is_empty(), "{name} 的 PNG 字节不得为空");
    assert!(
        png.starts_with(&[137, 80, 78, 71, 13, 10, 26, 10]),
        "{name} 必须以 PNG 魔数开头（docx-rs 仅支持 PNG 嵌入）"
    );
    assert!(png.len() > 24, "{name} 长度须覆盖 PNG 头+IHDR");

    // 2) IHDR 尺寸 = 标准总宽（含 20 模块静区）× X，高 = 渲染常量
    let code = Code128::encode(b"W221-LOT-B-001");
    let expect_w = code.len() as u32 * MODULE_WIDTH_PX;
    let ihdr_w = u32::from_be_bytes([png[16], png[17], png[18], png[19]]);
    let ihdr_h = u32::from_be_bytes([png[20], png[21], png[22], png[23]]);
    assert_eq!(ihdr_w, expect_w, "PNG 宽必须等于 Code128 声明总宽×X");
    assert_eq!(ihdr_h, BARCODE_HEIGHT_PX, "PNG 高必须等于渲染常量高度");

    // 3) 像素反向重构的条序必须与库对同一码值产出的标准序列逐条相等
    //    （证明打包/渲染没有吞条、变形、静默出空白图）
    let from_pixels = bars_from_png_row(png);
    let from_encoder: Vec<Bar> = code.bars().collect();
    assert_eq!(
        from_pixels, from_encoder,
        "嵌入图形的条序必须与条码值的标准 Code128 编码逐条一致"
    );

    // 4) 端到端：从像素重构的条序解码回读 == 种子条码文本值
    let decoded = code128::decode(&from_pixels).expect("像素条序应可按 ISO 15417 解码");
    assert_eq!(
        String::from_utf8(decoded).expect("解码载荷应为 ASCII"),
        "W221-LOT-B-001",
        "图形解码回读值必须等于该匹 barcode 实测值"
    );

    // 5) 图/文同源：正文仍含文本码值行；保密哨兵零出现
    let xml = docx_document_xml(&bytes);
    assert!(
        xml.contains("W221-LOT-B-001"),
        "标签正文必须仍含条码文本码值"
    );
    assert!(
        !xml.contains("SUP-SECRET-W221"),
        "supplier_piece_no 属保密列，不得进入标签（含图形元数据）"
    );
}

// ---------------------------------------------------------------------------
// ② 条码缺值（NULL / 空串）：按既有 fail-closed 路径 400 + VALIDATION_ERROR
// ---------------------------------------------------------------------------

#[tokio::test]
async fn missing_or_empty_barcode_is_rejected_not_blank_printed() {
    let db = seeded_db().await;
    let app = build_app(&db, make_auth());

    for id in [11u32, 12] {
        let (status, bytes) = send(&app, &format!("/erp/inventory/pieces/{id}/print")).await;
        let v = json_body(&bytes);
        assert_eq!(
            status,
            StatusCode::BAD_REQUEST,
            "匹 {id} 条码缺值必须 400（禁静默出空白条码），信封: {v}"
        );
        assert_eq!(
            v["code"], "VALIDATION_ERROR",
            "条码缺值属字段必填族，机器码必须 VALIDATION_ERROR，信封: {v}"
        );
        // 缺值即拒：响应必须是 JSON 信封，绝不允许误走渲染产出 docx 字节（zip 魔数 PK\x03\x04）
        assert!(
            !bytes.starts_with(&[0x50, 0x4B, 0x03, 0x04]),
            "匹 {id} 被拒时响应体不得是 docx(zip) 字节流"
        );
    }
}
