//! 付款申请 Handler
//!
//! 付款申请 HTTP 接口层，负责处理 HTTP 请求并调用 Service 层

use crate::container::AppState;
use crate::middleware::auth_context::AuthContext;
use crate::models::supplier;
use crate::services::ap_payment_request_service::{
    ApPaymentRequestListQuery, ApPaymentRequestService, CreateApPaymentRequest,
    UpdateApPaymentRequest,
};
use crate::utils::admin_checker;
use crate::utils::error::AppError;
use crate::utils::response::{ApiResponse, PaginatedResponse};
use axum::{
    Json,
    extract::{Path, Query, State},
};
use chrono::NaiveDate;
use sea_orm::{ColumnTrait, EntityTrait, QueryFilter};
use serde::{Deserialize, Serialize};
use serde_json::Value as JsonValue;
use tracing::{info, warn};
use validator::Validate;

/// 付款申请主单（表头）在无权限配置行且非 admin（含权限查询失败）的默认处理下
/// 必须隐藏的列集合：申请金额、外币金额、收款账号、收款银行。
/// 列表与详情共用本表定位主单行，隐藏键以常量表为唯一出处，handler 内不再
/// 重复书写字面量；判据源为出参即主单实体序列化键（models/ap_payment_request）。
const DEFAULT_HIDDEN_REQUEST_FIELDS: [&str; 4] = [
    "request_amount",
    "request_amount_foreign",
    "bank_account",
    "bank_name",
];

/// 付款申请明细行在同一默认处理下必须隐藏的列集合：apply_amount 属申请金额
/// 语义的同族键（该行为这笔申请向某一应付单申请支付的金额）。无明细时主单金额
/// 取表头 request_amount；已补明细时各行 apply_amount 之和即该单的实际申请金额，
/// 隐藏语义必须同步落到明细行上下文，否则摘除主单金额形同未隐。
/// invoice_id 与 notes 属分摊归属信息、非金额类，不隐藏，明细行容器原样保留。
const DEFAULT_HIDDEN_ITEM_FIELDS: [&str; 1] = ["apply_amount"];

/// 按给定列集合对单行 JSON 对象整键移除：主单行与明细行共用的执行单元。
/// 隐藏语义为整键移除而非置空，非对象行原样返回；调用方按行上下文传入对应列集。
fn hide_fields(row: &mut JsonValue, fields: &[&str]) {
    if let Some(obj) = row.as_object_mut() {
        for field in fields {
            obj.remove(*field);
        }
    }
}

/// 查询付款申请列表参数
#[allow(dead_code, reason = "反序列化输入字段")]
#[derive(Debug, Deserialize)]
pub struct ApPaymentRequestQueryParams {
    pub supplier_id: Option<i32>,
    pub approval_status: Option<String>,
    pub payment_type: Option<String>,
    pub start_date: Option<NaiveDate>,
    pub end_date: Option<NaiveDate>,
    pub page: Option<u64>,
    pub page_size: Option<u64>,
}

