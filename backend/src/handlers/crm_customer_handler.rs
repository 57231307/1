//! CRM客户管理 Handler
//!
//! 提供客户信息维护、标签管理、联系人管理等 API 接口

use axum::{
    Json,
    extract::{Path, Query, State},
};
use sea_orm::{ActiveModelTrait, EntityTrait, Set};
use serde::Deserialize;
use validator::Validate;

use crate::container::AppState;
use crate::middleware::auth_context::AuthContext;
use crate::models::crm_tag;
use crate::models::dto::crm_dto::{CreateLeadRequest, LeadQuery, UpdateLeadRequest};
use crate::services::crm::cust::CrmService;
use crate::services::customer_service::{
    CreateCustomerContactRequest, CustomerService, UpdateCustomerArgs, UpdateCustomerContactRequest,
};
use crate::utils::error::AppError;
use crate::utils::response::ApiResponse;

/// 客户查询参数
#[allow(dead_code, reason = "反序列化输入字段")]
#[derive(Debug, Deserialize)]
pub struct CustomerQueryParams {
    pub page: Option<u64>,
    pub page_size: Option<u64>,
    pub status: Option<String>,
    // 批次 111 P1-10：keyword 接入 LeadQuery 模糊搜索（移除 dead_code 标注）
    pub keyword: Option<String>,
}

/// P1-2h 修复（批次 81 v1 复审）：添加标签请求 DTO
/// 替代 add_tags 中的 Json<serde_json::Value>，提供强类型校验
#[allow(dead_code, reason = "反序列化输入字段")]
#[derive(Debug, Deserialize, Validate)]
pub struct AddTagsDto {
    /// 标签列表：必填，至少 1 个标签
    #[validate(length(min = 1, message = "标签列表不能为空"))]
    pub tags: Vec<String>,
}

/// P1-2h 修复（批次 81 v1 复审）：创建标签请求 DTO
/// 批次 122 v8 复审 P1 修复：增加 category 字段，真实持久化到 crm_tag 表
#[allow(dead_code, reason = "反序列化输入字段")]
#[derive(Debug, Deserialize, Validate)]
pub struct CreateTagDto {
    /// 标签名称：必填，长度至少 1
    #[validate(length(min = 1, max = 30, message = "标签名称长度必须在1到30字符之间"))]
    pub name: String,
    /// 颜色：可选，缺失时默认 "#1890ff"
    #[validate(length(max = 20, message = "颜色长度不能超过20字符"))]
    pub color: Option<String>,
    /// 标签分类：可选，如 customer/lead/supplier
    #[validate(length(max = 50, message = "分类长度不能超过50字符"))]
    pub category: Option<String>,
}

