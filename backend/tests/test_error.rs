//! AppError 单元测试

use axum::http::StatusCode;
use bingxi_backend::utils::error::AppError;

#[test]
fn test_error_display_database() {
    let err = AppError::DatabaseError("连接失败".to_string());
    assert!(err.to_string().contains("数据库错误"));
    assert!(err.to_string().contains("连接失败"));
}

#[test]
fn test_error_display_validation() {
    let err = AppError::ValidationError("字段不能为空".to_string());
    assert!(err.to_string().contains("验证错误"));
}

#[test]
fn test_error_display_not_found() {
    let err = AppError::NotFound("用户".to_string());
    assert!(err.to_string().contains("未找到"));
}

#[test]
fn test_error_display_resource_not_found() {
    let err = AppError::not_found("订单".to_string());
    assert!(err.to_string().contains("未找到"));
}

#[test]
fn test_error_display_business() {
    let err = AppError::BusinessError("库存不足".to_string());
    assert!(err.to_string().contains("业务错误"));
}

#[test]
fn test_error_display_unauthorized() {
    let err = AppError::Unauthorized("token过期".to_string());
    assert!(err.to_string().contains("未授权"));
}

#[test]
fn test_error_display_internal() {
    let err = AppError::InternalError("系统异常".to_string());
    assert!(err.to_string().contains("内部错误"));
}

#[test]
fn test_error_display_bad_request() {
    let err = AppError::BadRequest("参数错误".to_string());
    assert!(err.to_string().contains("请求错误"));
}

#[test]
fn test_error_display_permission_denied() {
    let err = AppError::PermissionDenied("无权限".to_string());
    assert!(err.to_string().contains("权限不足"));
}

#[test]
fn test_error_display_too_many_requests() {
    let err = AppError::TooManyRequests {
        retry_after: Some(60),
        message: "请求过于频繁".to_string(),
    };
    assert!(err.to_string().contains("请求过于频繁"));
}

#[test]
fn test_error_from_status_code() {
    let err: AppError = (StatusCode::NOT_FOUND, "未找到".to_string()).into();
    assert!(matches!(err, AppError::NotFound(_)));

    let err: AppError = (StatusCode::BAD_REQUEST, "参数错误".to_string()).into();
    assert!(matches!(err, AppError::BadRequest(_)));

    let err: AppError = (StatusCode::UNAUTHORIZED, "未授权".to_string()).into();
    assert!(matches!(err, AppError::Unauthorized(_)));

    let err: AppError = (StatusCode::FORBIDDEN, "禁止访问".to_string()).into();
    assert!(matches!(err, AppError::PermissionDenied(_)));

    let err: AppError = (StatusCode::INTERNAL_SERVER_ERROR, "服务器错误".to_string()).into();
    assert!(matches!(err, AppError::InternalError(_)));
}

#[test]
fn test_error_from_serde_json() {
    let json_err = serde_json::from_str::<serde_json::Value>("invalid json").unwrap_err();
    let app_err: AppError = json_err.into();
    assert!(matches!(app_err, AppError::InternalError(_)));
}

/// `From<validator::ValidationErrors> for AppError` 的契约锁（依据 R-10 钉三判据）：
/// (a) 机器码 = `VALIDATION_ERROR`；(b) HTTP 400；(c) 出参 message 是可外显的原因文案，
/// **不等于**脱敏常量，且不含表名/SQL/内部标识。
///
/// 依据 R-10 不再逐字符钉 `Display` 输出：该文案属可外显面，措辞会随校验消息微调，
/// 钉死整串只会造成脆断（与 `a7547184` 已建立的双向钉同构：钉语义边界，不钉措辞）。
#[test]
fn test_error_from_validation_errors() {
    use axum::response::IntoResponse;
    use bingxi_backend::utils::messages::err_msg;
    use validator::Validate;

    #[derive(Debug, Validate)]
    struct TestInput {
        #[validate(length(min = 1, max = 10))]
        name: String,
    }

    let input = TestInput {
        name: "".to_string(),
    };
    let validation_err = input.validate().unwrap_err();
    let app_err: AppError = validation_err.into();

    assert!(
        matches!(
            app_err,
            AppError::ValidationError(_) | AppError::ValidationErrorDisplayable(_)
        ),
        "校验错误必须落进校验族变体（可外显或脱敏两支之一），实得: {app_err:?}"
    );
    // 判据 (a)：机器码
    assert_eq!(
        app_err.error_code(),
        "VALIDATION_ERROR",
        "字段校验族的外显码必须是 VALIDATION_ERROR"
    );
    // 判据 (c) 的前半：走的是**可外显**那一支，而不是被降级成脱敏常数的 ValidationError
    assert!(
        matches!(app_err, AppError::ValidationErrorDisplayable(_)),
        "From<ValidationErrors> 必须产出可外显变体；落回 ValidationError 即把原因吞成常量（该反模式已收口四轮）"
    );
    // 判据 (b)：HTTP 状态
    assert_eq!(
        app_err.clone().into_response().status(),
        StatusCode::BAD_REQUEST,
        "字段校验族必须是 400，不得被拍平成 500"
    );
    // 判据 (c)：外显文案 = 可读原因，既不是脱敏常量，也不得夹带内部信息
    let message = app_err.to_response().message;
    assert!(
        !message.is_empty(),
        "外显 message 不得为空（空串等于静默丢弃拒绝原因）"
    );
    assert_ne!(
        message,
        err_msg::VALIDATION_PUBLIC,
        "校验原因被包装层重新降级成脱敏常量 ⇒ 用户看不到自己哪条字段错（该反模式已收口四轮）"
    );
    for internal in [
        "SELECT",
        "INSERT",
        "UPDATE",
        "relation",
        "column",
        "ap_payment",
    ] {
        assert!(
            !message.contains(internal),
            "外显文案不得含 SQL/表名/列名等内部标识 {internal}，实际: {message:?}"
        );
    }
}

#[test]
fn test_error_is_send_sync() {
    fn assert_send_sync<T: Send + Sync>() {}
    assert_send_sync::<AppError>();
}

#[test]
fn test_error_clone() {
    let err = AppError::DatabaseError("test".to_string());
    let cloned = err.clone();
    assert_eq!(err.to_string(), cloned.to_string());
}