/// 查询付款申请列表
pub async fn list_requests(
    Query(params): Query<ApPaymentRequestQueryParams>,
    State(state): State<AppState>,
    auth: AuthContext,
) -> Result<Json<ApiResponse<serde_json::Value>>, AppError> {
    info!(
        "用户 {} 查询付款申请列表，供应商 ID: {:?}",
        auth.username, params.supplier_id
    );

    let service = ApPaymentRequestService::new(state.db.clone());
    let page = params.page.unwrap_or(1).clamp(1, 1000); // 分页 clamp 防 DoS
    let page_size = params.page_size.unwrap_or(20).clamp(1, 100);
    // 提取行级数据权限上下文
    let data_scope_ctx = auth.to_data_scope_context();
    let (requests, total) = service
        .get_list(
            ApPaymentRequestListQuery {
                supplier_id: params.supplier_id,
                approval_status: params.approval_status,
                payment_type: params.payment_type,
                start_date: params.start_date,
                end_date: params.end_date,
                page,
                page_size,
            },
            Some(&data_scope_ctx),
        )
        .await?;

    info!(
        "用户 {} 查询付款申请成功，共 {} 条记录",
        auth.username, total
    );

    // 序列化失败传播 AppError，不返回 Null
    let mut items_json: Vec<serde_json::Value> = requests
        .into_iter()
        .map(|r| serde_json::to_value(r).map_err(AppError::from))
        .collect::<Result<Vec<_>, _>>()?;

    // 数据权限控制：获取角色数据权限并应用字段过滤
    if let Some(role_id) = auth.role_id {
        // 权限查询 Err 与 Ok(None) 不静默合并：Err 显式记 warn 后同走 fail-closed
        // 默认处理（本仓既有做法 = crm_handler::resolve_role_data_permission）。
        let permission = match state
            .data_permission_service
            .get_role_data_permission(role_id, "ap_payment_request")
            .await
        {
            Ok(permission) => permission,
            Err(error) => {
                tracing::warn!(
                    %error,
                    role_id,
                    resource_type = "ap_payment_request",
                    rule = "data_permission_lookup_fail_closed",
                    "角色数据权限查询失败，出参按无权限行 fail-closed 走默认处理"
                );
                None
            }
        };
        if let Some(permission) = permission {
            state.data_permission_service.filter_fields_batch(
                &mut items_json,
                &permission.allowed_fields,
                &permission.hidden_fields,
            );
        } else if !admin_checker::is_admin_role(&state.db, role_id).await {
            // admin 判定走本仓唯一权威源
            // `admin_checker::is_admin_role`（roles.code='admin'，查询失败 fail-closed=false），
            // 禁止角色主键字面量判定——播种漂移时字面量要么静默剔 admin 字段（功能坏）、
            // 要么静默给其他角色扩权（越权）。判定在循环外的分支条件处、每请求至多一次
            //（admin_checker 内部带 5 分钟缓存，同 crm_handler::apply_opportunity_field_permission
            // 既有范式）。
            // 如果没有配置数据权限且不是管理员，使用默认字段隐藏
            // 列表出参均为主单行，按主单列集逐行隐藏（列集定义见文件顶部常量表）
            for request in &mut items_json {
                hide_fields(request, &DEFAULT_HIDDEN_REQUEST_FIELDS);
            }
        }
    }

    let result = serde_json::to_value(PaginatedResponse::new(items_json, total, page, page_size))
        .map_err(|e| AppError::internal(e.to_string()))?;

    Ok(Json(ApiResponse::success(result)))
}

