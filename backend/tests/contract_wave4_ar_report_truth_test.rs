//! 契约波次 4 · AR 报表取真防回潮测试（G4/ar_report_truth）
//!
//! 三重防线（普查 A1/B2 + P1 500 拍平族）：
//! 1. **表名同源锁**：`services/ar_ops/report.rs` 内每一处 `ar_invoice` 都必须
//!    是复数 `ar_invoices`（与 `models/ar_invoice.rs:11` 的
//!    `#[sea_orm(table_name = "ar_invoices")]` 同源）；出现任何单数形态即判红。
//!    根因：裸 SQL 写 `FROM ar_invoice`（单数）在全新库首查即 42P01，被 handler
//!    的 internal 重包压成 500（普查 A1）。
//! 2. **解码兜底零容忍**：该文件内 `try_get_by_index::<..>(..)` 语句段中不得出现
//!    `unwrap_or`——列类型/表结构错配必须向上传播真实解码错误，禁止静默归零成
//!    "正常空数据"（普查 B2）。
//! 3. **shrink-only ratchet**：逐文件统计 `AppError::internal(` 命中数，任何回升
//!    即判红；基线 = 本轮修复后真实数目（含保留的真系统错误站点）。
//!
//! 另含**真实断言**：AR 报表 service 在表/列不存在（或解码错配）时返回的是**明确
//! 的、带真实原因的**错误（DATABASE_ERROR / NOT_FOUND 各自保真），而不是被 internal
//! 抹平后的通用"服务器内部错误"；并附 `#[ignore]` 的 TEST_DATABASE_URL 真库回退。

use axum::http::StatusCode;
use axum::response::IntoResponse;
use sea_orm::ConnectionTrait;
use sea_orm::DbErr;

use bingxi_backend::utils::error::AppError;

/// `services/ar_ops/report.rs` 源码（表名同源锁 + 解码兜底锁复用）。
const AR_REPORT_SRC: &str = include_str!("../src/services/ar_ops/report.rs");

// ---------------------------------------------------------------------------
// 1) 表名同源锁：每一处 ar_invoice 均为复数 ar_invoices
// ---------------------------------------------------------------------------

/// 返回首个「单数 ar_invoice」的字节偏移（其后紧跟非 's'），无则 None。
fn first_singular_ar_invoice(src: &str) -> Option<usize> {
    const NEEDLE: &str = "ar_invoice";
    let mut rest = src;
    let mut offset = 0usize;
    while let Some(rel) = rest.find(NEEDLE) {
        let abs = offset + rel;
        let after = &src[abs + NEEDLE.len()..];
        if !after.starts_with('s') {
            return Some(abs);
        }
        offset = abs + NEEDLE.len();
        rest = &src[offset..];
    }
    None
}

#[test]
fn ar_report_never_references_singular_ar_invoice_table() {
    if let Some(pos) = first_singular_ar_invoice(AR_REPORT_SRC) {
        // 打印命中行号，便于定位回潮的 FROM/JOIN 单数形态
        let line_no = AR_REPORT_SRC[..pos].lines().count();
        let line = AR_REPORT_SRC.lines().nth(line_no - 1).unwrap_or("");
        panic!(
            "report.rs 第 {line_no} 行出现单数表名 `ar_invoice`：「{}」——\
             真实表名是 `ar_invoices`（models/ar_invoice.rs:11），单数在全新库首查即 \
             42P01 并经 handler internal 重包压成 500（普查 A1）。",
            line.trim()
        );
    }
}

// ---------------------------------------------------------------------------
// 2) 解码兜底零容忍：try_get_by_index 语句段内不得有 unwrap_or
// ---------------------------------------------------------------------------

