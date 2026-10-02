//! 供应商资质附件真上传写侧契约锁（任务 260929：attachment_path 由手敲文本改为真上传）
//!
//! 锁定的缺陷与修复（现状实证：全页无 el-upload，文本框直写 VARCHAR(500)，超限裸 500）：
//! - 新端点 `POST /purchase/suppliers/{supplier_id}/qualifications/{qualification_id}/attachment`
//!   （`handlers/supplier_handler.rs::upload_supplier_qualification_attachment`）：
//!   1. 父资源归属门控 + data_scope（与四个资质端点同源：get_supplier(supplier_id, Some(&ctx))）；
//!   2. (supplier_id, qualification_id) 双键归属校验，错配返回**用户可见** business_displayable
//!      文案（BUSINESS_ERROR + 真实 message，不是脱敏的「业务处理失败」）；
//!   3. 扩展名白名单 pdf/jpg/jpeg/png（非白名单后缀被拒，即使内容合法）；
//!   4. magic bytes 校验：伪造 Content-Type / 合法后缀但内容非 PDF/JPEG/PNG 被拒；
//!      后缀与内容不同族（.png 实为 %PDF）同样被拒；
//!   5. 大小上限（5MB）显式拒绝，不落裸 500；
//!   6. 文件名服务端生成 `{supplier_id}_{qualification_id}.{ext}`，客户端原名不参与落盘名，
//!      上传后 attachment_path 回读 = 服务端受控 URL `/uploads/qualifications/...`；
//!   7. 读取端点 `GET .../attachment` 与双键归属同源（错配对读取同样可见文案拒绝）。
//! - 病毒扫描边界：CLAMAV_ENABLED 未启用时 warn 日志跳过（本仓禁止静默；CI 环境不联网扫描）。
//!
//! 覆盖策略（路线一，#4669 判责；表结构唯一来源 = backend/migration，不自建 DDL）：
//! - 全部用例经 `test_common::setup_test_db()` 打已迁移 PostgreSQL；真实 handler 端到端
//!   （tower oneshot + AuthContext 注入层）。注意 `suppliers` 属迁移种子参照表
//!   （SEALED，不清空）：本文件不写显式 id、不自造与迁移演示行（id 1/2）冲突的主键，
//!   每用例用纳秒唯一 supplier_code **新插**自己的两行供应商并按真实返回 id 断言；
//!   `supplier_qualifications` 是业务表（随夹具清空），资质 id 由 SERIAL 稳定给出。
//!   种子供应商 created_by=NULL（get_supplier 的「系统/共享种子记录」语义，不受行级过滤）。
//! - multipart 体手工构造（字段名 file），断言 HTTP 状态 + 失败信封 code/message 真实外显
//!   + DB 回读 attachment_path 受控 URL + 落盘/读回字节一致（成功用例结束后清理生成文件）。
//! - 错误族口径（与当前 handler 注释逐条核对，修复提交「反向族校正」定案）：
//!   扩展名白名单/magic bytes/后缀内容不符/大小超限均为**输入格式校验** → VALIDATION_ERROR
//!   （validation_displayable，文案可外显）；双键错配属业务归属拒绝 → BUSINESS_ERROR
//!   （business_displayable）。本文件按各自正确族断言，不再笼统写 BUSINESS_ERROR。

use axum::{
    Router,
    body::Body,
    extract::State,
    http::{Request, StatusCode},
    middleware::{Next, from_fn_with_state},
    response::Response,
    routing::get,
};
use bingxi_backend::container::AppState;
use bingxi_backend::handlers::supplier_handler;
use bingxi_backend::middleware::auth_context::AuthContext;
use bingxi_backend::models::{supplier, supplier_qualification};
use chrono::Utc;
use rust_decimal::Decimal;
use sea_orm::{ActiveModelTrait, EntityTrait, Set};
use serde_json::Value;
use std::str::FromStr;
use std::sync::Arc;
use tower::ServiceExt;

mod test_common;

fn dec(s: &str) -> Decimal {
    Decimal::from_str(s).unwrap()
}

