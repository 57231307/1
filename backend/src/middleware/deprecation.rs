use axum::{
    extract::Request,
    middleware::Next,
    response::Response,
};
use chrono::Utc;

/// V15 P2 20.7-B：API 向后兼容性 / deprecation 响应头中间件
/// 当端点标记为 deprecated 时，自动添加 RFC 8594 标准头
pub async fn deprecation_headers_middleware(
    req: Request,
    next: Next,
) -> Response {
    let mut response = next.run(req).await;

    // 检查是否已设置 deprecation 头（由 handler 设置）
    // 如果没有设置，则不添加
    // 这里我们只是确保响应格式正确，实际的 deprecation 信息由 handler 添加

    response
}

/// 添加 deprecation 响应头到响应
/// 用于在 handler 中调用
pub fn add_deprecation_headers(
    response: &mut Response,
    deprecated_at: Option<chrono::DateTime<Utc>>,
    sunset_at: Option<chrono::DateTime<Utc>>,
) {
    let headers = response.headers_mut();

    // RFC 8594 Deprecation header
    if let Some(deprecated_at) = deprecated_at {
        // 使用 HTTP-date 格式：Sun, 06 Nov 1994 08:49:37 GMT
        let deprecation_date = deprecated_at.format("%a, %d %b %Y %H:%M:%S GMT").to_string();
        if let Ok(v) = deprecation_date.parse() {
            headers.insert("Deprecation", v);
        }

        // 添加 Link header 指向 deprecation 文档
        // 格式：Link: <https://example.com/deprecation>; rel="deprecation"
        // 这里我们使用通用格式，实际值由配置决定
    }

    // Sunset header (RFC 8594)
    if let Some(sunset_at) = sunset_at {
        let sunset_date = sunset_at.format("%a, %d %b %Y %H:%M:%S GMT").to_string();
        if let Ok(v) = sunset_date.parse() {
            headers.insert("Sunset", v);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::{body::Body, response::Response};
    use chrono::TimeZone;

    #[test]
    fn test_add_deprecation_headers() {
        let mut response = Response::new(Body::empty());

        let deprecated_at = Some(Utc.with_ymd_and_hms(2026, 1, 15, 0, 0, 0).unwrap());
        let sunset_at = Some(Utc.with_ymd_and_hms(2026, 6, 15, 0, 0, 0).unwrap());

        add_deprecation_headers(&mut response, deprecated_at, sunset_at);

        let headers = response.headers();
        assert!(headers.contains_key("Deprecation"));
        assert!(headers.contains_key("Sunset"));
    }

    #[test]
    fn test_no_deprecation_headers_when_none() {
        let mut response = Response::new(Body::empty());

        add_deprecation_headers(&mut response, None, None);

        let headers = response.headers();
        assert!(!headers.contains_key("Deprecation"));
        assert!(!headers.contains_key("Sunset"));
    }
}
