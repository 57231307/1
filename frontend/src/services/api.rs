use gloo_net::http::{Request, Response};
use serde::{de::DeserializeOwned, Serialize, Deserialize};
use crate::models::api_response::ApiResponse;

/// API 基础路径
pub const API_BASE: &str = "/api/v1/erp";

/// 缓存条目结构体
#[derive(Serialize, Deserialize)]
struct CacheEntry {
    data: serde_json::Value,
    expires_at: u64,
}

/// API 服务
/// 提供统一的 HTTP 请求方法，支持自动重试和缓存
pub struct ApiService;

impl ApiService {
    /// 基础 API 路径
    const API_BASE: &'static str = "/api/v1/erp";

    /// 最大重试次数
    const MAX_RETRIES: u32 = 3;

    /// GET 请求（带重试和缓存）
    pub async fn get<T: DeserializeOwned>(url: &str) -> Result<T, String> {
        Self::request_with_retry::<T>("GET", url, None).await
    }

    /// POST 请求（带重试，成功后会清除相关缓存）
    pub async fn post<T: DeserializeOwned, B: Serialize>(url: &str, body: &B) -> Result<T, String> {
        let body_value = serde_json::to_value(body).map_err(|e| format!("序列化请求体失败：{}", e))?;
        Self::request_with_retry::<T>("POST", url, Some(&body_value)).await
    }

    /// PUT 请求（带重试，成功后会清除相关缓存）
    pub async fn put<T: DeserializeOwned, B: Serialize>(url: &str, body: &B) -> Result<T, String> {
        let body_value = serde_json::to_value(body).map_err(|e| format!("序列化请求体失败：{}", e))?;
        Self::request_with_retry::<T>("PUT", url, Some(&body_value)).await
    }

    /// DELETE 请求（带重试，成功后会清除相关缓存）
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
        if let Some(window) = web_sys::window() {
            if let Ok(event) = web_sys::Event::new("api_start_loading") {
                let _ = window.dispatch_event(&event);
            }
        }

        let result = Self::request_with_retry_inner::<T>(method, url, body).await;

        if let Some(window) = web_sys::window() {
            if let Ok(event) = web_sys::Event::new("api_stop_loading") {
                let _ = window.dispatch_event(&event);
            }
        }

