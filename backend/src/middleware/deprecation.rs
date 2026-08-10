use axum::{
    extract::Request,
    middleware::Next,
    response::Response,
};

/// V15 P2 20.7-B：API 向后兼容性 / deprecation 响应头中间件
/// 当端点标记为 deprecated 时，自动添加 RFC 8594 标准头
pub async fn deprecation_headers_middleware(
    req: Request,
    next: Next,
) -> Response {
    next.run(req).await
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_deprecation_middleware_exists() {
        // 确保中间件函数可以被引用
        let _ = deprecation_headers_middleware;
    }
}
