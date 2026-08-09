use axum::{
    extract::{Path, State},
    Json,
};
use sea_orm::{ActiveModelTrait, ColumnTrait, EntityTrait, QueryFilter, QueryOrder, Set};

use crate::container::AppState;
use crate::middleware::auth_context::AuthContext;
use crate::models::aging_grade_config::{
    self, CreateAgingGradeDto, UpdateAgingGradeDto,
};
use crate::utils::error::AppError;
use crate::utils::response::ApiResponse;

/// GET /api/v1/erp/aging-grades - 获取账龄档位列表
pub async fn list_aging_grades(
    State(state): State<AppState>,
    _auth: AuthContext,
) -> Result<Json<ApiResponse<Vec<aging_grade_config::Model>>>, AppError> {
    let grades = aging_grade_config::Entity::find()
        .filter(aging_grade_config::Column::IsActive.eq(true))
        .order_by_asc(aging_grade_config::Column::SortOrder)
        .all(&*state.db)
        .await?;
    Ok(Json(ApiResponse::success(grades)))
}

/// POST /api/v1/erp/aging-grades - 创建账龄档位
pub async fn create_aging_grade(
    State(state): State<AppState>,
    _auth: AuthContext,
    Json(dto): Json<CreateAgingGradeDto>,
) -> Result<Json<ApiResponse<aging_grade_config::Model>>, AppError> {
    let now = chrono::Utc::now();
    let grade = aging_grade_config::ActiveModel {
        id: sea_orm::ActiveValue::NotSet,
        grade_name: Set(dto.grade_name),
        min_days: Set(dto.min_days),
        max_days: Set(dto.max_days),
        sort_order: Set(dto.sort_order.unwrap_or(0)),
        is_active: Set(dto.is_active.unwrap_or(true)),
        remark: Set(dto.remark),
        created_at: Set(now),
        updated_at: Set(now),
    };
    let inserted = grade.insert(&*state.db).await?;
    Ok(Json(ApiResponse::success(inserted)))
}

/// PUT /api/v1/erp/aging-grades/:id - 更新账龄档位
pub async fn update_aging_grade(
    State(state): State<AppState>,
    Path(id): Path<i64>,
    _auth: AuthContext,
    Json(dto): Json<UpdateAgingGradeDto>,
) -> Result<Json<ApiResponse<aging_grade_config::Model>>, AppError> {
    let existing = aging_grade_config::Entity::find_by_id(id)
        .one(&*state.db)
        .await?
        .ok_or_else(|| AppError::not_found("账龄档位不存在"))?;

    let mut active: aging_grade_config::ActiveModel = existing.into();
    if let Some(v) = dto.grade_name {
        active.grade_name = Set(v);
    }
    if let Some(v) = dto.min_days {
        active.min_days = Set(v);
    }
    if let Some(v) = dto.max_days {
        active.max_days = Set(v);
    }
    if let Some(v) = dto.sort_order {
        active.sort_order = Set(v);
    }
    if let Some(v) = dto.is_active {
        active.is_active = Set(v);
    }
    if let Some(v) = dto.remark {
        active.remark = Set(Some(v));
    }
    active.updated_at = Set(chrono::Utc::now());
    let updated = active.update(&*state.db).await?;
    Ok(Json(ApiResponse::success(updated)))
}

/// DELETE /api/v1/erp/aging-grades/:id - 删除账龄档位
pub async fn delete_aging_grade(
    State(state): State<AppState>,
    Path(id): Path<i64>,
    _auth: AuthContext,
) -> Result<Json<ApiResponse<String>>, AppError> {
    let existing = aging_grade_config::Entity::find_by_id(id)
        .one(&*state.db)
        .await?
        .ok_or_else(|| AppError::not_found("账龄档位不存在"))?;

    let mut active: aging_grade_config::ActiveModel = existing.into();
    active.delete(&*state.db).await?;
    Ok(Json(ApiResponse::success("删除成功".to_string())))
}