fn make_auth(user_id: i32) -> AuthContext {
    AuthContext {
        user_id,
        username: format!("e2e_user_{user_id}"),
        role_id: Some(1),
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

/// 本组用例造出的行 id（全部来自真库 SERIAL/RETURNING，断言不手写第二套 id）
struct SeedIds {
    /// 资质归属的供应商 A
    supplier_a: i32,
    /// 用于构造双键错配的供应商 B
    supplier_b: i32,
    /// 归属 A 的可上传资质（首测目标）
    qual_a1: i32,
    /// 归属 A 的超限对照资质
    qual_a2: i32,
    /// 归属 A 且无附件的资质（GET 404 对照）
    qual_no_attach: i32,
}

/// 种子：新插两个供应商（suppliers 为参照表不清空 ⇒ 纳秒唯一 code、不写显式 id）；
/// 供应商 A 名下三条资质（valid_until 远期，避免过期派生干扰），B 无资质。
async fn seed(db: &sea_orm::DatabaseConnection) -> SeedIds {
    // supplier/supplier_qualification 的 created_at/updated_at 均为
    // DateTimeWithTimeZone（= DateTime<FixedOffset>，models/supplier.rs:78,80），
    // 用仓内惯用的 `Utc::now().into()` 构造
    let now: chrono::DateTime<chrono::FixedOffset> = Utc::now().into();
    let suffix = Utc::now().timestamp_nanos_opt().expect(
        "测试造数取当前时刻纳秒：Utc::now 必落在 chrono 纳秒可表示区间（约1678-2262 年），None 不可达",
    );
    let mut supplier_ids = Vec::new();
    for tag in ["A", "B"] {
        let sid = supplier::ActiveModel {
            supplier_code: Set(format!("SUP-ATTACH-{suffix}-{tag}")),
            supplier_name: Set(format!("附件测试供应商{suffix}{tag}")),
            supplier_short_name: Set(format!("附件{tag}")),
            supplier_type: Set("面料".to_string()),
            credit_code: Set(format!("CREDIT{suffix}{tag}")),
            registered_address: Set("测试地址".to_string()),
            legal_representative: Set("张三".to_string()),
            registered_capital: Set(dec("100")),
            establishment_date: Set(chrono::NaiveDate::from_ymd_opt(2020, 1, 1).unwrap()),
            taxpayer_type: Set("一般纳税人".to_string()),
            bank_name: Set("测试银行".to_string()),
            bank_account: Set("6222000000".to_string()),
            contact_phone: Set("13800000000".to_string()),
            created_at: Set(now),
            updated_at: Set(now),
            created_by: Set(None),
            is_processor: Set(false),
            ..Default::default()
        }
        .insert(db)
        .await
        .expect("种子供应商插入失败（真表 suppliers）")
        .id;
        supplier_ids.push(sid);
    }
    let (supplier_a, supplier_b) = (supplier_ids[0], supplier_ids[1]);

    let far = chrono::NaiveDate::from_ymd_opt(2030, 12, 31).unwrap();
    let mut qual_ids = Vec::new();
    for n in [1i32, 2, 3] {
        let qid = supplier_qualification::ActiveModel {
            // 自增主键：NotSet 交给序列发号（真表 id 是 SERIAL，禁手工塞值撞号）
            id: Default::default(),
            supplier_id: Set(supplier_a),
            qualification_name: Set(format!("营业执照{suffix}-{n}")),
            qualification_type: Set("证照".to_string()),
            qualification_no: Set(format!("NO-{suffix}-{n}")),
            issuing_authority: Set("市场监管局".to_string()),
            issue_date: Set(chrono::NaiveDate::from_ymd_opt(2024, 1, 1).unwrap()),
            valid_until: Set(far),
            attachment_path: Set(None),
            need_annual_check: Set(false),
            annual_check_record: Set(None),
            is_expired: Set(false),
            remarks: Set(None),
            created_at: Set(now),
            updated_at: Set(now),
        }
        .insert(db)
        .await
        .expect("种子资质插入失败（真表 supplier_qualifications）")
        .id;
        qual_ids.push(qid);
    }
    SeedIds {
        supplier_a,
        supplier_b,
        qual_a1: qual_ids[0],
        qual_a2: qual_ids[1],
        qual_no_attach: qual_ids[2],
    }
}

async fn seeded_app() -> (Router, sea_orm::DatabaseConnection, SeedIds) {
    let db = test_common::setup_test_db().await;
    let ids = seed(&db).await;
    let state = AppState {
        db: Arc::new(db.clone()),
        ..Default::default()
    };
    let app = Router::new()
        .route(
            "/suppliers/{id}/qualifications/{qualification_id}/attachment",
            get(supplier_handler::get_supplier_qualification_attachment)
                .post(supplier_handler::upload_supplier_qualification_attachment),
        )
        .with_state(state)
        .layer(from_fn_with_state(make_auth(100), inject_auth));
    (app, db, ids)
}

/// 手工构造 multipart/form-data 请求体（字段名固定 file，模拟前端 FormData.append）
fn multipart_body(boundary: &str, file_name: &str, content_type: &str, data: &[u8]) -> Vec<u8> {
    let mut body = Vec::new();
    body.extend_from_slice(
        format!(
            "--{}\r\nContent-Disposition: form-data; name=\"file\"; filename=\"{}\"\r\nContent-Type: {}\r\n\r\n",
            boundary, file_name, content_type
        )
        .as_bytes(),
    );
    body.extend_from_slice(data);
    body.extend_from_slice(format!("\r\n--{}--\r\n", boundary).as_bytes());
    body
}

async fn post_attachment(
    app: &Router,
    supplier_id: i32,
    qualification_id: i32,
    file_name: &str,
    declared_content_type: &str,
    data: &[u8],
) -> (StatusCode, Value) {
    let boundary = "----bxAttachmentTestBoundary";
    let body = multipart_body(boundary, file_name, declared_content_type, data);
    let resp = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri(format!(
                    "/suppliers/{supplier_id}/qualifications/{qualification_id}/attachment"
                ))
                .header(
                    axum::http::header::CONTENT_TYPE,
                    format!("multipart/form-data; boundary={boundary}"),
                )
                .body(Body::from(body))
                .unwrap(),
        )
        .await
        .unwrap();
    let status = resp.status();
    let bytes = axum::body::to_bytes(resp.into_body(), 4 * 1024 * 1024)
        .await
        .unwrap();
    (status, serde_json::from_slice(&bytes).unwrap())
}

async fn get_attachment(
    app: &Router,
    supplier_id: i32,
    qualification_id: i32,
) -> (StatusCode, Vec<u8>, Option<String>) {
    let resp = app
        .clone()
        .oneshot(
            Request::builder()
                .method("GET")
                .uri(format!(
                    "/suppliers/{supplier_id}/qualifications/{qualification_id}/attachment"
                ))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    let status = resp.status();
    let content_type = resp
        .headers()
        .get(axum::http::header::CONTENT_TYPE)
        .and_then(|v| v.to_str().ok())
        .map(|s| s.to_string());
    let bytes = axum::body::to_bytes(resp.into_body(), 16 * 1024 * 1024)
        .await
        .unwrap();
    (status, bytes.to_vec(), content_type)
}

/// 以 DB 实体回读 attachment_path（真相层，不看响应体）
async fn attachment_path_of(db: &sea_orm::DatabaseConnection, qid: i32) -> Option<String> {
    supplier_qualification::Entity::find_by_id(qid)
        .one(db)
        .await
        .unwrap()
        .unwrap_or_else(|| panic!("资质 {qid} 应存在"))
        .attachment_path
}

// =========================================================
// 1) 拒绝路径（不产生落盘与 DB 写入）
// =========================================================

/// 非白名单后缀被拒（内容本身是合法 PDF 也不放行——白名单是场景约束，不只防攻击）
#[tokio::test]
async fn upload_rejects_non_whitelisted_extension() {
    let (app, db, ids) = seeded_app().await;
    let mut data = b"%PDF-1.7 fake".to_vec();
    data.extend_from_slice(&[0u8; 64]);
    let (status, v) = post_attachment(
        &app,
        ids.supplier_a,
        ids.qual_a1,
        "invoice.pdf.exe",
        "application/pdf",
        &data,
    )
    .await;
    assert_eq!(status, StatusCode::BAD_REQUEST, "实际体: {v}");
    // 当前 handler 定案（反向族校正）：扩展名不在白名单属输入格式校验 → VALIDATION_ERROR
    assert_eq!(v["code"], "VALIDATION_ERROR", "实际体: {v}");
    assert!(
        v["message"]
            .as_str()
            .unwrap_or_default()
            .contains("仅支持 pdf/jpg/jpeg/png"),
        "拒绝文案必须可见且指向白名单，实际: {v}"
    );
    assert_eq!(
        attachment_path_of(&db, ids.qual_a1).await,
        None,
        "被拒绝的上传不得留下 attachment_path 写入"
    );
}

/// 伪造 Content-Type application/pdf + 合法 .pdf 后缀，但文件内容非 PDF/JPEG/PNG → 拒绝
#[tokio::test]
async fn upload_rejects_forged_content_type_with_bad_magic() {
    let (app, db, ids) = seeded_app().await;
    let (status, v) = post_attachment(
        &app,
        ids.supplier_a,
        ids.qual_a1,
        "license.pdf",
        "application/pdf",
        b"#!/bin/sh\necho not-a-pdf",
    )
    .await;
    assert_eq!(status, StatusCode::BAD_REQUEST, "实际体: {v}");
    // 内容格式非法属输入校验族（当前 handler 定案；文案已按安全边界去掉
    // 内部机制术语 magic bytes，断言锁当前可见文案）
    assert_eq!(v["code"], "VALIDATION_ERROR");
    assert!(
        v["message"]
            .as_str()
            .unwrap_or_default()
            .contains("附件内容不是有效的证照文件"),
        "拒绝必须指向内容格式校验，实际: {v}"
    );
    assert_eq!(
        attachment_path_of(&db, ids.qual_a1).await,
        None,
        "内容校验失败的请求不得写入 attachment_path"
    );
}

/// 合法后缀 + 合法 magic，但后缀与内容不同族（.png 实为 %PDF）→ 伪造拒绝
#[tokio::test]
async fn upload_rejects_extension_magic_family_mismatch() {
    let (app, _db, ids) = seeded_app().await;
    let (status, v) = post_attachment(
        &app,
        ids.supplier_a,
        ids.qual_a1,
        "scan.png",
        "image/png",
        b"%PDF-1.7 disguised-as-png",
    )
    .await;
    assert_eq!(status, StatusCode::BAD_REQUEST, "实际体: {v}");
    assert_eq!(v["code"], "VALIDATION_ERROR");
    assert!(
        v["message"]
            .as_str()
            .unwrap_or_default()
            .contains("与文件实际内容不符"),
        "实际: {v}"
    );
}

/// 大小超过 5MB 上限显式拒绝（校验文案可见，不是裸 500）
#[tokio::test]
async fn upload_rejects_oversize_with_visible_message_not_bare_500() {
    let (app, _db, ids) = seeded_app().await;
    let mut data = b"%PDF-1.7 ".to_vec();
    data.resize(5 * 1024 * 1024 + 1, 0x20);
    let (status, v) = post_attachment(
        &app,
        ids.supplier_a,
        ids.qual_a2,
        "big.pdf",
        "application/pdf",
        &data,
    )
    .await;
    assert_eq!(
        status,
        StatusCode::BAD_REQUEST,
        "超限必须 400，实际: {status}"
    );
    // 大小/格式类拒绝按本仓边界归校验族（当前 handler 定案，见文件头）
    assert_eq!(v["code"], "VALIDATION_ERROR");
    assert!(
        v["message"]
            .as_str()
            .unwrap_or_default()
            .contains("不能超过 5MB"),
        "实际: {v}"
    );
}

/// 双键错配（资质属供应商 A，路径挂供应商 B）→ 上传被拒且文案用户可见
#[tokio::test]
async fn upload_rejects_mismatched_supplier_qualification_pair_with_visible_message() {
    let (app, db, ids) = seeded_app().await;
    let (status, v) = post_attachment(
        &app,
        ids.supplier_b,
        ids.qual_a1,
        "license.pdf",
        "application/pdf",
        b"%PDF-1.7 someone-else's-license",
    )
    .await;
    assert_eq!(status, StatusCode::BAD_REQUEST, "实际体: {v}");
    // 归属错配是业务拒绝（非输入格式问题）→ BUSINESS_ERROR 外显真实文案
    assert_eq!(v["code"], "BUSINESS_ERROR");
    assert_eq!(
        v["message"], "该资质不属于此供应商，无法操作附件，请刷新后重试",
        "business_displayable 必须外显真实文案，不得脱敏为「业务处理失败」"
    );
    assert_ne!(v["message"], "业务处理失败");
    // 错配请求在落盘 IO 之前就被拒绝：本人供应商（A）的资质也不得被动写入
    assert_eq!(attachment_path_of(&db, ids.qual_a1).await, None);
}

/// 读取端同双键校验：错配对 GET 同样可见文案拒绝（不泄露文件字节）
#[tokio::test]
async fn download_rejects_mismatched_pair_with_visible_message() {
    let (app, _db, ids) = seeded_app().await;
    let resp = app
        .clone()
        .oneshot(
            Request::builder()
                .method("GET")
                .uri(format!(
                    "/suppliers/{}/qualifications/{}/attachment",
                    ids.supplier_b, ids.qual_a1
                ))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    let status = resp.status();
    let bytes = axum::body::to_bytes(resp.into_body(), 4 * 1024 * 1024)
        .await
        .unwrap();
    let v: Value = serde_json::from_slice(&bytes).unwrap();
    assert_eq!(status, StatusCode::BAD_REQUEST, "实际体: {v}");
    assert_eq!(
        v["message"], "该资质不属于此供应商，无法操作附件，请刷新后重试",
        "实际: {v}"
    );
}

// =========================================================
// 2) 成功路径：服务端生成文件名 + attachment_path 受控 URL 回读 + 鉴权读取字节一致
// =========================================================

#[tokio::test]
async fn upload_success_persists_server_generated_url_and_download_serves_bytes() {
    let (app, db, ids) = seeded_app().await;
    let content = b"%PDF-1.4 real-license-content-bytes";
    let (status, v) = post_attachment(
        &app,
        ids.supplier_a,
        ids.qual_a1,
        "营业执照 扫描件 final.pdf",
        "application/pdf",
        content,
    )
    .await;
    assert_eq!(status, StatusCode::OK, "上传必须 200，实际体: {v}");
    assert_eq!(v["message"], "资质附件上传成功");
    // 出参 attachment_path == 服务端生成受控 URL（客户端原名不参与落盘名与 URL）
    let expected_url = format!(
        "/uploads/qualifications/{}_{}.pdf",
        ids.supplier_a, ids.qual_a1
    );
    assert_eq!(
        v["data"]["attachment_path"].as_str(),
        Some(expected_url.as_str()),
        "实际体: {v}"
    );
    // DB 真相层回读（不看响应体）
    assert_eq!(
        attachment_path_of(&db, ids.qual_a1).await.as_deref(),
        Some(expected_url.as_str())
    );

    // 磁盘 + 读回：GET 端点（同一鉴权链）返回字节与上传内容一致
    let (status, got, content_type) = get_attachment(&app, ids.supplier_a, ids.qual_a1).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(content_type.as_deref(), Some("application/pdf"));
    assert_eq!(got, content.as_slice(), "读回字节必须与上传内容一致");

    // 清理本用例生成的受控文件（失败仅打印，不影响断言结论）
    let p = std::path::Path::new("uploads/qualifications")
        .join(format!("{}_{}.pdf", ids.supplier_a, ids.qual_a1));
    if let Err(e) = std::fs::remove_file(&p) {
        eprintln!("测试清理失败（不影响断言）: {e}");
    }
}

/// 未上传附件的资质：GET 404（资源不存在语义），不是 500
#[tokio::test]
async fn download_without_attachment_returns_not_found() {
    let (app, _db, ids) = seeded_app().await;
    let (status, _body, _ct) = get_attachment(&app, ids.supplier_a, ids.qual_no_attach).await;
    assert_eq!(status, StatusCode::NOT_FOUND);
}
