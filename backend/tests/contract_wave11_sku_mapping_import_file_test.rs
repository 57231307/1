//! SKU 对照批量导入（POST /purchase/sku-mappings/import）——multipart 文件链真库契约锁
//!
//! ## 被测缺陷（修复前必然失败的形态）
//! 前端 `importSkuMappings` 用 FormData append `file` 并显式
//! `Content-Type: multipart/form-data`（frontend/src/api/sku-mapping.ts），而后端旧签名是
//! `Json<ImportMappingsRequest>`：axum 的 Json 提取器对该请求直接在提取层拒绝，功能整体
//! 不可用且既有 e2e 无任何用例触碰这条链（"e2e 测不到的功能缺陷"族）。本锁把修好后的
//! 真实链路钉死：**multipart 上传 → 服务端按扩展名/magic 解析 CSV/xlsx → 权威表头映射
//! → ImportMappingRow → 复用 SkuMappingService::import_batch 落库**。
//!
//! ## 四条锁
//! 1. CSV 文件经真实 handler 路径导入 2 行（一行全字段、一行仅必填列），断言
//!    HTTP 200 + 成功信封 code=200 + data 计数（total/success/error）+ 裸 SQL 回读
//!    product_supplier_mappings 逐列（外键解析、numeric 数值、created_by 归属）；
//! 2. xlsx 文件（用既有导出工具 utils/xlsx_export.rs::build_xlsx 内存生成，不引新依赖）
//!    走同一 handler 导入成功并回读，钉住「按扩展名/magic 分流」与产品导入同范式；
//! 3. 形态类拒绝全部 400 且零落库：表头缺必需列 / 表头含未知列 / 仅表头无数据行 /
//!    单元格类型解析失败。族别必须 VALIDATION_ERROR（拒绝发生在 import_batch 之前，
//!    出现 BUSINESS_ERROR/DATABASE_ERROR 族即路径漂移）；参数校验类允许点名列名
//!    （error.rs 模块文档口径），但 message 不得回显表名/SQL/SQLSTATE 等内部细节；
//! 4. 旧 JSON 请求体（`{rows:[...]}` + application/json）必须被 Multipart 提取层拒掉
//!    （axum 0.8.1 `InvalidBoundary` = 400，registry 源码 axum-0.8.1/src/extract/multipart.rs
//!    `define_rejection! { #[status = BAD_REQUEST] ... }` 实证；非 415，提取器错误先于
//!    handler、出参是 axum 内建文本而非 AppError 信封，与产品导入端点同形），
//!    防止端点被悄悄改回 JSON 契约造成前后端再次脱钩。
//!
//! ## 期望值来源（防恒真/防第二套列清单）
//! 导入文件的表头列名与必需性**从 handler 源码解析权威映射表
//! `SKU_MAPPING_IMPORT_COLUMNS` 得到**（锚点缺失/常量不可解析即 panic 点名），测试不手抄
//! 第二套列清单；单元格取值按列名硬绑定到本夹具种下的真实父级行（products/product_colors
//! 自建 + 迁移种子 m0015 的演示供应商目录 SUP-DEMO-FAB-01/FAB-P001/FAB-P002/PC-A01），
//! 权威表若改名，取值生成器 panic，不会静默适配。
//! 父级供应商目录属封存参照表（夹具不清空），查不到种子行直接 panic——引用链前提失守
//! 必须点名，不许把「测不了」塌成「测过了」。
//!
//! ## 夹具与 id 带
//! 走既有 `setup_test_db()` 链（连已迁移 PostgreSQL、逐用例 TRUNCATE 业务表、保留迁移
//! 种子参照表；缺 TEST_DATABASE_URL 直接 panic，不静默降级 sqlite）。nextest 下真库用例
//! 串行执行。本文件独占 993_0xx 段（990_x/991_x/992_x/994_x/999_x 已被其它锁占用，不碰）；
//! users 不建：product_supplier_mappings.created_by 列无外键（m0008 建表原文实证），
//! 直接以操作人 id 断言归属即可。

mod test_common;

use axum::{
    Router,
    body::Body,
    extract::State,
    http::{Method, Request, StatusCode},
    middleware::{Next, from_fn_with_state},
    response::Response,
    routing::post,
};
use bingxi_backend::container::AppState;
use bingxi_backend::handlers::sku_mapping_handler;
use bingxi_backend::middleware::auth_context::AuthContext;
use bingxi_backend::utils::xlsx_export::{XlsxTable, build_xlsx};
use sea_orm::{ConnectionTrait, DatabaseConnection, DbBackend, Statement};
use serde_json::{Value, json};
use std::sync::Arc;
use test_common::setup_test_db;
use tower::ServiceExt;

const HANDLER_REL: &str = "src/handlers/sku_mapping_handler.rs";

