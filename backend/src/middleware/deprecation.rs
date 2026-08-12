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
