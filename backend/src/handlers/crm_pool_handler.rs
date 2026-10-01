//! CRM客户公海 Handler
//!
//! 提供客户公海池的列表查询、领取和回收功能
//! V15 P0-S08 修复：新增公海规则 CRUD 接口（保护期/领取上限/最大持有数）

use axum::{
    Json,
    extract::{Path, Query, State},
};
use serde::Deserialize;

use crate::container::AppState;
use crate::middleware::auth_context::AuthContext;
use crate::models::dto::crm_dto::BatchClaimRequest;
use crate::models::status::crm_lead as lead_status;
use crate::services::crm::cust::CrmService;
// V15 P0-S08：公海规则服务
use crate::services::crm::pool::PoolRuleService;
use crate::utils::data_scope::check_resource_owner;
use crate::utils::error::AppError;
use crate::utils::response::ApiResponse;

/// 公海客户查询参数
#[allow(dead_code, reason = "反序列化输入字段")]
#[derive(Debug, Deserialize)]
pub struct PoolQueryParams {
    pub page: Option<u64>,
    pub page_size: Option<u64>,
    // 批次 111 P1-10：source / keyword 接入 LeadQuery 过滤（移除 dead_code 标注）
    pub source: Option<String>,
    // v11 批次 153 P2-A：接入 industry 过滤（crm_lead.industry 列已通过 m0043 迁移添加）
    pub industry: Option<String>,
    pub keyword: Option<String>,
}

/// 领取客户请求
#[allow(dead_code, reason = "反序列化输入字段")]
#[derive(Debug, Deserialize)]
pub struct ClaimRequest {
    pub lead_id: i32,
}

/// 回收客户请求
#[allow(dead_code, reason = "反序列化输入字段")]
#[derive(Debug, Deserialize)]
pub struct RecycleRequest {
    pub lead_id: i32,
    pub reason: Option<String>,
}

