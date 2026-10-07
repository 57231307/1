//! 契约测试：internal 拍平治理 G2 组防回潮
//!
//! 覆盖本组（G2）15 个文件，三重防线（与 G1 同构）：
//! 1. **shrink-only ratchet**：逐文件统计 `AppError::internal(` 命中数，任何
//!    回升（超过下表基线）即判红；基线 = 修复后真实数目（含保留的
//!    真系统错误站点）。
//! 2. **AppError 调用零重包**：对"已知返回 `Result<_, AppError>` 的调用名"
//!    清单扫描 `调用名 ... .map_err(|..| AppError::internal` 组合，零命中；
//!    防止再次把 service 给出的 400/401/403/404 拍平成 500。
//! 3. **透传链路语义真实断言**：`not_found/business/validation/unauthorized/
//!    permission_denied` 经出参链路后 status 与 code 必须逐一对应各自族
//!    （404/NOT_FOUND、400/BUSINESS_ERROR、400/VALIDATION_ERROR、
//!    401/UNAUTHORIZED、403/FORBIDDEN），不得退化为"任意 4xx"式的宽松断言，
//!    且必须显式 `!= 500`。
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
/// 基线来源（修复后逐一 grep 坐实）——非零条目的保留理由（均为真系统错误，非拍平 bug）：
/// - system_update_handler.rs=4：GitHub 检查更新的上游网络故障（reqwest）1 处、
///   临时文件 IO 写入失败 1 处、temp_dir canonicalize 系统配置错误 1 处、
///   `map_update_error` 对 IoError/UnzipError/BackupError/NetworkError/IntegrityError/
///   ChecksumUnavailable 族的统一映射 1 处。
/// - import_export_handler.rs=4：axum `Response::builder().body()` 构造故障 2 处、
///   内存模板存储 `Mutex` 被污染（此前 panic 破坏状态）1 处、
///   导入任务创建后按 ID 反查不到（不可能态）1 处。
/// - auth_service.rs=3：`From<AuthError> for AppError` 的 TokenGenerationError/
///   HashingError/JwtError 族映射（JWT 编码 / Argon2 哈希 / 令牌结构异常，系统侧）。
/// - export_service.rs=2：printpdf 内建字体加载失败 / PDF save 写缓冲 IO 失败。
/// - bi_handler.rs=2：`tokio::time::timeout` Elapsed（服务自身 AppError 已由
///   第二个 `?` 原样透传，此处仅超时保护）。
/// - data_permission_handler.rs=1：对已校验为 object/null 的 Value 再次
///   `serde_json::to_string` 失败（不可能态系统异常，fail-closed 拒绝跳过安全检查）。
/// - email_queue_worker.rs=1：队列附件 Base64 解码失败（落库数据损坏，worker 侧）。
/// - bulk_color_approval_handler.rs=1：`bca_err` 对 service 显式 `Internal` 变体的映射。
/// - services/ai/mod.rs=1：信号量 `acquire_owned` 于已关闭 semaphore（生命周期异常）。
/// - ap_payment_request_handler.rs=1：`serde_json::to_value` 内部序列化故障。
/// - 其余 4 文件（slow_query/auth_handler_misc/webhook/color_card.analytics）已清零。
const INTERNAL_RATCHET: &[(&str, &str, usize)] = &[
    (
        "src/handlers/slow_query_handler.rs",
        include_str!("../src/handlers/slow_query_handler.rs"),
        0,
    ),
    (
        "src/handlers/system_update_handler.rs",
        include_str!("../src/handlers/system_update_handler.rs"),
        4,
    ),
    (
        "src/handlers/auth_handler_misc.rs",
        include_str!("../src/handlers/auth_handler_misc.rs"),
        0,
    ),
    (
        "src/handlers/webhook_handler.rs",
        include_str!("../src/handlers/webhook_handler.rs"),
        0,
    ),
    (
        "src/handlers/oa_announcement_handler.rs",
        include_str!("../src/handlers/oa_announcement_handler.rs"),
        1,
    ),
    (
        "src/handlers/import_export_handler.rs",
        include_str!("../src/handlers/import_export_handler.rs"),
        4,
    ),
    (
        "src/services/auth_service.rs",
        include_str!("../src/services/auth_service.rs"),
        3,
    ),
    (
        "src/handlers/color_card/analytics.rs",
        include_str!("../src/handlers/color_card/analytics.rs"),
        0,
    ),
    (
        "src/services/export_service.rs",
        include_str!("../src/services/export_service.rs"),
        2,
    ),
    (
        "src/handlers/bi_handler.rs",
        include_str!("../src/handlers/bi_handler.rs"),
        2,
    ),
    (
        "src/handlers/data_permission_handler.rs",
        include_str!("../src/handlers/data_permission_handler.rs"),
        1,
    ),
    (
        "src/services/email_queue_worker.rs",
        include_str!("../src/services/email_queue_worker.rs"),
        1,
    ),
    (
        "src/handlers/bulk_color_approval_handler.rs",
        include_str!("../src/handlers/bulk_color_approval_handler.rs"),
        1,
    ),
    (
        "src/services/ai/mod.rs",
        include_str!("../src/services/ai/mod.rs"),
        1,
    ),
    (
        "src/handlers/ap_payment_request_handler.rs",
        include_str!("../src/handlers/ap_payment_request_handler.rs"),
        1,
    ),
];