/// 本文件专属 id 带（993_0xx，见文件头「夹具与 id 带」）
const OPERATOR_ID: i32 = 993_001;
const PRODUCT_A_ID: i32 = 993_011;
const PRODUCT_B_ID: i32 = 993_012;
const COLOR_A_ID: i32 = 993_021;

/// 本文件自建的父级业务编码（products/product_colors 属逐用例 TRUNCATE 的业务表）
const PRODUCT_A_CODE: &str = "SKU-IMP-P1";
const PRODUCT_B_CODE: &str = "SKU-IMP-P2";
const COLOR_A_NO: &str = "C-RED-01";

/// 迁移种子 m0015 的演示供应商目录（suppliers/supplier_products/supplier_product_colors
/// 属封存参照表，只引用不自建；按稳定业务编码引用，id 运行时查库解析，不硬编码）
const SEED_SUPPLIER_CODE: &str = "SUP-DEMO-FAB-01";
const SEED_SP_P001: &str = "FAB-P001";
const SEED_SP_P002: &str = "FAB-P002";
const SEED_SPC_A01: &str = "PC-A01";

/// 手工构造 multipart 请求体的 boundary（与请求头声明一致；字段名 file 对齐前端 FormData）
const BOUNDARY: &str = "----bxSkuMappingImportTestBoundary";

/// 表头级/单元格级拒绝共用的机器族别（参数校验族；断言族别与列名点名，不断文案原文）
const VALIDATION_FAMILY: &str = "VALIDATION_ERROR";

// ---------------------------------------------------------------------------
// 助手：裸 SQL / 种子 / 权威表解析
// ---------------------------------------------------------------------------

fn pg_stmt(sql: &str) -> Statement {
    Statement::from_sql_and_values(
        DbBackend::Postgres,
        sql.to_string(),
        Vec::<sea_orm::Value>::new(),
    )
}

async fn exec(db: &DatabaseConnection, sql: &str) {
    db.execute_raw(pg_stmt(sql))
        .await
        .unwrap_or_else(|e| panic!("种子 SQL 执行失败: {e}\nSQL: {sql}"));
}

async fn one_count(db: &DatabaseConnection, sql: &str) -> i64 {
    let row = db
        .query_one_raw(pg_stmt(sql))
        .await
        .unwrap_or_else(|e| panic!("计数查询失败: {e}\nSQL: {sql}"));
    let r = row.unwrap_or_else(|| panic!("计数查询没有返回行: SQL: {sql}"));
    r.try_get::<i64>("", "n")
        .unwrap_or_else(|e| panic!("计数列解码失败: {e}\nSQL: {sql}"))
}

/// 封存参照行必须存在：查不到即 panic 点名（父级前提失守不许塌成空集合）。
async fn require_seed_id(db: &DatabaseConnection, sql: &str, label: &str) -> i32 {
    let row = db
        .query_one_raw(pg_stmt(sql))
        .await
        .unwrap_or_else(|e| panic!("查询迁移种子 {label} 失败: {e}"));
    let r = row.unwrap_or_else(|| {
        panic!("迁移种子 {label} 不存在——m0015 供应商目录参照链漂移，本锁父级前提失守\nSQL: {sql}")
    });
    r.try_get::<i32>("", "id")
        .unwrap_or_else(|e| panic!("种子 {label} 的 id 解码失败: {e}"))
}

fn read_handler_source() -> String {
    let path = format!("{}/{}", env!("CARGO_MANIFEST_DIR"), HANDLER_REL);
    std::fs::read_to_string(&path).unwrap_or_else(|e| panic!("读取 {path} 失败: {e}"))
}

/// 从 handler 源码解析权威映射表 `SKU_MAPPING_IMPORT_COLUMNS` → Vec<(列名, 必需)>。
///
/// 期望值唯一来源是源码权威表本身（与迁移原文解析锁同一手法）：锚点缺失、条目括号
/// 不闭合、引用的 `COL_*` 常量定义不可解析——一律 panic 点名，禁止静默产出残缺清单
/// 让表头校验退化成「两边都空」。
fn parse_authoritative_columns(src: &str) -> Vec<(String, bool)> {
    let anchor = "const SKU_MAPPING_IMPORT_COLUMNS: &[(&str, bool)] = &[";
    let start = src.find(anchor).unwrap_or_else(|| {
        panic!("sku_mapping_handler.rs 缺少权威表锚点 {anchor:?}——权威表形态漂移")
    });
    let rest = &src[start + anchor.len()..];
    let end = rest
        .find("];")
        .unwrap_or_else(|| panic!("权威表字面量数组未闭合（找不到 `];`）"));
    let body = &rest[..end];

    let mut columns: Vec<(String, bool)> = Vec::new();
    let mut cursor = body;
    while let Some(open) = cursor.find("(COL_") {
        cursor = &cursor[open + 1..];
        let close = cursor
            .find(')')
            .unwrap_or_else(|| panic!("权威表条目右括号未闭合: {cursor}"));
        let entry = &cursor[..close];
        cursor = &cursor[close..];
        let mut parts = entry.split(',');
        let const_name = parts
            .next()
            .unwrap_or_else(|| panic!("权威表条目缺少列名常量引用: {entry}"))
            .trim();
        let required_token = parts
            .next()
            .unwrap_or_else(|| panic!("权威表条目 {const_name} 缺少必需性标志"))
            .trim();
        let required = match required_token {
            "true" => true,
            "false" => false,
            other => panic!("权威表条目 {const_name} 的必需性标志不是字面量 true/false: {other}"),
        };
        let decl_anchor = format!("const {const_name}: &str = \"");
        let dstart = src.find(&decl_anchor).unwrap_or_else(|| {
            panic!("权威表引用了列名常量 {const_name}，但源码找不到其定义（列名真源分裂）")
        });
        let drest = &src[dstart + decl_anchor.len()..];
        let dend = drest
            .find("\";")
            .unwrap_or_else(|| panic!("列名常量 {const_name} 的定义字面量未闭合"));
        columns.push((drest[..dend].to_string(), required));
    }
    assert!(
        !columns.is_empty(),
        "权威表解析出空列清单——解析失守，拒绝继续比对"
    );
    columns
}