/// GET /api/v1/erp/crm/pool - 获取公海客户列表
pub async fn list_pool(
    State(state): State<AppState>,
    auth: AuthContext,
    Query(params): Query<PoolQueryParams>,
) -> Result<Json<ApiResponse<serde_json::Value>>, AppError> {
    let service = CrmService::new(state.db.clone());

    let page = params.page.unwrap_or(1).clamp(1, 1000); // 批次 95 P3-3~8：分页 clamp 防 DoS
    let page_size = params.page_size.unwrap_or(20).clamp(1, 100);

    // 查询公海客户（owner_id为空或特定状态的线索）
    let query = crate::models::dto::crm_dto::LeadQuery {
        lead_status: Some(lead_status::POOL.to_string()),
        // 批次 111 P1-10：透传 source / keyword 到 LeadQuery，由 list_leads 服务执行过滤
        source: params.source,
        keyword: params.keyword,
        // v11 批次 153 P2-A：透传 industry 到 LeadQuery
        industry: params.industry,
        page: Some(page),
        page_size: Some(page_size),
    };

    // 行级数据权限按正常列表入口（crm_handler::list_leads）同口径接入：
    // scope 上下文缺省会整体跳过服务层 apply_department_scope_with_pool 行级过滤
    // （services/crm/lead.rs:143-151）——公海入口跨行泄露根因，禁止再省略该实参。
    let data_scope_ctx = auth.to_data_scope_context();
    let result = service.list_leads(query, Some(&data_scope_ctx)).await?;

    // 转换分页结果为列表
    let data = result
        .get("data")
        .cloned()
        .unwrap_or(serde_json::Value::Null);
    let total = result
        .get("total")
        .and_then(|v| v.as_u64())
        .unwrap_or_default();

    // 转换为响应格式
    let mut items: Vec<serde_json::Value> = data
        .as_array()
        .cloned()
        .unwrap_or_default()
        .into_iter()
        .map(|lead| {
            // lead 是 serde_json::Value，使用 .get("field") 访问
            let created_at_str = lead
                .get("created_at")
                .and_then(|v| v.as_str())
                .map(|s| s.to_string());
            let days_in_pool = created_at_str
                .as_deref()
                .and_then(|s| chrono::DateTime::parse_from_rfc3339(s).ok())
                .map(|dt| {
                    chrono::Utc::now()
                        .signed_duration_since(dt.with_timezone(&chrono::Utc))
                        .num_days()
                })
                .unwrap_or_default();

            serde_json::json!({
                "id": lead.get("id").cloned().unwrap_or(serde_json::Value::Null),
                "lead_no": lead.get("lead_no").cloned().unwrap_or(serde_json::Value::Null),
                "company_name": lead.get("company_name").cloned().unwrap_or(serde_json::Value::Null),
                "contact_name": lead.get("contact_name").cloned().unwrap_or(serde_json::Value::Null),
                "mobile_phone": lead.get("mobile_phone").cloned().unwrap_or(serde_json::Value::Null),
                "email": lead.get("email").cloned().unwrap_or(serde_json::Value::Null),
                "lead_source": lead.get("lead_source").cloned().unwrap_or(serde_json::Value::Null),
                "created_at": created_at_str,
                "days_in_pool": days_in_pool,
            })
        })
        .collect();

    // 字段级数据权限：判定分支与 crm_handler::list_leads 同构，复用同一判定源
    // （data_permission_service.get_role_data_permission，admin 依据 roles.code='admin'，
    // admin_checker.rs:87）与同一掩码实现（utils/field_mask::mask_phone/mask_email），
    // 公海侧不另造更松的规则：
    // - 配置了数据权限行 → filter_fields_batch（hidden/allowed 优先，不叠加默认打码）；
    // - 无权限行且 role_id != 1 → phone/email 掩码（查询 Err 同走此分支，fail-closed）。
    if let Some(role_id) = auth.role_id {
        if let Ok(Some(permission)) = state
            .data_permission_service
            .get_role_data_permission(role_id, "crm_lead")
            .await
        {
            state.data_permission_service.filter_fields_batch(
                &mut items,
                &permission.allowed_fields,
                &permission.hidden_fields,
            );
        } else if role_id != 1 {
            for lead in items.iter_mut() {
                if let Some(obj) = lead.as_object_mut() {
                    // 输出键为 crm_lead 真实列 mobile_phone（models/crm_lead.rs:40）；
                    // 本入口出参由上方按挑选字段构造，不含 address 键，无需再移除。
                    if let Some(phone) = obj.get("mobile_phone").and_then(|v| v.as_str()) {
                        obj.insert(
                            "mobile_phone".to_string(),
                            serde_json::Value::String(crate::utils::field_mask::mask_phone(phone)),
                        );
                    }
                    if let Some(email) = obj.get("email").and_then(|v| v.as_str()) {
                        obj.insert(
                            "email".to_string(),
                            serde_json::Value::String(crate::utils::field_mask::mask_email(email)),
                        );
                    }
                }
            }
        }
    }

    Ok(Json(ApiResponse::success(serde_json::json!({
        "items": items,
        "total": total,
        "page": page,
        "page_size": page_size,
    }))))
}