#[test]
fn g2_app_error_internal_count_is_shrink_only() {
    for (path, src, cap) in INTERNAL_RATCHET {
        let hits = src.matches("AppError::internal(").count();
        assert!(
            hits <= *cap,
            "{path}: `AppError::internal(` 命中 {hits} 处，超过基线 {cap} 处——\
             把已返回 AppError 的 service 调用重包成 internal 会把 400/401/403/404 \
             拍平成 500 并丢失真实 code（本组已清理一轮，禁止回潮）。"
        );
    }
}

// ---------------------------------------------------------------------------
// 2) 已知返回 AppError 的调用名 × map_err(internal) 零命中
// ---------------------------------------------------------------------------

/// 被测文件源码（去注释压缩前原文），供调用名扫描复用。
const G2_SOURCES: &[(&str, &str)] = &[
    (
        "src/handlers/webhook_handler.rs",
        include_str!("../src/handlers/webhook_handler.rs"),
    ),
    (
        "src/handlers/slow_query_handler.rs",
        include_str!("../src/handlers/slow_query_handler.rs"),
    ),
    (
        "src/handlers/auth_handler_misc.rs",
        include_str!("../src/handlers/auth_handler_misc.rs"),
    ),
    (
        "src/handlers/oa_announcement_handler.rs",
        include_str!("../src/handlers/oa_announcement_handler.rs"),
    ),
];

