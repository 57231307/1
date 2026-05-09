use axum::{
    extract::{Path, Query, State},
    Json,
};
use serde::{Deserialize, Serialize};
use validator::Validate;

use crate::models::dto::PageRequest;
use crate::services::customer_service::CustomerService;
use crate::utils::error::AppError;
use crate::utils::response::ApiResponse;
use crate::utils::app_state::AppState;

/// 创建客户请求
#[derive(Debug, Deserialize, Validate)]
pub struct CreateCustomerRequest {
    #[validate(length(min = 1, max = 50, message = "客户编码长度必须在1到50个字符之间"))]
    pub customer_code: String,
    #[validate(length(min = 1, max = 200, message = "客户名称长度必须在1到200个字符之间"))]
    pub customer_name: String,
    #[validate(length(max = 100, message = "联系人名称长度不能超过100个字符"))]
    pub contact_person: Option<String>,
    #[validate(length(max = 20, message = "联系电话长度不能超过20个字符"))]
    pub contact_phone: Option<String>,
    #[validate(email(message = "邮箱格式不正确"))]
    pub contact_email: Option<String>,
    #[validate(length(max = 500, message = "地址长度不能超过500个字符"))]
    pub address: Option<String>,
    #[validate(length(max = 100, message = "城市长度不能超过100个字符"))]
    pub city: Option<String>,
    #[validate(length(max = 100, message = "省份长度不能超过100个字符"))]
    pub province: Option<String>,
    #[validate(length(max = 20, message = "邮编长度不能超过20个字符"))]
    pub postal_code: Option<String>,
    pub credit_limit: Option<String>,
    pub payment_terms: Option<i32>,
    #[validate(length(max = 50, message = "税号长度不能超过50个字符"))]
    pub tax_id: Option<String>,
    #[validate(length(max = 200, message = "银行名称长度不能超过200个字符"))]
    pub bank_name: Option<String>,
    #[validate(length(max = 50, message = "银行账号长度不能超过50个字符"))]
    pub bank_account: Option<String>,
    pub customer_type: Option<String>,
    #[validate(length(max = 1000, message = "备注长度不能超过1000个字符"))]
    pub notes: Option<String>,
}

/// 更新客户请求
#[derive(Debug, Deserialize, Validate)]
pub struct UpdateCustomerRequest {
    #[validate(length(min = 1, max = 200, message = "客户名称长度必须在1到200个字符之间"))]
    pub customer_name: Option<String>,
    #[validate(length(max = 100, message = "联系人名称长度不能超过100个字符"))]
    pub contact_person: Option<String>,
    #[validate(length(max = 20, message = "联系电话长度不能超过20个字符"))]
    pub contact_phone: Option<String>,
    #[validate(email(message = "邮箱格式不正确"))]
    pub contact_email: Option<String>,
    #[validate(length(max = 500, message = "地址长度不能超过500个字符"))]
    pub address: Option<String>,
    #[validate(length(max = 100, message = "城市长度不能超过100个字符"))]
    pub city: Option<String>,
    #[validate(length(max = 100, message = "省份长度不能超过100个字符"))]
    pub province: Option<String>,
    #[validate(length(max = 20, message = "邮编长度不能超过20个字符"))]
    pub postal_code: Option<String>,
    pub credit_limit: Option<String>,
    pub payment_terms: Option<i32>,
    #[validate(length(max = 50, message = "税号长度不能超过50个字符"))]
    pub tax_id: Option<String>,
    #[validate(length(max = 200, message = "银行名称长度不能超过200个字符"))]
    pub bank_name: Option<String>,
    #[validate(length(max = 50, message = "银行账号长度不能超过50个字符"))]
    pub bank_account: Option<String>,
    pub customer_type: Option<String>,
    pub status: Option<String>,
    #[validate(length(max = 1000, message = "备注长度不能超过1000个字符"))]
    pub notes: Option<String>,
}

/// 客户响应
#[derive(Debug, Serialize)]
pub struct CustomerResponse {
    pub id: i32,
    pub customer_code: String,
    pub customer_name: String,
    pub contact_person: Option<String>,
    pub contact_phone: Option<String>,
    pub contact_email: Option<String>,
    pub address: Option<String>,
    pub city: Option<String>,
    pub province: Option<String>,
    pub country: String,
    pub postal_code: Option<String>,
    pub credit_limit: String,
    pub payment_terms: i32,
    pub tax_id: Option<String>,
    pub bank_name: Option<String>,
    pub bank_account: Option<String>,
    pub status: String,
    pub customer_type: String,
    pub notes: Option<String>,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub updated_at: chrono::DateTime<chrono::Utc>,
}