/// 获取付款申请详情
///
/// 出参由主单字段与明细行数组 items 两部分构成，字段权限对两部分同源生效：
/// 配置了数据权限行时，主单与明细行都按同一份白名单与隐藏集合处理；无配置行时
/// admin 放行、非 admin 按主单与明细各自的常量列集默认隐藏；权限查询失败显式记
/// 警告后按无权限行走同一默认处理。
/// role_id 缺失时本出口不做任何字段处理、原文发出——权限中间件在更早处已拒无角色
/// 请求，本分支是纵深防御层而非该场景的入口门；把该形态收紧为默认隐藏属行为变更，
/// 需另行裁定。
pub async fn get_request(
    Path(id): Path<i32>,
    State(state): State<AppState>,
    auth: AuthContext,
) -> Result<Json<ApiResponse<serde_json::Value>>, AppError> {
    info!("用户 {} 查询付款申请详情 ID: {}", auth.username, id);

    let service = ApPaymentRequestService::new(state.db.clone());
    // 提取行级数据权限上下文（IDOR 防护）
    let data_scope_ctx = auth.to_data_scope_context();
    let request = service.get_by_id(id, Some(&data_scope_ctx)).await?;

    info!(
        "用户 {} 查询付款申请详情成功：{}",
        auth.username, request.request_no
    );

    let mut request_json = serde_json::to_value(request)?;

    // 明细行随详情一并回读：创建契约可送 items，行落在 ap_payment_request_item；
    // 出参不带该键时调用方无法核实行是否写入。
    // 查询先行于权限块，明细在块内按自身行上下文与主单同一份权限结果逐行过滤，
    // 合并对象的出参构造收尾——字段过滤是顶层操作、不递归进入数组，若明细在过滤
    // 之后才挂上，白名单与隐藏集合都触达不到明细行，主单金额被隐的同时明细金额
    // 会原样发出（序列化失败传播 AppError，不返回 Null）
    let items = crate::models::ap_payment_request_item::Entity::find()
        .filter(crate::models::ap_payment_request_item::Column::RequestId.eq(id))
        .all(&*state.db)
        .await?;
    let mut items_json: Vec<JsonValue> = items
        .into_iter()
        .map(|item| serde_json::to_value(item).map_err(AppError::from))
        .collect::<Result<Vec<_>, _>>()?;

    // 数据权限控制：获取角色数据权限并应用字段过滤（主单与明细行同块同源）
    if let Some(role_id) = auth.role_id {
        // Err 记 warn 后同走 fail-closed 默认处理（与 list_requests 同款，不静默）
        let permission = match state
            .data_permission_service
            .get_role_data_permission(role_id, "ap_payment_request")
            .await
        {
            Ok(permission) => permission,
            Err(error) => {
                tracing::warn!(
                    %error,
                    role_id,
                    resource_type = "ap_payment_request",
                    rule = "data_permission_lookup_fail_closed",
                    "角色数据权限查询失败，出参按无权限行 fail-closed 走默认处理"
                );
                None
            }
        };
        if let Some(permission) = permission {
            state.data_permission_service.filter_fields(
                &mut request_json,
                &permission.allowed_fields,
                &permission.hidden_fields,
            );
            state.data_permission_service.filter_fields_batch(
                &mut items_json,
                &permission.allowed_fields,
                &permission.hidden_fields,
            );
        } else if !admin_checker::is_admin_role(&state.db, role_id).await {
            // 与列表同一单源判定（见 list_requests 内 admin 判定注释），每请求至多一次
            // 如果没有配置数据权限且不是管理员，使用默认字段隐藏：
            // 主单按主单列集、明细行按明细列集，各自行上下文同源处置
            hide_fields(&mut request_json, &DEFAULT_HIDDEN_REQUEST_FIELDS);
            for item in &mut items_json {
                hide_fields(item, &DEFAULT_HIDDEN_ITEM_FIELDS);
            }
        }
    }

    request_json["items"] = JsonValue::Array(items_json);

    Ok(Json(ApiResponse::success(request_json)))
}

/// 创建付款申请
#[axum::debug_handler]
pub async fn create_request(
    State(state): State<AppState>,
    auth: AuthContext,
    Json(req): Json<CreateApPaymentRequest>,
) -> Result<Json<ApiResponse<serde_json::Value>>, AppError> {
    info!(
        "用户 {} 创建付款申请，供应商 ID: {}",
        auth.username, req.supplier_id
    );

    req.validate().map_err(|e| {
        warn!("用户 {} 创建付款申请验证失败：{}", auth.username, e);
        AppError::from(e)
    })?;

    let service = ApPaymentRequestService::new(state.db.clone());
    let request = service.create(req, auth.user_id).await?;

    info!(
        "用户 {} 创建付款申请成功：{}",
        auth.username, request.request_no
    );

    Ok(Json(ApiResponse::success_with_message(
        serde_json::to_value(request)?,
        "付款申请创建成功",
    )))
}

/// 更新付款申请
#[axum::debug_handler]
pub async fn update_request(
    Path(id): Path<i32>,
    State(state): State<AppState>,
    auth: AuthContext,
    Json(req): Json<UpdateApPaymentRequest>,
) -> Result<Json<ApiResponse<JsonValue>>, AppError> {
    info!("用户 {} 更新付款申请 ID: {}", auth.username, id);

    req.validate().map_err(|e| {
        warn!("用户 {} 更新付款申请验证失败：{}", auth.username, e);
        AppError::from(e)
    })?;

    let service = ApPaymentRequestService::new(state.db.clone());
    // IDOR 防护——更新前先校验资源归属（复用列表/详情的 get_by_id + data_scope_ctx）
    let data_scope_ctx = auth.to_data_scope_context();
    service.get_by_id(id, Some(&data_scope_ctx)).await?;

    let request = service.update(id, req, auth.user_id).await?;

    info!(
        "用户 {} 更新付款申请成功：{}",
        auth.username, request.request_no
    );

    Ok(Json(ApiResponse::success_with_message(
        serde_json::to_value(request)?,
        "付款申请更新成功",
    )))
}

