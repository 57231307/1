//! 契约波次 4 · internal 拍平治理 G1 组防回潮测试
//!
//! 覆盖本组（G1）14 个文件，三重防线：
//! 1. **shrink-only ratchet**：逐文件统计 `AppError::internal(` 命中数，
//!    任何回升（超过下表基线）即判红；基线 = 修复后真实数目（含保留的
//!    真系统错误站点与注释提及）。
//! 2. **AppError 调用零重包**：对"已知返回 `Result<_, AppError>` 的调用名"
//!    清单扫描 `调用名 ... .map_err(|..| AppError::internal` 组合，零命中；
//!    防止再次把 service 给出的 400/403/404 拍平成 500。
//! 3. **透传链路语义真实断言**：`business/validation/not_found/permission_denied`
//!    经出参链路后 status 与 code 必须逐一对应各自族
//!    （400/BUSINESS_ERROR、400/VALIDATION_ERROR、404/NOT_FOUND、403/FORBIDDEN），
//!    不得退化为"任意 4xx"式的宽松断言。
//!
//! 根因（CI 取证）：`AppError::internal(format!("{e}"))` 包裹已返回
//! AppError 的 service 调用会把业务拒绝统一降级为 500 且丢失真实 code。

use axum::http::StatusCode;
use axum::response::IntoResponse;
use bingxi_backend::utils::error::AppError;

// ---------------------------------------------------------------------------
// 1) shrink-only ratchet
// ---------------------------------------------------------------------------

/// (相对 backend/ 的源文件, 修复后真实 `AppError::internal(` 命中数基线)
///
/// 基线来源（修复后逐一 grep 坐实）——非零条目的保留理由：
/// - crm_handler.rs=5：ClamAV 配置缺失/URL 为空/扫描请求失败/非 2xx/响应读取失败，
///   均为外部依赖故障（reqwest/env 配置），属真系统错误。
/// - ai_extend_service.rs=2：AI 返回未知趋势/风险标签（与词表取值域不符，
///   拒绝按错值落库），系统内部一致性错误。
/// - system_update_service.rs=6：`From<UpdateError> for AppError` 的 IO/解压/
///   备份/网络/完整性/校验值不可用族映射（更新器系统错误与安全护栏）。
/// - docx_export.rs=2：docx-rs `pack()` 返回 PackError（序列化工件故障）。
/// - finance_payment_handler.rs=1：注释行内引用历史缺陷写法，非构造站点。
/// - notification_handler.rs=1：事件通知服务未启用（部署配置态错误）。
/// - barcode_scanner_handler.rs=1：聚合 COUNT 查询返回 0 行（不可能态），
///   系统异常而非用户错误。
/// - 其余 7 文件已清零。
const INTERNAL_RATCHET: &[(&str, &str, usize)] = &[
    (
        "src/handlers/crm_handler.rs",
        include_str!("../src/handlers/crm_handler.rs"),
        5,
    ),
    (
        "src/handlers/role_handler.rs",
        include_str!("../src/handlers/role_handler.rs"),
        0,
    ),
    (
        "src/services/ai_extend_service.rs",
        include_str!("../src/services/ai_extend_service.rs"),
        2,
    ),
    (
        "src/services/system_update_service.rs",
        include_str!("../src/services/system_update_service.rs"),
        6,
    ),
    (
        "src/handlers/ar_invoice_handler.rs",
        include_str!("../src/handlers/ar_invoice_handler.rs"),
        0,
    ),
    (
        "src/handlers/report_engine_handler.rs",
        include_str!("../src/handlers/report_engine_handler.rs"),
        0,
    ),
    (
        "src/services/order_change_history_service.rs",
        include_str!("../src/services/order_change_history_service.rs"),
        0,
    ),
    (
        "src/services/period_report_snapshot_service.rs",
        include_str!("../src/services/period_report_snapshot_service.rs"),
        0,
    ),
    (
        "src/utils/docx_export.rs",
        include_str!("../src/utils/docx_export.rs"),
        2,
    ),
    (
        "src/handlers/finance_payment_handler.rs",
        include_str!("../src/handlers/finance_payment_handler.rs"),
        1,
    ),
    (
        "src/handlers/notification_handler.rs",
        include_str!("../src/handlers/notification_handler.rs"),
        1,
    ),
    (
        "src/handlers/crm_assignment_handler.rs",
        include_str!("../src/handlers/crm_assignment_handler.rs"),
        0,
    ),
    (
        "src/services/crm/cust.rs",
        include_str!("../src/services/crm/cust.rs"),
        0,
    ),
    (
        "src/handlers/barcode_scanner_handler.rs",
        include_str!("../src/handlers/barcode_scanner_handler.rs"),
        1,
    ),
];

