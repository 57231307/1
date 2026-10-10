use axum::{
    Json,
    extract::{Path, State},
};
use sea_orm::{ActiveModelTrait, ColumnTrait, EntityTrait, QueryFilter, QueryOrder, Set};

use crate::container::AppState;
use crate::middleware::auth_context::AuthContext;
use crate::models::customer;
use crate::models::customer_address::{self, CreateCustomerAddressDto, UpdateCustomerAddressDto};
use crate::services::crm::cust::CrmService;
use crate::utils::admin_checker;
use crate::utils::data_scope::DataScopeContext;
use crate::utils::error::AppError;
use crate::utils::response::ApiResponse;

/// 地址行出参的 PII 形态转换：调用方先把整行序列化成 JSON，本函数按客户域既有
/// 单一真源 `CrmService::mask_customer_pii_defaults`（`services/crm/cust.rs`）逐行改写，
/// 本文件不另起第二套脱敏函数、不另写掩码规则、不重写列名清单。
///
/// 判据来源（与 `handlers/crm_customer_handler.rs` 的默认脱敏层逐项同源）：
/// - admin 判定 = `utils/admin_checker.rs` 的 `is_admin_role`（roles.code='admin'；
///   `role_id` 取不到或角色查询失败按 fail-closed=false 处理，与"无角色必脱敏"同方向），
///   每请求在行循环外算一次复用，不下放进逐行循环（该函数内部带 5 分钟缓存）；
/// - 非 admin：`contact_phone` 命中 `utils/field_mask.rs` 权威电话键集合被 `mask_phone`
///   打码（键保留、值打码，不删键），`address` 整键移除；
/// - admin：原文放行（含 `address`），既有原值契约不变。
///
/// 归属可见性不在本函数：行级门 `ensure_customer_parent_access` 已先行拒绝不可见客户，
/// 本函数只处理通过门之后的出参形态，不改状态码、不外显拒绝原因。
///
/// 客户域的第二层（`data_permission_service` 中 resource_type="customer" 的
/// allowed/hidden 配置行）在本文件**不叠加**：那份配置的字段名面向 `customers` 表列，
/// 套到 `customer_addresses` 行上会按另一张表的字段清单删掉非 PII 列（如 `is_default`、
/// `province`），地址行是否吃这份配置属待裁定项；本函数只应用默认脱敏层。
///
/// 入参 `rows` 是已序列化的地址行切片：列表出口传全部行，写响应出口传单元素切片。
async fn apply_address_pii_mask(
    state: &AppState,
    role_id: Option<i32>,
    rows: &mut [serde_json::Value],
) {
    let is_admin = match role_id {
        Some(role_id) => admin_checker::is_admin_role(&state.db, role_id).await,
        None => false,
    };
    for row in rows.iter_mut() {
        *row = CrmService::mask_customer_pii_defaults(std::mem::take(row), is_admin);
    }
}

/// 归属经父客户继承的读/写门：地址表本身无 owner_id/department_id 列，
/// 可见性判定取父客户的 owner_id + department_id（与 ar_reconciliation 同范式）。
async fn ensure_customer_parent_access(
    db: &sea_orm::DatabaseConnection,
    ctx: &DataScopeContext,
    customer_id: i32,
) -> Result<(), AppError> {
    let parent = customer::Entity::find_by_id(customer_id)
        .one(db)
        .await?
        .ok_or_else(|| AppError::not_found("客户不存在"))?;
    if !crate::utils::data_scope::check_resource_owner(
        ctx,
        Some(parent.owner_id),
        parent.department_id,
    ) {
        return Err(AppError::permission_denied("无权访问该客户地址"));
    }
    Ok(())
}

/// GET /api/v1/erp/customers/:id/addresses - 获取客户收货地址列表
/// 出参：经 `apply_address_pii_mask` 改写的地址行数组（非 admin 会话 `contact_phone`
/// 为打码值、`address` 键不存在）。
///
/// 外层保持 `Vec<Value>`：数组形状仍可被 `frontend/scripts/check-api-envelope.mjs`
/// 静态判定；元素用 Value 而非 `customer_address::Model`，是因为脱敏会整键移除
/// `address`，用 Model 声明出参等于谎称该键恒在（前端据此写出的消费点会静默取到 undefined）。
pub async fn list_customer_addresses(
    State(state): State<AppState>,
    Path(customer_id): Path<i32>,
    auth: AuthContext,
) -> Result<Json<ApiResponse<Vec<serde_json::Value>>>, AppError> {
    let ctx = auth.to_data_scope_context();
    ensure_customer_parent_access(&state.db, &ctx, customer_id).await?;

    let addresses = customer_address::Entity::find()
        .filter(customer_address::Column::CustomerId.eq(customer_id))
        .order_by_desc(customer_address::Column::IsDefault)
        .order_by_desc(customer_address::Column::CreatedAt)
        .all(&*state.db)
        .await?;
    let mut rows = addresses
        .into_iter()
        .map(serde_json::to_value)
        .collect::<Result<Vec<serde_json::Value>, serde_json::Error>>()?;
    apply_address_pii_mask(&state, auth.role_id, &mut rows).await;
    Ok(Json(ApiResponse::success(rows)))
}