/// (返回 `Result<_, AppError>` 的调用名, 出现于哪些 G2 文件)——
/// 签名逐一在服务层源码核实（webhook_service.rs:67/103/118/341/362/368、
/// slow_query_collector.rs:132、totp_service.rs:9/47/114、
/// oa_announcement_service 的 list_for_user/publish/archive），
/// 任何一处被 `.map_err(|..| AppError::internal(..))` 重包即根因回潮。
const APP_ERROR_RETURNING_CALLS: &[(&str, &[&str])] = &[
    ("create_webhook", &["src/handlers/webhook_handler.rs"]),
    ("list_webhooks", &["src/handlers/webhook_handler.rs"]),
    ("delete_webhook", &["src/handlers/webhook_handler.rs"]),
    ("test_webhook", &["src/handlers/webhook_handler.rs"]),
    ("get_webhook", &["src/handlers/webhook_handler.rs"]),
    ("trigger_webhook", &["src/handlers/webhook_handler.rs"]),
    ("collect_once", &["src/handlers/slow_query_handler.rs"]),
    (
        "generate_totp_secret",
        &["src/handlers/auth_handler_misc.rs"],
    ),
    ("verify_and_enable", &["src/handlers/auth_handler_misc.rs"]),
    (
        "generate_recovery_codes",
        &["src/handlers/auth_handler_misc.rs"],
    ),
    ("publish", &["src/handlers/oa_announcement_handler.rs"]),
    ("archive", &["src/handlers/oa_announcement_handler.rs"]),
    (
        "list_for_user",
        &["src/handlers/oa_announcement_handler.rs"],
    ),
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
fn g2_app_error_returning_calls_are_never_rewrapped_as_internal() {
    for (call, files) in APP_ERROR_RETURNING_CALLS {
        for path in *files {
            let src = G2_SOURCES
                .iter()
                .find(|(p, _)| p == path)
                .unwrap_or_else(|| panic!("G2_SOURCES 缺少文件: {path}"))
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

/// 对应根因场景：色卡/webhook 目标不存在由 service 给 404，handler 旧代码
/// internal 压成 500——修复后 404/NOT_FOUND 必须原样透出。
#[test]
fn g2_service_not_found_passes_through_as_404_not_found_code() {
    let err = AppError::not_found("客户不存在");
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

/// 对应根因场景：OA 公告缺少可见对象配置 / webhook 无上次发送记录，
/// 属业务拒绝（400/BUSINESS_ERROR），不得 internal。
#[test]
fn g2_service_business_passes_through_as_400_business_code() {
    let err = AppError::business("无上次发送记录，无法重试（请先触发一次 webhook）");
    assert_eq!(err.error_code(), "BUSINESS_ERROR");
    assert_eq!(err.to_response().code, "BUSINESS_ERROR");
    let status = err.clone().into_response().status();
    assert_eq!(status, StatusCode::BAD_REQUEST);
    assert_ne!(status, StatusCode::INTERNAL_SERVER_ERROR);
}

/// 对应根因场景：数据权限 custom_condition 非法（400/VALIDATION_ERROR）。
/// displayable 族出参携带真实拒绝原因（"点提交只见服务器内部错误"的反面）。
#[test]
fn g2_service_validation_passes_through_as_400_validation_code() {
    let err = AppError::validation_displayable("custom_condition 必须是对象或 null");
    assert_eq!(err.error_code(), "VALIDATION_ERROR");
    assert_eq!(err.to_response().code, "VALIDATION_ERROR");
    let status = err.clone().into_response().status();
    assert_eq!(status, StatusCode::BAD_REQUEST);
    assert_ne!(status, StatusCode::INTERNAL_SERVER_ERROR);
    assert_eq!(
        err.to_response().message,
        "custom_condition 必须是对象或 null"
    );
}

/// 鉴权类拒绝保持 401/UNAUTHORIZED 语义（本组 auth_handler_misc 特别要求），
/// 不得用 business/internal 冒充。
#[test]
fn g2_auth_unauthorized_passes_through_as_401_unauthorized_code() {
    let err = AppError::unauthorized("无效的令牌");
    assert_eq!(err.error_code(), "UNAUTHORIZED");
    assert_eq!(err.to_response().code, "UNAUTHORIZED");
    let status = err.clone().into_response().status();
    assert_eq!(status, StatusCode::UNAUTHORIZED);
    assert_ne!(
        status,
        StatusCode::INTERNAL_SERVER_ERROR,
        "鉴权拒绝不得被拍平成 500"
    );
}

/// 越权类拒绝保持 403/FORBIDDEN 语义（如 webhook/色卡发放权限校验失败）。
#[test]
fn g2_permission_denied_passes_through_as_403_forbidden_code() {
    let err = AppError::permission_denied("系统更新操作仅限管理员（code=admin）执行");
    assert_eq!(err.error_code(), "FORBIDDEN");
    assert_eq!(err.to_response().code, "FORBIDDEN");
    let status = err.clone().into_response().status();
    assert_eq!(status, StatusCode::FORBIDDEN);
    assert_ne!(
        status,
        StatusCode::INTERNAL_SERVER_ERROR,
        "越权拒绝不得被拍平成 500"
    );
}

/// 数据库故障仍归 DATABASE_ERROR（非 INTERNAL_ERROR）：本组把被误标 internal 的
/// DbErr 站点改判为 database 后，排障 code 应稳定为 DATABASE_ERROR，
/// 出参仍脱敏（不外泄 SQL/表名）。
#[test]
fn g2_database_error_uses_database_code_and_is_sanitized() {
    let err = AppError::database("connection timed out: users");
    assert_eq!(err.error_code(), "DATABASE_ERROR");
    assert_eq!(err.to_response().code, "DATABASE_ERROR");
    assert_eq!(
        err.clone().into_response().status(),
        StatusCode::INTERNAL_SERVER_ERROR
    );
    // 真实原因只进日志，出参不得泄露表名/SQL
    assert!(
        !err.to_response().message.contains("users"),
        "数据库错误原文不得进 HTTP 出参"
    );
}