/// POST /api/v1/erp/crm/pool/claim - 从公海领取客户
///
/// 行级边界（#204）：领取**只能作用于公海行**。
/// - RBAC 层：本路径 `/api/v1/erp/crm/pool/claim` 由 URL 段推导出资源 `pool`、
///   动作 `create`（`middleware/permission.rs:259` `extract_resource_info`；`crm` 是模块
///   前缀 `utils/path_utils.rs:75`，`resolve_module_prefixed_resource` 默认臂保留原名），
///   admin 角色在 `check_permission`（`permission.rs:536` `admin_checker::is_admin_role`）
///   整体放行。claim 与 recycle 推导出的是**同一个键 `pool:create`**，RBAC 无法区分二者，
///   所以"谁能对哪一行写"只能由行级数据权限决定。
/// - 行级层：公海行的业务语义是"无归属人、对有权进公海者开放"（`utils/data_scope.rs:212-214`
///   RLS 公海放行注释），因此这里**不套用私海归属门**（否则 self 销售领取他人公海行会被
///   打死——`utils/data_scope.rs` Self 分支的公海放行属 #203 待用户拍板项，本处不依赖它）；
///   一旦命中非公海行（即他人私海），立即回落到与 `get_lead` 正常路径同一个
///   `check_resource_owner`：`DataScope::All` 可越界、`Dept` 需资源部门在可见集合内、
///   `Self_` 仅本人行，否则 403。
pub async fn claim_from_pool(
    State(state): State<AppState>,
    auth: AuthContext,
    Json(req): Json<ClaimRequest>,
) -> Result<Json<ApiResponse<serde_json::Value>>, AppError> {
    let service = CrmService::new(state.db.clone());

    let data_scope_ctx = auth.to_data_scope_context();
    // 先无 ctx 取行是为了区分"公海行 vs 私海行"：公海行的 owner_id 仍是回收前的
    // 原归属人（回收只改 lead_status），直接用 ctx 取行会把公海行也判成越权。
    let lead = service.get_lead(req.lead_id, None).await?;

    if lead.lead_status.as_deref() != Some(lead_status::POOL) {
        // 非公海行：必须通过既有行级归属校验，否则 403（不给"借领取改写他人私海行"留口子）
        if !check_resource_owner(&data_scope_ctx, Some(lead.owner_id), lead.department_id) {
            return Err(AppError::permission_denied(format!(
                "无权领取线索 {}（数据范围限制）",
                req.lead_id
            )));
        }
        return Err(AppError::business_displayable("该客户不在公海中"));
    }

    // 更新线索归属人
    // 注意（#204 附带项，本批不修，已上报）：本单条领取路径只把 lead_status 置为 NEW，
    // 并未写 owner_id/owner_name（UpdateLeadRequest 无 owner 字段），而批量领取
    // `services/crm/pool.rs:119-130` build_claimed_active 真写 owner_id —— 两条领取
    // 路径归属语义不一致，单条领取后线索仍挂在原归属人名下。统一语义会改变本端点
    // 响应结构（或引入公海规则校验），属契约变更，需用户拍板后与前端一起改。
    let update_req = crate::models::dto::crm_dto::UpdateLeadRequest {
        lead_status: Some(lead_status::NEW.to_string()),
        ..Default::default()
    };

    // 批次 94 P2-10：注入真实操作人 user_id 用于审计日志
    let updated_lead = service
        .update_lead(req.lead_id, update_req, auth.user_id)
        .await?;

    // 记录领取日志
    tracing::info!(
        "用户 {} 从公海领取客户 {}: {}",
        auth.username,
        updated_lead.id,
        updated_lead.company_name.as_deref().unwrap_or("未知")
    );

    Ok(Json(ApiResponse::success_with_message(
        serde_json::to_value(updated_lead)?,
        "客户领取成功",
    )))
}