/// POST /api/v1/erp/customers/:id/addresses - 创建客户收货地址
/// 写响应同样过 `apply_address_pii_mask`：与读出口共用一套形态，否则"读侧打码、
/// 写响应原文"就是同资源不同入口的旁路（客户域读写出口同口径的先例见
/// `handlers/crm_customer_handler.rs`）。
pub async fn create_customer_address(
    State(state): State<AppState>,
    Path(customer_id): Path<i32>,
    auth: AuthContext,
    Json(dto): Json<CreateCustomerAddressDto>,
) -> Result<Json<ApiResponse<serde_json::Value>>, AppError> {
    let ctx = auth.to_data_scope_context();
    ensure_customer_parent_access(&state.db, &ctx, customer_id).await?;

    if dto.is_default.unwrap_or(false) {
        clear_default_addresses(&state, customer_id).await?;
    }

    let now = chrono::Utc::now();
    let address = customer_address::ActiveModel {
        id: sea_orm::ActiveValue::NotSet,
        customer_id: Set(customer_id),
        contact_name: Set(dto.contact_name),
        contact_phone: Set(dto.contact_phone),
        province: Set(dto.province),
        city: Set(dto.city),
        district: Set(dto.district),
        address: Set(dto.address),
        postal_code: Set(dto.postal_code),
        is_default: Set(dto.is_default.unwrap_or(false)),
        remark: Set(dto.remark),
        created_at: Set(now),
        updated_at: Set(now),
    };
    let inserted = address.insert(&*state.db).await?;
    let mut value = serde_json::to_value(inserted)?;
    apply_address_pii_mask(&state, auth.role_id, std::slice::from_mut(&mut value)).await;
    Ok(Json(ApiResponse::success(value)))
}

/// PUT /api/v1/erp/customers/:customer_id/addresses/:address_id - 更新客户收货地址
/// 写响应同样过 `apply_address_pii_mask`：本端点按"缺键=不改"逐列更新，只改非 PII 列
/// （如 `remark`）也会把整行读回来，原文回显即等于绕过读侧脱敏。
pub async fn update_customer_address(
    State(state): State<AppState>,
    Path((customer_id, address_id)): Path<(i32, i64)>,
    auth: AuthContext,
    Json(dto): Json<UpdateCustomerAddressDto>,
) -> Result<Json<ApiResponse<serde_json::Value>>, AppError> {
    let ctx = auth.to_data_scope_context();
    ensure_customer_parent_access(&state.db, &ctx, customer_id).await?;

    let existing = customer_address::Entity::find_by_id(address_id)
        .one(&*state.db)
        .await?
        .ok_or_else(|| AppError::not_found("收货地址不存在"))?;

    if existing.customer_id != customer_id {
        return Err(AppError::permission_denied("无权修改该地址"));
    }

    if dto.is_default.unwrap_or(false) {
        clear_default_addresses(&state, customer_id).await?;
    }

    let mut active: customer_address::ActiveModel = existing.into();
    if let Some(v) = dto.contact_name {
        active.contact_name = Set(v);
    }
    if let Some(v) = dto.contact_phone {
        active.contact_phone = Set(v);
    }
    if let Some(v) = dto.province {
        active.province = Set(Some(v));
    }
    if let Some(v) = dto.city {
        active.city = Set(Some(v));
    }
    if let Some(v) = dto.district {
        active.district = Set(Some(v));
    }
    if let Some(v) = dto.address {
        active.address = Set(v);
    }
    if let Some(v) = dto.postal_code {
        active.postal_code = Set(Some(v));
    }
    if let Some(v) = dto.is_default {
        active.is_default = Set(v);
    }
    if let Some(v) = dto.remark {
        active.remark = Set(Some(v));
    }
    active.updated_at = Set(chrono::Utc::now());
    let updated = active.update(&*state.db).await?;
    let mut value = serde_json::to_value(updated)?;
    apply_address_pii_mask(&state, auth.role_id, std::slice::from_mut(&mut value)).await;
    Ok(Json(ApiResponse::success(value)))
}

/// DELETE /api/v1/erp/customers/:customer_id/addresses/:address_id - 删除客户收货地址
pub async fn delete_customer_address(
    State(state): State<AppState>,
    Path((customer_id, address_id)): Path<(i32, i64)>,
    auth: AuthContext,
) -> Result<Json<ApiResponse<String>>, AppError> {
    let ctx = auth.to_data_scope_context();
    ensure_customer_parent_access(&state.db, &ctx, customer_id).await?;

    let existing = customer_address::Entity::find_by_id(address_id)
        .one(&*state.db)
        .await?
        .ok_or_else(|| AppError::not_found("收货地址不存在"))?;

    if existing.customer_id != customer_id {
        return Err(AppError::permission_denied("无权删除该地址"));
    }

    let active: customer_address::ActiveModel = existing.into();
    active.delete(&*state.db).await?;
    Ok(Json(ApiResponse::success("删除成功".to_string())))
}

/// 清除客户的默认地址标记
async fn clear_default_addresses(state: &AppState, customer_id: i32) -> Result<(), AppError> {
    let defaults = customer_address::Entity::find()
        .filter(customer_address::Column::CustomerId.eq(customer_id))
        .filter(customer_address::Column::IsDefault.eq(true))
        .all(&*state.db)
        .await?;
    for addr in defaults {
        let mut active: customer_address::ActiveModel = addr.into();
        active.is_default = Set(false);
        active.update(&*state.db).await?;
    }
    Ok(())
}