#[test]
fn ar_report_does_not_swallow_decode_errors_with_unwrap_or() {
    // 去空白压缩，消除多行换行造成的形态差异
    let compressed: String = AR_REPORT_SRC
        .chars()
        .filter(|c| !c.is_whitespace())
        .collect();
    let needle = "try_get_by_index";
    let mut from = 0usize;
    while let Some(rel) = compressed[from..].find(needle) {
        let abs = from + rel;
        let stmt_end = compressed[abs..]
            .find(';')
            .map(|p| abs + p)
            .unwrap_or(compressed.len());
        let stmt = &compressed[abs..stmt_end];
        assert!(
            !stmt.contains("unwrap_or"),
            "report.rs 存在 `try_get_by_index(..)` 接 `.unwrap_or(..)` 的解码兜底：\
             「{stmt}」——列类型/表结构错配时不得静默归零成正常空数据，\
             必须向上传播真实解码错误（普查 B2）。"
        );
        from = abs + needle.len();
    }
}

// ---------------------------------------------------------------------------
// 3) shrink-only ratchet
// ---------------------------------------------------------------------------

/// (相对 backend/ 的源文件, 修复后真实 `AppError::internal(` 命中数基线)
///
/// 非零条目的保留理由（逐站点核对返回类型后坐实）：
/// - report/exp.rs=18：printpdf 字体加载/PDF 保存、zip 写入、serde 序列化等外部库
///   故障——非 AppError 重包，属真不可归类系统错误（规则 b 末条）。
/// - email_service.rs=11：reqwest 发送失败/非 2xx、腾讯云/阿里云 HMAC 初始化、
///   serde 序列化、时间戳无效——外部集成与真系统错误。
/// - init_service.rs=1：`From<InitError>` 的密码哈希错误（bcrypt/argon2 故障）。
/// - ai/recipe_opt.rs=1：k-NN 加权聚合返回 None（不可能态，内部一致性错误）。
/// - quality_inspection_handler.rs=1：期望 JSON 对象却拿到非对象（内部逻辑不变量）。
/// - utils/webhook_signature.rs=1：对本地已算出的 hex 再解码失败（内部故障）。
/// - 其余 11 个清单文件本轮已把 internal 重包清零或改归类（database/bad_request/`?`）。
const INTERNAL_RATCHET: &[(&str, &str, usize)] = &[
    (
        "src/services/ar_ops/report.rs",
        include_str!("../src/services/ar_ops/report.rs"),
        0,
    ),
    (
        "src/handlers/ar_report_handler.rs",
        include_str!("../src/handlers/ar_report_handler.rs"),
        0,
    ),
    (
        "src/handlers/ar_reconciliation_handler.rs",
        include_str!("../src/handlers/ar_reconciliation_handler.rs"),
        0,
    ),
    (
        "src/handlers/ap_reconciliation_handler.rs",
        include_str!("../src/handlers/ap_reconciliation_handler.rs"),
        0,
    ),
    (
        "src/handlers/bulk_product_handler.rs",
        include_str!("../src/handlers/bulk_product_handler.rs"),
        0,
    ),
    (
        "src/handlers/product_handler.rs",
        include_str!("../src/handlers/product_handler.rs"),
        0,
    ),
    (
        "src/handlers/cost_collection_handler.rs",
        include_str!("../src/handlers/cost_collection_handler.rs"),
        0,
    ),
    (
        "src/handlers/logistics_handler.rs",
        include_str!("../src/handlers/logistics_handler.rs"),
        0,
    ),
    (
        "src/handlers/omni_audit_handler.rs",
        include_str!("../src/handlers/omni_audit_handler.rs"),
        0,
    ),
    (
        "src/handlers/advanced/forecast.rs",
        include_str!("../src/handlers/advanced/forecast.rs"),
        0,
    ),
    (
        "src/services/customer_ops/query.rs",
        include_str!("../src/services/customer_ops/query.rs"),
        0,
    ),
    (
        "src/services/report/exp.rs",
        include_str!("../src/services/report/exp.rs"),
        18,
    ),
    (
        "src/services/email_service.rs",
        include_str!("../src/services/email_service.rs"),
        11,
    ),
    (
        "src/services/init_service.rs",
        include_str!("../src/services/init_service.rs"),
        1,
    ),
    (
        "src/services/ai/recipe_opt.rs",
        include_str!("../src/services/ai/recipe_opt.rs"),
        1,
    ),
    (
        "src/handlers/quality_inspection_handler.rs",
        include_str!("../src/handlers/quality_inspection_handler.rs"),
        1,
    ),
    (
        "src/utils/webhook_signature.rs",
        include_str!("../src/utils/webhook_signature.rs"),
        1,
    ),
];