/// 客户域**字段级**数据权限的唯一实现（形态与线索侧
/// `crm_handler::apply_lead_field_permission` 逐项同构，不复用其函数体是因为判定源按
/// resource_type 分行）。消费面（终局口径：客户域读写出口统一权限版）：
/// - `customer_handler.rs` 四出口：list_customers/get_customer（读）与
///   create_customer/update_customer（写响应）——customers 形状行同一判定源；
/// - 本文件出口：list_customers/get_customer/list_contacts（增强页读）与
///   update_customer/create_contact/update_contact（写响应）——同页四形状一致，
///   contacts/客户行掩码保留键（掩码非删键契约在本函数默认分支与 filter_fields_batch
///   两侧同时成立）；
/// - `crm_lead` 形状出口（`crm_handler.rs` 读写出口及本页 `create_customer`/`add_tags`
///   写响应）**不**使用本函数，走线索侧 `apply_lead_field_permission`——
///   两域权限判定语义不同不强行统一；
/// - `sales_fabric_order_handler.rs` 属销售域，维持 `mask_contact_fields_for_role`
///   直调（标准销售订单读出口的既有唯一掩码实现），不消费客户域权限行。
/// 判定源 = `data_permission_service.get_role_data_permission(role_id, "customer")`，
/// 但**两层叠加而非二选一**：
/// 1. 先 `CrmService::mask_customer_pii_defaults`（掩码列集合 = `utils/field_mask` 权威定义
///    `mask_contact_fields_for_role`，非 admin 另加 `address` 整键移除；admin 由该函数
///    自身放行原文），非 admin 恒定执行；
/// 2. 再叠加权限行的 `filter_fields_batch`（allowed 白名单保留 / hidden 移除）。
/// 之所以不能像线索域那样"配了权限行就只按配置处理"：`filter_fields`（
/// `services/data_permission_service.rs:202-222`）只会**删键**、不会把值还原成原文，
/// 而本域改造前的读出口（`customer_handler.rs` 列表/详情）是**无条件掩码**的——
/// 若走"配置即放行"，配了 `allowed_fields` 含 `contact_phone` 的非 admin 角色会从
/// 掩码变原文，等于借本批"收敛唯一实现"顺手放松权限；同时增强入口（本文件读出口）
/// 改造前完全不掩码，两种入口结果必须对每一种角色都一致，否则"同资源不同入口"
/// 又成旁路。叠加后的结果对每一种角色都 ≥ 改造前。
/// 代价与待裁定：本域 `allowed_fields` 因此**不能**作为"放行 PII 原文"的通道
/// （线索/商机域保持其既有"配置即授权"语义不变）。若产品要让客户域也支持
/// "显式配置即原文"，那是口径决策（需同时决定两个入口如何同步放松），交用户裁定。
/// 权限查询 `Err` 与 `role_id` 缺失：记 warn 后仍走第 1 层默认脱敏（fail-closed）。
/// 本文件不重写列名清单。
///
/// 本函数只处理出参：不改状态码、不外显任何拒绝原因（权限拒绝仍走
/// `AppError::permission_denied` 的固定脱敏信封 + `FORBIDDEN` 码，真实原因只进日志）。
pub(crate) async fn apply_customer_field_permission(
    state: &AppState,
    role_id: Option<i32>,
    rows: &mut [serde_json::Value],
) {
    // 第 1 层：默认脱敏对非 admin 恒定执行（admin 由 mask_customer_pii_defaults 自身放行原文）
    for row in rows.iter_mut() {
        *row = CrmService::mask_customer_pii_defaults(std::mem::take(row), role_id);
    }

    // 第 2 层：叠加权限行的 allowed/hidden（只删键，不会把第 1 层的掩码还原成原文）
    if let Some(rid) = role_id {
        match state
            .data_permission_service
            .get_role_data_permission(rid, "customer")
            .await
        {
            Ok(Some(permission)) => {
                state.data_permission_service.filter_fields_batch(
                    rows,
                    &permission.allowed_fields,
                    &permission.hidden_fields,
                );
            }
            // Ok(None)：无权限行 → 只保留第 1 层默认脱敏
            Ok(None) => {}
            // 查询失败必须显式记 warn（不静默），第 1 层默认脱敏已经生效即 fail-closed
            Err(error) => {
                tracing::warn!(
                    %error,
                    role_id = rid,
                    resource_type = "customer",
                    "角色数据权限查询失败，客户域出参按默认脱敏处理（不叠加配置过滤）"
                );
            }
        }
    }
}

/// POST /api/v1/erp/crm/customers - 创建客户（通过线索）
pub async fn create_customer(
    State(state): State<AppState>,
    auth: AuthContext,
    Json(req): Json<CreateLeadRequest>,
) -> Result<Json<ApiResponse<serde_json::Value>>, AppError> {
    let service = CrmService::new(state.db.clone());
    let lead = service
        .create_lead(req, auth.user_id, &auth.username)
        .await?;
    // 本端点落的是线索域行（create_lead → crm_lead::Model，含 mobile_phone/tel_phone/email/
    // address），写响应因此复用**线索侧**同一个字段级权限唯一实现（与 crm_handler 的
    // create_lead/update_lead 写响应同源），不再整行原文直出。
    let mut value = serde_json::to_value(lead)?;
    crate::handlers::crm_handler::apply_lead_field_permission(
        &state,
        auth.role_id,
        std::slice::from_mut(&mut value),
    )
    .await;
    Ok(Json(ApiResponse::success(value)))
}

