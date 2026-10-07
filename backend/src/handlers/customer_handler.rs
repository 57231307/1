use axum::{
    Json,
    extract::{Path, Query, State},
};
use serde::Deserialize;
use validator::Validate;

use crate::container::AppState;
use crate::handlers::crm_customer_handler::apply_customer_field_permission;
use crate::middleware::auth_context::AuthContext;
use crate::models::dto::PageRequest;
use crate::services::customer_service::{CreateCustomerArgs, CustomerService, UpdateCustomerArgs};
use crate::utils::admin_checker::is_admin_role;
use crate::utils::data_permission::{DEFAULT_HIDDEN_FIELDS, DataPermissionFilter};
use crate::utils::error::AppError;
use crate::utils::response::ApiResponse;
// 客户列表导出端点用带水印的 xlsx 构造器（utils/xlsx_export.rs）
use crate::utils::xlsx_export::{WatermarkConfig, XlsxTable, build_xlsx_response_with_watermark};

/// 创建客户请求
#[allow(dead_code, reason = "反序列化输入字段")]
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
    #[validate(custom(function = validate_customer_type))]
    pub customer_type: Option<String>,
    /// 国家（缺省"中国"）
    pub country: Option<String>,
    /// 客户状态（active/inactive）
    pub status: Option<String>,
    /// 客户行业
    #[validate(length(max = 100, message = "行业长度不能超过100个字符"))]
    pub customer_industry: Option<String>,
    /// 主营产品
    #[validate(length(max = 500, message = "主营产品长度不能超过500个字符"))]
    pub main_products: Option<String>,
    /// 年采购额
    pub annual_purchase: Option<rust_decimal::Decimal>,
    /// 质量要求
    #[validate(length(max = 500, message = "质量要求长度不能超过500个字符"))]
    pub quality_requirement: Option<String>,
    /// 验货标准
    #[validate(length(max = 500, message = "验货标准长度不能超过500个字符"))]
    pub inspection_standard: Option<String>,
    #[validate(length(max = 1000, message = "备注长度不能超过1000个字符"))]
    pub notes: Option<String>,
}

/// 验证客户类型：委托唯一词表模块 `constants::customer_type::check`
/// （波0 等价重构——允许值集合与拒绝 code/族别/文案逐字符不变，值集合只在该模块出现一次）
fn validate_customer_type(customer_type: &str) -> Result<(), validator::ValidationError> {
    crate::constants::customer_type::check(customer_type)
}

/// 更新客户请求
#[allow(dead_code, reason = "反序列化输入字段")]
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
    #[validate(custom(function = validate_customer_type))]
    pub customer_type: Option<String>,
    pub status: Option<String>,
    /// 国家
    pub country: Option<String>,
    /// 客户行业
    #[validate(length(max = 100, message = "行业长度不能超过100个字符"))]
    pub customer_industry: Option<String>,
    /// 主营产品
    #[validate(length(max = 500, message = "主营产品长度不能超过500个字符"))]
    pub main_products: Option<String>,
    /// 年采购额
    pub annual_purchase: Option<rust_decimal::Decimal>,
    /// 质量要求
    #[validate(length(max = 500, message = "质量要求长度不能超过500个字符"))]
    pub quality_requirement: Option<String>,
    /// 验货标准
    #[validate(length(max = 500, message = "验货标准长度不能超过500个字符"))]
    pub inspection_standard: Option<String>,
    #[validate(length(max = 1000, message = "备注长度不能超过1000个字符"))]
    pub notes: Option<String>,
}

/// 客户域读出口的第一层：按 `field_permissions` 表（`resource_type="customer"`）做
/// 字段级读过滤，调用方为 `list_customers` / `get_customer`。
///
/// 判定顺序：按入参 `role_id` 查该角色的字段配置行 → 逐行先
/// `filter_fields_by_read_permission`（`can_read=false` 整键移除）、再 `mask_fields`
/// （`mask_strategy="MASK"` 置 `"***"`，该字面量属 `FieldPermissionService` 的实现，
/// 本函数不重写、不复制列名清单）。`role_id` 缺失、配置行为空、或配置查询返回 `Err`
/// （`unwrap_or_default` 降级为空配置）时是空操作，出参原样返回。
///
/// 与第二层 `crm_customer_handler::apply_customer_field_permission`（`data_permissions`
/// 行 + 默认脱敏掩码）是**两个判定源、两层叠加**，不是二选一：客户域读出口（列表/详情/
/// 360）按"先本函数、后第二层"的顺序串联调用；不消费 `field_permissions` 配置行的
/// 其它出口只走单层。
pub(crate) async fn apply_customer_field_config_mask(
    state: &AppState,
    role_id: Option<i32>,
    rows: &mut [serde_json::Value],
) {
    let Some(role_id) = role_id else {
        return;
    };
    let field_perm_svc =
        crate::services::field_permission_service::FieldPermissionService::new(state.db.clone());
    let field_perms = field_perm_svc
        .list_field_permissions(Some("customer"), Some(role_id))
        .await
        .unwrap_or_default();
    if field_perms.is_empty() {
        return;
    }
    for row in rows.iter_mut() {
        // 先过滤无读权限的字段
        field_perm_svc.filter_fields_by_read_permission(row, &field_perms);
        // 再对需要掩码的字段进行掩码处理
        field_perm_svc.mask_fields(row, &field_perms);
    }
}

