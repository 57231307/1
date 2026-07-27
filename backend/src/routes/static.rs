//! 静态资源域路由

use axum::{
    body::Body,
    extract::Path,
    http::{header, header::HeaderValue, Request, StatusCode},
    response::{self, Response},
    routing::get,
    Router,
};
use std::convert::Infallible;
use std::path::{Component, PathBuf};

use crate::utils::app_state::AppState;

/// 规范化静态资源路径，拒绝 `..`、绝对路径段、反斜杠（防路径遍历）
fn sanitize_static_path(input: &str) -> Option<PathBuf> {
    if input.is_empty() || input.contains('\\') {
        return None;
    }
    let p = std::path::Path::new(input);
    let has_invalid = p
        .components()
        .any(|c| matches!(c, Component::ParentDir | Component::Prefix(_) | Component::RootDir));
    if has_invalid {
        return None;
    }
    Some(p.to_path_buf())
}

/// 构建带状态码的纯文本响应；构造失败回退为 Internal Error
fn build_text_response(status: StatusCode, body: &str) -> Response {
    response::Response::builder()
        .status(status)
        .body(Body::from(body.to_string()))
        .unwrap_or_else(|e| {
            tracing::error!("Failed to build {:?} response: {:?}", status, e);
            response::Response::new(Body::from("Internal Error"))
        })
}

/// canonicalize `FRONTEND_STATIC_DIR`，失败返回 None（用于后续边界校验）
async fn resolve_static_dir() -> Option<PathBuf> {
    let dir = std::env::var("FRONTEND_STATIC_DIR")
        .unwrap_or_else(|_| "/workspace/frontend/static".to_string());
    tokio::fs::canonicalize(&dir).await.ok()
}

/// canonicalize 主路径，失败回退 backend/static；两者均失败返回 None
async fn canonicalize_with_fallback(safe_path: &PathBuf) -> Option<PathBuf> {
    let dir = std::env::var("FRONTEND_STATIC_DIR")
        .unwrap_or_else(|_| "/workspace/frontend/static".to_string());
    let primary = PathBuf::from(&dir).join(safe_path);
    if let Ok(p) = tokio::fs::canonicalize(&primary).await {
        return Some(p);
    }
    let cargo_dir = std::env::var("CARGO_MANIFEST_DIR")
        .unwrap_or_else(|_| "/workspace/backend".to_string());
    let fallback = PathBuf::from(cargo_dir).join("static").join(safe_path);
    tokio::fs::canonicalize(&fallback).await.ok()
}

/// 校验 resolved_path 在 static_dir 边界内（防符号链接逃逸），越界返回 400
fn ensure_within_static_dir(resolved: &PathBuf, dir: Option<&PathBuf>) -> Result<(), Response> {
    if let Some(d) = dir {
        if !resolved.starts_with(d) {
            tracing::warn!(
                "拒绝符号链接越界访问: resolved={:?}, static_dir={:?}",
                resolved,
                d
            );
            return Err(build_text_response(StatusCode::BAD_REQUEST, "Invalid path"));
        }
    }
    Ok(())
}

/// 构建 WASM 资源响应（content_type + 可选 cache-control）
fn build_wasm_response(content: Vec<u8>, content_type: &'static str, cache: bool) -> Response {
    let mut res = response::Response::new(Body::from(content));
    res.headers_mut()
        .insert(header::CONTENT_TYPE, HeaderValue::from_static(content_type));
    if cache {
        res.headers_mut().insert(
            header::CACHE_CONTROL,
            HeaderValue::from_static("public, max-age=3600"),
        );
    }
    res
}

/// /static/*path handler：返回 CSS 等静态资源（含路径遍历与符号链接逃逸防御）
async fn serve_static_asset(Path(path): Path<String>) -> Result<Response, Infallible> {
    let safe_path = match sanitize_static_path(&path) {
        Some(p) => p,
        None => {
            tracing::warn!("拒绝非法静态资源路径（疑似路径遍历攻击）: input={:?}", path);
            return Ok(build_text_response(StatusCode::BAD_REQUEST, "Invalid path"));
        }
    };
    let static_dir = resolve_static_dir().await;
    let resolved = match canonicalize_with_fallback(&safe_path).await {
        Some(p) => p,
        None => return Ok(build_text_response(StatusCode::NOT_FOUND, "/* File not found */")),
    };
    if let Err(resp) = ensure_within_static_dir(&resolved, static_dir.as_ref()) {
        return Ok(resp);
    }
    match tokio::fs::read(&resolved).await {
        Ok(content) => {
            let mut res = response::Response::new(Body::from(content));
            res.headers_mut()
                .insert(header::CONTENT_TYPE, HeaderValue::from_static("text/css"));
            Ok(res)
        }
        Err(_) => Ok(build_text_response(StatusCode::NOT_FOUND, "/* File not found */")),
    }
}

/// /bingxi_frontend.js handler：返回 WASM 加载器 JS（主路径 → dist fallback → 占位文案）
async fn serve_wasm_loader_js(_req: Request<Body>) -> Result<Response, Infallible> {
    let primary = "/workspace/frontend/target/wasm32-unknown-unknown/release/bingxi_frontend.js";
    if let Ok(content) = tokio::fs::read(primary).await {
        return Ok(build_wasm_response(content, "application/javascript", true));
    }
    let fallback = format!(
        "{}/dist/bingxi_frontend.js",
        std::env::var("CARGO_MANIFEST_DIR").unwrap_or_else(|_| "/workspace/backend".to_string())
    );
    if let Ok(content) = tokio::fs::read(&fallback).await {
        return Ok(build_wasm_response(content, "application/javascript", false));
    }
    Ok(response::Response::new(Body::from(
        "console.log('WASM loader not found')",
    )))
}

/// /bingxi_frontend_bg.wasm handler：返回 WASM 二进制（主路径 → dist fallback → 空体）
async fn serve_wasm_binary(_req: Request<Body>) -> Result<Response, Infallible> {
    let primary = "/workspace/frontend/target/wasm32-unknown-unknown/release/bingxi_frontend_bg.wasm";
    if let Ok(content) = tokio::fs::read(primary).await {
        return Ok(build_wasm_response(content, "application/wasm", true));
    }
    let fallback = format!(
        "{}/dist/bingxi_frontend_bg.wasm",
        std::env::var("CARGO_MANIFEST_DIR").unwrap_or_else(|_| "/workspace/backend".to_string())
    );
    if let Ok(content) = tokio::fs::read(&fallback).await {
        return Ok(build_wasm_response(content, "application/wasm", false));
    }
    let mut res = response::Response::new(Body::empty());
    res.headers_mut()
        .insert(header::CONTENT_TYPE, HeaderValue::from_static("application/wasm"));
    Ok(res)
}

/// 静态资源服务路由聚合（Catch-all 通配路由，挂到主 Router）
pub fn static_assets_handler() -> Router<AppState> {
    Router::<AppState>::new()
        .route("/static/*path", get(serve_static_asset))
        .route("/bingxi_frontend.js", get(serve_wasm_loader_js))
        .route("/bingxi_frontend_bg.wasm", get(serve_wasm_binary))
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