/// 获取客户列表
pub async fn list_customers(
    State(state): State<AppState>,
    Query(query): Query<CustomerListQuery>,
) -> Result<Json<ApiResponse<crate::utils::response::PaginatedResponse<serde_json::Value>>>, AppError>
{
    let page_req = PageRequest {
        page: query.page.unwrap_or(1),
        page_size: query.page_size.unwrap_or(20),
    };

    let customer_service = CustomerService::new(state.db.clone());
    let result = customer_service
        .list_customers(page_req, query.status, query.customer_type, query.keyword)
        .await?;

    let customers_json: Vec<serde_json::Value> = result
        .data
        .into_iter()
        .map(|c| serde_json::to_value(c).map_err(|e| AppError::InternalError(format!("序列化失败: {}", e))))
        .collect::<Result<Vec<_>, _>>()?;

    Ok(Json(ApiResponse::success(
        crate::utils::response::PaginatedResponse::new(
            customers_json,
            result.total,
            result.page,
            result.page_size,
        ),
    )))
}

/// 获取客户详情
pub async fn get_customer(
    State(state): State<AppState>,
    Path(id): Path<i32>,
) -> Result<Json<ApiResponse<serde_json::Value>>, AppError> {
    let customer_service = CustomerService::new(state.db.clone());
    let customer = customer_service.get_customer(id).await?;
    let customer_json = serde_json::to_value(customer).map_err(|e| AppError::InternalError(format!("序列化失败: {}", e)))?;
    Ok(Json(ApiResponse::success(customer_json)))
}

/// 创建客户
pub async fn create_customer(
    State(state): State<AppState>,
    auth: crate::middleware::auth_context::AuthContext,
    Json(payload): Json<CreateCustomerRequest>,
) -> Result<Json<ApiResponse<serde_json::Value>>, AppError> {
    payload.validate()?;

    let customer_service = CustomerService::new(state.db.clone());

    let credit_limit = payload
        .credit_limit
        .and_then(|s| s.parse::<rust_decimal::Decimal>().ok())
        .unwrap_or(rust_decimal::Decimal::ZERO);

    let customer_type = payload
        .customer_type
        .unwrap_or_else(|| "retail".to_string());

    let customer = customer_service
        .create_customer(
            payload.customer_code,
            payload.customer_name,
            payload.contact_person,
            payload.contact_phone,
            payload.contact_email,
            payload.address,
            payload.city,
            payload.province,
            Some("中国".to_string()),
            payload.postal_code,
            credit_limit,
            payload.payment_terms.unwrap_or(30),
            payload.tax_id,
            payload.bank_name,
            payload.bank_account,
            customer_type,
            payload.notes,
            Some(auth.user_id),
        )
        .await?;

    let customer_json = serde_json::to_value(customer).map_err(|e| AppError::InternalError(format!("序列化失败: {}", e)))?;
    Ok(Json(ApiResponse::success_with_msg(
        customer_json,
        "客户创建成功",
    )))
}

/// 更新客户
pub async fn update_customer(
    State(state): State<AppState>,
    Path(id): Path<i32>,
    auth: crate::middleware::auth_context::AuthContext,
    Json(payload): Json<UpdateCustomerRequest>,
) -> Result<Json<ApiResponse<serde_json::Value>>, AppError> {
    payload.validate()?;

    let customer_service = CustomerService::new(state.db.clone());

    let credit_limit = payload
        .credit_limit
        .and_then(|s| s.parse::<rust_decimal::Decimal>().ok());

    let customer = customer_service
        .update_customer(
            id,
            payload.customer_name,
            payload.contact_person,
            payload.contact_phone,
            payload.contact_email,
            payload.address,
            payload.city,
            payload.province,
            payload.postal_code,
            credit_limit,
            payload.payment_terms,
            payload.tax_id,
            payload.bank_name,
            payload.bank_account,
            payload.customer_type,
            payload.status,
            payload.notes,
        )
        .await?;

    let customer_json = serde_json::to_value(customer).map_err(|e| AppError::InternalError(format!("序列化失败: {}", e)))?;
    Ok(Json(ApiResponse::success_with_msg(
        customer_json,
        "客户更新成功",
    )))
}

/// 删除客户
pub async fn delete_customer(
    State(state): State<AppState>,
    Path(id): Path<i32>,
) -> Result<Json<ApiResponse<()>>, AppError> {
    let customer_service = CustomerService::new(state.db.clone());
    customer_service.delete_customer(id).await?;
    Ok(Json(ApiResponse::success_with_msg((), "客户删除成功")))
}

/// 客户查询参数
#[derive(Debug, Deserialize)]
pub struct CustomerListQuery {
    pub page: Option<u64>,
    pub page_size: Option<u64>,
    pub status: Option<String>,
    pub customer_type: Option<String>,
    pub keyword: Option<String>,
}