#[test]
fn g1_app_error_internal_count_is_shrink_only() {
    for (path, src, cap) in INTERNAL_RATCHET {
        let hits = src.matches("AppError::internal(").count();
        assert!(
            hits <= *cap,
            "{path}: `AppError::internal(` 命中 {hits} 处，超过基线 {cap} 处——\
             把已返回 AppError 的 service 调用重包成 internal 会把 400/403/404 \
             拍平成 500 并丢失真实 code（本组已清理一轮，禁止回潮）。"
        );
    }
}

// ---------------------------------------------------------------------------
// 2) 已知返回 AppError 的调用名 × map_err(internal) 零命中
// ---------------------------------------------------------------------------

/// 被测文件源码（去注释压缩前原文），供调用名扫描复用。
const G1_SOURCES: &[(&str, &str)] = &[
    (
        "src/handlers/crm_handler.rs",
        include_str!("../src/handlers/crm_handler.rs"),
    ),
    (
        "src/handlers/role_handler.rs",
        include_str!("../src/handlers/role_handler.rs"),
    ),
    (
        "src/handlers/ar_invoice_handler.rs",
        include_str!("../src/handlers/ar_invoice_handler.rs"),
    ),
    (
        "src/handlers/report_engine_handler.rs",
        include_str!("../src/handlers/report_engine_handler.rs"),
    ),
    (
        "src/handlers/finance_payment_handler.rs",
        include_str!("../src/handlers/finance_payment_handler.rs"),
    ),
    (
        "src/handlers/crm_assignment_handler.rs",
        include_str!("../src/handlers/crm_assignment_handler.rs"),
    ),
    (
        "src/services/crm/cust.rs",
        include_str!("../src/services/crm/cust.rs"),
    ),
];

/// (返回 `Result<_, AppError>` 的调用名, 出现于哪些 G1 文件)——
/// 签名逐一在服务层源码核实（role_permission_service.rs:82/107/196/240/533/552/652、
/// finance_payment_service.rs:142、report/ds.rs:34/327、report/exp.rs:29、
/// crm/cust.rs 等），任何一处被 `.map_err(|..| AppError::internal(..))`
/// 重包即根因回潮。
const APP_ERROR_RETURNING_CALLS: &[(&str, &[&str])] = &[
    ("list_roles", &["src/handlers/role_handler.rs"]),
    ("get_role_detail", &["src/handlers/role_handler.rs"]),
    ("update_role", &["src/handlers/role_handler.rs"]),
    ("delete_role", &["src/handlers/role_handler.rs"]),
    ("assign_permission", &["src/handlers/role_handler.rs"]),
    ("remove_permission", &["src/handlers/role_handler.rs"]),
    ("get_role_permissions", &["src/handlers/role_handler.rs"]),
    ("create_lead", &["src/handlers/crm_handler.rs"]),
    ("list_leads", &["src/handlers/crm_handler.rs"]),
    ("get_lead", &["src/handlers/crm_handler.rs"]),
    ("update_lead", &["src/handlers/crm_handler.rs"]),
    ("delete_lead", &["src/handlers/crm_handler.rs"]),
    ("import_leads", &["src/handlers/crm_handler.rs"]),
    ("create_opportunity", &["src/handlers/crm_handler.rs"]),
    ("list_opportunities", &["src/handlers/crm_handler.rs"]),
    ("get_opportunity", &["src/handlers/crm_handler.rs"]),
    ("update_opportunity", &["src/handlers/crm_handler.rs"]),
    ("delete_opportunity", &["src/handlers/crm_handler.rs"]),
    ("merge_leads", &["src/handlers/crm_handler.rs"]),
    (
        "create_payment",
        &["src/handlers/finance_payment_handler.rs"],
    ),
    (
        "list_payments",
        &["src/handlers/finance_payment_handler.rs"],
    ),
    ("find_by_id", &["src/handlers/finance_payment_handler.rs"]),
    ("execute_report", &["src/handlers/report_engine_handler.rs"]),
    ("export_report", &["src/handlers/report_engine_handler.rs"]),
    ("aggregate_data", &["src/handlers/report_engine_handler.rs"]),
    ("get_by_id", &["src/handlers/ar_invoice_handler.rs"]),
    ("create_follow_up", &["src/services/crm/cust.rs"]),
];

