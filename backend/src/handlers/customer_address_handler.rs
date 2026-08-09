use axum::{
    extract::{Path, State},
    Json,
};
use sea_orm::{ActiveModelTrait, ColumnTrait, EntityTrait, QueryFilter, QueryOrder, Set};

use crate::container::AppState;
use crate::middleware::auth_context::AuthContext;
use crate::models::customer_address::{
    self, CreateCustomerAddressDto, UpdateCustomerAddressDto,
};
use crate::utils::error::AppError;
use crate::utils::response::ApiResponse;

/// GET /api/v1/erp/customers/:id/addresses - 获取客户收货地址列表
pub async fn list_customer_addresses(
    State(state): State<AppState>,
    Path(customer_id): Path<i32>,
    _auth: AuthContext,
) -> Result<Json<ApiResponse<Vec<customer_address::Model>>>, AppError> {
    let addresses = customer_address::Entity::find()
        .filter(customer_address::Column::CustomerId.eq(customer_id))
        .order_by_desc(customer_address::Column::IsDefault)
        .order_by_desc(customer_address::Column::CreatedAt)
        .all(&*state.db)
        .await?;
    Ok(Json(ApiResponse::success(addresses)))
}

/// POST /api/v1/erp/customers/:id/addresses - 创建客户收货地址
pub async fn create_customer_address(
    State(state): State<AppState>,
    Path(customer_id): Path<i32>,
    _auth: AuthContext,
    Json(dto): Json<CreateCustomerAddressDto>,
) -> Result<Json<ApiResponse<customer_address::Model>>, AppError> {
    // 如果设为默认地址，先取消其他默认地址
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
    Ok(Json(ApiResponse::success(inserted)))
}

/// PUT /api/v1/erp/customers/:customer_id/addresses/:address_id - 更新客户收货地址
pub async fn update_customer_address(
    State(state): State<AppState>,
    Path((customer_id, address_id)): Path<(i32, i64)>,
    _auth: AuthContext,
    Json(dto): Json<UpdateCustomerAddressDto>,
) -> Result<Json<ApiResponse<customer_address::Model>>, AppError> {
    let existing = customer_address::Entity::find_by_id(address_id)
        .one(&*state.db)
        .await?
        .ok_or_else(|| AppError::not_found("收货地址不存在"))?;

    if existing.customer_id != customer_id {
        return Err(AppError::permission_denied("无权修改该地址"));
    }

    // 如果设为默认地址，先取消其他默认地址
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
    Ok(Json(ApiResponse::success(updated)))
}

/// DELETE /api/v1/erp/customers/:customer_id/addresses/:address_id - 删除客户收货地址
pub async fn delete_customer_address(
    State(state): State<AppState>,
    Path((customer_id, address_id)): Path<(i32, i64)>,
    _auth: AuthContext,
) -> Result<Json<ApiResponse<String>>, AppError> {
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
async fn clear_default_addresses(
    state: &AppState,
    customer_id: i32,
) -> Result<(), AppError> {
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