/// GET /api/v1/erp/crm/customers - 获取客户列表（线索列表）
pub async fn list_customers(
    State(state): State<AppState>,
    auth: AuthContext,
    Query(params): Query<CustomerQueryParams>,
) -> Result<Json<ApiResponse<serde_json::Value>>, AppError> {
    let service = CrmService::new(state.db.clone());

    let query = LeadQuery {
        page: params.page,
        page_size: params.page_size,
        lead_status: params.status,
        // 批次 111 P1-10：透传 keyword 到 LeadQuery，由 list_leads 服务执行模糊搜索
        source: None,
        keyword: params.keyword,
        // v11 批次 153 P2-A：industry 字段新增，此处不过滤
        industry: None,
    };

    // V15 P0-S01：提取行级数据权限上下文
    let data_scope_ctx = auth.to_data_scope_context();
    let result = service.list_leads(query, Some(&data_scope_ctx)).await?;
    // 增强页列表整行原文直出 = 同一份线索行在标准入口打码、本入口出原文的旁路，
    // 收口到本文件客户侧唯一实现（裁定口径：增强页出参一致性优先——同页
    // GET/:id、PUT、contacts 三形状统一走 resource_type=customer 权限行 +
    // mask_customer_pii_defaults 默认掩码，不引用线索域 crm_lead 行配置，两域
    // 权限判定语义不同不强行统一；掩码列集合仍全仓唯一 = utils/field_mask）。
    let mut value = serde_json::to_value(result)?;
    if let Some(list) = crate::handlers::crm_handler::paginated_list_array_mut(&mut value) {
        apply_customer_field_permission(&state, auth.role_id, list).await;
    }
    Ok(Json(ApiResponse::success(value)))
}

/// GET /api/v1/erp/crm/customers/:id - 获取客户详情
pub async fn get_customer(
    State(state): State<AppState>,
    auth: AuthContext,
    Path(id): Path<i32>,
) -> Result<Json<ApiResponse<serde_json::Value>>, AppError> {
    let service = CrmService::new(state.db.clone());
    // V15 P0-S01：提取行级数据权限上下文（IDOR 防护）
    let data_scope_ctx = auth.to_data_scope_context();
    let lead = service.get_lead(id, Some(&data_scope_ctx)).await?;
    // 增强页详情与本页列表/更新/联系人同挂客户侧唯一实现（裁定口径见 list_customers
    // 处注释）：mobile_phone/tel_phone/email 命中权威掩码列集合、address 整键移除，
    // 不引用 crm_lead 行配置；整行原文直出即旁路。
    let mut value = serde_json::to_value(lead)?;
    apply_customer_field_permission(&state, auth.role_id, std::slice::from_mut(&mut value)).await;
    Ok(Json(ApiResponse::success(value)))
}

/// CRM 增强客户更新请求 DTO：客户编辑弹窗提交字段，含状态（active/inactive）
/// 走客户域 CustomerService 更新链路落库
#[derive(Debug, serde::Deserialize)]
pub struct UpdateEnhancedCustomerRequest {
    pub customer_name: Option<String>,
    pub contact_person: Option<String>,
    pub contact_phone: Option<String>,
    pub contact_email: Option<String>,
    pub address: Option<String>,
    pub customer_type: Option<String>,
    /// 税号（前端字段名 tax_number → 后端 tax_id）
    pub tax_number: Option<String>,
    pub credit_limit: Option<rust_decimal::Decimal>,
    pub bank_name: Option<String>,
    pub bank_account: Option<String>,
    /// 客户状态（active/inactive）——31c 停用矩阵的核心字段
    pub status: Option<String>,
}

