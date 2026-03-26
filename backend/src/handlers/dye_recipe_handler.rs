//! 染色配方管理Handler

use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    response::IntoResponse,
    Json,
};
use chrono::{DateTime, Utc};
use rust_decimal::Decimal;
use sea_orm::{
    ActiveModelTrait, ColumnTrait, DatabaseConnection, EntityTrait, PaginatorTrait, QueryFilter,
    QueryOrder, Set,
};
use serde::Deserialize;
use std::sync::Arc;

use crate::models::dye_recipe;
use crate::utils::response::{ApiResponse, PaginatedResponse};

#[derive(Debug, Deserialize)]
pub struct DyeRecipeListQuery {
    pub page: Option<u64>,
    pub page_size: Option<u64>,
    pub recipe_no: Option<String>,
    pub color_code: Option<String>,
    pub color_name: Option<String>,
    pub dye_type: Option<String>,
    pub status: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct CreateDyeRecipeRequest {
    pub recipe_no: String,
    pub color_code: String,
    pub color_name: String,
    pub fabric_type: Option<String>,
    pub dye_type: Option<String>,
    pub chemical_formula: Option<String>,
    pub temperature: Option<f64>,
    pub time_minutes: Option<i32>,
    pub ph_value: Option<f64>,
    pub liquor_ratio: Option<f64>,
    pub auxiliaries: Option<serde_json::Value>,
    pub status: Option<String>,
    pub version: Option<i32>,
    pub parent_recipe_id: Option<i32>,
    pub remarks: Option<String>,
    pub created_by: Option<i32>,
}

#[derive(Debug, Deserialize)]
pub struct UpdateDyeRecipeRequest {
    pub color_code: Option<String>,
    pub color_name: Option<String>,
    pub fabric_type: Option<String>,
    pub dye_type: Option<String>,
    pub chemical_formula: Option<String>,
    pub temperature: Option<f64>,
    pub time_minutes: Option<i32>,
    pub ph_value: Option<f64>,
    pub liquor_ratio: Option<f64>,
    pub auxiliaries: Option<serde_json::Value>,
    pub status: Option<String>,
    pub remarks: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct ApproveRecipeRequest {
    pub approved_by: i32,
}

#[derive(Debug, Deserialize)]
pub struct CreateVersionRequest {
    pub remarks: Option<String>,
    pub created_by: Option<i32>,
}

pub async fn list_dye_recipes(
    State(db): State<Arc<DatabaseConnection>>,
    Query(query): Query<DyeRecipeListQuery>,
) -> impl IntoResponse {
    let page = query.page.unwrap_or(1);
    let page_size = query.page_size.unwrap_or(20);

    let mut q = dye_recipe::Entity::find();

    if let Some(recipe_no) = &query.recipe_no {
        q = q.filter(dye_recipe::Column::RecipeNo.contains(recipe_no));
    }
    if let Some(color_code) = &query.color_code {
        q = q.filter(dye_recipe::Column::ColorCode.contains(color_code));
    }
    if let Some(color_name) = &query.color_name {
        q = q.filter(dye_recipe::Column::ColorName.contains(color_name));
    }
    if let Some(dye_type) = &query.dye_type {
        q = q.filter(dye_recipe::Column::DyeType.eq(dye_type));
    }
    if let Some(status) = &query.status {
        q = q.filter(dye_recipe::Column::Status.eq(status));
    }

    q = q.order_by_desc(dye_recipe::Column::CreatedAt);

    match q.paginate(&*db, page_size).fetch_page(page - 1).await {
        Ok(recipes) => {
            let total = recipes.len() as u64;
            let paginated = PaginatedResponse::new(recipes, total, page, page_size);
            paginated.into_response()
        }
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ApiResponse::<()>::error(&format!("获取配方列表失败：{}", e))),
        )
            .into_response(),
    }
}

pub async fn get_dye_recipe(
    State(db): State<Arc<DatabaseConnection>>,
    Path(id): Path<i32>,
) -> impl IntoResponse {
    match dye_recipe::Entity::find_by_id(id).one(&*db).await {
        Ok(Some(recipe)) => (StatusCode::OK, Json(ApiResponse::success(recipe))).into_response(),
        Ok(None) => (
            StatusCode::NOT_FOUND,
            Json(ApiResponse::<()>::error("配方不存在")),
        )
            .into_response(),
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ApiResponse::<()>::error(&format!("获取配方失败：{}", e))),
        )
            .into_response(),
    }
}

