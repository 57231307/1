//! ClamAV 扫描依赖故障契约锁：故障返回 503 SERVICE_UNAVAILABLE，不得谎报 500 DATABASE_ERROR
//!
//! 锁定的语义三分（对 `crm_handler.rs::scan_leads_for_viruses` 与
//! `supplier_handler.rs::scan_qualification_attachment_for_viruses`）：
//! ①未配置 / 不可达 / 非 2xx / 响应读取失败 → HTTP 503、code=SERVICE_UNAVAILABLE、
//!   公网脱敏文案「服务暂时不可用，请稍后重试」；且 fail-closed：拒绝发生在落盘/导入
//!   之前，磁盘与 DB 均无痕迹；
//! ②命中病毒 → 4xx（BUSINESS_ERROR + 用户可见拒绝文案，business_displayable 形态）；
//! ③扫描通过 → 200 正常落盘（证明 503 故障门没有把可用路径也拦死）。
//!
//! 覆盖策略（对齐 `contract_wave2_supplier_qualification_attachment_test.rs` 先例；
//! 表结构唯一来源 = backend/migration；双连接夹具）
//! - supplier 侧：`setup_test_db()` 真 PG（已迁移+清业务表）+ 真实 handler 端到端
//!   （tower oneshot + AuthContext 注入），断 HTTP 状态 + 信封 code/message
//!   + DB 回读 attachment_path + 磁盘文件存在性。供应商父行不自建：
//!   suppliers 属迁移播种且不清空的参照表（m0015 演示供应商，按 supplier_code
//!   稳定标识查取），资质行由用例自建（supplier_qualifications 属业务表，每次清空）。
//! - crm 侧：真实 import_leads handler 端到端，DB 用 `connect_empty_schema_db()`
//!   连的**已建库但未跑迁移**空库（无任何 crm 表）——若扫描失败后仍进入
//!   service.import_leads，会得到 DATABASE_ERROR/500 而不是 503+SERVICE_UNAVAILABLE，
//!   因此该断言本身就证明「拒绝先于导入」。
//! - 「不可达」用 loopback 上「刚 bind 过随即 close」的死端口（连接必被拒），
//!   「命中病毒 / 非 2xx / 扫描通过」用测试进程内的最小 TCP HTTP 假响应器充当扫描服务
//!   ——这是在**提供外部依赖的应答**（其契约面就是 HTTP 状态 + 响应体），不是 mock
//!   业务逻辑。夹具无条件跳过：环境异常（端口/线程失败）直接 panic 显式失败。

mod test_common;

use axum::{
    Router,
    body::Body,
    extract::State,
    http::{Request, StatusCode},
    middleware::{Next, from_fn_with_state},
    response::Response,
    routing::post,
};
use bingxi_backend::container::AppState;
use bingxi_backend::handlers::{crm_handler, supplier_handler};
use bingxi_backend::middleware::auth_context::AuthContext;
use bingxi_backend::models::{supplier, supplier_qualification};
use bingxi_backend::utils::error::gateway_msg;
use chrono::Utc;
use sea_orm::{ActiveModelTrait, ColumnTrait, DatabaseConnection, EntityTrait, QueryFilter};
use serde_json::Value;
use tower::ServiceExt;

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

/// 取迁移播种且不清空的参照供应商（m0015 演示供应商，按稳定标识 supplier_code 查取），
/// 并自建其下 5 条资质行（supplier_qualifications 属业务表，每次 setup_test_db 清空，
/// 显式 ID 911..915 稳定可断言）。
///
/// 资质 ID 段 911..915 专用：与 wave2 附件测试的落盘名错开，两个测试二进制并行时
/// 互不踩踏 uploads/qualifications 下的文件。
async fn seed_qualifications(db: &DatabaseConnection) -> i32 {
    let parent = supplier::Entity::find()
        .filter(supplier::Column::SupplierCode.eq("SUP-DEMO-FAB-01"))
        .one(db)
        .await
        .expect("夹具：查询迁移播种参照供应商失败")
        .expect("夹具：m0015 演示供应商 SUP-DEMO-FAB-01 应存在（suppliers 为不清空的参照表）");
    let sid = parent.id;

    // created_at/updated_at 列为 DateTimeWithTimeZone（= DateTime<FixedOffset>），
    // 仓内惯用 `Utc::now().into()`（同 src/handlers/import_export_handler.rs:811）
    let now: chrono::DateTime<chrono::FixedOffset> = Utc::now().into();
    let far = chrono::NaiveDate::from_ymd_opt(2030, 12, 31).unwrap();
    for qid in [911i32, 912, 913, 914, 915] {
        supplier_qualification::ActiveModel {
            id: sea_orm::ActiveValue::Set(qid),
            supplier_id: sea_orm::ActiveValue::Set(sid),
            qualification_name: sea_orm::ActiveValue::Set(format!("营业执照{qid}")),
            qualification_type: sea_orm::ActiveValue::Set("证照".to_string()),
            qualification_no: sea_orm::ActiveValue::Set(format!("NO-{qid}")),
            issuing_authority: sea_orm::ActiveValue::Set("市场监管局".to_string()),
            issue_date: sea_orm::ActiveValue::Set(
                chrono::NaiveDate::from_ymd_opt(2024, 1, 1).unwrap(),
            ),
            valid_until: sea_orm::ActiveValue::Set(far),
            attachment_path: sea_orm::ActiveValue::Set(None),
            need_annual_check: sea_orm::ActiveValue::Set(false),
            annual_check_record: sea_orm::ActiveValue::Set(None),
            is_expired: sea_orm::ActiveValue::Set(false),
            remarks: sea_orm::ActiveValue::Set(None),
            created_at: sea_orm::ActiveValue::Set(now),
            updated_at: sea_orm::ActiveValue::Set(now),
        }
        .insert(db)
        .await
        .unwrap_or_else(|e| panic!("夹具：资质 {qid} 种子写入失败: {e}"));
    }
    sid
}