#[test]
fn ar_group_app_error_internal_count_is_shrink_only() {
    for (path, src, cap) in INTERNAL_RATCHET {
        let hits = src.matches("AppError::internal(").count();
        assert!(
            hits <= *cap,
            "{path}: `AppError::internal(` 命中 {hits} 处，超过基线 {cap} 处——\
             把已返回 AppError 的 service 调用或 DbErr 重包成 internal 会把 400/403/404 \
             或真实数据库故障统一降级为「服务器内部错误」并丢失原因（本组已清理一轮，禁止回潮）。"
        );
    }
}

/// 已知返回 `Result<_, AppError>` 的 AR service 调用名 × map_err(internal) 零命中，
/// 防止 ar_report_handler 再把 service 的真实 status/code 拍平成 500（普查 A2）。
const AR_APP_ERROR_RETURNING_CALLS: &[&str] = &[
    "get_statistics_report",
    "get_daily_report",
    "get_monthly_report",
    "get_aging_report",
    "get_aging_by_salesperson",
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

#[test]
fn ar_report_handler_passes_service_apperror_through() {
    let handler_src = include_str!("../src/handlers/ar_report_handler.rs");
    let norm = strip_comments_and_compress(handler_src);
    for call in AR_APP_ERROR_RETURNING_CALLS {
        // 违规形态：service.get_x(..).await.map_err(|e|AppError::internal(..))?;
        // 修复形态：service.get_x(..).await?;  （? 之前不含 map_err/internal）
        let mut from = 0usize;
        while let Some(rel) = norm[from..].find(call) {
            let at = from + rel + call.len();
            let search_end = norm[at..]
                .find('?')
                .map(|p| at + p)
                .unwrap_or((at + 200).min(norm.len()));
            let seg = &norm[at..search_end];
            assert!(
                !seg.contains(".map_err(") || !seg.contains("AppError::internal("),
                "ar_report_handler.rs: 调用 `{call}(..)` 后以 \
                 `.map_err(|..| AppError::internal(..))` 重包——该方法返回 \
                 Result<_, AppError>，必须 `?` 透传保留真实 status/code（普查 A2）。"
            );
            from = at;
        }
    }
}

// ---------------------------------------------------------------------------
// 4) 真实语义断言：表/列不存在 → 保真的错误，而非被 internal 抹平
// ---------------------------------------------------------------------------

/// B2 侧真实断言：列类型/解码错配（SeaORM 以 `DbErr::Type` 上抛）经 `?`/From 归类为
/// DATABASE_ERROR——携带真实原因的数据库故障族，绝不返回被 internal 抹平的
/// INTERNAL_ERROR，也不得静默成「Ok + 金额为 0」。
#[test]
fn decode_type_mismatch_surfaces_as_database_error_not_internal() {
    let decode_fault = DbErr::Type(
        "cannot convert column 'unpaid_amount' of relation 'ar_invoices': type mismatch"
            .to_string(),
    );
    let err = AppError::from(decode_fault);

    // 归类为真实数据库错误，而非通用内部错误
    assert_eq!(
        err.error_code(),
        "DATABASE_ERROR",
        "解码错配必须归类为 DATABASE_ERROR，不得被 internal 抹平"
    );
    assert_ne!(err.error_code(), "INTERNAL_ERROR");
    // 出参仍是脱敏常量（含表名/列名的真实原文只进 ERROR 日志，不外泄）
    assert_eq!(
        err.clone().into_response().status(),
        StatusCode::INTERNAL_SERVER_ERROR
    );
    let public = err.to_response().message;
    assert!(
        !public.contains("ar_invoices") && !public.contains("unpaid_amount"),
        "真实表/列名不得进 HTTP 出参，只能进日志：实际外显 = {public}"
    );
}

/// A2 侧真实断言：service 层给出的 not_found（记录/关系缺失族）经 `?` 透传后
/// 保真为 404 NOT_FOUND，而非被 handler 的 internal 重包降级成 500。
#[test]
fn service_not_found_passes_through_as_404_not_flattened_to_500() {
    let not_found = AppError::from(DbErr::RecordNotFound("对账单不存在".to_string()));
    assert_eq!(not_found.error_code(), "NOT_FOUND");
    let status = not_found.clone().into_response().status();
    assert_eq!(status, StatusCode::NOT_FOUND);
    assert_ne!(
        status,
        StatusCode::INTERNAL_SERVER_ERROR,
        "not_found 透传后绝不允许退化为 500「服务器内部错误」（普查 A2 根因）"
    );
}

/// internal 抹平形态的反证：确认 INTERNAL_ERROR 与 DATABASE_ERROR/NOT_FOUND 是
/// 不同的 code，从而上面两条断言并非宽松通过。
#[test]
fn internal_flatten_is_a_distinct_code_from_real_errors() {
    let flattened = AppError::internal("统计报表聚合查询失败");
    assert_eq!(flattened.error_code(), "INTERNAL_ERROR");
    assert_ne!(flattened.error_code(), "DATABASE_ERROR");
    assert_ne!(flattened.error_code(), "NOT_FOUND");
}

// ---------------------------------------------------------------------------
// 5) daily/monthly 出参形状锁：载荷必须是裸数组行集，行键逐字符钉死
//    （与前端 `frontend/src/api/ar.ts` 的 `ApiResponse<ARDailyReport[]>` /
//     `ApiResponse<ARMonthlyReport[]>` 信封声明同源；任何一方改形状必须先改这里）
// ---------------------------------------------------------------------------

/// 日报行键（report.rs `get_daily_report` 内 `json!` 逐字面值）
const DAILY_ROW_KEYS: &[&str] = &[
    "\"date\":",
    "\"invoice_count\":",
    "\"invoice_amount\":",
    "\"paid_amount\":",
    "\"unpaid_amount\":",
];

/// 月报行键（report.rs `get_monthly_report` 内 `json!` 逐字面值）
const MONTHLY_ROW_KEYS: &[&str] = &[
    "\"month\":",
    "\"invoice_count\":",
    "\"invoice_amount\":",
    "\"paid_amount\":",
    "\"unpaid_amount\":",
];

/// 截取 `start_marker` 到 `end_marker` 之间的源码段（函数体形状锁用）
fn report_fn_body<'s>(src: &'s str, start_marker: &str, end_marker: &str) -> &'s str {
    let start = src.find(start_marker).unwrap_or_else(|| {
        panic!("report.rs 未找到 `{start_marker}`——AR 日报/月报函数已漂移，形状锁需要随之重写")
    });
    let rest = &src[start..];
    let end = rest
        .find(end_marker)
        .unwrap_or_else(|| panic!("report.rs 未找到 `{end_marker}` 终止标记"));
    &rest[..end]
}