/// 按权威列名生成本夹具的一种数据行（全字段行 idx=0 / 仅必填列行 idx=1）。
///
/// 取值与列名硬绑定：权威表若新增/改名列，命中 `_ => panic` 分支点名——测试不许静默
/// 适配第二套列名，改名要么同步更新本夹具（对着真实语义改），要么红。
fn file_row_for(columns: &[(String, bool)], idx: usize) -> Vec<String> {
    let value = |column: &str| -> String {
        match (column, idx) {
            ("product_code", 0) => PRODUCT_A_CODE.to_string(),
            ("product_code", _) => PRODUCT_B_CODE.to_string(),
            ("color_no", 0) => COLOR_A_NO.to_string(),
            ("supplier_code", _) => SEED_SUPPLIER_CODE.to_string(),
            ("supplier_product_code", 0) => SEED_SP_P001.to_string(),
            ("supplier_product_code", _) => SEED_SP_P002.to_string(),
            ("supplier_color_no", 0) => SEED_SPC_A01.to_string(),
            ("supplier_price", 0) => "12.50".to_string(),
            ("min_order_quantity", 0) => "30".to_string(),
            ("lead_time", 0) => "7".to_string(),
            ("is_primary", 0) => "true".to_string(),
            ("priority", 0) => "1".to_string(),
            ("is_enabled", 0) => "true".to_string(),
            ("remarks", 0) => "对照导入锁行A".to_string(),
            (_, 1) => String::new(),
            (other, _) => panic!(
                "权威表列 {other} 未在测试取值器中登记——列集合已变更，须显式对齐真实语义后更新夹具"
            ),
        }
    };
    columns.iter().map(|(c, _)| value(c)).collect()
}

fn csv_bytes(columns: &[(String, bool)], rows: &[Vec<String>]) -> Vec<u8> {
    let headers: Vec<&str> = columns.iter().map(|(c, _)| c.as_str()).collect();
    let mut out = headers.join(",").into_bytes();
    out.push(b'\n');
    for row in rows {
        out.extend_from_slice(row.join(",").as_bytes());
        out.push(b'\n');
    }
    out
}

/// 用既有导出工具生成真实 xlsx 字节（与产品侧模板同一 writer，不引入新依赖）
fn xlsx_bytes(columns: &[(String, bool)], rows: &[Vec<String>]) -> Vec<u8> {
    let table = XlsxTable {
        sheet_name: "SKU 对照导入".to_string(),
        headers: columns.iter().map(|(c, _)| c.clone()).collect(),
        rows: rows.to_vec(),
    };
    build_xlsx(&table).expect("既有导出工具 build_xlsx 生成 xlsx 字节失败")
}

// ---------------------------------------------------------------------------
// HTTP 形态夹具（路由形状与真实挂载一致：routes/purchase.rs 的 /sku-mappings/import）
// ---------------------------------------------------------------------------