        result
    }

    /// 从 sessionStorage 缓存中读取数据
    async fn get_from_cache(url: &str) -> Result<Option<serde_json::Value>, String> {
        if let Some(window) = web_sys::window() {
            if let Ok(Some(storage)) = window.session_storage() {
                if let Ok(Some(json)) = storage.get_item(&format!("api_cache:{}", url)) {
                    if let Ok(entry) = serde_json::from_str::<CacheEntry>(&json) {
                        if entry.expires_at > js_sys::Date::now() as u64 {
                            return Ok(Some(entry.data));
                        }
                    }
                }
            }
        }
        Ok(None)
    }

    /// 将数据写入 sessionStorage 缓存
    async fn set_cache(url: &str, data: serde_json::Value, ttl_secs: u64) {
        let entry = CacheEntry {
            data,
            expires_at: (js_sys::Date::now() as u64) + ttl_secs * 1000,
        };
        if let Some(window) = web_sys::window() {
            if let Ok(Some(storage)) = window.session_storage() {
                if let Ok(json) = serde_json::to_string(&entry) {
                    let _ = storage.set_item(&format!("api_cache:{}", url), &json);
                }
            }
        }
    }

    /// 清除匹配指定模式的缓存条目
    async fn invalidate_cache(pattern: &str) {
        if let Some(window) = web_sys::window() {
            if let Ok(Some(storage)) = window.session_storage() {
                if let Ok(length) = storage.length() {
                    let mut keys_to_remove = Vec::new();
                    for i in 0..length {
                        if let Ok(Some(key)) = storage.key(i) {
                            if key.starts_with("api_cache:") && key.contains(pattern) {
                                keys_to_remove.push(key);
                            }
                        }
                    }
                    for key in keys_to_remove {
                        let _ = storage.remove_item(&key);
                    }
                }
            }
        }
    }

    async fn request_with_retry_inner<T: DeserializeOwned>(
        method: &str,
        url: &str,
        body: Option<&serde_json::Value>,
    ) -> Result<T, String> {
        let mut last_error = None;
        let full_url = format!("{}{}", Self::API_BASE, url);

        // GET 请求尝试从缓存读取
        if method == "GET" {
            if let Ok(Some(cached)) = Self::get_from_cache(&full_url).await {
                if let Ok(data) = serde_json::from_value(cached) {
                    return Ok(data);
                }
            }
        }

        for attempt in 0..Self::MAX_RETRIES {
            match Self::do_request(method, &full_url, body).await {
                Ok(response) => {
                    // 先读取原始文本，用于缓存
                    let response_text = response.text().await
                        .unwrap_or_else(|_| "{}".to_string());

                    // 解析为通用 JSON
                    let api_response_value: serde_json::Value = match serde_json::from_str(&response_text) {
                        Ok(v) => v,
                        Err(e) => {
                            last_error = Some(format!("解析响应失败：{}", e));
                            break;
                        }
                    };

                    // 检查 success 字段
                    let success = api_response_value.get("success")
                        .and_then(|v| v.as_bool())
                        .unwrap_or(false);

                    if success {
                        // 提取 data 字段
                        if let Some(data_value) = api_response_value.get("data") {
                            // GET 请求成功后写入缓存（默认5分钟）
                            if method == "GET" {
                                Self::set_cache(&full_url, data_value.clone(), 300).await;
                            }
                            // POST/PUT/DELETE 成功后清除相关缓存
                            else if method == "POST" || method == "PUT" || method == "DELETE" {
                                Self::invalidate_cache(url).await;
                            }

                            // 将 data 解析为 T
                            match serde_json::from_value::<T>(data_value.clone()) {
                                Ok(data) => return Ok(data),
                                Err(e) => {
                                    last_error = Some(format!("解析响应数据失败：{}", e));
                                    break;
                                }
                            }
                        } else if method == "DELETE" {
                            // DELETE 成功后清除相关缓存
                            Self::invalidate_cache(url).await;
                            return serde_json::from_value(serde_json::json!(null))
                                .map_err(|e| format!("无法为 DELETE 请求构造空响应: {}", e));
                        } else {
                            return Err("请求成功，但未返回数据".to_string());
                        }
                    } else {
                        let error_msg = api_response_value.get("error")
                            .and_then(|v| v.as_str())
                            .or_else(|| api_response_value.get("message").and_then(|v| v.as_str()))
                            .unwrap_or("请求失败");
                        return Err(error_msg.to_string());
                    }
                }
                Err(e) => {
                    last_error = Some(e.clone());
                    
                    if attempt < Self::MAX_RETRIES - 1 {
                        let delay_ms = 1000 * 2u64.pow(attempt);
                        gloo::timers::future::TimeoutFuture::new(delay_ms as u32).await;
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

        // 加入 credentials 以支持携带 HttpOnly Cookie
        let mut request_with_headers = request_builder
            .credentials(web_sys::RequestCredentials::Include)
            .header("Content-Type", "application/json")
            .header("X-Requested-With", "XMLHttpRequest");

        // 兼容过渡期：如果 localStorage 还有旧的 Token 也一并带上
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
        
        if status == 401 {
            let error_body = response
                .text()
                .await
                .unwrap_or_else(|_| "{}".to_string());
            
            let backend_error = serde_json::from_str::<serde_json::Value>(&error_body)
                .ok()
                .and_then(|v| v.get("error").and_then(|e| e.as_str()).map(String::from));
            
            crate::utils::storage::Storage::remove_token();
            if let Some(win) = web_sys::window() {
                if let Ok(loc) = win.location().pathname() {
                    if !loc.contains("/login") {
                        let _ = win.location().set_href("/login");
                    }
                }
            }

            let error_msg = backend_error.unwrap_or_else(|| "会话已过期，请重新登录".to_string());
            return Err(error_msg);
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