/// 删除付款申请
pub async fn delete_request(
    Path(id): Path<i32>,
    State(state): State<AppState>,
    auth: AuthContext,
) -> Result<Json<ApiResponse<()>>, AppError> {
    info!("用户 {} 删除付款申请 ID: {}", auth.username, id);

    let service = ApPaymentRequestService::new(state.db.clone());
    // IDOR 防护——删除前先校验资源归属（复用列表/详情的 get_by_id + data_scope_ctx）
    let data_scope_ctx = auth.to_data_scope_context();
    service.get_by_id(id, Some(&data_scope_ctx)).await?;

    // 传入真实操作人 user_id 用于审计日志
    service.delete(id, auth.user_id).await?;

    info!("用户 {} 删除付款申请成功", auth.username);

    Ok(Json(ApiResponse::success_with_message(
        (),
        "付款申请删除成功",
    )))
}

/// 提交付款申请
pub async fn submit_request(
    Path(id): Path<i32>,
    State(state): State<AppState>,
    auth: AuthContext,
) -> Result<Json<ApiResponse<JsonValue>>, AppError> {
    info!("用户 {} 提交付款申请 ID: {}", auth.username, id);

    let service = ApPaymentRequestService::new(state.db.clone());
    let request = service.submit(id, auth.user_id).await?;

    // 发送付款申请通知给审批人（admin/manager 角色用户，而非提交人本人）
    if state.event_notification_service.is_none() {
        tracing::error!("事件通知服务未装配（container 应无条件构造），此处站内通知将缺失");
    }
    if let Some(ref event_service) = state.event_notification_service {
        let supplier_name = if let Ok(Some(sup)) = supplier::Entity::find_by_id(request.supplier_id)
            .one(&*state.db)
            .await
        {
            sup.supplier_name
        } else {
            String::new()
        };

        // 查询所有 admin/manager 角色用户（付款审批人）
        let approver_ids = fetch_approver_user_ids(&state.db).await;

        if !approver_ids.is_empty() {
            use crate::models::notification::NotificationPriority;
            use crate::services::event_notification_service::NotificationPayload;
            let payload = NotificationPayload {
                user_ids: approver_ids.clone(),
                title: format!("付款申请待审批：{}", request.request_no),
                content: format!(
                    "供应商 {} 的付款申请 {}，金额 {} 需要您审批",
                    supplier_name, request.request_no, request.request_amount
                ),
                priority: NotificationPriority::High,
                business_type: Some("FINANCE".to_string()),
                business_id: Some(i64::from(request.id)),
                action_url: Some(format!("/finance/payment-request/{}", request.id)),
            };
            if let Err(e) = event_service.notify_multiple_users(payload).await {
                warn!(error = %e, request_id = request.id, "付款申请提交通知发送失败");
            }
        }
    }

    info!(
        "用户 {} 提交付款申请成功：{}",
        auth.username, request.request_no
    );

    Ok(Json(ApiResponse::success_with_message(
        serde_json::to_value(request)?,
        "付款申请提交成功",
    )))
}

/// 审批付款申请
pub async fn approve_request(
    Path(id): Path<i32>,
    State(state): State<AppState>,
    auth: AuthContext,
) -> Result<Json<ApiResponse<JsonValue>>, AppError> {
    info!("用户 {} 审批付款申请 ID: {}", auth.username, id);

    let service = ApPaymentRequestService::new(state.db.clone());
    let request = service.approve(id, auth.user_id).await?;

    // 发送审批通过通知
    if state.event_notification_service.is_none() {
        tracing::error!("事件通知服务未装配（container 应无条件构造），此处站内通知将缺失");
    }
    if let Some(ref event_service) = state.event_notification_service {
        // 通知发送失败显式记 warn，不静默吞错
        if let Err(e) = event_service
            .notify_approval_result(
                request.created_by,
                &request.request_no,
                true,
                auth.user_id,
                &auth.username,
                None,
            )
            .await
        {
            tracing::warn!(error = %e, request_id = id, "付款申请审批通过通知发送失败");
        }
    }

    info!(
        "用户 {} 审批付款申请通过：{}",
        auth.username, request.request_no
    );

    Ok(Json(ApiResponse::success_with_message(
        serde_json::to_value(request)?,
        "付款申请审批通过",
    )))
}

