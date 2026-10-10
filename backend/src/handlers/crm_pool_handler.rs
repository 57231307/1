//! CRM客户公海 Handler
//!
//! 提供客户公海池的列表查询、领取和回收功能，以及公海规则 CRUD（保护期/领取上限/最大持有数）

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
// 公海规则服务
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
    // source / keyword 透传 LeadQuery 参与过滤
    pub source: Option<String>,
    // industry 参与过滤（crm_lead.industry 列由 m0043 迁移添加）
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

    let page = params.page.unwrap_or(1).clamp(1, 1000); // 分页 clamp 防 DoS
    let page_size = params.page_size.unwrap_or(20).clamp(1, 100);

    // 查询公海客户（owner_id为空或特定状态的线索）
    let query = crate::models::dto::crm_dto::LeadQuery {
        lead_status: Some(lead_status::POOL.to_string()),
        // source / keyword 透传到 LeadQuery，由 list_leads 服务执行过滤
        source: params.source,
        keyword: params.keyword,
        // industry 透传到 LeadQuery
        industry: params.industry,
        page: Some(page),
        page_size: Some(page_size),
    };

    // 行级数据权限按正常列表入口（crm_handler::list_leads）同口径接入：
    // scope 上下文缺省会整体跳过服务层 apply_department_scope_with_pool 行级过滤
    // （services/crm/lead.rs:152 的 `if let Some(ctx)` 判断）——公海行会跨行泄露，
    // 该实参不可省略。
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

    // 字段级数据权限：与 crm_handler::list_leads / get_lead 同一实现（同一判定源
    // get_role_data_permission + 同一掩码实现），公海侧不另造更松的规则：
    // - 配置了数据权限行 → filter_fields_batch（hidden/allowed 优先，不叠加默认打码）；
    // - 无权限行且非 admin → 默认脱敏（查询 Err 与 role_id 缺失同走此分支，fail-closed）。
    //   admin 判定基准以本仓唯一权威源 `admin_checker::is_admin_role`（roles.code='admin'，
    //   查询失败 fail-closed=false）为准，禁止角色主键字面量判定——播种漂移时字面量
    //   要么静默剔 admin 字段、要么给恰好命中字面量的其他角色静默扩权。
    // 本入口出参由上方按挑选字段构造，不含 address 键；mobile_phone/email 若存在则掩码。
    crate::handlers::crm_handler::apply_lead_field_permission(&state, auth.role_id, &mut items)
        .await;

    Ok(Json(ApiResponse::success(serde_json::json!({
        "items": items,
        "total": total,
        "page": page,
        "page_size": page_size,
    }))))
}

/// 公海写响应（领取 / 回收）的出参处理：整行 `crm_lead::Model` 必须走与读路径
/// **完全同一个**字段级数据权限实现
/// （`crm_handler::apply_lead_field_permission` → 有权限行 `filter_fields_batch`，
/// 无权限行且非 admin 走 `CrmService::mask_lead_pii_defaults`）。
///
/// 领取/回收成功的出参是整行序列化，携带 `mobile_phone`/`tel_phone`/`email`/`address`
/// 明文，而同一个非 admin 角色走 `GET /crm/leads/:id` 或列表时是被打码的——
/// 若此处不过掩码即构成"列表打码、写响应原文"的旁路：
/// 点一次"领取"即可批量换取他人联系方式原文。
/// 本函数只处理出参：不改状态码、不外显任何拒绝原因（权限拒绝仍走
/// `AppError::permission_denied` 的固定脱敏信封 + `FORBIDDEN` 码）。
async fn mask_lead_write_response(
    state: &AppState,
    role_id: Option<i32>,
    lead: &crate::models::crm_lead::Model,
) -> Result<serde_json::Value, AppError> {
    let mut value = serde_json::to_value(lead)?;
    crate::handlers::crm_handler::apply_lead_field_permission(
        state,
        role_id,
        std::slice::from_mut(&mut value),
    )
    .await;
    Ok(value)
}

