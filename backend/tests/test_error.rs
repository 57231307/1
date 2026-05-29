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
    let err = AppError::ResourceNotFound("订单".to_string());
    assert!(err.to_string().contains("资源不存在"));
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

#[test]
fn test_error_from_validation_errors() {
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
    assert!(matches!(app_err, AppError::ValidationError(_)));
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
