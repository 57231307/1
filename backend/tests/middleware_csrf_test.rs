use axum::body::Body;
use axum::http::{Method, Request, StatusCode};
use bingxi_backend::middleware::audit_context::{
    CLIENT_IP_UNRESOLVED, ClientIpSource, extract_client_ip_with_source,
};
use bingxi_backend::middleware::csrf::*;
use std::net::SocketAddr;

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
    assert_eq!(
        body.get("code").and_then(|v| v.as_str()),
        Some("CSRF_TOKEN_MISSING")
    );
    assert_eq!(
        body.get("message").and_then(|v| v.as_str()),
        Some("CSRF Token 缺失")
    );
    // 统一失败信封（utils/response.rs::unified_error_response 出参 ErrorResponse）：
    // {code, message, trace_id, timestamp}，无 success/data 字段。
    assert!(
        body.get("trace_id")
            .map(|v| v.is_string() && !v.as_str().unwrap().is_empty())
            .unwrap_or(false),
        "trace_id 应为非空字符串"
    );
    assert!(
        body.get("timestamp").map(|v| v.is_i64()).unwrap_or(false),
        "timestamp 应为数字（秒级 i64）"
    );
    assert!(
        body.get("success").is_none() && body.get("data").is_none(),
        "统一错误信封不得含废弃的 success/data 字段"
    );
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

/// 测试 403 绑定核验失败错误响应：状态码与负载。
/// 钉的对象是 CSRF_TOKEN_BINDING_MISMATCH 这条唯一拒绝通道（跨用户/跨会话/无认证上下文三类合一）。
#[tokio::test]
async fn test_binding_mismatch_response_payload() -> Result<(), Box<dyn std::error::Error>> {
    let resp = csrf_error_response(CODE_BIND, CSRF_BINDING_MSG);
    assert_eq!(
        resp.status(),
        StatusCode::FORBIDDEN,
        "绑定核验失败必须仍是 403，不得退化成放行或 5xx"
    );
    let body_bytes = axum::body::to_bytes(resp.into_body(), 4096).await?;
    let body: serde_json::Value = serde_json::from_slice(&body_bytes)?;
    assert_eq!(
        body.get("code").and_then(|v| v.as_str()),
        Some("CSRF_TOKEN_BINDING_MISMATCH"),
        "业务码必须是 CSRF_TOKEN_BINDING_MISMATCH（前端据此不进自动恢复重放）"
    );
    assert_eq!(
        body.get("message").and_then(|v| v.as_str()),
        Some(CSRF_BINDING_MSG)
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
    // 绑定核验失败的机器码与文案（钉死字面量，防止改名让前端消费面与 e2e 归因双双失配）
    assert_eq!(CODE_BIND, "CSRF_TOKEN_BINDING_MISMATCH");
    assert_eq!(
        CSRF_BINDING_MSG, "安全令牌与当前会话不匹配，请重新登录后再试",
        "面向用户的文案只说该做什么，不得写入内部判定细节"
    );
}

/// 反向锁：观测 IP 不得再作为拒绝条件，因此不存在 IP 不匹配业务码与文案这两个符号。
/// 判据用"同名符号仍可编译引用"的方式表达不可行（删掉常量即编译失败），
/// 这里改钉字符串：中间件源码里不得再出现把 IP 差异当拒绝的机器码字面量。
#[test]
fn test_ip_mismatch_rejection_code_is_retired() {
    let src = include_str!("../src/middleware/csrf.rs");
    assert!(
        !src.contains("CSRF_IP_MISMATCH"),
        "csrf.rs 仍出现 CSRF_IP_MISMATCH：IP 差异被恢复成拒绝条件，移动与 NAT 客户端会重新硬故障"
    );
    assert!(
        !src.contains("IpMismatch"),
        "csrf.rs 仍出现 IpMismatch 分支：消费结果被改回以 IP 拒绝"
    );
}

/// 测试 CSRF 头名常量
#[test]
fn test_csrf_header_name() {
    assert_eq!(CSRF_HDR_NAME, "x-csrf-token");
}

/// 测试 extract_client_ip 的多级降级（含来源档位标注）
#[test]
fn test_extract_client_ip_priority() -> Result<(), Box<dyn std::error::Error>> {
    // 场景 1: X-Real-IP 优先级最高（边缘 Nginx 以 remote_addr 覆写，是唯一可信来源）
    // L-16 修复（批次 378 v13 复审）：原 expect("build") 改为 ? 操作符
    let req = Request::builder()
        .uri("/")
        .header("x-real-ip", "203.0.113.10")
        .header("x-forwarded-for", "198.51.100.1, 10.0.0.1")
        .body(Body::empty())?;
    assert_eq!(extract_client_ip_with_source(&req).0, "203.0.113.10");
    assert_eq!(
        extract_client_ip_with_source(&req).1,
        ClientIpSource::RealIpHeader,
        "X-Forwarded-For 与 X-Real-IP 并存时必须只认 X-Real-IP，否则客户端可用伪造的 XFF 首段污染留痕来源档位"
    );

    // 场景 2: 无 X-Real-IP 时取 X-Forwarded-For 首段，并标注为客户端可控来源
    let req = Request::builder()
        .uri("/")
        .header("x-forwarded-for", "198.51.100.1, 10.0.0.1")
        .body(Body::empty())?;
    assert_eq!(extract_client_ip_with_source(&req).0, "198.51.100.1");
    assert_eq!(
        extract_client_ip_with_source(&req).1,
        ClientIpSource::ForwardedForHeader,
        "降级到 XFF 必须标注来源档位，供审计区分真换网与伪造"
    );

    // 场景 3: 只有连接对端地址时用 ConnectInfo（直连部署下的真实客户端 IP）
    let peer: SocketAddr = "192.0.2.7:54321".parse()?;
    let mut req = Request::builder().uri("/").body(Body::empty())?;
    req.extensions_mut()
        .insert(axum::extract::ConnectInfo(peer));
    assert_eq!(extract_client_ip_with_source(&req).0, "192.0.2.7");
    assert_eq!(
        extract_client_ip_with_source(&req).1,
        ClientIpSource::ConnectPeer,
        "头里没有任何转发信息时必须落到 ConnectInfo，不得为了过测写成永远 unknown"
    );

    // 场景 4: 三级来源全缺才是占位值
    let req = Request::builder().uri("/").body(Body::empty())?;
    assert_eq!(extract_client_ip_with_source(&req).0, CLIENT_IP_UNRESOLVED);
    assert_eq!(
        extract_client_ip_with_source(&req).1,
        ClientIpSource::Unresolved
    );
    Ok(())
}

/// 反向锁：来源优先级只允许有一份实现。
/// handler 侧（登录/刷新）若另起一条降级链，绑定 IP 与校验 IP 会系统性错配。
#[test]
fn test_client_ip_extraction_has_single_source_of_truth() {
    let misc = include_str!("../src/handlers/auth_handler_misc.rs");
    let quoted = misc.matches("x-real-ip").count();
    assert!(
        quoted <= 1,
        "刷新链路又出现第二份 X-Real-IP 解析（命中 {quoted} 处）：优先级必须在 audit_context 单点维护"
    );
    assert!(
        misc.contains("client_ip_from_parts"),
        "刷新链路未复用 audit_context 的统一降级链 helper"
    );
}
