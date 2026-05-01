use gloo_net::http::{Request, Response};
use serde::{de::DeserializeOwned, Serialize};
use crate::models::api_response::ApiResponse;

/// API 基础路径
pub const API_BASE: &str = "/api/v1/erp";

/// API 服务
/// 提供统一的 HTTP 请求方法，支持自动重试
pub struct ApiService;

impl ApiService {
    /// 基础 API 路径
    const API_BASE: &'static str = "/api/v1/erp";

    /// 最大重试次数
    const MAX_RETRIES: u32 = 3;

    /// GET 请求（带重试）
    pub async fn get<T: DeserializeOwned>(url: &str) -> Result<T, String> {
        Self::request_with_retry::<T>("GET", url, None).await
    }

    /// POST 请求（带重试）
    pub async fn post<T: DeserializeOwned, B: Serialize>(url: &str, body: &B) -> Result<T, String> {
        let body_value = serde_json::to_value(body).map_err(|e| format!("序列化请求体失败：{}", e))?;
        Self::request_with_retry::<T>("POST", url, Some(&body_value)).await
    }

    /// PUT 请求（带重试）
    pub async fn put<T: DeserializeOwned, B: Serialize>(url: &str, body: &B) -> Result<T, String> {
        let body_value = serde_json::to_value(body).map_err(|e| format!("序列化请求体失败：{}", e))?;
        Self::request_with_retry::<T>("PUT", url, Some(&body_value)).await
    }

    /// DELETE 请求（带重试）
    pub async fn delete(url: &str) -> Result<(), String> {
        let _result: serde_json::Value = Self::request_with_retry::<serde_json::Value>("DELETE", url, None).await?;
        Ok(())
    }

    /// 带重试的请求方法
    /// 
    /// # 参数
    /// * `method` - HTTP 方法 (GET, POST, PUT, DELETE)
    /// * `url` - 请求路径（相对于 API_BASE）
    /// * `body` - 请求体（可选）
    /// 
    /// # 返回
    /// * `Ok(T)` - 请求成功，返回解析后的数据
    /// * `Err(String)` - 请求失败，返回错误信息
    async fn request_with_retry<T: DeserializeOwned>(
        method: &str,
        url: &str,
        body: Option<&serde_json::Value>,
    ) -> Result<T, String> {
        let mut last_error = None;
        let full_url = format!("{}{}", Self::API_BASE, url);

        for attempt in 0..Self::MAX_RETRIES {
            match Self::do_request(method, &full_url, body).await {
                Ok(response) => {
                    match response.json::<ApiResponse<T>>().await {
                        Ok(api_response) => {
                            if api_response.success {
                                if let Some(data) = api_response.data {
                                    return Ok(data);
                                } else if method == "DELETE" {
                                    return serde_json::from_value(serde_json::json!(null))
                                        .map_err(|e| format!("无法为 DELETE 请求构造空响应: {}", e));
                                } else {
                                    return Err("请求成功，但未返回数据".to_string());
                                }
                            } else {
                                let error_msg = api_response.error
                                    .or(api_response.message)
                                    .unwrap_or_else(|| "请求失败".to_string());
                                return Err(error_msg);
                            }
                        }
                        Err(e) => {
                            last_error = Some(format!("解析响应失败：{}", e));
                            break;
                        }
                    }
                }
                Err(e) => {
                    last_error = Some(e.clone());
                    
                    if attempt < Self::MAX_RETRIES - 1 {
                        let delay_ms = 1000 * 2u64.pow(attempt);
                        gloo_timers::future::TimeoutFuture::new(delay_ms as u32).await;
                    }
                }
            }
        }

        Err(last_error.unwrap_or_else(|| "未知错误".to_string()))
    }

    /// 执行实际的 HTTP 请求
    async fn do_request(
        method: &str,
        url: &str,
        body: Option<&serde_json::Value>,
    ) -> Result<Response, String> {
        let token = crate::utils::storage::Storage::get_token()
            .unwrap_or_else(|| "".to_string());

        let request_builder = match method {
            "GET" => Request::get(url),
            "POST" => Request::post(url),
            "PUT" => Request::put(url),
            "DELETE" => Request::delete(url),
            _ => return Err(format!("不支持的 HTTP 方法：{}", method)),
        };

        let mut request_with_headers = request_builder
            .header("Content-Type", "application/json")
            .header("X-Requested-With", "XMLHttpRequest");

        if !token.is_empty() {
            let auth_header = format!("Bearer {}", token);
            request_with_headers = request_with_headers.header("Authorization", auth_header.as_str());
        }

        let request = match body {
            Some(body_value) => request_with_headers.json(body_value)
                .map_err(|e: gloo_net::Error| format!("序列化请求体失败：{}", e))?,
            None => request_with_headers.build()
                .map_err(|e: gloo_net::Error| format!("构建请求失败：{}", e))?,
        };

        let response = request
            .send()
            .await
            .map_err(|e| format!("网络请求失败：{}", e))?;

        let status = response.status();
        
        // 前端 Token 过期自动处理：拦截 401 未授权错误
        if status == 401 {
            crate::utils::storage::Storage::remove_token();
            if let Some(win) = web_sys::window() {
                // 如果当前不在登录页，则强制跳转到登录页
                if let Ok(loc) = win.location().pathname() {
                    if !loc.contains("/login") {
                        let _ = win.location().set_href("/login");
                    }
                }
            }
            return Err("会话已过期，请重新登录".to_string());
        }

        if response.ok() {
            Ok(response)
        } else {
            let error_text = response
                .text()
                .await
                .unwrap_or_else(|_| "未知错误".to_string());
            Err(format!("请求失败 ({}): {}", status, error_text))
        }
    }
}