/// POST /api/v1/erp/crm/pool/claim - 从公海领取客户
///
/// 行级边界：领取**只能作用于公海行**。
/// - RBAC 层：本路径 `/api/v1/erp/crm/pool/claim` 由 URL 段推导出资源 `pool`、
///   动作 `create`（`middleware/permission.rs:269` `extract_resource_info`；`crm` 是模块
///   前缀 `utils/path_utils.rs:75`，`resolve_module_prefixed_resource` 默认臂保留原名），
///   admin 角色在 `check_permission`（`permission.rs:534`，内部 `admin_checker::is_admin_role`）
///   整体放行。claim 与 recycle 推导出的是**同一个键 `pool:create`**，RBAC 无法区分二者，
///   所以"谁能对哪一行写"只能由行级数据权限决定。
/// - 行级层：公海行的业务语义是"无归属人、对有权进公海者开放"（`utils/data_scope.rs`
///   `PoolVisibility::Open` 注释——读侧公海行可见性与归属条件相互独立，与 DB 层
///   RLS 的独立公海 OR 支同源），因此这里**不套用私海归属门**（否则 self 销售领取
///   他人公海行会被打死）；一旦命中非公海行（即他人私海），立即回落到与 `get_lead`
///   正常路径同一个 `check_resource_owner`：`DataScope::All` 可越界、`Dept` 需资源
///   部门在可见集合内、`Self_` 仅本人行，否则 403。写侧越权门未随读侧放开做任何放松。
/// 校验边界（公海规则统一，批量/单条同一套）：本路径经 `claim_lead_ownership`
/// 走与 `POST /crm/pool/:id/claim` 相同的每日领取上限 / 最大持有数 /
/// 保护期校验（判据 `last_claimed_at`，原领取人本人重领豁免），见
/// `services/crm/pool.rs` 文件头。
/// 出参边界：成功响应不整行原文回传，必须走与读路径同一实现的
/// `mask_lead_write_response`（详见该函数文档）。
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

    // 领取即转移归属：写 owner_id/owner_name 与 lead_status=new，
    // 复用批量领取同一个 `services/crm/pool.rs::build_claimed_active`（单一归属实现，
    // 消除两条领取路径的归属漂移——若不写归属，线索仍挂在回收前的原归属人名下，
    // 领取人在自己的 self 数据范围列表里看不到刚领取的行）。
    // 归属人展示名取 `auth.username`：AuthContext（middleware/auth_context.rs:58-83）
    // 只有 username 一个身份展示字段，无真实姓名字段；users.real_name 未随令牌/权限
    // 中间件注入上下文，取它需新增一次跨模块查库与上下文改造，故此处落真实登录名——
    // owner_name 只允许来自真实身份字段，不得由 user_id 拼出（本仓硬规则：禁造展示名）。
    let updated_lead = service
        .claim_lead_ownership(lead, auth.user_id, &auth.username)
        .await?;

    // 记录领取日志
    tracing::info!(
        "用户 {} 从公海领取客户 {}: {}",
        auth.username,
        updated_lead.id,
        updated_lead.company_name.as_deref().unwrap_or("未知")
    );

    Ok(Json(ApiResponse::success_with_message(
        mask_lead_write_response(&state, auth.role_id, &updated_lead).await?,
        "客户领取成功",
    )))
}

/// POST /api/v1/erp/crm/pool/recycle - 回收客户到公海
///
/// 行级边界（越权写防护）：回收写的是**私海行**，属行级归属判定范畴。
/// `get_lead` 必须带 ctx 调用——`services/crm/lead.rs:464` `get_lead` 内的
/// `check_resource_owner` 包在 `if let Some(ctx)` 内，传 None 即整体跳过 →
/// 任意用户可把他人私海线索回收进公海。
/// 权限依据（均为既有机制，不新增权限键、不放宽任何判定）：
/// - RBAC：与 claim 推导出同一个键 `pool:create`（`middleware/permission.rs:269` +
///   `utils/path_utils.rs:75/102`），admin 角色在 `permission.rs:534` `check_permission`
///   内经 `admin_checker::is_admin_role`（roles.code='admin'）处整体放行；
/// - 行级：与 `list_leads`/`get_lead`/`update_lead`/`delete_lead` 正常路径同口径注入
///   `auth.to_data_scope_context()` 后复用 `check_resource_owner`
///   （`utils/data_scope.rs:155`）——`DataScope::All`（admin/总经理）可越界回收
///   他人行；`Dept` 需资源 department_id ∈ 可见部门集合；`Self_` 仅原归属人本人；
///   不满足即 403（与 update/delete 线索的越权语义完全一致，不额外收紧也不放松：
///   例如 Dept 用户回收 department_id 为 NULL 的行同样会被拒，这是既有 get_lead 口径）。
/// 出参边界：成功响应不整行原文回传，必须走与读路径同一实现的
/// `mask_lead_write_response`（详见该函数文档）——回收同样会整行返回 mobile_phone/
/// tel_phone/email/address，与领取端点是同一条旁路的两个入口。
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

    // 传入真实操作人 user_id 用于审计日志
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
        mask_lead_write_response(&state, auth.role_id, &updated_lead).await?,
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
// 公海规则 CRUD 接口
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
    /// 适用客户类型：all / wholesale / retail / vip（规则作用域档位，非 customers 列值域；
    /// 运行时只有 `all` 生效，其余三档当前无消费分支，见 `create_pool_rule` 注释）
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
    // ⚠️ 本四值（all/wholesale/retail/vip）是 **pool 规则表**里"规则作用域"的取值
    // （crm_pool_rules 之类，规则按客户类型分档），与 customers.customer_type 列的
    // 唯一词表（`constants::customer_type::ALLOWED`）是**两回事**，不并入、不共享：
    // 字面重合的 retail/wholesale 只是巧合同名 token，语义层不同（本处 `vip` 也只是
    // 规则作用域档位，不代表 customers 行能有 vip）。
    // ⚠️ 运行时**只有 `all` 被消费**——
    // `services/crm/pool.rs` 取规则值时固定 `CustomerType.eq("all")`，
    // wholesale/retail/vip 三档写进规则表后没有任何判定分支读取（`list_rules` 只回显、
    // 不参与规则生效）⇒ per-type 是死分支，配了也不生效。
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
