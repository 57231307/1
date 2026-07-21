
use axum::{
    extract::{Path, Query, State},
    Json,
};
// v9 P1-G 修复：移除未使用的 Serialize import
use serde::Deserialize;
use validator::Validate;

use crate::middleware::auth_context::AuthContext;
use crate::models::dto::PageRequest;
use crate::services::customer_service::{CreateCustomerArgs, CustomerService, UpdateCustomerArgs};
use crate::utils::app_state::AppState;
use crate::utils::data_permission::{DataPermissionFilter, DEFAULT_HIDDEN_FIELDS};
use crate::utils::error::AppError;
use crate::utils::response::ApiResponse;
// V15 P0-S15/P0-S12 补齐（Batch 474）：导出端点使用水印版 xlsx 工具
use crate::utils::xlsx_export::{
    build_xlsx_response_with_watermark, WatermarkConfig, XlsxTable,
};

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
        page: query.page.unwrap_or(1).clamp(1, 1000), // 批次 95 P3-3~8：分页 clamp 防 DoS
        page_size: query.page_size.unwrap_or(20).clamp(1, 100),
    };

    // 获取数据权限过滤器
    let permission_filter = get_permission_filter(&state, &auth, "customer").await?;

    // V15 P0-S01：提取行级数据权限上下文
    let data_scope_ctx = auth.to_data_scope_context();

    let customer_service = CustomerService::new(state.db.clone(), state.search_client.clone());
    let result = customer_service
        .list_customers_with_filter(
            page_req,
            query.status,
            query.customer_type,
            query.keyword,
            permission_filter,
            Some(&data_scope_ctx),
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
    let permission_filter = get_permission_filter(&state, &auth, "customer").await?;

    // V15 P0-S01：提取行级数据权限上下文（IDOR 防护）
    let data_scope_ctx = auth.to_data_scope_context();

    let customer_service = CustomerService::new(state.db.clone(), state.search_client.clone());
    let customer_json = customer_service
        .get_customer_with_filter(id, permission_filter, Some(&data_scope_ctx))
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

    let customer_service = CustomerService::new(state.db.clone(), state.search_client.clone());

    // P2-1 修复（批次 388 v13 复审）：原 parse().ok().unwrap_or(ZERO) 静默置零信用额度，
    // 用户输入非法值时无任何提示，改为显式校验报错
    let credit_limit = match payload.credit_limit.as_deref() {
        Some(s) if !s.is_empty() => s.parse::<rust_decimal::Decimal>().map_err(|e| {
            AppError::validation(format!("信用额度格式错误：{}（请输入有效数字）", e))
        })?,
        _ => rust_decimal::Decimal::ZERO,
    };

    let customer_type = payload
        .customer_type
        .unwrap_or_else(|| "retail".to_string());

    // 自动生成客户编码
    let customer_code = match payload.customer_code {
        Some(code) if !code.is_empty() => code,
        _ => customer_service.generate_customer_code().await?,
    };

    let customer = customer_service
        .create_customer(CreateCustomerArgs {
            customer_code,
            customer_name: payload.customer_name,
            contact_person: payload.contact_person,
            contact_phone: payload.contact_phone,
            contact_email: payload.contact_email,
            address: payload.address,
            city: payload.city,
            province: payload.province,
            country: Some("中国".to_string()),
            postal_code: payload.postal_code,
            credit_limit,
            payment_terms: payload.payment_terms.unwrap_or(crate::constants::DEFAULT_PAYMENT_TERMS_DAYS),
            tax_id: payload.tax_id,
            bank_name: payload.bank_name,
            bank_account: payload.bank_account,
            customer_type,
            notes: payload.notes,
            created_by: Some(auth.user_id),
        })
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

    let customer_service = CustomerService::new(state.db.clone(), state.search_client.clone());

    // M-1 修复：检查数据权限
    // 使用 created_by 做数据隔离：
    // - 管理员（role_id=1）可修改所有客户
    // - 普通用户只能修改自己创建的客户
    let customer = customer_service.get_customer(id, None).await?;
    let is_admin = auth.role_id == Some(1);
    let is_owner = customer.created_by == Some(auth.user_id);
    if !is_admin && !is_owner {
        return Err(AppError::permission_denied("无权修改该客户信息".to_string()));
    }

    // P2-1 修复（批次 388 v13 复审）：原 parse().ok() 静默吞错，
    // 用户输入非法值时信用额度不更新且无提示，改为显式校验报错
    let credit_limit = match payload.credit_limit.as_deref() {
        Some(s) if !s.is_empty() => Some(s.parse::<rust_decimal::Decimal>().map_err(|e| {
            AppError::validation(format!("信用额度格式错误：{}（请输入有效数字）", e))
        })?),
        _ => None,
    };

    let customer = customer_service
        .update_customer(UpdateCustomerArgs {
            customer_id: id,
            customer_name: payload.customer_name,
            contact_person: payload.contact_person,
            contact_phone: payload.contact_phone,
            contact_email: payload.contact_email,
            address: payload.address,
            city: payload.city,
            province: payload.province,
            postal_code: payload.postal_code,
            credit_limit,
            payment_terms: payload.payment_terms,
            tax_id: payload.tax_id,
            bank_name: payload.bank_name,
            bank_account: payload.bank_account,
            customer_type: payload.customer_type,
            status: payload.status,
            notes: payload.notes,
            // 批次 101 v6 复审 P2-1：透传操作人 user_id 用于审计日志
            user_id: auth.user_id,
        })
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
    let customer_service = CustomerService::new(state.db.clone(), state.search_client.clone());

    // M-1 修复：检查数据权限
    // 使用 created_by 做数据隔离：
    // - 管理员（role_id=1）可删除所有客户
    // - 普通用户只能删除自己创建的客户
    let customer = customer_service.get_customer(id, None).await?;
    let is_admin = auth.role_id == Some(1);
    let is_owner = customer.created_by == Some(auth.user_id);
    if !is_admin && !is_owner {
        return Err(AppError::permission_denied("无权删除该客户".to_string()));
    }

    // 批次 101 v6 复审 P2-2：透传操作人 user_id 用于审计日志
    customer_service.delete_customer(id, auth.user_id).await?;
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
///
/// P2-1 修复（批次 388 v13 复审）：原返回 Option 静默吞 DB 错误，
/// 改为 Result<Option<...>, AppError> 并在 Err 时 tracing::warn! 记录
async fn get_permission_filter(
    state: &AppState,
    auth: &AuthContext,
    resource_type: &str,
) -> Result<Option<DataPermissionFilter>, AppError> {
    let role_id = match auth.role_id {
        Some(id) => id,
        None => return Ok(None),
    };

    // 管理员角色不过滤
    if role_id == 1 {
        return Ok(None);
    }

    // 获取角色的数据权限配置
    match state
        .data_permission_service
        .get_role_data_permission(role_id, resource_type)
        .await
    {
        Ok(Some(permission)) => {
            // 有配置权限，使用配置的字段
            Ok(Some(DataPermissionFilter::new(
                permission.allowed_fields.unwrap_or_default(),
                permission.hidden_fields.unwrap_or_default(),
            )))
        }
        Ok(None) => {
            // 没有配置权限，使用默认隐藏字段
            Ok(Some(DataPermissionFilter::new(
                vec![],
                DEFAULT_HIDDEN_FIELDS
                    .iter()
                    .map(|s| s.to_string())
                    .collect(),
            )))
        }
        Err(e) => {
            // P2-1 修复（批次 388 v13 复审）：原 Err(_) 静默吞错，
            // 改为 warn 日志记录 + 返回默认隐藏字段（降级处理，不阻断主流程）
            tracing::warn!(
                role_id,
                resource_type,
                error = %e,
                "批次 388 P2-1: 查询角色数据权限失败，使用默认隐藏字段降级处理"
            );
            Ok(Some(DataPermissionFilter::new(
                vec![],
                DEFAULT_HIDDEN_FIELDS
                    .iter()
                    .map(|s| s.to_string())
                    .collect(),
            )))
        }
    }
}

/// 客户导出表头（16 列）
fn customer_export_headers() -> Vec<String> {
    vec![
        "客户编码".to_string(),
        "客户名称".to_string(),
        "客户类型".to_string(),
        "状态".to_string(),
        "联系人".to_string(),
        "联系电话".to_string(),
        "邮箱".to_string(),
        "地址".to_string(),
        "城市".to_string(),
        "省份".to_string(),
        "税号".to_string(),
        "开户行".to_string(),
        "银行账号".to_string(),
        "信用额度".to_string(),
        "账期(天)".to_string(),
        "创建时间".to_string(),
    ]
}

/// 从 serde_json::Value 提取客户行数据（list_customers_with_filter 返回 Value）
/// 统一用空字符串兜底，避免字段缺失导致 panic
fn build_customer_row(item: &serde_json::Value) -> Vec<String> {
    let s = |k: &str| -> String {
        item.get(k).and_then(|v| v.as_str()).unwrap_or("").to_string()
    };
    let opt_s = |k: &str| -> String {
        item.get(k)
            .and_then(|v| v.as_str())
            .map(|x| x.to_string())
            .unwrap_or_default()
    };
    let opt_d = |k: &str| -> String {
        // Decimal 字段序列化后为字符串或数字，统一取字符串形式
        item.get(k)
            .and_then(|v| v.as_str())
            .map(|x| x.to_string())
            .unwrap_or_else(|| "0".to_string())
    };
    let opt_i = |k: &str| -> String {
        item.get(k)
            .and_then(|v| v.as_i64())
            .map(|x| x.to_string())
            .unwrap_or_else(|| "0".to_string())
    };
    let opt_ts = |k: &str| -> String {
        item.get(k)
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string()
    };
    vec![
        s("customer_code"),
        s("customer_name"),
        s("customer_type"),
        s("status"),
        opt_s("contact_person"),
        opt_s("contact_phone"),
        opt_s("contact_email"),
        opt_s("address"),
        opt_s("city"),
        opt_s("province"),
        opt_s("tax_id"),
        opt_s("bank_name"),
        opt_s("bank_account"),
        opt_d("credit_limit"),
        opt_i("payment_terms"),
        opt_ts("created_at"),
    ]
}

/// 构造客户列表 xlsx 表格
fn build_customers_table(items: &[serde_json::Value]) -> XlsxTable {
    XlsxTable {
        sheet_name: "客户列表".to_string(),
        headers: customer_export_headers(),
        rows: items.iter().map(build_customer_row).collect(),
    }
}

/// 异步记录客户导出操作（审计自身）
fn record_customers_export_audit(state: &AppState, auth: &AuthContext, row_count: usize) {
    use crate::models::audit_log::{OperationType, Severity};
    use crate::services::audit_log_service::{AuditEvent, AuditLogService};
    use std::sync::Arc;
    let svc = AuditLogService::new(state.db.clone());
    let event = AuditEvent {
        user_id: Some(auth.user_id),
        username: Some(auth.username.clone()),
        operation_type: OperationType::Export,
        severity: Severity::Info,
        resource_type: Some("customer".to_string()),
        resource_id: None,
        resource_name: Some("客户列表导出".to_string()),
        description: Some(format!("导出 {} 条客户数据（含水印）", row_count)),
        request_method: Some("GET".to_string()),
        request_path: Some("/api/v1/customers/export".to_string()),
        before_snapshot: None,
        after_snapshot: None,
    };
    Arc::new(svc).record_async(event, None);
}

/// V15 P0-S12 + P0-S15 新增（Batch 474）：客户列表导出为带水印的 xlsx
///
/// 端点：`GET /api/v1/customers/export`
///
/// 设计要点：
/// - 复用 `list_customers` 的查询参数（status/customer_type/keyword）
/// - 通过 `CustomerService::list_customers_with_filter` 一次性查询（page_size=10000 防 OOM）
/// - 行级数据权限：与 `list_customers` 一致，调用 `to_data_scope_context` + `get_permission_filter`
/// - 水印：操作员（AuthContext.username）+ 导出时间（ISO8601）+ 资源类型说明
///   - IP 暂为 None（middleware 未把 client_ip 注入 AuthContext，后续批次补齐）
///
/// 规则 3：导出统一使用 xlsx 格式（含水印），错误用 AppError 表达。
pub async fn export_customers(
    State(state): State<AppState>,
    Query(query): Query<CustomerListQuery>,
    auth: AuthContext,
) -> Result<axum::response::Response, AppError> {
    // V15 P0-S12：复用 list 逻辑，page_size 取上限 10000 防止单次导出过大
    let page_req = PageRequest {
        page: 1,
        page_size: 10000,
    };

    let permission_filter = get_permission_filter(&state, &auth, "customer").await?;
    let data_scope_ctx = auth.to_data_scope_context();

    let customer_service = CustomerService::new(state.db.clone(), state.search_client.clone());
    let result = customer_service
        .list_customers_with_filter(
            page_req,
            query.status.clone(),
            query.customer_type.clone(),
            query.keyword.clone(),
            permission_filter,
            Some(&data_scope_ctx),
        )
        .await?;

    let row_count = result.items.len();
    let table = build_customers_table(&result.items);
    let watermark = WatermarkConfig {
        operator: Some(auth.username.clone()),
        ip_address: None, // 后续批次从 ConnectInfo 提取
        exported_at: Some(chrono::Utc::now().to_rfc3339()),
        extra: Some(format!("客户列表导出（共 {} 条）", row_count)),
    };
    let filename = format!(
        "customers_export_{}",
        chrono::Utc::now().format("%Y%m%d%H%M%S")
    );
    record_customers_export_audit(&state, &auth, row_count);
    build_xlsx_response_with_watermark(&table, &filename, &watermark)
}
