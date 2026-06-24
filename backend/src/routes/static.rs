//! 静态资源域路由
//!
//! 处理前端静态资源、CSS 样式、JavaScript/WASM 前端加载器等静态文件服务接口。

use axum::{
    body::Body,
    extract::Path,
    http::{header, header::HeaderValue, Request, StatusCode},
    response,
    routing::get,
    Router,
};
use std::convert::Infallible;
use std::path::{Component, PathBuf};

use crate::utils::app_state::AppState;

/// 规范化静态资源路径，拒绝包含 `..` 或绝对路径段的请求
///
/// 防御路径遍历漏洞（bug.md #1）：
/// - 拒绝 `..` 父目录段
/// - 拒绝 `\` 与 `//` 双重斜杠
/// - 拒绝绝对路径前缀（`/foo`）
fn sanitize_static_path(input: &str) -> Option<PathBuf> {
    // 拒绝空路径与包含反斜杠的 Windows 风格路径
    if input.is_empty() || input.contains('\\') {
        return None;
    }

    let p = std::path::Path::new(input);

    // 拒绝绝对路径与包含 `..` 段
    let has_invalid_component = p
        .components()
        .any(|c| matches!(c, Component::ParentDir | Component::Prefix(_) | Component::RootDir));
    if has_invalid_component {
        return None;
    }

    Some(p.to_path_buf())
}

/// 静态资源服务（Catch-all 通配路由，需要直接挂到主 Router）
pub fn static_assets_handler() -> Router<AppState> {
    Router::<AppState>::new()
        // 静态文件服务 - CSS样式文件
        .route(
            "/static/*path",
            get({
                move |Path(path): Path<String>| async move {
                    // 防御路径遍历漏洞（bug.md #1）：先规范化路径，拒绝 `..` 段
                    let safe_path = match sanitize_static_path(&path) {
                        Some(p) => p,
                        None => {
                            tracing::warn!(
                                "拒绝非法静态资源路径（疑似路径遍历攻击）: input={:?}",
                                path
                            );
                            return Ok::<_, Infallible>(
                                response::Response::builder()
                                    .status(StatusCode::BAD_REQUEST)
                                    .body(Body::from("Invalid path"))
                                    .unwrap_or_else(|e| {
                                        tracing::error!(
                                            "Failed to build 400 response: {:?}",
                                            e
                                        );
                                        response::Response::new(Body::from("Internal Error"))
                                    }),
                            );
                        }
                    };

                    let static_dir = std::env::var("FRONTEND_STATIC_DIR")
                        .unwrap_or_else(|_| "/workspace/frontend/static".to_string());
                    let static_path = PathBuf::from(&static_dir).join(&safe_path);
                    if let Ok(content) = tokio::fs::read(&static_path).await {
                        let body = Body::from(content);
                        let mut res = response::Response::new(body);
                        res.headers_mut()
                            .insert(header::CONTENT_TYPE, HeaderValue::from_static("text/css"));
                        return Ok::<_, Infallible>(res);
                    }
                    let cargo_manifest_dir = std::env::var("CARGO_MANIFEST_DIR")
                        .unwrap_or_else(|_| "/workspace/backend".to_string());
                    let fallback = PathBuf::from(cargo_manifest_dir)
                        .join("static")
                        .join(&safe_path);
                    if let Ok(content) = tokio::fs::read(&fallback).await {
                        let body = Body::from(content);
                        let mut res = response::Response::new(body);
                        res.headers_mut()
                            .insert(header::CONTENT_TYPE, HeaderValue::from_static("text/css"));
                        return Ok(res);
                    }
                    let body = Body::from("/* File not found */");
                    Ok(response::Response::builder()
                        .status(StatusCode::NOT_FOUND)
                        .body(body)
                        .unwrap_or_else(|e| {
                            tracing::error!("Failed to build 404 response: {:?}", e);
                            response::Response::new(Body::from("Internal Error"))
                        }))
                }
            }),
        )
        // 前端 WASM loader JS 文件
        .route(
            "/bingxi_frontend.js",
            get({
                let wasm_dir = "/workspace/frontend/target/wasm32-unknown-unknown/release";
                move |_req: Request<Body>| async move {
                    let js_file = format!("{}/bingxi_frontend.js", wasm_dir);
                    if let Ok(content) = tokio::fs::read(&js_file).await {
                        let body = Body::from(content);
                        let mut res = response::Response::new(body);
                        res.headers_mut().insert(
                            header::CONTENT_TYPE,
                            HeaderValue::from_static("application/javascript"),
                        );
                        res.headers_mut().insert(
                            header::CACHE_CONTROL,
                            HeaderValue::from_static("public, max-age=3600"),
                        );
                        return Ok::<_, Infallible>(res);
                    }
                    let fallback = format!(
                        "{}/dist/bingxi_frontend.js",
                        std::env::var("CARGO_MANIFEST_DIR")
                            .unwrap_or_else(|_| "/workspace/backend".to_string())
                    );
                    if let Ok(content) = tokio::fs::read(&fallback).await {
                        let body = Body::from(content);
                        let mut res = response::Response::new(body);
                        res.headers_mut().insert(
                            header::CONTENT_TYPE,
                            HeaderValue::from_static("application/javascript"),
                        );
                        return Ok(res);
                    }
                    let body = Body::from("console.log('WASM loader not found')");
                    Ok(response::Response::new(body))
                }
            }),
        )
        // 前端 WASM 二进制文件
        .route(
            "/bingxi_frontend_bg.wasm",
            get({
                let wasm_dir = "/workspace/frontend/target/wasm32-unknown-unknown/release";
                move |_req: Request<Body>| async move {
                    let wasm_file = format!("{}/bingxi_frontend_bg.wasm", wasm_dir);
                    if let Ok(content) = tokio::fs::read(&wasm_file).await {
                        let body = Body::from(content);
                        let mut res = response::Response::new(body);
                        res.headers_mut().insert(
                            header::CONTENT_TYPE,
                            HeaderValue::from_static("application/wasm"),
                        );
                        res.headers_mut().insert(
                            header::CACHE_CONTROL,
                            HeaderValue::from_static("public, max-age=3600"),
                        );
                        return Ok::<_, Infallible>(res);
                    }
                    let fallback = format!(
                        "{}/dist/bingxi_frontend_bg.wasm",
                        std::env::var("CARGO_MANIFEST_DIR")
                            .unwrap_or_else(|_| "/workspace/backend".to_string())
                    );
                    if let Ok(content) = tokio::fs::read(&fallback).await {
                        let body = Body::from(content);
                        let mut res = response::Response::new(body);
                        res.headers_mut().insert(
                            header::CONTENT_TYPE,
                            HeaderValue::from_static("application/wasm"),
                        );
                        return Ok(res);
                    }
                    let body = Body::empty();
                    let mut res = response::Response::new(body);
                    res.headers_mut().insert(
                        header::CONTENT_TYPE,
                        HeaderValue::from_static("application/wasm"),
                    );
                    Ok(res)
                }
            }),
        )
}