#[test]
fn ar_daily_and_monthly_payload_is_bare_array_with_pinned_row_keys() {
    let cases = [
        (
            "get_daily_report",
            report_fn_body(
                AR_REPORT_SRC,
                "pub async fn get_daily_report",
                "pub fn build_daily_sql_and_params",
            ),
            DAILY_ROW_KEYS,
        ),
        (
            "get_monthly_report",
            report_fn_body(
                AR_REPORT_SRC,
                "pub async fn get_monthly_report",
                "pub fn build_monthly_sql_and_params",
            ),
            MONTHLY_ROW_KEYS,
        ),
    ];
    for (name, body, keys) in cases {
        assert!(
            body.contains("Vec<serde_json::Value>") && body.contains("Ok(json!(result))"),
            "ar_ops/report.rs 的 `{name}` 必须以 `Vec<serde_json::Value>` 收集后 \
             `Ok(json!(result))` 出参——载荷是裸数组行集（前端信封 ApiResponse<Row[]> 同源）；\
             改成 {{rows,total}} 等对象包装属契约变更，必须前后端+本锁同步改。"
        );
        assert!(
            !body.contains("\"rows\"") && !body.contains("\"total\""),
            "`{name}` 出参里出现 rows/total 包装键——与裸数组定稿形状冲突（若确需聚合口径，\
            走独立端点 /ar/reports/statistics，勿在行集端点夹带）。"
        );
        for key in keys {
            assert!(
                body.contains(key),
                "`{name}` 行 json! 缺少钉死键 {key}——行形状漂移会击穿前端列渲染与信封判定。"
            );
        }
    }
}