/// POST /api/v1/erp/crm/pool/recycle - 回收客户到公海
///
/// 行级边界（#204 越权写修复点）：回收写的是**私海行**，属行级归属判定范畴。
/// 修复前用 `get_lead(lead_id, None)`，`services/crm/lead.rs:360-367` 的
/// `check_resource_owner` 包在 `if let Some(ctx)` 内，传 None 即整体跳过 →
/// 任意用户可把他人私海线索回收进公海。
/// 权限依据（均为既有机制，不新增权限键、不放宽任何判定）：
/// - RBAC：与 claim 推导出同一个键 `pool:create`（`middleware/permission.rs:259` +
///   `utils/path_utils.rs:75/102`），admin 角色在 `permission.rs:536`
///   `admin_checker::is_admin_role`（roles.code='admin'）处整体放行；
/// - 行级：与 `list_leads`/`get_lead`/`update_lead`/`delete_lead` 正常路径同口径注入
///   `auth.to_data_scope_context()` 后复用 `check_resource_owner`
///   （`utils/data_scope.rs:149-171`）——`DataScope::All`（admin/总经理）可越界回收
///   他人行；`Dept` 需资源 department_id ∈ 可见部门集合；`Self_` 仅原归属人本人；
///   不满足即 403（与 update/delete 线索的越权语义完全一致，不额外收紧也不放松：
///   例如 Dept 用户回收 department_id 为 NULL 的行同样会被拒，这是既有 get_lead 口径）。
pub async fn recycle_to_pool(
    State(state): State<AppState>,
    auth: AuthContext,
    Json(req): Json<RecycleRequest>,
) -> Result<Json<ApiResponse<serde_json::Value>>, AppError> {
    let service = CrmService::new(state.db.clone());

    // 获取线索（带行级数据权限：非归属人且非可越界角色 → 403，写操作不发生）
    let data_scope_ctx = auth.to_data_scope_context();
    let lead = service.get_lead(req.lead_id, Some(&data_scope_ctx)).await?;

    // 检查状态
    if lead.lead_status.as_deref() == Some(lead_status::POOL) {
        return Err(AppError::business_displayable("该客户已在公海中"));
    }

    // 更新线索状态为公海
    let update_req = crate::models::dto::crm_dto::UpdateLeadRequest {
        lead_status: Some(lead_status::POOL.to_string()),
        ..Default::default()
    };

    // 批次 94 P2-10：注入真实操作人 user_id 用于审计日志
    let updated_lead = service
        .update_lead(req.lead_id, update_req, auth.user_id)
        .await?;

    // 记录回收日志
    tracing::info!(
        target: "crm_audit",
        user_id = auth.user_id,
        username = %auth.username,
        lead_id = req.lead_id,
        reason = ?req.reason,
        source = "pool_recycle",
        "客户回收到公海"
    );

    Ok(Json(ApiResponse::success_with_message(
        serde_json::to_value(updated_lead)?,
        "客户已回收到公海",
    )))
}

/// POST /api/v1/erp/crm/pool/:customer_id/claim - 领取指定公海客户
pub async fn claim_specific(
    State(state): State<AppState>,
    auth: AuthContext,
    Path(customer_id): Path<i32>,
) -> Result<Json<ApiResponse<serde_json::Value>>, AppError> {
    let service = CrmService::new(state.db.clone());
    let claimed = service
        .claim_pool_customers(vec![customer_id], auth.user_id, &auth.username)
        .await?;

    if claimed == 0 {
        return Err(AppError::business_displayable("该客户不在公海中或领取失败"));
    }

    tracing::info!("用户 {} 从公海领取客户 {}", auth.username, customer_id);

    Ok(Json(ApiResponse::success_with_message(
        serde_json::json!({ "claimed": claimed }),
        "客户领取成功",
    )))
}

/// POST /api/v1/erp/crm/pool/batch-claim - 批量领取公海客户
pub async fn batch_claim(
    State(state): State<AppState>,
    auth: AuthContext,
    Json(req): Json<BatchClaimRequest>,
) -> Result<Json<ApiResponse<serde_json::Value>>, AppError> {
    let service = CrmService::new(state.db.clone());
    let claimed = service
        .claim_pool_customers(req.customer_ids, auth.user_id, &auth.username)
        .await?;

    tracing::info!(
        "用户 {} 批量领取公海客户，成功 {} 条",
        auth.username,
        claimed
    );

    let msg = format!("成功领取 {} 个客户", claimed);
    Ok(Json(ApiResponse::success_with_message(
        serde_json::json!({ "claimed": claimed }),
        &msg,
    )))
}

// =====================================================
// V15 P0-S08 修复：公海规则 CRUD 接口
// =====================================================
// 提供保护期/领取上限/最大持有数规则的查询、创建、更新、删除
// 路由前缀：/pool/rules

