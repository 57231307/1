use gloo_net::http::{Request, Response};
use serde::de::DeserializeOwned;

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
    pub async fn post<T: DeserializeOwned>(url: &str, body: &serde_json::Value) -> Result<T, String> {
        Self::request_with_retry::<T>("POST", url, Some(body)).await
    }

    /// PUT 请求（带重试）
    pub async fn put<T: DeserializeOwned>(url: &str, body: &serde_json::Value) -> Result<T, String> {
        Self::request_with_retry::<T>("PUT", url, Some(body)).await
    }

    /// DELETE 请求（带重试）
    pub async fn delete(url: &str) -> Result<(), String> {
        let _result: serde_json::Value = Self::request_with_retry::<serde_json::Value>("DELETE", url, None).await?;
        // DELETE 请求通常不需要返回值，忽略结果
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
                    // 尝试解析响应
                    if method == "DELETE" {
                        return Err("DELETE 请求不应返回数据".to_string());
                    } else {
                        match response.json::<T>().await {
                            Ok(data) => return Ok(data),
                            Err(e) => {
                                last_error = Some(format!("解析响应失败：{}", e));
                                // 解析错误不重试
                                break;
                            }
                        }
                    }
                }
                Err(e) => {
                    last_error = Some(e.clone());
                    
                    // 如果不是最后一次尝试，等待一段时间后重试
                    if attempt < Self::MAX_RETRIES - 1 {
                        // 指数退避：1s, 2s, 4s
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
        // 获取 Token
        let token = crate::utils::storage::Storage::get_token()
            .unwrap_or_else(|| "".to_string());

        // 构建请求
        let request_builder = match method {
            "GET" => Request::get(url),
            "POST" => Request::post(url),
            "PUT" => Request::put(url),
            "DELETE" => Request::delete(url),
            _ => return Err(format!("不支持的 HTTP 方法：{}", method)),
        };

        // 添加请求头
        let mut request_with_headers = request_builder.header("Content-Type", "application/json");
        
        if !token.is_empty() {
            let auth_header = format!("Bearer {}", token);
            request_with_headers = request_with_headers.header("Authorization", auth_header.as_str());
        }

        // 添加请求体（如果是 POST 或 PUT）
        let request = match body {
            Some(body_value) => request_with_headers.json(body_value)
                .map_err(|e: gloo_net::Error| format!("序列化请求体失败：{}", e))?,
            None => request_with_headers.build()
                .map_err(|e: gloo_net::Error| format!("构建请求失败：{}", e))?,
        };

        // 发送请求
        let response = request
            .send()
            .await
            .map_err(|e| format!("网络请求失败：{}", e))?;

        // 检查响应状态
        if response.ok() {
            Ok(response)
        } else {
            let status = response.status();
            let error_text = response
                .text()
                .await
                .unwrap_or_else(|_| "未知错误".to_string());
            Err(format!("请求失败 ({}): {}", status, error_text))
        }
    }
}