/// 获取客户列表
pub async fn list_customers(
    State(state): State<AppState>,
    Query(query): Query<CustomerListQuery>,
    auth: AuthContext,
) -> Result<Json<ApiResponse<crate::utils::response::PaginatedResponse<serde_json::Value>>>, AppError>
{
    let page_req = PageRequest {
        page: query.page.unwrap_or(1).clamp(1, 1000), // 深翻页防护：page 钳位 1..=1000
        page_size: query.page_size.unwrap_or(20).clamp(1, 100),
    };

    // 获取数据权限过滤器
    let permission_filter = get_permission_filter(&state, &auth, "customer").await?;

    // 行级数据权限上下文：作为 data_scope 参数下传 list_customers_with_filter，
    // 按当前用户数据范围（self/dept/all）限定可见客户行
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

    // 字段级读权限过滤（field_permissions 表）：判定语义见本文件
    // `apply_customer_field_config_mask`，列表/详情/360 共用同一实现
    let mut masked_items = result.items;
    apply_customer_field_config_mask(&state, auth.role_id, &mut masked_items).await;

    // 客户域 PII 出参脱敏的唯一实现 `crm_customer_handler::apply_customer_field_permission`
    // （列表/详情/360 同一入口，同资源同形状）：第 1 层 `utils/field_mask::mask_contact_fields_for_role`
    // 掩码手机号/邮箱列集合并对非 admin 移除 `address`，第 2 层叠加角色数据权限行的
    // allowed/hidden（只删键，不会把掩码还原成原文）。admin 由
    // `utils/admin_checker::is_admin_role`（roles.code='admin'）判定；列名清单只在该
    // 工具模块维护，本 handler 不重写第二套分支。
    apply_customer_field_permission(&state, auth.role_id, &mut masked_items).await;

    Ok(Json(ApiResponse::success(
        crate::utils::response::PaginatedResponse::new(
            masked_items,
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

    // 行级数据权限上下文：下传 get_customer_with_filter，由其内部
    // `utils/data_scope.rs::check_resource_owner` 判归属（IDOR 防护），越权返回 403
    let data_scope_ctx = auth.to_data_scope_context();

    let customer_service = CustomerService::new(state.db.clone(), state.search_client.clone());
    let customer_json = customer_service
        .get_customer_with_filter(id, permission_filter, Some(&data_scope_ctx))
        .await?;

    // 字段级读权限过滤：与列表出口共用 `apply_customer_field_config_mask`，
    // 详情单行按 `slice::from_mut` 传入同一批处理入口
    let mut customer_json = customer_json;
    apply_customer_field_config_mask(
        &state,
        auth.role_id,
        std::slice::from_mut(&mut customer_json),
    )
    .await;

    // PII 脱敏与列表同一收口 `apply_customer_field_permission`（两层判定见列表出口
    // 上方说明），保证详情与列表对同一角色给出同形状的掩码结果
    apply_customer_field_permission(
        &state,
        auth.role_id,
        std::slice::from_mut(&mut customer_json),
    )
    .await;

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

    // 信用额度：缺省/空串取 Decimal::ZERO 入库；非空但解析失败出 VALIDATION 并外显原因，
    // 不走 `ok().unwrap_or()` 把非法输入静默变成 0
    let credit_limit = match payload.credit_limit.as_deref() {
        Some(s) if !s.is_empty() => s.parse::<rust_decimal::Decimal>().map_err(|e| {
            AppError::validation_displayable(format!("信用额度格式错误：{}（请输入有效数字）", e))
        })?,
        _ => rust_decimal::Decimal::ZERO,
    };

    // 缺省与词表收编到唯一模块：None→Ok(OTHER)（渠道未知不猜零售）；
    // Some 合法值原文返回（上方 payload.validate() 已拒非法值，此步不可能新增拒绝）。
    let customer_type =
        crate::constants::customer_type::validate(payload.customer_type.as_deref())?;

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
            country: Some(payload.country.unwrap_or_else(|| "中国".to_string())),
            postal_code: payload.postal_code,
            credit_limit,
            payment_terms: payload
                .payment_terms
                .unwrap_or(crate::constants::DEFAULT_PAYMENT_TERMS_DAYS),
            tax_id: payload.tax_id,
            bank_name: payload.bank_name,
            bank_account: payload.bank_account,
            customer_type,
            status: payload.status,
            customer_industry: payload.customer_industry,
            main_products: payload.main_products,
            annual_purchase: payload.annual_purchase,
            quality_requirement: payload.quality_requirement,
            inspection_standard: payload.inspection_standard,
            notes: payload.notes,
            created_by: Some(auth.user_id),
        })
        .await?;

    // 写响应收口：客户域字段级权限唯一实现（默认脱敏分支与读侧 mask_contact_fields_for_role
    // 同一权威列集合），不得整行原文回传 contact_phone/contact_email/address 明文
    let mut customer_json = serde_json::to_value(customer)
        .map_err(|e| AppError::internal(format!("序列化失败: {}", e)))?;
    apply_customer_field_permission(
        &state,
        auth.role_id,
        std::slice::from_mut(&mut customer_json),
    )
    .await;
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

    // 行级数据权限（IDOR）防护：复用 get_customer 内部的 check_resource_owner
    // （customers 归属列 = owner_id，dept=department_id；权威口径见
    // `migration/src/domain/rls_dept/mod.rs:74`），与 update_supplier/delete_supplier/delete_order
    // 的「先 get_X(Some(&data_scope_ctx))」写法同源——self 仅本人、dept 限可见部门集合、
    // all 放行；越权返回 403（permission_denied），不静默放行。
    let data_scope_ctx = auth.to_data_scope_context();
    let existing = customer_service
        .get_customer(id, Some(&data_scope_ctx))
        .await?;
    // 写侧越权门：上方的 get_customer 只保证"看得见"（DataScope::All 可读全域），
    // 跨 owner 的**写**另由 `crm_write_guard::ensure_cross_owner_write_allowed` 判定——
    // 须本人行，或持有权限键 resource_type="crm"/action="cross_owner_write"
    // （`CROSS_OWNER_WRITE_KEY`），否则 403。
    // 归属列与读门同取 `owner_id`（customers 的 RLS 归属列，权威口径见
    // `migration/src/domain/rls_dept/mod.rs`）；`created_by` 只是可空审计列，不参与归属判定。
    crate::handlers::crm_write_guard::ensure_cross_owner_write_allowed(
        state.db.clone(),
        &auth,
        &data_scope_ctx,
        Some(existing.owner_id),
        existing.department_id,
        "客户更新",
    )
    .await?;

    // 信用额度：缺省/空串写 None（不更新该列）；非空但解析失败出 VALIDATION 并外显原因，
    // 不走 `parse().ok()` 把非法输入静默当成"没改"
    let credit_limit = match payload.credit_limit.as_deref() {
        Some(s) if !s.is_empty() => Some(s.parse::<rust_decimal::Decimal>().map_err(|e| {
            AppError::validation_displayable(format!("信用额度格式错误：{}（请输入有效数字）", e))
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
            country: payload.country,
            customer_industry: payload.customer_industry,
            main_products: payload.main_products,
            annual_purchase: payload.annual_purchase,
            quality_requirement: payload.quality_requirement,
            inspection_standard: payload.inspection_standard,
            notes: payload.notes,
            // 操作人 user_id 随参数下传，由 update_with_audit 写入 audit_log 的 actor
            user_id: auth.user_id,
        })
        .await?;

    // 写响应收口：与 create_customer 同一实现，更新成功响应不得整行原文回传 PII
    // 写响应与读侧同源打码，不旁路回传原文 PII
    let mut customer_json = serde_json::to_value(customer)
        .map_err(|e| AppError::internal(format!("序列化失败: {}", e)))?;
    apply_customer_field_permission(
        &state,
        auth.role_id,
        std::slice::from_mut(&mut customer_json),
    )
    .await;
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

    // V15：行级数据权限（IDOR）防护——删除前先按当前用户数据范围校验资源归属，
    // 复用 get_customer 内部 check_resource_owner（customers 归属列 = owner_id），
    // 与 update_supplier/delete_supplier/delete_order
    // 的「先 get_X(Some(&data_scope_ctx))」写法同源；越权返回 403（permission_denied），不静默放行。
    let data_scope_ctx = auth.to_data_scope_context();
    let existing = customer_service
        .get_customer(id, Some(&data_scope_ctx))
        .await?;
    // 方案 A：删除是跨 owner 写的最强形态，须 owner 本人或持有代操作键（All 范围）；
    // 归属列与上方读门逐字同源（owner_id，非可空审计列 created_by）。
    crate::handlers::crm_write_guard::ensure_cross_owner_write_allowed(
        state.db.clone(),
        &auth,
        &data_scope_ctx,
        Some(existing.owner_id),
        existing.department_id,
        "客户删除",
    )
    .await?;

    // 操作人 user_id 随删除请求一并下传，供服务层写审计日志
    customer_service.delete_customer(id, auth.user_id).await?;
    Ok(Json(ApiResponse::success_with_message((), "客户删除成功")))
}

/// 客户查询参数
#[allow(dead_code, reason = "反序列化输入字段")]
#[derive(Debug, Deserialize)]
pub struct CustomerListQuery {
    pub page: Option<u64>,
    pub page_size: Option<u64>,
    pub status: Option<String>,
    pub customer_type: Option<String>,
    pub keyword: Option<String>,
    /// 敏感导出 fail-closed：导出审批令牌（仅 export 端点校验）
    pub download_token: Option<String>,
}

/// 构建下推到数据库层的字段级数据权限过滤器 `DataPermissionFilter`；调用方为本域的
/// `list_customers` / `get_customer` / `export_customers`。
///
/// 入参 `resource_type` 是权限资源名（客户域固定传 `"customer"`）。出参 `Ok(None)` =
/// 不加字段过滤（`role_id` 缺失、admin 由 `utils/admin_checker::is_admin_role`
/// （roles.code='admin'）判定、或 `data_scope="all"`）；`Ok(Some(filter))` 携带
/// allowed/hidden 列名集合，角色无权限配置行时 hidden 取 `DEFAULT_HIDDEN_FIELDS`。
/// 权限查询返回 `Err` 时记 warn 后按 `DEFAULT_HIDDEN_FIELDS` 降级，查询继续不中断。
async fn get_permission_filter(
    state: &AppState,
    auth: &AuthContext,
    resource_type: &str,
) -> Result<Option<DataPermissionFilter>, AppError> {
    let role_id = match auth.role_id {
        Some(id) => id,
        None => return Ok(None),
    };

    // 管理员角色或全量数据权限范围不做字段过滤；admin 判定走唯一权威源
    // `utils/admin_checker::is_admin_role`（roles.code='admin'），不用角色主键字面量
    if is_admin_role(&state.db, role_id).await || auth.data_scope.as_deref() == Some("all") {
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
            // 权限查询失败不阻断主流程：记 warn 后按 DEFAULT_HIDDEN_FIELDS 降级，
            // 即"读不到配置就按最严格默认隐藏列"，不放大可见范围
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
        item.get(k)
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string()
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

/// V15 P0-S12 + P0-S15 新增（Batch 474）：客户列表导出为带水印的 xlsx；端点：`GET /api/v1/customers/export`；设计要点： - 复用 `list_customers` 的查询参数（status/customer_type/keyword） - 通过 `CustomerService::list_customers_with_filter` 一次性查询（page_size=10000 防 OOM） - 行级数据权限
/// 与 `list_customers` 一致，调用 `to_data_scope_context` + `get_permission_filter` - 水印：操作员（AuthContext.username）+ 导出时间（ISO8601）+ 资源类型说明 - IP 暂为 None（middleware 未把 client_ip 注入 AuthContext，后续批次补齐）；规则 3：导出统一使用 xlsx 格式（含水印），错误用 AppError 表达。
pub async fn export_customers(
    State(state): State<AppState>,
    Query(query): Query<CustomerListQuery>,
    auth: AuthContext,
) -> Result<axum::response::Response, AppError> {
    // 敏感导出 fail-closed：校验审批令牌
    let approval =
        crate::services::export_approval_service::ExportApprovalService::new(state.db.clone())
            .enforce_export_download(query.download_token.as_deref(), "customer")
            .await?;

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
    // 敏感导出 fail-closed：记录令牌消费（流式导出用 filename + row_count 作逻辑标识）
    let _ = crate::services::export_approval_service::ExportApprovalService::new(state.db.clone())
        .record_download(
            approval.id,
            filename.clone(),
            row_count as i64,
            String::new(),
        )
        .await;
    record_customers_export_audit(&state, &auth, row_count);
    build_xlsx_response_with_watermark(&table, &filename, &watermark)
}