#[cfg(test)]
mod tests {
    use super::*;

    /// 测试合法路径：通过
    #[test]
    fn test_sanitize_accepts_valid_paths() {
        assert!(sanitize_static_path("style.css").is_some());
        assert!(sanitize_static_path("css/main.css").is_some());
        assert!(sanitize_static_path("a/b/c/d.js").is_some());
    }

    /// 测试路径遍历攻击：应被拒绝
    ///
    /// 修复 bug.md #1：原代码接受 `../../../etc/passwd` 等路径进行任意文件读取
    #[test]
    fn test_sanitize_rejects_path_traversal() {
        assert!(sanitize_static_path("../../../etc/passwd").is_none());
        assert!(sanitize_static_path("..\\..\\windows\\system32").is_none());
        assert!(sanitize_static_path("a/../../etc/passwd").is_none());
    }

    /// 测试绝对路径：应被拒绝
    #[test]
    fn test_sanitize_rejects_absolute_paths() {
        assert!(sanitize_static_path("/etc/passwd").is_none());
        assert!(sanitize_static_path("/absolute/path").is_none());
    }

    /// 测试空路径与 Windows 风格反斜杠：应被拒绝
    #[test]
    fn test_sanitize_rejects_empty_and_windows_paths() {
        assert!(sanitize_static_path("").is_none());
        assert!(sanitize_static_path("..\\config").is_none());
    }
}
