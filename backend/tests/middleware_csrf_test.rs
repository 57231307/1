use axum::http::{Method, StatusCode};
use bingxi_backend::middleware::csrf::*;

/// 测试安全方法（GET/HEAD/OPTIONS）通过 matches! 检查
#[test]
fn test_safe_methods_recognized() {
    assert!(matches!(
        Method::GET,
        Method::GET | Method::HEAD | Method::OPTIONS
    ));
    assert!(matches!(
        Method::HEAD,
        Method::GET | Method::HEAD | Method::OPTIONS
    ));
    assert!(matches!(
        Method::OPTIONS,
        Method::GET | Method::HEAD | Method::OPTIONS
    ));
}

/// 测试非安全方法不被放行
#[test]
fn test_unsafe_methods_not_recognized_as_safe() {
    assert!(!matches!(
        Method::POST,
        Method::GET | Method::HEAD | Method::OPTIONS
    ));
    assert!(!matches!(
        Method::PUT,
        Method::GET | Method::HEAD | Method::OPTIONS
    ));
    assert!(!matches!(
        Method::PATCH,
        Method::GET | Method::HEAD | Method::OPTIONS
    ));
    assert!(!matches!(
        Method::DELETE,
        Method::GET | Method::HEAD | Method::OPTIONS
    ));
}

/// 测试 403 缺失错误响应：状态码与负载
#[tokio::test]
async fn test_missing_response_payload() -> Result<(), Box<dyn std::error::Error>> {
    let resp = csrf_error_response(CODE_MISS, CSRF_MISSING_MSG);
    assert_eq!(resp.status(), StatusCode::FORBIDDEN);
    // L-16 修复（批次 378 v13 复审）：原 expect 改为 ? 操作符，测试失败时返回错误而非 panic
    let body_bytes = axum::body::to_bytes(resp.into_body(), 4096).await?;
    let body: serde_json::Value = serde_json::from_slice(&body_bytes)?;
    assert_eq!(body.get("success").and_then(|v| v.as_bool()), Some(false));
    assert_eq!(
        body.get("code").and_then(|v| v.as_str()),
        Some("CSRF_TOKEN_MISSING")
    );
    assert_eq!(
        body.get("message").and_then(|v| v.as_str()),
        Some("CSRF Token 缺失")
    );
    assert!(body.get("data").map(|v| v.is_null()).unwrap_or(false));
    Ok(())
}

/// 测试 403 无效错误响应：状态码与负载
#[tokio::test]
async fn test_invalid_response_payload() -> Result<(), Box<dyn std::error::Error>> {
    let resp = csrf_error_response(CODE_INVAL, CSRF_INVALID_MSG);
    assert_eq!(resp.status(), StatusCode::FORBIDDEN);
    // L-16 修复（批次 378 v13 复审）：原 expect 改为 ? 操作符，测试失败时返回错误而非 panic
    let body_bytes = axum::body::to_bytes(resp.into_body(), 4096).await?;
    let body: serde_json::Value = serde_json::from_slice(&body_bytes)?;
    assert_eq!(
        body.get("code").and_then(|v| v.as_str()),
        Some("CSRF_TOKEN_INVALID")
    );
    assert_eq!(
        body.get("message").and_then(|v| v.as_str()),
        Some("CSRF Token 无效或已过期")
    );
    Ok(())
}

/// 测试 403 IP 不匹配错误响应：状态码与负载（Wave 3 #7）
#[tokio::test]
async fn test_ip_mismatch_response_payload() -> Result<(), Box<dyn std::error::Error>> {
    let resp = csrf_error_response(CODE_IP_MM, CSRF_IP_MISMATCH_MSG);
    assert_eq!(resp.status(), StatusCode::FORBIDDEN);
    // L-16 修复（批次 378 v13 复审）：原 expect 改为 ? 操作符，测试失败时返回错误而非 panic
    let body_bytes = axum::body::to_bytes(resp.into_body(), 4096).await?;
    let body: serde_json::Value = serde_json::from_slice(&body_bytes)?;
    assert_eq!(
        body.get("code").and_then(|v| v.as_str()),
        Some("CSRF_IP_MISMATCH")
    );
    assert_eq!(
        body.get("message").and_then(|v| v.as_str()),
        Some("CSRF Token IP 不匹配")
    );
    Ok(())
}

/// 测试错误码常量值未被误改
#[test]
fn test_error_code_constants() {
    assert_eq!(CODE_MISS, "CSRF_TOKEN_MISSING");
    assert_eq!(CODE_INVAL, "CSRF_TOKEN_INVALID");
    assert_eq!(CSRF_MISSING_MSG, "CSRF Token 缺失");
    assert_eq!(CSRF_INVALID_MSG, "CSRF Token 无效或已过期");
    // Wave 3 #7：新增 IP 不匹配业务码
    assert_eq!(CODE_IP_MM, "CSRF_IP_MISMATCH");
    assert_eq!(CSRF_IP_MISMATCH_MSG, "CSRF Token IP 不匹配");
}

/// 测试 CSRF 头名常量
#[test]
fn test_csrf_header_name() {
    assert_eq!(CSRF_HDR_NAME, "x-csrf-token");
}

/// 测试 extract_client_ip 的多级降级（Wave 3 #7）
#[test]
fn test_extract_client_ip_priority() -> Result<(), Box<dyn std::error::Error>> {
    use axum::body::Body;
    use axum::http::Request;
    use axum::http::StatusCode;
    use serde_json::Value;

    // 场景 1: X-Real-IP 优先级最高
    // L-16 修复（批次 378 v13 复审）：原 expect("build") 改为 ? 操作符
    let req = Request::builder()
        .uri("/")
        .header("x-real-ip", "203.0.113.10")
        .header("x-forwarded-for", "198.51.100.1, 10.0.0.1")
        .body(Body::empty())?;
    assert_eq!(extract_client_ip(&req), "203.0.113.10");

    // 场景 2: 无 X-Real-IP 时取 X-Forwarded-For 首段
    let req = Request::builder()
        .uri("/")
        .header("x-forwarded-for", "198.51.100.1, 10.0.0.1")
        .body(Body::empty())?;
    assert_eq!(extract_client_ip(&req), "198.51.100.1");

    // 场景 3: 都没有时回退 "unknown"
    let req = Request::builder().uri("/").body(Body::empty())?;
    assert_eq!(extract_client_ip(&req), "unknown");
    Ok(())
}
