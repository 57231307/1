
use axum::{
    extract::{Path, Query, State},
    Json,
};
use serde::{Deserialize, Serialize};
use validator::Validate;

use crate::middleware::auth_context::AuthContext;
use crate::models::dto::PageRequest;
use crate::services::customer_service::CustomerService;
use crate::utils::app_state::AppState;
use crate::utils::data_permission::{DataPermissionFilter, DEFAULT_HIDDEN_FIELDS};
use crate::utils::error::AppError;
use crate::utils::response::ApiResponse;

/// 创建客户请求
#[derive(Debug, Deserialize, Validate)]
pub struct CreateCustomerRequest {
    #[validate(length(min = 1, max = 50, message = "客户编码长度必须在1到50个字符之间"))]
    pub customer_code: Option<String>,
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
    #[validate(custom = "validate_customer_type")]
    pub customer_type: Option<String>,
    #[validate(length(max = 1000, message = "备注长度不能超过1000个字符"))]
    pub notes: Option<String>,
}

/// 验证客户类型
fn validate_customer_type(customer_type: &str) -> Result<(), validator::ValidationError> {
    let valid_types = [
        "retail",
        "wholesale",
        "distributor",
        "manufacturer",
        "other",
    ];
    if valid_types.contains(&customer_type) {
        Ok(())
    } else {
        Err(validator::ValidationError::new("invalid_customer_type"))
    }
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
    #[validate(custom = "validate_customer_type")]
    pub customer_type: Option<String>,
    pub status: Option<String>,
    #[validate(length(max = 1000, message = "备注长度不能超过1000个字符"))]
    pub notes: Option<String>,
}

/// 获取客户列表
pub async fn list_customers(
    State(state): State<AppState>,
    Query(query): Query<CustomerListQuery>,
    auth: AuthContext,
) -> Result<Json<ApiResponse<crate::utils::response::PaginatedResponse<serde_json::Value>>>, AppError>
{
    let page_req = PageRequest {
        page: query.page.unwrap_or(1),
        page_size: query.page_size.unwrap_or(20),
    };

    // 获取数据权限过滤器
    let permission_filter = get_permission_filter(&state, &auth, "customer").await;

    let customer_service = CustomerService::new(state.db.clone());
    let result = customer_service
        .list_customers_with_filter(
            page_req,
            query.status,
            query.customer_type,
            query.keyword,
            permission_filter,
        )
        .await?;

    Ok(Json(ApiResponse::success(
        crate::utils::response::PaginatedResponse::new(
            result.items,
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
    auth: AuthContext,
) -> Result<Json<ApiResponse<serde_json::Value>>, AppError> {
    // 获取数据权限过滤器
    let permission_filter = get_permission_filter(&state, &auth, "customer").await;

    let customer_service = CustomerService::new(state.db.clone());
    let customer_json = customer_service
        .get_customer_with_filter(id, permission_filter)
        .await?;

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

    // 自动生成客户编码
    let customer_code = match payload.customer_code {
        Some(code) if !code.is_empty() => code,
        _ => customer_service.generate_customer_code().await?,
    };

    let customer = customer_service
        .create_customer(
            customer_code,
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
            payload.payment_terms.unwrap_or(crate::constants::DEFAULT_PAYMENT_TERMS_DAYS),
            payload.tax_id,
            payload.bank_name,
            payload.bank_account,
            customer_type,
            payload.notes,
            Some(auth.user_id),
        )
        .await?;

    let customer_json = serde_json::to_value(customer)
        .map_err(|e| AppError::internal(format!("序列化失败: {}", e)))?;
    Ok(Json(ApiResponse::success_with_message(
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

    // M-1 修复：检查数据权限
    // 客户表无 tenant_id 字段，使用 created_by 做数据隔离：
    // - 管理员（role_id=1）可修改所有客户
    // - 普通用户只能修改自己创建的客户
    let customer = customer_service.get_customer(id).await?;
    let is_admin = auth.role_id == Some(1);
    let is_owner = customer.created_by == Some(auth.user_id);
    if !is_admin && !is_owner {
        return Err(AppError::permission_denied("无权修改该客户信息".to_string()));
    }

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

    let customer_json = serde_json::to_value(customer)
        .map_err(|e| AppError::internal(format!("序列化失败: {}", e)))?;
    Ok(Json(ApiResponse::success_with_message(
        customer_json,
        "客户更新成功",
    )))
}

/// 删除客户
pub async fn delete_customer(
    State(state): State<AppState>,
    Path(id): Path<i32>,
    auth: AuthContext,
) -> Result<Json<ApiResponse<()>>, AppError> {
    let customer_service = CustomerService::new(state.db.clone());

    // M-1 修复：检查数据权限
    // 客户表无 tenant_id 字段，使用 created_by 做数据隔离：
    // - 管理员（role_id=1）可删除所有客户
    // - 普通用户只能删除自己创建的客户
    let customer = customer_service.get_customer(id).await?;
    let is_admin = auth.role_id == Some(1);
    let is_owner = customer.created_by == Some(auth.user_id);
    if !is_admin && !is_owner {
        return Err(AppError::permission_denied("无权删除该客户".to_string()));
    }

    customer_service.delete_customer(id).await?;
    Ok(Json(ApiResponse::success_with_message((), "客户删除成功")))
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

/// 获取数据权限过滤器
///
/// 根据角色权限构建数据库层面的字段过滤器，将数据权限过滤下推到数据库层
///
/// # 参数
/// - `state`: 应用状态
/// - `auth`: 认证上下文
/// - `resource_type`: 资源类型（如 "customer"）
///
/// # 返回
/// 返回数据权限过滤器，如果管理员或无需过滤则返回 None
async fn get_permission_filter(
    state: &AppState,
    auth: &AuthContext,
    resource_type: &str,
) -> Option<DataPermissionFilter> {
    let role_id = auth.role_id?;

    // 管理员角色不过滤
    if role_id == 1 {
        return None;
    }

    // 获取角色的数据权限配置
    match state
        .data_permission_service
        .get_role_data_permission(role_id, resource_type)
        .await
    {
        Ok(Some(permission)) => {
            // 有配置权限，使用配置的字段
            Some(DataPermissionFilter::new(
                permission.allowed_fields.unwrap_or_default(),
                permission.hidden_fields.unwrap_or_default(),
            ))
        }
        Ok(None) => {
            // 没有配置权限，使用默认隐藏字段
            Some(DataPermissionFilter::new(
                vec![],
                DEFAULT_HIDDEN_FIELDS
                    .iter()
                    .map(|s| s.to_string())
                    .collect(),
            ))
        }
        Err(_) => {
            // 查询出错，使用默认隐藏字段
            Some(DataPermissionFilter::new(
                vec![],
                DEFAULT_HIDDEN_FIELDS
                    .iter()
                    .map(|s| s.to_string())
                    .collect(),
            ))
        }
    }
}