pub async fn create_dye_recipe(
    State(db): State<Arc<DatabaseConnection>>,
    Json(req): Json<CreateDyeRecipeRequest>,
) -> impl IntoResponse {
    let recipe = dye_recipe::ActiveModel {
        id: Set(0),
        recipe_no: Set(req.recipe_no),
        color_code: Set(req.color_code),
        color_name: Set(req.color_name),
        fabric_type: Set(req.fabric_type),
        dye_type: Set(req.dye_type),
        chemical_formula: Set(req.chemical_formula),
        temperature: Set(req.temperature.and_then(|t| Decimal::from_f64_retain(t))),
        time_minutes: Set(req.time_minutes),
        ph_value: Set(req.ph_value.and_then(|p| Decimal::from_f64_retain(p))),
        liquor_ratio: Set(req.liquor_ratio.and_then(|l| Decimal::from_f64_retain(l))),
        auxiliaries: Set(req.auxiliaries),
        status: Set(req.status.unwrap_or_else(|| "草稿".to_string())),
        version: Set(req.version.or(Some(1))),
        parent_recipe_id: Set(req.parent_recipe_id),
        approved_by: Set(None),
        approved_at: Set(None),
        remarks: Set(req.remarks),
        created_by: Set(req.created_by),
        created_at: Set(Utc::now()),
        updated_at: Set(Utc::now()),
    };

    match recipe.insert(&*db).await {
        Ok(created) => (
            StatusCode::CREATED,
            Json(ApiResponse::success_with_msg(created, "配方创建成功")),
        )
            .into_response(),
        Err(e) => (
            StatusCode::BAD_REQUEST,
            Json(ApiResponse::<()>::error(&format!("创建配方失败：{}", e))),
        )
            .into_response(),
    }
}

pub async fn update_dye_recipe(
    State(db): State<Arc<DatabaseConnection>>,
    Path(id): Path<i32>,
    Json(req): Json<UpdateDyeRecipeRequest>,
) -> impl IntoResponse {
    let mut recipe: dye_recipe::ActiveModel = match dye_recipe::Entity::find_by_id(id).one(&*db).await {
        Ok(Some(r)) => r.into(),
        Ok(None) => {
            return (
                StatusCode::NOT_FOUND,
                Json(ApiResponse::<()>::error("配方不存在")),
            )
                .into_response();
        }
        Err(e) => {
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ApiResponse::<()>::error(&format!("获取配方失败：{}", e))),
            )
                .into_response();
        }
    };

    if let Some(color_code) = req.color_code {
        recipe.color_code = Set(color_code);
    }
    if let Some(color_name) = req.color_name {
        recipe.color_name = Set(color_name);
    }
    if let Some(fabric_type) = req.fabric_type {
        recipe.fabric_type = Set(Some(fabric_type));
    }
    if let Some(dye_type) = req.dye_type {
        recipe.dye_type = Set(Some(dye_type));
    }
    if let Some(chemical_formula) = req.chemical_formula {
        recipe.chemical_formula = Set(Some(chemical_formula));
    }
    if let Some(temperature) = req.temperature {
        recipe.temperature = Set(Decimal::from_f64_retain(temperature));
    }
    if let Some(time_minutes) = req.time_minutes {
        recipe.time_minutes = Set(Some(time_minutes));
    }
    if let Some(ph_value) = req.ph_value {
        recipe.ph_value = Set(Decimal::from_f64_retain(ph_value));
    }
    if let Some(liquor_ratio) = req.liquor_ratio {
        recipe.liquor_ratio = Set(Decimal::from_f64_retain(liquor_ratio));
    }
    if let Some(auxiliaries) = req.auxiliaries {
        recipe.auxiliaries = Set(Some(auxiliaries));
    }
    if let Some(status) = req.status {
        recipe.status = Set(status);
    }
    if let Some(remarks) = req.remarks {
        recipe.remarks = Set(Some(remarks));
    }

    recipe.updated_at = Set(Utc::now());

    match recipe.update(&*db).await {
        Ok(updated) => (
            StatusCode::OK,
            Json(ApiResponse::success_with_msg(updated, "配方更新成功")),
        )
            .into_response(),
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ApiResponse::<()>::error(&format!("更新配方失败：{}", e))),
        )
            .into_response(),
    }
}

pub async fn delete_dye_recipe(
    State(db): State<Arc<DatabaseConnection>>,
    Path(id): Path<i32>,
) -> impl IntoResponse {
    match dye_recipe::Entity::delete_by_id(id).exec(&*db).await {
        Ok(_) => (
            StatusCode::OK,
            Json(ApiResponse::success_with_msg((), "配方删除成功")),
        )
            .into_response(),
        Err(e) => (
            StatusCode::BAD_REQUEST,
            Json(ApiResponse::<()>::error(&format!("删除配方失败：{}", e))),
        )
            .into_response(),
    }
}