/// PUT /api/v1/erp/crm/customers/enhanced/:id - 更新客户（增强路由，客户域落库含 status）
pub async fn update_customer(
    State(state): State<AppState>,
    auth: AuthContext,
    Path(id): Path<i32>,
    Json(req): Json<UpdateEnhancedCustomerRequest>,
) -> Result<Json<ApiResponse<serde_json::Value>>, AppError> {
    let customer_service = CustomerService::new(state.db.clone(), state.search_client.clone());

    // 行级数据权限门（与其它写入口同判定源，非新增机制）：增强页 PUT 落的是 customers 表行，
    // 故先走本域既有 `get_customer(id, Some(&ctx))` 预检——customers 的归属列是
    // **owner_id**（`migration/src/domain/rls_dept/mod.rs:74`「customers/crm_lead/
    // crm_opportunity 以 owner_id 为归属列，suppliers/sales_orders 以 created_by」），
    // dept=department_id，判定源与标准入口（`customer_handler.rs` 写出口）及
    // `utils/data_scope.rs::check_resource_owner` 逐字同源（同 `merge_leads`/
    // `claim_from_pool` 的"先判后写"收口形态）。行不可见即整笔由 `permission_denied`
    // 出 403（固定脱敏文案 + FORBIDDEN 码，真实原因只进服务端日志），不存在
    // "跳过不可见行继续写"的降级。admin 的 `DataScope::All` 通道按
    // `check_resource_owner` 既有语义原样通过，本门只把"完全没有门"变成"与其它入口同门"。
    let data_scope_ctx = auth.to_data_scope_context();
    let existing = customer_service
        .get_customer(id, Some(&data_scope_ctx))
        .await?;
    // 方案 A（用户 2026-10-02 裁定）：可见 ≠ 可改。跨 owner 写另需 crm/cross_owner_write 键，
    // 放行时代操作事实进结构化日志（各写出口本身已落 update_with_audit 审计行）。
    // 归属列与上方读门逐字同源（owner_id），不取可空审计列 created_by。
    crate::handlers::crm_write_guard::ensure_cross_owner_write_allowed(
        state.db.clone(),
        &auth,
        &data_scope_ctx,
        Some(existing.owner_id),
        existing.department_id,
        "客户更新（增强入口）",
    )
    .await?;

    // 批次 94 P2-10：注入真实操作人 user_id 用于审计日志
    let customer = customer_service
        .update_customer(UpdateCustomerArgs {
            customer_id: id,
            customer_name: req.customer_name,
            contact_person: req.contact_person,
            contact_phone: req.contact_phone,
            contact_email: req.contact_email,
            address: req.address,
            city: None,
            province: None,
            postal_code: None,
            credit_limit: req.credit_limit,
            payment_terms: None,
            tax_id: req.tax_number,
            bank_name: req.bank_name,
            bank_account: req.bank_account,
            customer_type: req.customer_type,
            status: req.status,
            country: None,
            customer_industry: None,
            main_products: None,
            annual_purchase: None,
            quality_requirement: None,
            inspection_standard: None,
            notes: None,
            user_id: auth.user_id,
        })
        .await?;

    // 写响应不得整行原文回传（contact_phone/contact_email/address 明文），与读出口同源收口
    let mut value = serde_json::to_value(customer)?;
    apply_customer_field_permission(&state, auth.role_id, std::slice::from_mut(&mut value)).await;
    Ok(Json(ApiResponse::success(value)))
}

/// DELETE /api/v1/erp/crm/customers/:id - 删除客户
pub async fn delete_customer(
    State(state): State<AppState>,
    auth: AuthContext,
    Path(id): Path<i32>,
) -> Result<Json<ApiResponse<serde_json::Value>>, AppError> {
    let service = CrmService::new(state.db.clone());
    // 行级数据权限门（与其它写入口同判定源，非新增机制）：本删除落点是 crm_lead 行
    // （与本页详情出口 `get_lead(id, Some(&ctx))` 同一张表），故先用 ctx 走
    // `get_lead` 预检——判定源即 `utils/data_scope.rs::check_resource_owner`
    // （owner=owner_id、dept=department_id，与 `crm_handler.rs` update_lead 同一收口形态）。
    // 行不可见即整笔 403（`permission_denied` 固定脱敏文案 + FORBIDDEN 码，
    // 真实原因只进服务端日志），杜绝仅凭 RBAC 键按 id 删他人行。
    // admin 的 `DataScope::All` 通道按既有语义原样通过，未收紧。
    let data_scope_ctx = auth.to_data_scope_context();
    let existing = service.get_lead(id, Some(&data_scope_ctx)).await?;
    // 方案 A（用户 2026-10-02 裁定）：可见 ≠ 可删。增强入口删除落的是 crm_lead 行，
    // 跨 owner 写须 owner 本人或持有 crm/cross_owner_write 代表键（放行即 tracing::info
    // 留痕）；无键 403 出参恒为固定脱敏常量。门后 service.delete_lead 的 FK 引用预校验
    // 与并发兜底（G-221 收口，被引用线索删 400 不裸 500）逐字保留。
    crate::handlers::crm_write_guard::ensure_cross_owner_write_allowed(
        state.db.clone(),
        &auth,
        &data_scope_ctx,
        Some(existing.owner_id),
        existing.department_id,
        "客户删除（增强入口，落线索行）",
    )
    .await?;
    // 批次 94 P2-10：注入真实操作人 user_id 用于审计日志
    service.delete_lead(id, auth.user_id).await?;
    Ok(Json(ApiResponse::success(
        serde_json::json!({"deleted": true}),
    )))
}