fn make_auth() -> AuthContext {
    AuthContext {
        user_id: OPERATOR_ID,
        username: "skum_w11_import_op".to_string(),
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

/// 业务父级自建行（products/product_colors 属可清空业务表；显式 DELETE 让种子幂等，
/// 即便单独重跑本用例也不会因残留撞主键）
async fn seed_products(db: &DatabaseConnection) {
    exec(
        db,
        &format!("DELETE FROM product_colors WHERE id={COLOR_A_ID}"),
    )
    .await;
    exec(
        db,
        &format!("DELETE FROM products WHERE id IN ({PRODUCT_A_ID},{PRODUCT_B_ID})"),
    )
    .await;
    exec(
        db,
        &format!(
            "INSERT INTO products (id, code, name) VALUES \
             ({PRODUCT_A_ID},'{PRODUCT_A_CODE}','对照导入锁产品甲'),\
             ({PRODUCT_B_ID},'{PRODUCT_B_CODE}','对照导入锁产品乙')"
        ),
    )
    .await;
    exec(
        db,
        &format!(
            "INSERT INTO product_colors (id, product_id, color_no, color_name) VALUES \
             ({COLOR_A_ID},{PRODUCT_A_ID},'{COLOR_A_NO}','导入锁色红')"
        ),
    )
    .await;
}

async fn app_with_db() -> (Arc<DatabaseConnection>, Router) {
    let db = Arc::new(setup_test_db().await);
    seed_products(&db).await;
    let state = AppState {
        db: db.clone(),
        ..Default::default()
    };
    let app = Router::new()
        .route(
            "/sku-mappings/import",
            post(sku_mapping_handler::import_mappings),
        )
        .with_state(state)
        .layer(from_fn_with_state(make_auth(), inject_auth));
    (db, app)
}

fn multipart_body(file_name: &str, data: &[u8]) -> Vec<u8> {
    let mut body = Vec::new();
    body.extend_from_slice(
        format!(
            "--{BOUNDARY}\r\nContent-Disposition: form-data; name=\"file\"; filename=\"{file_name}\"\r\nContent-Type: application/octet-stream\r\n\r\n"
        )
        .as_bytes(),
    );
    body.extend_from_slice(data);
    body.extend_from_slice(format!("\r\n--{BOUNDARY}--\r\n").as_bytes());
    body
}

/// 上传文件走真实 handler 路径，取 (状态码, JSON 信封)。
/// 成功/参数校验拒绝都必须是 AppError/ApiResponse 的 JSON 信封；
/// 若 handler 之外的形态冒出非 JSON 体（如被改回 JSON 提取器后的原始 415 文本），
/// 这里保留原文供断言侧点名，不静默。
async fn upload(app: &Router, file_name: &str, data: &[u8]) -> (StatusCode, Value) {
    let resp = app
        .clone()
        .oneshot(
            Request::builder()
                .method(Method::POST)
                .uri("/sku-mappings/import")
                .header(
                    "content-type",
                    format!("multipart/form-data; boundary={BOUNDARY}"),
                )
                .body(Body::from(multipart_body(file_name, data)))
                .unwrap(),
        )
        .await
        .unwrap();
    read_status_and_json(resp).await
}

async fn read_status_and_json(resp: Response) -> (StatusCode, Value) {
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

async fn count_mappings(db: &DatabaseConnection) -> i64 {
    one_count(
        db,
        &format!(
            "SELECT COUNT(*) AS n FROM product_supplier_mappings WHERE product_id IN ({PRODUCT_A_ID},{PRODUCT_B_ID})"
        ),
    )
    .await
}

/// 结构列回读（外键解析结果、可选列 NULL 形态、创建归属——断库内真值，不依赖响应 JSON）
async fn fetch_landed(db: &DatabaseConnection, product_id: i32) -> Value {
    let row = db
        .query_one_raw(Statement::from_sql_and_values(
            DbBackend::Postgres,
            "SELECT product_color_id, supplier_id, supplier_product_id, supplier_product_color_id, lead_time, is_primary, priority, is_enabled, remarks, created_by FROM product_supplier_mappings WHERE product_id=$1".to_string(),
            vec![product_id.into()],
        ))
        .await
        .unwrap_or_else(|e| panic!("回读 product_id={product_id} 的映射行失败: {e}"));
    let r = row.unwrap_or_else(|| panic!("product_supplier_mappings 里没有 product_id={product_id} 的行（导入声称成功但没落库？）"));
    json!({
        "product_color_id": r.try_get::<Option<i32>>("", "product_color_id").unwrap(),
        "supplier_id": r.try_get::<i32>("", "supplier_id").unwrap(),
        "supplier_product_id": r.try_get::<i32>("", "supplier_product_id").unwrap(),
        "supplier_product_color_id": r.try_get::<Option<i32>>("", "supplier_product_color_id").unwrap(),
        "lead_time": r.try_get::<Option<i32>>("", "lead_time").unwrap(),
        "is_primary": r.try_get::<bool>("", "is_primary").unwrap(),
        "priority": r.try_get::<i32>("", "priority").unwrap(),
        "is_enabled": r.try_get::<bool>("", "is_enabled").unwrap(),
        "remarks": r.try_get::<Option<String>>("", "remarks").unwrap(),
        "created_by": r.try_get::<Option<i32>>("", "created_by").unwrap(),
    })
}

/// numeric 列按数值谓词计数（::text 渲染会随 PG 版本补 scale，不拿文本当契约）
async fn count_row_a_decimals(db: &DatabaseConnection, supplier_id: i32) -> i64 {
    one_count(
        db,
        &format!(
            "SELECT COUNT(*) AS n FROM product_supplier_mappings \
              WHERE product_id={PRODUCT_A_ID} AND supplier_id={supplier_id} \
                AND supplier_price = 12.50 AND min_order_quantity = 30"
        ),
    )
    .await
}

/// 拒绝类锁的族别与脱敏断言：400 + VALIDATION_ERROR + 点名列名（参数校验类允许外显）
/// + 不回显内部细节（表名/SQL 关键词/SQLSTATE）。
fn assert_reject_v400(v: &Value, status: StatusCode, expect_column_named: Option<&str>) {
    assert_eq!(
        status,
        StatusCode::BAD_REQUEST,
        "形态类拒绝必须 400（不得 500/200），实得信封: {v}"
    );
    assert_eq!(
        v["code"], VALIDATION_FAMILY,
        "拒绝必须归 VALIDATION_ERROR 族（文件形态校验先于任何落库；\
         出现 BUSINESS_ERROR/DATABASE_ERROR 族说明校验位置或构造族别漂移），实得: {v}"
    );
    let message = v["message"].as_str().unwrap_or_default();
    assert!(
        !message.contains("product_supplier_mappings")
            && !message.contains("INSERT")
            && !message.contains("SQLSTATE")
            && !message.contains("23505"),
        "参数校验类文案允许点名用户提交的列名，但不得回显表名/SQL/SQLSTATE 等内部细节，实得: {v}"
    );
    if let Some(column) = expect_column_named {
        assert!(
            message.contains(column),
            "缺列/未知列/类型错误必须点名涉事列名（fail-visible，不许只报『导入失败』含糊），实得: {v}"
        );
    }
}

// ---------------------------------------------------------------------------
// 1. CSV 全链成功：内存生成 csv 字节 → 真实 handler → 200 信封计数 + 裸 SQL 逐列回读
//    （外键按编码解析成真实 id、可选列 NULL 形态、numeric 数值、created_by 归属）。
//    改坏什么必红：端点被改回 JSON 提取器（上传直接非 200）；权威表与行构造脱钩
//    （缺列误判/取值错位）；import_batch 引用解析漂移（回读 id 不等）；
//    出参计数键漂移（data.total_count/success_count/error_count 断言红）。
// ---------------------------------------------------------------------------
#[tokio::test]
async fn csv_upload_imports_via_real_handler_and_lands_resolved_rows() {
    let (db, app) = app_with_db().await;
    let columns = parse_authoritative_columns(&read_handler_source());
    let rows = vec![file_row_for(&columns, 0), file_row_for(&columns, 1)];
    let (status, v) = upload(&app, "sku_import.csv", &csv_bytes(&columns, &rows)).await;

    assert_eq!(
        status,
        StatusCode::OK,
        "合法 CSV 上传必须 200，实得信封: {v}"
    );
    assert_eq!(v["code"], 200, "成功信封 code 必须 200，实得 {v}");
    let data = &v["data"];
    assert_eq!(
        data["total_count"],
        json!(2),
        "total_count 契约键，实得 {v}"
    );
    assert_eq!(
        data["success_count"],
        json!(2),
        "success_count 契约键，实得 {v}"
    );
    assert_eq!(
        data["error_count"],
        json!(0),
        "error_count 契约键，实得 {v}"
    );
    assert_eq!(
        data["errors"].as_array().map(Vec::len),
        Some(0),
        "全成功时 errors 必须是空数组（不能缺键，也不能塞半截错误），实得 {v}"
    );
    // errors 元素键形状契约（前端 SkuMappingImportError 对齐 ImportError 四键）：
    // 用一行引用不存在的坏行取证一次真实形状。
    let bad_columns: Vec<String> = columns.iter().map(|(c, _)| c.clone()).collect();
    let mut bad_row = file_row_for(&columns, 0);
    bad_row.iter_mut().enumerate().for_each(|(i, cell)| {
        if bad_columns[i] == "product_code" {
            *cell = "SKU-IMP-不存在".to_string();
        }
    });
    let (bad_status, bad_v) =
        upload(&app, "sku_import_bad.csv", &csv_bytes(&columns, &[bad_row])).await;
    assert_eq!(
        bad_status,
        StatusCode::OK,
        "行级引用校验失败仍是 200 信封（部分导入语义），实得 {bad_v}"
    );
    let bad_data = &bad_v["data"];
    assert_eq!(
        bad_data["error_count"],
        json!(1),
        "坏行必须计入 error_count，实得 {bad_v}"
    );
    let first_error = &bad_data["errors"][0];
    assert_eq!(first_error["row"], json!(1), "行级错误的 row 是数据行序号");
    assert_eq!(
        first_error["column"],
        json!("product_code"),
        "errors[].column 键必须存在"
    );
    assert!(
        first_error["message"].is_string(),
        "errors[].message 必须存在"
    );
    assert_eq!(
        first_error["value"],
        json!("SKU-IMP-不存在"),
        "errors[].value 回显用户提交原值（脱敏红线不含内部数据，用户自己输入允许）"
    );

    // —— 库内真值回读 ——
    let supplier_id = require_seed_id(
        &db,
        &format!("SELECT id FROM suppliers WHERE supplier_code='{SEED_SUPPLIER_CODE}'"),
        "演示供应商 SUP-DEMO-FAB-01",
    )
    .await;
    let sp1_id = require_seed_id(
        &db,
        &format!(
            "SELECT id FROM supplier_products WHERE supplier_id={supplier_id} AND product_code='{SEED_SP_P001}'"
        ),
        "供应商商品 FAB-P001",
    )
    .await;
    let sp2_id = require_seed_id(
        &db,
        &format!(
            "SELECT id FROM supplier_products WHERE supplier_id={supplier_id} AND product_code='{SEED_SP_P002}'"
        ),
        "供应商商品 FAB-P002",
    )
    .await;
    let spc1_id = require_seed_id(
        &db,
        &format!(
            "SELECT id FROM supplier_product_colors WHERE supplier_product_id={sp1_id} AND color_no='{SEED_SPC_A01}'"
        ),
        "供应商色号 PC-A01",
    )
    .await;

    let landed_a = fetch_landed(&db, PRODUCT_A_ID).await;
    assert_eq!(
        landed_a["product_color_id"],
        json!(COLOR_A_ID),
        "行A 的色号编码必须解析成自建 product_colors.id"
    );
    assert_eq!(
        landed_a["supplier_id"],
        json!(supplier_id),
        "行A 的 supplier_code 必须解析成种子供应商 id"
    );
    assert_eq!(
        landed_a["supplier_product_id"],
        json!(sp1_id),
        "行A 的 supplier_product_code 必须解析成 FAB-P001 id"
    );
    assert_eq!(
        landed_a["supplier_product_color_id"],
        json!(spc1_id),
        "行A 的 supplier_color_no 必须解析成 PC-A01 id"
    );
    assert_eq!(landed_a["lead_time"], json!(7), "行A lead_time 原值落库");
    assert_eq!(landed_a["is_primary"], json!(true), "行A is_primary=true");
    assert_eq!(landed_a["priority"], json!(1), "行A priority=1");
    assert_eq!(landed_a["is_enabled"], json!(true), "行A is_enabled=true");
    assert_eq!(
        landed_a["remarks"],
        json!("对照导入锁行A"),
        "行A remarks 原文落库"
    );
    assert_eq!(
        landed_a["created_by"],
        json!(OPERATOR_ID),
        "created_by 必须取 AuthContext.user_id"
    );
    assert_eq!(
        count_row_a_decimals(&db, supplier_id).await,
        1,
        "行A numeric 列 supplier_price/min_order_quantity 按数值 12.50/30 落库"
    );

    let landed_b = fetch_landed(&db, PRODUCT_B_ID).await;
    assert_eq!(
        landed_b["product_color_id"],
        json!(null),
        "行B 未填 color_no ⇒ product_color_id 必须 NULL（不许瞎匹配）"
    );
    assert_eq!(
        landed_b["supplier_id"],
        json!(supplier_id),
        "行B supplier_code 解析"
    );
    assert_eq!(
        landed_b["supplier_product_id"],
        json!(sp2_id),
        "行B supplier_product_code 解析成 FAB-P002 id"
    );
    assert_eq!(
        landed_b["supplier_product_color_id"],
        json!(null),
        "行B 未填 supplier_color_no ⇒ NULL"
    );
    assert_eq!(landed_b["lead_time"], json!(null), "行B 可空列缺省 NULL");
    assert_eq!(
        landed_b["is_primary"],
        json!(false),
        "行B is_primary 缺省 false（import_batch 缺省语义，不许读成 true）"
    );
    assert_eq!(landed_b["priority"], json!(1), "行B priority 缺省 1");
    assert_eq!(
        landed_b["is_enabled"],
        json!(true),
        "行B is_enabled 缺省 true"
    );
    assert_eq!(landed_b["remarks"], json!(null), "行B remarks 缺省 NULL");
}

// ---------------------------------------------------------------------------
// 2. xlsx 全链成功：既有导出工具 build_xlsx 内存生成真实 xlsx（ZIP 容器）→ 同 handler
//    → 200 + 落库回读。钉「按扩展名/magic 分流」与产品导入同范式；把 xlsx 塞 CSV 文本
//    解析会炸「无效的 UTF-8 数据」，此锁让该回归（历史上产品侧踩过的坑）在本端点无处藏身。
//    改坏什么必红：扩展名分派被删/写反（上传 .xlsx 走 CSV 解析 ⇒ 非 200 红）；
//    XlsxImporter magic 校验被绕过口径变化 ⇒ 生成工具与校验同源，漂移即红。
// ---------------------------------------------------------------------------
#[tokio::test]
async fn xlsx_upload_imports_via_real_handler_and_lands_resolved_rows() {
    let (db, app) = app_with_db().await;
    let columns = parse_authoritative_columns(&read_handler_source());
    let rows = vec![file_row_for(&columns, 0), file_row_for(&columns, 1)];
    let (status, v) = upload(&app, "sku_import.xlsx", &xlsx_bytes(&columns, &rows)).await;

    assert_eq!(
        status,
        StatusCode::OK,
        "合法 xlsx 上传必须 200（不得被当 CSV 文本解析），实得信封: {v}"
    );
    assert_eq!(v["code"], 200, "成功信封 code 必须 200，实得 {v}");
    assert_eq!(
        v["data"]["success_count"],
        json!(2),
        "xlsx 两行必须全成功，实得 {v}"
    );
    assert_eq!(v["data"]["error_count"], json!(0), "实得 {v}");
    assert_eq!(
        count_mappings(&db).await,
        2,
        "xlsx 导入必须真实落库 2 行（信封成功数与库内行数不符=假成功）"
    );
    let landed_a = fetch_landed(&db, PRODUCT_A_ID).await;
    let supplier_id = require_seed_id(
        &db,
        &format!("SELECT id FROM suppliers WHERE supplier_code='{SEED_SUPPLIER_CODE}'"),
        "演示供应商 SUP-DEMO-FAB-01",
    )
    .await;
    assert_eq!(
        landed_a["supplier_id"],
        json!(supplier_id),
        "xlsx 行A 外键解析同 CSV 口径"
    );
    assert_eq!(
        landed_a["product_color_id"],
        json!(COLOR_A_ID),
        "xlsx 行A 色号解析"
    );
    assert_eq!(
        landed_a["created_by"],
        json!(OPERATOR_ID),
        "xlsx 导入 created_by 归属操作人"
    );
    // numeric 列逐字对拍（与 CSV 锁同一谓词，补强 xlsx 成功路径的读回维度）：
    // 行A 的 supplier_price/min_order_quantity 必须按数值 12.50/30 真实落库。
    // 修复前的裸 SQL 把 Decimal 绑成 Value::String，import_batch 会抛 42804、
    // success_count 变 0 ⇒ 上面 651 行先红；此处再确认落库的数值本身正确，
    // 堵住「不抛错但绑成别的数值」的漂移，让本锁的绿是真绿而非只断不抛。
    assert_eq!(
        count_row_a_decimals(&db, supplier_id).await,
        1,
        "xlsx 行A numeric 列 supplier_price/min_order_quantity 按数值 12.50/30 落库"
    );
}

// ---------------------------------------------------------------------------
// 3. 形态类拒绝族锁：表头缺必需列 / 表头含未知列 / 仅表头无数据行 / 单元格类型错误 /
//    必填单元格为空——全部 400 + VALIDATION_ERROR + 点名涉事列 + 零落库。
//    禁止把「文件不合法」糊成「200 + 0 条成功」（前端只会显示导入失败，缺陷再次隐身）。
//    改坏什么必红：校验被放宽（变 200/落库）红；族别用错（business/database）红；
//    文案退化成脱敏常量不点名列 ⇒ contains(列名) 红；内部细节回潮 ⇒ 脱敏断言红。
// ---------------------------------------------------------------------------
#[tokio::test]
async fn malformed_upload_rejected_400_validation_family_with_zero_writes() {
    let (db, app) = app_with_db().await;
    let src_columns = parse_authoritative_columns(&read_handler_source());
    let rows = vec![file_row_for(&src_columns, 0)];

    // —— 3a. 表头缺必需列：去掉 supplier_code 整列（列名从权威表来，不手抄）——
    let missing_cols: Vec<(String, bool)> = src_columns
        .iter()
        .filter(|(c, _)| c != "supplier_code")
        .cloned()
        .collect();
    assert!(
        missing_cols.len() + 1 == src_columns.len(),
        "权威表里必须存在可移除的必需列 supplier_code（夹具前提）"
    );
    let missing_rows: Vec<Vec<String>> = missing_rows_for(&missing_cols, &src_columns, &rows);
    let (status, v) = upload(
        &app,
        "missing_col.csv",
        &csv_bytes(&missing_cols, &missing_rows),
    )
    .await;
    assert_reject_v400(&v, status, Some("supplier_code"));
    assert_eq!(count_mappings(&db).await, 0, "表头缺列的拒绝必须零落库");

    // —— 3b. 表头含未知列 ——
    let mut unknown_cols = src_columns.clone();
    unknown_cols.push(("bogus_column".to_string(), false));
    let unknown_rows: Vec<Vec<String>> = rows
        .iter()
        .map(|r| {
            let mut rr = r.clone();
            rr.push("x".to_string());
            rr
        })
        .collect();
    let (status, v) = upload(
        &app,
        "unknown_col.csv",
        &csv_bytes(&unknown_cols, &unknown_rows),
    )
    .await;
    assert_reject_v400(&v, status, Some("bogus_column"));
    assert_eq!(count_mappings(&db).await, 0, "未知列的拒绝必须零落库");

    // —— 3c. 仅表头、无数据行（不许静默「0 条成功」）——
    let (status, v) = upload(&app, "header_only.csv", &csv_bytes(&src_columns, &[])).await;
    assert_eq!(
        status,
        StatusCode::BAD_REQUEST,
        "空数据文件必须 400，不能 200+0 成功糊过去，实得: {v}"
    );
    assert_eq!(v["code"], VALIDATION_FAMILY, "族别红: {v}");
    assert_eq!(count_mappings(&db).await, 0, "空数据文件必须零落库");

    // —— 3d. 单元格类型解析失败（supplier_price=abc）——
    let mut bad_type_rows = rows.clone();
    let price_idx = src_columns
        .iter()
        .position(|(c, _)| c == "supplier_price")
        .expect("权威表必须含 supplier_price 列（夹具前提）");
    bad_type_rows[0][price_idx] = "abc".to_string();
    let (status, v) = upload(
        &app,
        "bad_type.csv",
        &csv_bytes(&src_columns, &bad_type_rows),
    )
    .await;
    assert_reject_v400(&v, status, Some("supplier_price"));
    assert_eq!(count_mappings(&db).await, 0, "类型错误的拒绝必须零落库");

    // —— 3e. 必填单元格为空（supplier_code 列在但值空）——
    let mut empty_required = rows.clone();
    let sup_idx = src_columns
        .iter()
        .position(|(c, _)| c == "supplier_code")
        .expect("权威表必须含 supplier_code 列（夹具前提）");
    empty_required[0][sup_idx] = String::new();
    let (status, v) = upload(
        &app,
        "empty_required.csv",
        &csv_bytes(&src_columns, &empty_required),
    )
    .await;
    assert_reject_v400(&v, status, Some("supplier_code"));
    assert_eq!(count_mappings(&db).await, 0, "必填为空的拒绝必须零落库");
}

/// 3a 用：把「按完整权威表生成的行」投影到「删掉某列后的表」上（保持取值与列名对齐，
/// 不在测试里重排语义）。
fn missing_rows_for(
    missing_cols: &[(String, bool)],
    src_columns: &[(String, bool)],
    rows: &[Vec<String>],
) -> Vec<Vec<String>> {
    rows.iter()
        .map(|row| {
            missing_cols
                .iter()
                .map(|(name, _)| {
                    let idx = src_columns
                        .iter()
                        .position(|(c, _)| c == name)
                        .unwrap_or_else(|| panic!("删列后的表引用了原表没有的列 {name}"));
                    row[idx].clone()
                })
                .collect()
        })
        .collect()
}

// ---------------------------------------------------------------------------
// 4. 回归方向锁：旧 JSON 请求体必须被拒（端点已是 multipart 专属契约）。
//    axum 0.8.1 的 Multipart 提取器对非 multipart Content-Type 报 InvalidBoundary=400
//    （registry 源码 axum-0.8.1/src/extract/multipart.rs `define_rejection! { #[status =
//    BAD_REQUEST] ... }`；不是 415——415 对应「Json 提取器拒非 JSON 体」的方向，本锁
//    方向相反（Multipart 提取器拒非 multipart 体），状态码随之相反，按提取器实际语义钉
//    400）。提取层拒绝先于 handler/AppError，出参为 axum
//    内建文本、无 JSON code 可断，故本锁只断状态码 + 零落库，防止端点被改回 JSON 契约
//    后本测试静默失效（若改回 JSON：该请求会变成 200/415，两种形态都在这里红）。
// ---------------------------------------------------------------------------
#[tokio::test]
async fn legacy_json_body_upload_is_rejected_by_multipart_extractor() {
    let (db, app) = app_with_db().await;
    let resp = app
        .clone()
        .oneshot(
            Request::builder()
                .method(Method::POST)
                .uri("/sku-mappings/import")
                .header("content-type", "application/json")
                .body(Body::from(json!({ "rows": [] }).to_string()))
                .unwrap(),
        )
        .await
        .unwrap();
    let status = resp.status();
    assert_eq!(
        status,
        StatusCode::BAD_REQUEST,
        "JSON 请求体必须被 Multipart 提取层拒成 400 InvalidBoundary（axum 0.8.1 实测语义）；实得 {status}"
    );
    assert_eq!(
        count_mappings(&db).await,
        0,
        "提取层拒绝必须零落库（根本没进 handler）"
    );
}
