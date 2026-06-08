//! 统一输入验证中间件
//!
//! 提供 `ValidatedJson<T>` 提取器，在反序列化 JSON 请求体之后自动调用
//! `validator::Validate::validate()`，失败时返回 `AppError::BadRequest`。
//!
//! 使用示例（Handler 中）：
//! ```ignore
//! use crate::middleware::validation::ValidatedJson;
//! use axum::Json;
//! use validator::Validate;
//! use serde::Deserialize;
//!
//! #[derive(Debug, Deserialize, Validate)]
//! pub struct CreateUserDto {
//!     #[validate(length(min = 3, max = 32))]
//!     pub username: String,
//!     #[validate(email)]
//!     pub email: String,
//! }
//!
//! pub async fn create_user(
//!     ValidatedJson(payload): ValidatedJson<CreateUserDto>,
//! ) -> Result<Json<ApiResponse<()>>, AppError> {
//!     // payload 已通过反序列化 + 字段校验
//!     Ok(Json(ApiResponse::success(())))
//! }
//! ```
//!
//! 注意：
//! - DTO 需要同时派生 `Deserialize` 与 `Validate`。
//! - 字段级错误信息会作为 `AppError::BadRequest` 返回给客户端。

use axum::{
    extract::{FromRequest, Request},
    Json,
};
use validator::Validate;

use crate::utils::error::AppError;

/// 已通过反序列化和字段校验的 JSON 请求体包装器
pub struct ValidatedJson<T>(pub T);

impl<T> ValidatedJson<T> {
    /// 获取内部值的引用
    pub fn inner(&self) -> &T {
        &self.0
    }

    /// 消费包装器并返回内部值
    pub fn into_inner(self) -> T {
        self.0
    }
}

#[axum::async_trait]
impl<T, S> FromRequest<S> for ValidatedJson<T>
where
    T: serde::de::DeserializeOwned + Validate,
    S: Send + Sync,
{
    type Rejection = AppError;

    async fn from_request(req: Request, state: &S) -> Result<Self, Self::Rejection> {
        // 1. 反序列化 JSON 请求体
        let Json(value) = Json::<T>::from_request(req, state)
            .await
            .map_err(|e| AppError::BadRequest(format!("参数解析失败: {}", e)))?;

        // 2. 调用 validator 校验字段
        value
            .validate()
            .map_err(|e| AppError::BadRequest(format!("参数验证失败: {}", e)))?;

        Ok(ValidatedJson(value))
    }
}