/// POST /api/v1/erp/crm/customers/:id/tags - 添加标签
pub async fn add_tags(
    State(state): State<AppState>,
    auth: AuthContext,
    Path(id): Path<i32>,
    Json(req): Json<AddTagsDto>,
) -> Result<Json<ApiResponse<serde_json::Value>>, AppError> {
    // P1-2h 修复（批次 81 v1 复审）：强类型 DTO + validator 替代 Json<Value>
    req.validate().map_err(AppError::from)?;

    let service = CrmService::new(state.db.clone());

    // 方案 A（用户 2026-10-02 裁定）：挂标签落的是 crm_lead 行的 tags 列，同属**线索更新**。
    // 修复前本入口无任何归属校验（service.update_lead 内不注入 ctx），任意用户可按 id
    // 改他人线索标签；现先按 ctx 读行（与其它写入口同判定源）再过跨 owner 写门。
    let data_scope_ctx = auth.to_data_scope_context();
    let existing = service.get_lead(id, Some(&data_scope_ctx)).await?;
    crate::handlers::crm_write_guard::ensure_cross_owner_write_allowed(
        state.db.clone(),
        &auth,
        &data_scope_ctx,
        Some(existing.owner_id),
        existing.department_id,
        "线索标签更新",
    )
    .await?;

    let update_req = UpdateLeadRequest {
        tags: Some(req.tags),
        ..Default::default()
    };

    // 批次 94 P2-10：注入真实操作人 user_id 用于审计日志
    let lead = service.update_lead(id, update_req, auth.user_id).await?;
    // 出参形状是线索行（crm_lead::Model），写响应复用线索侧同一字段级权限唯一实现
    let mut value = serde_json::to_value(lead)?;
    crate::handlers::crm_handler::apply_lead_field_permission(
        &state,
        auth.role_id,
        std::slice::from_mut(&mut value),
    )
    .await;
    Ok(Json(ApiResponse::success(value)))
}

/// GET /api/v1/erp/crm/customers/:id/contacts - 获取联系人列表；批次 90b
/// P2-12：原实现从 crm_lead 拼接 JSON 伪联系人，改为查 customer_contacts 表真实数据。
pub async fn list_contacts(
    State(state): State<AppState>,
    auth: AuthContext,
    Path(customer_id): Path<i32>,
) -> Result<Json<ApiResponse<serde_json::Value>>, AppError> {
    let service = CustomerService::new(state.db.clone(), state.search_client.clone());
    let contacts = service.list_customer_contacts(customer_id).await?;

    // 联系人列表读出口（终局口径：客户域读写出口统一权限版）：本文件唯一实现
    // `apply_customer_field_permission`（默认脱敏分支=权威掩码列集合 utils/field_mask，
    // phone/email 掩码**保留键**不删键；有权限行走同一 filter_fields_batch），与本页
    // 写响应及标准客户入口同一判定源；整行原文直出即旁路。
    // 形状漂移（非数组）显式报错不静默。
    let mut value = serde_json::to_value(contacts)?;
    match value.as_array_mut() {
        Some(rows) => apply_customer_field_permission(&state, auth.role_id, rows).await,
        None => {
            tracing::error!(
                "联系人列表出参不是数组，客户域字段级数据权限未应用（形状漂移，不静默）"
            )
        }
    }
    Ok(Json(ApiResponse::success(value)))
}

/// POST /api/v1/erp/crm/customers/:id/contacts - 创建联系人；批次 90b P2-12：实现前端 detail.vue "新增联系人" 占位符的真实后端。
#[axum::debug_handler]
pub async fn create_contact(
    State(state): State<AppState>,
    auth: AuthContext,
    Path(customer_id): Path<i32>,
    Json(req): Json<CreateCustomerContactRequest>,
) -> Result<Json<ApiResponse<serde_json::Value>>, AppError> {
    req.validate().map_err(AppError::from)?;

    let service = CustomerService::new(state.db.clone(), state.search_client.clone());
    let contact = service
        .create_customer_contact(customer_id, req, auth.user_id)
        .await?;

    // 联系人行的 phone/email 属同一权威 PII 列集合，写响应不得整行原文直出
    let mut value = serde_json::to_value(contact)?;
    apply_customer_field_permission(&state, auth.role_id, std::slice::from_mut(&mut value)).await;
    Ok(Json(ApiResponse::success_with_message(
        value,
        "联系人创建成功",
    )))
}