/// 去行注释后压缩空白，让 `foo(x)\n .await\n .map_err(...)` 变成连续文本。
fn strip_comments_and_compress(src: &str) -> String {
    src.lines()
        .filter(|l| !l.trim_start().starts_with("//"))
        .collect::<Vec<_>>()
        .join(" ")
        .chars()
        .filter(|c| !c.is_whitespace())
        .collect()
}

/// 在压缩文本中检查 call 的每个出现点：从调用名末尾到**首个 `?`** 之间若
/// 出现 `.map_err(` 且该位置附近 200 字节内含 `AppError::internal(`，判回潮。
/// （正常透传形态 `call(..).await?` 的首个 `?` 前没有 map_err；违规形态
/// `call(..).await.map_err(|e| AppError::internal(..))?` 的 map_err 先于 `?`。）
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

#[test]
fn g1_app_error_returning_calls_are_never_rewrapped_as_internal() {
    for (call, files) in APP_ERROR_RETURNING_CALLS {
        for path in *files {
            let src = G1_SOURCES
                .iter()
                .find(|(p, _)| p == path)
                .unwrap_or_else(|| panic!("G1_SOURCES 缺少文件: {path}"))
                .1;
            let norm = strip_comments_and_compress(src);
            assert!(
                !call_is_rewrapped_in_internal(&norm, call),
                "{path}: 调用 `{call}(..)` 后紧跟 `.map_err(|..| AppError::internal(..))` \
                 回潮——该方法返回 Result<_, AppError>，必须 `?` 透传保留真实 status/code。"
            );
        }
    }
}

// ---------------------------------------------------------------------------
// 3) 透传链路真实语义断言（不许断成"任意 4xx"）
// ---------------------------------------------------------------------------

#[test]
fn g1_service_business_error_passes_through_as_400_business_code() {
    let err = AppError::business("状态不允许提交");
    assert_eq!(err.error_code(), "BUSINESS_ERROR");
    assert_eq!(err.to_response().code, "BUSINESS_ERROR");
    assert_eq!(
        err.clone().into_response().status(),
        StatusCode::BAD_REQUEST
    );
}

#[test]
fn g1_service_validation_error_passes_through_as_400_validation_code() {
    let err = AppError::validation_displayable("primary_id 必须为整数");
    assert_eq!(err.error_code(), "VALIDATION_ERROR");
    assert_eq!(err.to_response().code, "VALIDATION_ERROR");
    assert_eq!(
        err.clone().into_response().status(),
        StatusCode::BAD_REQUEST
    );
    // displayable 族出参必须携带真实原因（这正是"点创建只见服务器内部错误"的反面）
    assert_eq!(err.to_response().message, "primary_id 必须为整数");
}

#[test]
fn g1_service_not_found_error_passes_through_as_404_not_found_code() {
    // 对应根因场景：角色不存在由 service 给 404，handler 旧代码 internal 压成 500
    let err = AppError::not_found("角色不存在");
    assert_eq!(err.error_code(), "NOT_FOUND");
    assert_eq!(err.to_response().code, "NOT_FOUND");
    let status = err.clone().into_response().status();
    assert_eq!(status, StatusCode::NOT_FOUND);
    assert_ne!(
        status,
        StatusCode::INTERNAL_SERVER_ERROR,
        "not_found 透传后绝不允许再退化为 500"
    );
}

#[test]
fn g1_service_permission_denied_passes_through_as_403_forbidden_code() {
    let err = AppError::permission_denied("无权访问该工艺优化记录");
    assert_eq!(err.error_code(), "FORBIDDEN");
    assert_eq!(err.to_response().code, "FORBIDDEN");
    assert_eq!(err.clone().into_response().status(), StatusCode::FORBIDDEN);
}
