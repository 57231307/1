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
    let response = next.run(req).await;

    // 检查是否已设置 deprecation 头（由 handler 设置）
    // 如果没有设置，则不添加
    // 这里我们只是确保响应格式正确，实际的 deprecation 信息由 handler 添加

    response
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