/// supplier 资质附件端点挂在真库（已迁移+清业务表）app 上（真实 handler 端到端）。
fn build_supplier_app(db: &DatabaseConnection) -> Router {
    let state = AppState {
        db: std::sync::Arc::new(db.clone()),
        ..Default::default()
    };
    Router::new()
        .route(
            "/suppliers/{id}/qualifications/{qualification_id}/attachment",
            post(supplier_handler::upload_supplier_qualification_attachment),
        )
        .with_state(state)
        .layer(from_fn_with_state(make_auth(100), inject_auth))
}

/// crm 线索导入端点挂在**空 schema 库**（双连接夹具，`connect_empty_schema_db`）：
/// 库里没有任何 crm 业务表——扫描失败若被吞、流程若走到 import_leads，会得到
/// DATABASE_ERROR/500 而不是 503，断言因此能自证「拒绝先于导入」。
fn build_crm_app(db: &DatabaseConnection) -> Router {
    let state = AppState {
        db: std::sync::Arc::new(db.clone()),
        ..Default::default()
    };
    Router::new()
        .route("/crm/leads/import", post(crm_handler::import_leads))
        .with_state(state)
        .layer(from_fn_with_state(make_auth(100), inject_auth))
}

fn multipart_body(
    boundary: &str,
    field_name: &str,
    file_name: &str,
    content_type: &str,
    data: &[u8],
) -> Vec<u8> {
    let mut body = Vec::new();
    body.extend_from_slice(
        format!(
            "--{}\r\nContent-Disposition: form-data; name=\"{}\"; filename=\"{}\"\r\nContent-Type: {}\r\n\r\n",
            boundary, field_name, file_name, content_type
        )
        .as_bytes(),
    );
    body.extend_from_slice(data);
    body.extend_from_slice(format!("\r\n--{}--\r\n", boundary).as_bytes());
    body
}