/// 拒绝付款申请
#[allow(dead_code, reason = "序列化/反序列化字段")]
#[derive(Debug, Deserialize, Serialize)]
pub struct RejectRequest {
    pub reason: String,
}

pub async fn reject_request(
    Path(id): Path<i32>,
    State(state): State<AppState>,
    auth: AuthContext,
    Json(req): Json<RejectRequest>,
) -> Result<Json<ApiResponse<JsonValue>>, AppError> {
    // 拒绝理由服务端强制（照 handlers/quotation_handler.rs 的 reject 样板）：
    // 前端 inputValidator 只拦误操作，拦不住直连 API 的空串绕过；理由为空即
    // 审计断链，必须在入口拒绝。`ap_payment_request.rejected_reason` 列型为
    // TEXT（migration/src/domain/business/m0012_add_ap_ar_finance_analysis.rs:84），
    // 无字符数上限，故不引入自造长度校验、不截断。
    let reason = req.reason.trim().to_string();
    if reason.is_empty() {
        // 出参只给人看的定性说明；记录 ID 等内部定位信息只进日志
        warn!(
            "用户 {} 拒绝付款申请被驳回：拒绝理由为空（trim 后），ID: {}",
            auth.username, id
        );
        return Err(AppError::validation_displayable(
            "拒绝理由不能为空，请说明拒绝原因以便留痕".to_string(),
        ));
    }

    info!(
        "用户 {} 拒绝付款申请 ID: {}, 原因：{}",
        auth.username, id, reason
    );

    let service = ApPaymentRequestService::new(state.db.clone());
    let request = service.reject(id, reason.clone(), auth.user_id).await?;

    // 发送审批拒绝通知
    if state.event_notification_service.is_none() {
        tracing::error!("事件通知服务未装配（container 应无条件构造），此处站内通知将缺失");
    }
    if let Some(ref event_service) = state.event_notification_service {
        // 通知发送失败显式记 warn，不静默吞错
        if let Err(e) = event_service
            .notify_approval_result(
                request.created_by,
                &request.request_no,
                false,
                auth.user_id,
                &auth.username,
                Some(&reason),
            )
            .await
        {
            tracing::warn!(error = %e, request_id = id, "付款申请审批拒绝通知发送失败");
        }
    }

    info!(
        "用户 {} 拒绝付款申请成功：{}",
        auth.username, request.request_no
    );

    Ok(Json(ApiResponse::success_with_message(
        serde_json::to_value(request)?,
        "付款申请已拒绝",
    )))
}

/// 查询付款审批人：admin 和 manager 角色的活跃用户 id 列表
async fn fetch_approver_user_ids(db: &sea_orm::DatabaseConnection) -> Vec<i32> {
    use crate::models::{role, user};
    use crate::utils::admin_checker::{ADMIN_ROLE_CODE, MANAGER_ROLE_CODE};
    use sea_orm::{ColumnTrait, EntityTrait, QueryFilter, sea_query::Cond};

    // 先查 admin/manager 角色 id
    let role_ids: Vec<i32> = role::Entity::find()
        .filter(
            Cond::any()
                .add(role::Column::Code.eq(ADMIN_ROLE_CODE))
                .add(role::Column::Code.eq(MANAGER_ROLE_CODE)),
        )
        .all(db)
        .await
        .map(|roles| roles.into_iter().map(|r| r.id).collect())
        .unwrap_or_default();

    if role_ids.is_empty() {
        return vec![];
    }

    // 查这些角色下的用户
    user::Entity::find()
        .filter(user::Column::RoleId.is_in(role_ids))
        .all(db)
        .await
        .map(|users| users.into_iter().map(|u| u.id).collect())
        .unwrap_or_default()
}