/// PUT /api/v1/erp/crm/customers/:id/contacts/:contact_id - 更新联系人；批次 90b P2-12：实现联系人编辑功能。
#[axum::debug_handler]
pub async fn update_contact(
    Path((_customer_id, contact_id)): Path<(i32, i32)>,
    State(state): State<AppState>,
    auth: AuthContext,
    Json(req): Json<UpdateCustomerContactRequest>,
) -> Result<Json<ApiResponse<serde_json::Value>>, AppError> {
    req.validate().map_err(AppError::from)?;

    let service = CustomerService::new(state.db.clone(), state.search_client.clone());
    let contact = service
        .update_customer_contact(contact_id, req, auth.user_id)
        .await?;

    // 同 create_contact：联系人写响应走客户域字段级权限唯一实现，不整行原文回传
    let mut value = serde_json::to_value(contact)?;
    apply_customer_field_permission(&state, auth.role_id, std::slice::from_mut(&mut value)).await;
    Ok(Json(ApiResponse::success_with_message(
        value,
        "联系人更新成功",
    )))
}

/// DELETE /api/v1/erp/crm/customers/:id/contacts/:contact_id - 删除联系人；批次 90b P2-12：实现联系人删除功能。
pub async fn delete_contact(
    Path((_customer_id, contact_id)): Path<(i32, i32)>,
    State(state): State<AppState>,
    _auth: AuthContext,
) -> Result<Json<ApiResponse<()>>, AppError> {
    let service = CustomerService::new(state.db.clone(), state.search_client.clone());
    service.delete_customer_contact(contact_id).await?;

    Ok(Json(ApiResponse::success_with_message(
        (),
        "联系人删除成功",
    )))
}

/// GET /api/v1/erp/crm/tags - 获取标签列表；批次 122 v8 复审 P1 修复：原返回硬编码 5 个标签，改为查 crm_tag 表真实数据。
pub async fn list_tags(
    State(state): State<AppState>,
    _auth: AuthContext,
) -> Result<Json<ApiResponse<Vec<crm_tag::Model>>>, AppError> {
    let tags = crm_tag::Entity::find().all(&*state.db).await?;
    Ok(Json(ApiResponse::success(tags)))
}

/// POST /api/v1/erp/crm/tags - 创建标签；批次 122 v8 复审 P1 修复：原 create_tag 仅用时间戳生成假 id
/// 返回，不持久化。 现真实 INSERT 到 crm_tag 表，返回含 id/name/color/category/created_at 的完整记录。
pub async fn create_tag(
    State(state): State<AppState>,
    auth: AuthContext,
    Json(req): Json<CreateTagDto>,
) -> Result<Json<ApiResponse<crm_tag::Model>>, AppError> {
    req.validate().map_err(AppError::from)?;

    let new_tag = crm_tag::ActiveModel {
        name: Set(req.name),
        color: Set(req.color.unwrap_or_else(|| "#1890ff".to_string())),
        category: Set(req.category),
        created_by: Set(Some(auth.user_id)),
        ..Default::default()
    };

    let tag = new_tag.insert(&*state.db).await?;
    Ok(Json(ApiResponse::success_with_message(tag, "标签创建成功")))
}

/// DELETE /api/v1/erp/crm/tags/:id - 删除标签；批次 122 v8 复审 P1 修复：原 delete_tag 直接返回
/// {"deleted": true} 空操作。 现真实 DELETE FROM crm_tag WHERE id = ?，标签不存在时返回 404。
pub async fn delete_tag(
    State(state): State<AppState>,
    _auth: AuthContext,
    Path(id): Path<i32>,
) -> Result<Json<ApiResponse<serde_json::Value>>, AppError> {
    let result = crm_tag::Entity::delete_by_id(id).exec(&*state.db).await?;

    if result.rows_affected == 0 {
        return Err(AppError::not_found(format!("标签 {} 未找到", id)));
    }

    Ok(Json(ApiResponse::success_with_message(
        serde_json::json!({"deleted": true, "id": id}),
        "标签删除成功",
    )))
}

