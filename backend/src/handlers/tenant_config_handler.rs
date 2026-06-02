use axum::{
    extract::{Path, Query, State},
    Json,
};
use serde::{Deserialize, Serialize};

use crate::middleware::auth_context::AuthContext;
use crate::services::tenant_billing_service::TenantBillingService;
use crate::services::tenant_service::TenantService;
use crate::utils::app_state::AppState;
use crate::utils::error::AppError;
use crate::utils::response::ApiResponse;

#[derive(Debug, Deserialize)]
pub struct TenantConfigQuery {
    pub key: Option<String>,
    pub config_type: Option<String>,
}

#[allow(dead_code)]
#[derive(Debug, Deserialize)]
pub struct SetConfigRequest {
    pub key: String,
    pub value: String,
    pub config_type: Option<String>,
    pub description: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct CreatePlanRequest {
    pub code: String,
    pub name: String,
    pub description: Option<String>,
    pub max_users: i32,
    pub max_storage_mb: i32,
    pub max_api_calls_per_day: i32,
    pub price_monthly: String,
    pub price_yearly: String,
    pub features: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct TenantConfigItem {
    pub id: i32,
    pub tenant_id: i32,
    pub config_key: String,
    pub config_value: String,
    pub config_type: String,
    pub description: Option<String>,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Serialize)]
pub struct BillingPlanItem {
    pub id: i32,
    pub code: String,
    pub name: String,
    pub description: Option<String>,
    pub max_users: i32,
    pub max_storage_mb: i32,
    pub max_api_calls_per_day: i32,
    pub price_monthly: String,
    pub price_yearly: String,
    pub features: Option<String>,
    pub is_active: bool,
    pub created_at: String,
}

#[derive(Debug, Serialize)]
pub struct UsageStatistics {
    pub tenant_id: i32,
    pub tenant_name: String,
    pub plan_name: Option<String>,
    pub current_users: i64,
    pub max_users: i32,
    pub storage_used_mb: i64,
    pub max_storage_mb: i32,
    pub api_calls_today: i64,
    pub max_api_calls_per_day: i32,
    pub usage_percentages: UsagePercentages,
}

#[derive(Debug, Serialize)]
pub struct UsagePercentages {
    pub users: f64,
    pub storage: f64,
    pub api_calls: f64,
}

pub async fn list_configs(
    State(state): State<AppState>,
    auth: AuthContext,
    Query(query): Query<TenantConfigQuery>,
) -> Result<Json<ApiResponse<Vec<TenantConfigItem>>>, AppError> {
    // 如果没有租户信息，返回空列表
    let tenant_id = match auth.tenant_id {
        Some(id) => id,
        None => return Ok(Json(ApiResponse::success(vec![]))),
    };

    let service = TenantService::new(state.db);

    if let Some(key) = &query.key {
        let value = service.get_tenant_config(tenant_id, key).await?;
        let items = value
            .map(|v| {
                vec![TenantConfigItem {
                    id: 0,
                    tenant_id,
                    config_key: key.clone(),
                    config_value: v,
                    config_type: query
                        .config_type
                        .clone()
                        .unwrap_or_else(|| "STRING".to_string()),
                    description: None,
                    created_at: chrono::Utc::now().to_rfc3339(),
                    updated_at: chrono::Utc::now().to_rfc3339(),
                }]
            })
            .unwrap_or_default();

        return Ok(Json(ApiResponse::success(items)));
    }

    Ok(Json(ApiResponse::success(vec![])))
}

pub async fn set_config(
    State(state): State<AppState>,
    auth: AuthContext,
    Json(req): Json<SetConfigRequest>,
) -> Result<Json<ApiResponse<()>>, AppError> {
    let tenant_id = auth
        .tenant_id
        .ok_or_else(|| AppError::BadRequest("缺少租户信息".to_string()))?;

    let service = TenantService::new(state.db);
    let config_type = req.config_type.as_deref().unwrap_or("STRING");

    service
        .set_tenant_config(tenant_id, &req.key, &req.value, config_type)
        .await?;

    tracing::info!("租户 {} 配置已更新: key={}", tenant_id, req.key);

    Ok(Json(ApiResponse::success_with_message((), "配置已保存")))
}

pub async fn delete_config(
    State(state): State<AppState>,
    auth: AuthContext,
    Path(key): Path<String>,
) -> Result<Json<ApiResponse<()>>, AppError> {
    use crate::models::tenant_config;
    use sea_orm::{ColumnTrait, EntityTrait, QueryFilter};

    let tenant_id = auth
        .tenant_id
        .ok_or_else(|| AppError::BadRequest("缺少租户信息".to_string()))?;

    // 删除租户配置
    let result = tenant_config::Entity::delete_many()
        .filter(tenant_config::Column::TenantId.eq(tenant_id))
        .filter(tenant_config::Column::ConfigKey.eq(&key))
        .exec(state.db.as_ref())
        .await?;

    if result.rows_affected == 0 {
        return Err(AppError::NotFound("配置不存在".to_string()));
    }

    tracing::info!("用户 {} 删除租户配置: key={}", auth.username, key);

    Ok(Json(ApiResponse::success_with_message((), "配置已删除")))
}

pub async fn list_plans(
    State(state): State<AppState>,
    _auth: AuthContext,
) -> Result<Json<ApiResponse<Vec<BillingPlanItem>>>, AppError> {
    use crate::models::tenant_plan;
    use sea_orm::{ColumnTrait, EntityTrait, QueryFilter};

    let plans = tenant_plan::Entity::find()
        .filter(tenant_plan::Column::IsActive.eq(true))
        .all(state.db.as_ref())
        .await?;

    let items: Vec<BillingPlanItem> = plans
        .into_iter()
        .map(|p| BillingPlanItem {
            id: p.id,
            code: p.code,
            name: p.name,
            description: p.description,
            max_users: p.max_users,
            max_storage_mb: p.max_storage_mb,
            max_api_calls_per_day: p.max_api_calls_per_day,
            price_monthly: p.price_monthly.to_string(),
            price_yearly: p.price_yearly.to_string(),
            features: p.features,
            is_active: p.is_active,
            created_at: p.created_at.to_rfc3339(),
        })
        .collect();

    Ok(Json(ApiResponse::success(items)))
}

pub async fn create_plan(
    State(state): State<AppState>,
    _auth: AuthContext,
    Json(req): Json<CreatePlanRequest>,
) -> Result<Json<ApiResponse<BillingPlanItem>>, AppError> {
    use crate::models::tenant_plan;
    use chrono::Utc;
    use rust_decimal::Decimal;
    use sea_orm::{ActiveModelTrait, Set};

    let price_monthly: Decimal = req
        .price_monthly
        .parse()
        .map_err(|_| AppError::BadRequest("月度价格格式错误".to_string()))?;
    let price_yearly: Decimal = req
        .price_yearly
        .parse()
        .map_err(|_| AppError::BadRequest("年度价格格式错误".to_string()))?;

    let now = Utc::now();
    let active_model = tenant_plan::ActiveModel {
        id: Default::default(),
        code: Set(req.code),
        name: Set(req.name.clone()),
        description: Set(req.description),
        max_users: Set(req.max_users),
        max_storage_mb: Set(req.max_storage_mb),
        max_api_calls_per_day: Set(req.max_api_calls_per_day),
        price_monthly: Set(price_monthly),
        price_yearly: Set(price_yearly),
        features: Set(req.features),
        is_active: Set(true),
        created_at: Set(now),
        updated_at: Set(now),
    };

    let plan = active_model.insert(state.db.as_ref()).await?;

    Ok(Json(ApiResponse::success_with_message(
        BillingPlanItem {
            id: plan.id,
            code: plan.code,
            name: plan.name,
            description: plan.description,
            max_users: plan.max_users,
            max_storage_mb: plan.max_storage_mb,
            max_api_calls_per_day: plan.max_api_calls_per_day,
            price_monthly: plan.price_monthly.to_string(),
            price_yearly: plan.price_yearly.to_string(),
            features: plan.features,
            is_active: plan.is_active,
            created_at: plan.created_at.to_rfc3339(),
        },
        "计费套餐创建成功",
    )))
}

pub async fn get_plan(
    State(state): State<AppState>,
    _auth: AuthContext,
    Path(id): Path<i32>,
) -> Result<Json<ApiResponse<BillingPlanItem>>, AppError> {
    use crate::models::tenant_plan;
    use sea_orm::EntityTrait;

    let plan = tenant_plan::Entity::find_by_id(id)
        .one(state.db.as_ref())
        .await?
        .ok_or_else(|| AppError::NotFound("套餐不存在".to_string()))?;

    Ok(Json(ApiResponse::success(BillingPlanItem {
        id: plan.id,
        code: plan.code,
        name: plan.name,
        description: plan.description,
        max_users: plan.max_users,
        max_storage_mb: plan.max_storage_mb,
        max_api_calls_per_day: plan.max_api_calls_per_day,
        price_monthly: plan.price_monthly.to_string(),
        price_yearly: plan.price_yearly.to_string(),
        features: plan.features,
        is_active: plan.is_active,
        created_at: plan.created_at.to_rfc3339(),
    })))
}

pub async fn get_usage_statistics(
    State(state): State<AppState>,
    auth: AuthContext,
) -> Result<Json<ApiResponse<UsageStatistics>>, AppError> {
    let tenant_id = auth
        .tenant_id
        .ok_or_else(|| AppError::BadRequest("缺少租户信息".to_string()))?;

    let service = TenantService::new(state.db.clone());
    let billing_service = TenantBillingService::new(state.db);

    let tenant = service
        .get_tenant(tenant_id)
        .await?
        .ok_or_else(|| AppError::NotFound("租户不存在".to_string()))?;

    let users = service
        .get_tenant_users(tenant_id)
        .await
        .unwrap_or_default();
    let current_users = users.len() as i64;

    // 获取套餐限额
    let (max_users, max_storage_mb, max_api_calls, plan_name) =
        if let Ok(Some(current_plan)) = billing_service.get_current_plan(tenant_id).await {
            (
                current_plan.plan.max_users as i64,
                current_plan.plan.max_storage_mb as i64,
                current_plan.plan.max_api_calls_per_day as i64,
                Some(current_plan.plan.name),
            )
        } else {
            // 默认免费套餐限额
            (10, 1024, 10000, Some("免费版".to_string()))
        };

    // 获取实际使用量
    let (storage_used_mb, api_calls_today) =
        if let Ok(Some(usage)) = billing_service.get_usage_stats(tenant_id).await {
            (usage.storage_used_mb, usage.api_calls_today)
        } else {
            (0, 0)
        };

    let user_pct = if max_users > 0 {
        (current_users as f64 / max_users as f64) * 100.0
    } else {
        0.0
    };
    let storage_pct = if max_storage_mb > 0 {
        (storage_used_mb as f64 / max_storage_mb as f64) * 100.0
    } else {
        0.0
    };
    let api_pct = if max_api_calls > 0 {
        (api_calls_today as f64 / max_api_calls as f64) * 100.0
    } else {
        0.0
    };

    Ok(Json(ApiResponse::success(UsageStatistics {
        tenant_id,
        tenant_name: tenant.name,
        plan_name,
        current_users,
        max_users: max_users as i32,
        storage_used_mb,
        max_storage_mb: max_storage_mb as i32,
        api_calls_today,
        max_api_calls_per_day: max_api_calls as i32,
        usage_percentages: UsagePercentages {
            users: (user_pct * 100.0).round() / 100.0,
            storage: (storage_pct * 100.0).round() / 100.0,
            api_calls: (api_pct * 100.0).round() / 100.0,
        },
    })))
}