/// 创建公海规则请求
#[allow(dead_code, reason = "反序列化输入字段")]
#[derive(Debug, Deserialize)]
pub struct CreatePoolRuleRequest {
    pub name: String,
    /// 规则类型：protection_period / claim_limit / max_holdings
    pub rule_type: String,
    pub rule_value: i32,
    /// 适用客户类型：all / wholesale / retail / vip
    pub customer_type: String,
    pub notes: Option<String>,
}

/// 更新公海规则请求
#[allow(dead_code, reason = "反序列化输入字段")]
#[derive(Debug, Deserialize)]
pub struct UpdatePoolRuleRequest {
    pub rule_value: Option<i32>,
    pub is_enabled: Option<bool>,
    pub notes: Option<String>,
}

/// GET /api/v1/erp/crm/pool/rules - 列出所有公海规则
pub async fn list_pool_rules(
    State(state): State<AppState>,
    _auth: AuthContext,
) -> Result<Json<ApiResponse<serde_json::Value>>, AppError> {
    let service = PoolRuleService::new(state.db.clone());
    let rules = service.list_rules().await?;
    Ok(Json(ApiResponse::success(serde_json::to_value(rules)?)))
}

/// POST /api/v1/erp/crm/pool/rules - 创建公海规则
pub async fn create_pool_rule(
    State(state): State<AppState>,
    auth: AuthContext,
    Json(req): Json<CreatePoolRuleRequest>,
) -> Result<Json<ApiResponse<serde_json::Value>>, AppError> {
    // 参数校验
    if !matches!(
        req.rule_type.as_str(),
        "protection_period" | "claim_limit" | "max_holdings"
    ) {
        return Err(AppError::validation_displayable(
            "规则类型必须为 protection_period / claim_limit / max_holdings",
        ));
    }
    if !matches!(
        req.customer_type.as_str(),
        "all" | "wholesale" | "retail" | "vip"
    ) {
        return Err(AppError::validation_displayable(
            "客户类型必须为 all / wholesale / retail / vip",
        ));
    }
    if req.rule_value < 0 {
        return Err(AppError::validation_displayable("规则数值不能为负数"));
    }

    let service = PoolRuleService::new(state.db.clone());
    let rule = service
        .create_rule(
            req.name,
            req.rule_type,
            req.rule_value,
            req.customer_type,
            req.notes,
        )
        .await?;

    tracing::info!("用户 {} 创建公海规则 id={}", auth.username, rule.id);
    Ok(Json(ApiResponse::success_with_message(
        serde_json::to_value(rule)?,
        "公海规则创建成功",
    )))
}

/// PUT /api/v1/erp/crm/pool/rules/:id - 更新公海规则
pub async fn update_pool_rule(
    State(state): State<AppState>,
    auth: AuthContext,
    Path(id): Path<i32>,
    Json(req): Json<UpdatePoolRuleRequest>,
) -> Result<Json<ApiResponse<serde_json::Value>>, AppError> {
    let service = PoolRuleService::new(state.db.clone());
    let rule = service
        .update_rule(id, req.rule_value, req.is_enabled, req.notes)
        .await?;

    tracing::info!("用户 {} 更新公海规则 id={}", auth.username, id);
    Ok(Json(ApiResponse::success_with_message(
        serde_json::to_value(rule)?,
        "公海规则更新成功",
    )))
}

/// DELETE /api/v1/erp/crm/pool/rules/:id - 删除公海规则
pub async fn delete_pool_rule(
    State(state): State<AppState>,
    auth: AuthContext,
    Path(id): Path<i32>,
) -> Result<Json<ApiResponse<serde_json::Value>>, AppError> {
    let service = PoolRuleService::new(state.db.clone());
    service.delete_rule(id).await?;

    tracing::info!("用户 {} 删除公海规则 id={}", auth.username, id);
    Ok(Json(ApiResponse::success_with_message(
        serde_json::json!({ "id": id }),
        "公海规则删除成功",
    )))
}