/// GET /api/v1/erp/crm/customers/:id/tags - 获取客户已挂载标签对象列表
/// 单次 JOIN 查询（customer_tag INNER JOIN crm_tag），无 N+1。
pub async fn list_customer_tags(
    State(state): State<AppState>,
    _auth: AuthContext,
    Path(id): Path<i32>,
) -> Result<Json<ApiResponse<Vec<crate::services::crm::CustomerTagBrief>>>, AppError> {
    use sea_orm::{ColumnTrait, QueryFilter, QueryOrder};

    let tags: Vec<crate::services::crm::CustomerTagBrief> = crm_tag::Entity::find()
        .inner_join(crate::models::customer_tag::Entity)
        .filter(crate::models::customer_tag::Column::CustomerId.eq(id))
        .order_by(crm_tag::Column::Id, sea_orm::Order::Asc)
        .into_model::<crate::services::crm::CustomerTagBrief>()
        .all(&*state.db)
        .await?;

    Ok(Json(ApiResponse::success(tags)))
}

/// POST /api/v1/erp/crm/customers/:id/tags/:tagId - 给客户挂载标签（幂等）
/// 如果关联已存在，不报错直接返回成功。
pub async fn attach_customer_tag(
    State(state): State<AppState>,
    auth: AuthContext,
    Path((customer_id, tag_id)): Path<(i32, i32)>,
) -> Result<Json<ApiResponse<serde_json::Value>>, AppError> {
    use crate::models::customer_tag;
    use sea_orm::{ColumnTrait, QueryFilter};

    // 校验客户存在
    let _customer = crate::models::customer::Entity::find_by_id(customer_id)
        .one(&*state.db)
        .await?
        .ok_or_else(|| AppError::not_found(format!("客户 {} 不存在", customer_id)))?;

    // 校验标签存在
    let _tag = crm_tag::Entity::find_by_id(tag_id)
        .one(&*state.db)
        .await?
        .ok_or_else(|| AppError::not_found(format!("标签 {} 不存在", tag_id)))?;

    // 幂等检查：已存在则直接返回成功
    let existing = customer_tag::Entity::find()
        .filter(customer_tag::Column::CustomerId.eq(customer_id))
        .filter(customer_tag::Column::TagId.eq(tag_id))
        .one(&*state.db)
        .await?;

    if existing.is_some() {
        return Ok(Json(ApiResponse::success_with_message(
            serde_json::json!({"customer_id": customer_id, "tag_id": tag_id, "already_exists": true}),
            "标签已关联",
        )));
    }

    let new_rel = customer_tag::ActiveModel {
        customer_id: Set(customer_id),
        tag_id: Set(tag_id),
        created_by: Set(Some(auth.user_id)),
        ..Default::default()
    };
    new_rel.insert(&*state.db).await?;

    Ok(Json(ApiResponse::success_with_message(
        serde_json::json!({"customer_id": customer_id, "tag_id": tag_id}),
        "标签关联成功",
    )))
}

/// DELETE /api/v1/erp/crm/customers/:id/tags/:tagId - 解除客户标签关联
pub async fn detach_customer_tag(
    State(state): State<AppState>,
    _auth: AuthContext,
    Path((customer_id, tag_id)): Path<(i32, i32)>,
) -> Result<Json<ApiResponse<serde_json::Value>>, AppError> {
    use crate::models::customer_tag;
    use sea_orm::{ColumnTrait, QueryFilter};

    let result = customer_tag::Entity::delete_many()
        .filter(customer_tag::Column::CustomerId.eq(customer_id))
        .filter(customer_tag::Column::TagId.eq(tag_id))
        .exec(&*state.db)
        .await?;

    if result.rows_affected == 0 {
        return Err(AppError::not_found(format!(
            "客户 {} 与标签 {} 的关联不存在",
            customer_id, tag_id
        )));
    }

    Ok(Json(ApiResponse::success_with_message(
        serde_json::json!({"customer_id": customer_id, "tag_id": tag_id, "deleted": true}),
        "标签解除成功",
    )))
}