// ---------------------------------------------------------------------------
// 6) 真库回退（#[ignore]，需 TEST_DATABASE_URL）
// ---------------------------------------------------------------------------

/// 需要真实 Postgres（`TEST_DATABASE_URL`）：
/// 1. 迁移完成的库上，`ArService::get_statistics_report` 应返回 Ok（复数表名
///    `ar_invoices` 可解析，全新库首查不再 42P01）。
/// 2. 对确定不存在的表跑裸查询，`DbErr` 经 `From` 归类为 DATABASE_ERROR——
///    证明表不存在是「带真实原因的 5xx」而非静默 Ok 归零。
///
/// 运行：`cargo test --test contract_wave4_ar_report_truth_test -- --ignored --nocapture`
#[tokio::test]
#[ignore = "需要 TEST_DATABASE_URL 指向已迁移的真实 Postgres"]
async fn ar_report_against_real_db() {
    let url = std::env::var("TEST_DATABASE_URL").expect("TEST_DATABASE_URL 未设置");
    let db = sea_orm::Database::connect(&url)
        .await
        .expect("连接 TEST_DATABASE_URL 失败");
    let db = std::sync::Arc::new(db);

    // 1. 真实表存在 → 首查成功（回归普查 A1）
    let svc = bingxi_backend::services::ar_service::ArService::new(db.clone());
    let report = svc
        .get_statistics_report(None, None, None)
        .await
        .expect("复数表名 ar_invoices 在全新/已迁移库上首查应成功，不得 42P01");
    assert!(report.get("total_invoices").is_some());

    // 1b. daily/monthly 出参必须是数组行集，且每行含全部钉死键（与前端信封声明同源）
    let daily = svc
        .get_daily_report(None, None, None)
        .await
        .expect("get_daily_report 在已迁移库上应返回 Ok");
    let daily_rows = daily
        .as_array()
        .unwrap_or_else(|| panic!("get_daily_report 载荷必须是 JSON 数组，实际 = {daily}"));
    if let Some(first) = daily_rows.first() {
        for key in [
            "date",
            "invoice_count",
            "invoice_amount",
            "paid_amount",
            "unpaid_amount",
        ] {
            assert!(
                first.get(key).is_some(),
                "日报首行缺少钉死键 {key}：实际行 = {first}"
            );
        }
    }
    let monthly = svc
        .get_monthly_report(None, None, None)
        .await
        .expect("get_monthly_report 在已迁移库上应返回 Ok");
    let monthly_rows = monthly
        .as_array()
        .unwrap_or_else(|| panic!("get_monthly_report 载荷必须是 JSON 数组，实际 = {monthly}"));
    if let Some(first) = monthly_rows.first() {
        for key in [
            "month",
            "invoice_count",
            "invoice_amount",
            "paid_amount",
            "unpaid_amount",
        ] {
            assert!(
                first.get(key).is_some(),
                "月报首行缺少钉死键 {key}：实际行 = {first}"
            );
        }
    }

    // 2. 不存在的表 → 真实解码/查询错误必须上抛为 DATABASE_ERROR，而非静默归零
    let probe = db
        .query_one_raw(sea_orm::Statement::from_sql_and_values(
            sea_orm::DatabaseBackend::Postgres,
            "SELECT COUNT(*) FROM ar_invoices_probe_missing_relation",
            vec![],
        ))
        .await;
    let err = AppError::from(probe.expect_err("缺失关系必须返回 Err，不得 Ok 归零"));
    assert_eq!(
        err.error_code(),
        "DATABASE_ERROR",
        "表不存在（42P01）应归类为带真实原因的 DATABASE_ERROR"
    );
}
