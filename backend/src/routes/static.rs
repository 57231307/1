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

/// 静态资源服务（Catch-all 通配路由，需要直接挂到主 Router）
pub fn static_assets_handler() -> Router {
    Router::new()
        // 静态文件服务 - CSS样式文件
        .route(
            "/static/*path",
            get({
                move |Path(path): Path<String>| async move {
                    let static_dir = std::env::var("FRONTEND_STATIC_DIR")
                        .unwrap_or_else(|_| "/workspace/frontend/static".to_string());
                    let static_path = format!("{}/{}", static_dir, path);
                    if let Ok(content) = tokio::fs::read(&static_path).await {
                        let body = Body::from(content);
                        let mut res = response::Response::new(body);
                        res.headers_mut()
                            .insert(header::CONTENT_TYPE, HeaderValue::from_static("text/css"));
                        return Ok::<_, Infallible>(res);
                    }
                    let fallback = format!(
                        "{}/static/{}",
                        std::env::var("CARGO_MANIFEST_DIR")
                            .unwrap_or_else(|_| "/workspace/backend".to_string()),
                        path
                    );
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

/// 静态资源域统一入口（空 Router，调用方须在主 Router 上调用 static_assets_handler）
pub fn routes() -> Router {
    Router::new()
}