async fn post_multipart(
    app: &Router,
    uri: &str,
    field_name: &str,
    file_name: &str,
    data: &[u8],
) -> (StatusCode, Value) {
    let boundary = "----bxScanFailureBoundary";
    let body = multipart_body(
        boundary,
        field_name,
        file_name,
        "application/octet-stream",
        data,
    );
    let resp = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri(uri)
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

async fn attachment_path_of(db: &DatabaseConnection, qid: i32) -> Option<String> {
    supplier_qualification::Entity::find_by_id(qid)
        .one(db)
        .await
        .unwrap()
        .unwrap_or_else(|| panic!("资质 {qid} 应存在"))
        .attachment_path
}

fn disk_path_of(sid: i32, qid: i32) -> std::path::PathBuf {
    std::path::Path::new("uploads/qualifications").join(format!("{sid}_{qid}.pdf"))
}

/// 503 故障族的统一断言：状态 503、code=SERVICE_UNAVAILABLE、公网脱敏文案，
/// 且出参任何字段不得外泄扫描服务 URL/端口/配置键名/内部机制细节。
fn assert_service_unavailable_envelope(v: &Value, ctx: &str) {
    assert_eq!(v["code"], "SERVICE_UNAVAILABLE", "{ctx}: 实际体 {v}");
    assert_eq!(
        v["message"],
        gateway_msg::SERVICE_UNAVAILABLE_PUBLIC,
        "{ctx}: 503 出参必须是脱敏公网文案，实际体 {v}"
    );
    let serialized = v.to_string();
    for forbidden in ["CLAMAV", "ClamAV", "127.0.0.1", "http://", "URL"] {
        assert!(
            !serialized.contains(forbidden),
            "{ctx}: 出参不得外泄内部细节（含 {forbidden:?}），实际体 {v}"
        );
    }
}

/// loopback 死端口：bind 后立即 close，后续连接必被拒（不依赖外网）。
fn dead_port() -> u16 {
    let l = std::net::TcpListener::bind("127.0.0.1:0").expect("死端口 bind 失败");
    let port = l.local_addr().expect("死端口 local_addr 失败").port();
    drop(l);
    port
}

/// 测试进程内的最小 HTTP 响应器，充当 ClamAV REST 依赖（提供外部依赖的应答面：
/// HTTP 状态行 + 响应体；扫描判定逻辑仍走被测 handler 真实代码）。
/// 每个连接：读取请求（read_timeout 内读完即回 200 状态行 + 指定 body），Connection: close。
fn spawn_fake_scan_server(status_line: &'static str, body: &'static str) -> u16 {
    let listener = std::net::TcpListener::bind("127.0.0.1:0").expect("假扫描器 bind 失败");
    let port = listener
        .local_addr()
        .expect("假扫描器 local_addr 失败")
        .port();
    std::thread::spawn(move || {
        use std::io::{Read, Write};
        for stream in listener.incoming() {
            let Ok(mut stream) = stream else { continue };
            let _ = stream.set_read_timeout(Some(std::time::Duration::from_millis(200)));
            // 吞掉请求（小文件，一两个分段即到；读到超时/EOF 即视为读完）
            let mut chunk = [0u8; 1024];
            while let Ok(n) = stream.read(&mut chunk) {
                if n == 0 {
                    break;
                }
            }
            let resp = format!(
                "HTTP/1.1 {}\r\nContent-Type: text/plain\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
                status_line,
                body.len(),
                body
            );
            let _ = stream.write_all(resp.as_bytes());
            let _ = stream.flush();
        }
    });
    port
}

/// 全部场景在**一个测试函数内串行**执行：CLAMAV_* 是进程级环境变量，
/// 本文件的断言场景必须互不串扰；tokio::test 默认 current_thread，
/// 读写环境变量的都是主线程（假扫描器线程不触碰 env），unsafe set_var 的前提成立。
#[tokio::test]
async fn scan_dependency_failures_return_503_never_500_and_never_touch_disk() {
    // 双连接：supplier 侧要建资质种子 → 真 PG 已迁移库（setup_test_db）；
    // crm 侧要"库里没有业务表"这一负前提 → 已建库未跑迁移的空 schema 库。
    let db = test_common::setup_test_db().await;
    let sid = seed_qualifications(&db).await;
    let app = build_supplier_app(&db);
    let crm_db = test_common::connect_empty_schema_db().await;
    let crm_app = build_crm_app(&crm_db);
    let clean_pdf = b"%PDF-1.7 scan-failure-fixture";

    // ---------- ①a 扫描服务不可达 → 503 + 文件未落盘 ----------
    let dead = dead_port();
    // SAFETY: 本测试为 current_thread 单线程执行、串行改环境；假扫描器线程不读取 env。
    unsafe {
        std::env::set_var("CLAMAV_ENABLED", "true");
        std::env::set_var("CLAMAV_URL", format!("http://127.0.0.1:{dead}"));
    }
    let (status, v) = post_multipart(
        &app,
        &format!("/suppliers/{sid}/qualifications/911/attachment"),
        "file",
        "license.pdf",
        clean_pdf,
    )
    .await;
    assert_eq!(
        status,
        StatusCode::SERVICE_UNAVAILABLE,
        "不可达必须 503，实际体 {v}"
    );
    assert_ne!(
        status,
        StatusCode::INTERNAL_SERVER_ERROR,
        "严禁再谎报 500（不可达）"
    );
    assert_service_unavailable_envelope(&v, "扫描服务不可达(supplier)");
    assert_eq!(
        attachment_path_of(&db, 911).await,
        None,
        "503 拒绝不得写 attachment_path"
    );
    assert!(
        !disk_path_of(sid, 911).exists(),
        "503 拒绝不得落盘（拒绝必须先于写盘）"
    );

    // crm 侧同故障：空 schema 库无任何 crm 表 + 503/SERVICE_UNAVAILABLE
    // ⇒ 拒绝先于 import_leads（未落库）
    let xlsx_bytes = {
        let mut d = vec![0x50u8, 0x4B, 0x03, 0x04];
        d.extend_from_slice(b"fake-zip-leads-payload");
        d
    };
    let (status, v) = post_multipart(
        &crm_app,
        "/crm/leads/import",
        "file",
        "leads.xlsx",
        &xlsx_bytes,
    )
    .await;
    assert_eq!(
        status,
        StatusCode::SERVICE_UNAVAILABLE,
        "不可达必须 503（crm），实际体 {v}"
    );
    assert_service_unavailable_envelope(&v, "扫描服务不可达(crm)");

    // ---------- ①b 未配置（键缺失）/ 配置为空 → 503 而不是 500 ----------
    unsafe { std::env::remove_var("CLAMAV_URL") }
    let (status, v) = post_multipart(
        &app,
        &format!("/suppliers/{sid}/qualifications/912/attachment"),
        "file",
        "license.pdf",
        clean_pdf,
    )
    .await;
    assert_eq!(
        status,
        StatusCode::SERVICE_UNAVAILABLE,
        "未配置必须 503，实际体 {v}"
    );
    assert_ne!(
        status,
        StatusCode::INTERNAL_SERVER_ERROR,
        "严禁再谎报 500（未配置）"
    );
    assert_service_unavailable_envelope(&v, "扫描服务未配置(supplier)");
    assert_eq!(attachment_path_of(&db, 912).await, None);
    assert!(!disk_path_of(sid, 912).exists(), "未配置拒绝不得落盘");

    unsafe { std::env::set_var("CLAMAV_URL", "") }
    let (status, v) = post_multipart(
        &crm_app,
        "/crm/leads/import",
        "file",
        "leads.xlsx",
        &xlsx_bytes,
    )
    .await;
    assert_eq!(
        status,
        StatusCode::SERVICE_UNAVAILABLE,
        "配置为空必须 503（crm），实际体 {v}"
    );
    assert_service_unavailable_envelope(&v, "扫描服务配置为空(crm)");

    // ---------- ①c 扫描服务返回非 2xx → 503（fail-closed，不放行） ----------
    let down_port = spawn_fake_scan_server("503 Service Unavailable", "scanner down");
    unsafe { std::env::set_var("CLAMAV_URL", format!("http://127.0.0.1:{down_port}")) }
    let (status, v) = post_multipart(
        &app,
        &format!("/suppliers/{sid}/qualifications/914/attachment"),
        "file",
        "license.pdf",
        clean_pdf,
    )
    .await;
    assert_eq!(
        status,
        StatusCode::SERVICE_UNAVAILABLE,
        "非 2xx 必须 503，实际体 {v}"
    );
    assert_service_unavailable_envelope(&v, "扫描服务非 2xx(supplier)");
    assert_eq!(attachment_path_of(&db, 914).await, None);
    assert!(!disk_path_of(sid, 914).exists(), "非 2xx 拒绝不得落盘");

    // ---------- ② 命中病毒 → 4xx + 外显拒绝文案（不是 503/500） ----------
    let virus_port = spawn_fake_scan_server("200 OK", "stream: Win.Test.EICAR_HDB-1 FOUND");
    unsafe { std::env::set_var("CLAMAV_URL", format!("http://127.0.0.1:{virus_port}")) }
    let (status, v) = post_multipart(
        &app,
        &format!("/suppliers/{sid}/qualifications/913/attachment"),
        "file",
        "license.pdf",
        clean_pdf,
    )
    .await;
    assert!(
        status.is_client_error(),
        "命中病毒必须 4xx，实际 {status}，体 {v}"
    );
    assert_eq!(
        v["code"], "BUSINESS_ERROR",
        "命中病毒走 business_displayable，实际体 {v}"
    );
    assert_eq!(
        v["message"], "资质附件未通过病毒扫描，已被拒绝，请向供应商索取干净的文件后重新上传",
        "命中病毒必须外显用户可见拒绝文案，实际体 {v}"
    );
    assert_eq!(
        attachment_path_of(&db, 913).await,
        None,
        "命中病毒不得写 attachment_path"
    );
    assert!(!disk_path_of(sid, 913).exists(), "命中病毒不得落盘");

    // ---------- ③ 扫描通过 → 200 正常落盘（503 改造不得误伤可用路径） ----------
    let ok_port = spawn_fake_scan_server("200 OK", "stream: OK");
    unsafe { std::env::set_var("CLAMAV_URL", format!("http://127.0.0.1:{ok_port}")) }
    let (status, v) = post_multipart(
        &app,
        &format!("/suppliers/{sid}/qualifications/915/attachment"),
        "file",
        "license.pdf",
        clean_pdf,
    )
    .await;
    assert_eq!(status, StatusCode::OK, "扫描通过必须 200，实际体 {v}");
    assert_eq!(
        attachment_path_of(&db, 915).await.as_deref(),
        Some(format!("/uploads/qualifications/{sid}_915.pdf").as_str()),
        "扫描通过应正常落库"
    );

    // 清理：成功用例的真实落盘文件 + 本测试设置的环境变量（失败显式打印，不静默）
    if let Err(e) = std::fs::remove_file(disk_path_of(sid, 915)) {
        eprintln!("测试清理落盘文件失败（不影响断言结论）: {e}");
    }
    unsafe {
        std::env::remove_var("CLAMAV_ENABLED");
        std::env::remove_var("CLAMAV_URL");
    }
}