pub async fn approve_recipe(
    State(db): State<Arc<DatabaseConnection>>,
    Path(id): Path<i32>,
    Json(req): Json<ApproveRecipeRequest>,
) -> impl IntoResponse {
    let mut recipe: dye_recipe::ActiveModel = match dye_recipe::Entity::find_by_id(id).one(&*db).await {
        Ok(Some(r)) => r.into(),
        Ok(None) => {
            return (
                StatusCode::NOT_FOUND,
                Json(ApiResponse::<()>::error("配方不存在")),
            )
                .into_response();
        }
        Err(e) => {
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ApiResponse::<()>::error(&format!("获取配方失败：{}", e))),
            )
                .into_response();
        }
    };

    recipe.status = Set("已审核".to_string());
    recipe.approved_by = Set(Some(req.approved_by));
    recipe.approved_at = Set(Some(Utc::now()));
    recipe.updated_at = Set(Utc::now());

    match recipe.update(&*db).await {
        Ok(updated) => (
            StatusCode::OK,
            Json(ApiResponse::success_with_msg(updated, "配方审核成功")),
        )
            .into_response(),
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ApiResponse::<()>::error(&format!("审核配方失败：{}", e))),
        )
            .into_response(),
    }
}

pub async fn create_new_version(
    State(db): State<Arc<DatabaseConnection>>,
    Path(id): Path<i32>,
    Json(req): Json<CreateVersionRequest>,
) -> impl IntoResponse {
    let original = match dye_recipe::Entity::find_by_id(id).one(&*db).await {
        Ok(Some(r)) => r,
        Ok(None) => {
            return (
                StatusCode::NOT_FOUND,
                Json(ApiResponse::<()>::error("配方不存在")),
            )
                .into_response();
        }
        Err(e) => {
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ApiResponse::<()>::error(&format!("获取配方失败：{}", e))),
            )
                .into_response();
        }
    };

    let new_version = original.version.unwrap_or(1) + 1;
    let new_recipe_no = format!("{}-V{}", original.recipe_no, new_version);

    let new_recipe = dye_recipe::ActiveModel {
        id: Set(0),
        recipe_no: Set(new_recipe_no),
        color_code: Set(original.color_code),
        color_name: Set(original.color_name),
        fabric_type: Set(original.fabric_type),
        dye_type: Set(original.dye_type),
        chemical_formula: Set(original.chemical_formula),
        temperature: Set(original.temperature),
        time_minutes: Set(original.time_minutes),
        ph_value: Set(original.ph_value),
        liquor_ratio: Set(original.liquor_ratio),
        auxiliaries: Set(original.auxiliaries),
        status: Set("草稿".to_string()),
        version: Set(Some(new_version)),
        parent_recipe_id: Set(Some(id)),
        approved_by: Set(None),
        approved_at: Set(None),
        remarks: Set(req.remarks),
        created_by: Set(req.created_by),
        created_at: Set(Utc::now()),
        updated_at: Set(Utc::now()),
    };

    match new_recipe.insert(&*db).await {
        Ok(created) => (
            StatusCode::CREATED,
            Json(ApiResponse::success_with_msg(created, "配方新版本创建成功")),
        )
            .into_response(),
        Err(e) => (
            StatusCode::BAD_REQUEST,
            Json(ApiResponse::<()>::error(&format!("创建新版本失败：{}", e))),
        )
            .into_response(),
    }
}

pub async fn get_recipes_by_color(
    State(db): State<Arc<DatabaseConnection>>,
    Path(color_code): Path<String>,
) -> impl IntoResponse {
    match dye_recipe::Entity::find()
        .filter(dye_recipe::Column::ColorCode.eq(color_code))
        .filter(dye_recipe::Column::Status.eq("已审核"))
        .order_by_desc(dye_recipe::Column::Version)
        .all(&*db)
        .await
    {
        Ok(recipes) => (StatusCode::OK, Json(ApiResponse::success(recipes))).into_response(),
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ApiResponse::<()>::error(&format!("获取配方列表失败：{}", e))),
        )
            .into_response(),
    }
}

pub async fn get_recipe_versions(
    State(db): State<Arc<DatabaseConnection>>,
    Path(id): Path<i32>,
) -> impl IntoResponse {
    match dye_recipe::Entity::find()
        .filter(dye_recipe::Column::ParentRecipeId.eq(id))
        .or_filter(dye_recipe::Column::Id.eq(id))
        .order_by_asc(dye_recipe::Column::Version)
        .all(&*db)
        .await
    {
        Ok(recipes) => (StatusCode::OK, Json(ApiResponse::success(recipes))).into_response(),
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ApiResponse::<()>::error(&format!("获取配方版本失败：{}", e))),
        )
            .into_response(),
    }
}
