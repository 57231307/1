//! 决策类 handler
//!
//! 包含异常检测、销售合同、销售价格、租户管理四类与业务决策相关的端点。

use axum::{extract::State, Json};
use chrono::Utc;
use rust_decimal::prelude::ToPrimitive;
use sea_orm::{ActiveModelTrait, EntityTrait, QueryOrder};
use serde::{Deserialize, Serialize};

use crate::middleware::auth_context::AuthContext;
use crate::models::sales_contract::Entity as SalesContractEntity;
use crate::models::sales_price::Entity as SalesPriceEntity;
use crate::models::tenant::Entity as TenantEntity;
use crate::services::ai::AiAnalysisService;
use crate::utils::app_state::AppState;
use crate::utils::error::AppError;
use crate::utils::response::ApiResponse;

// ============================================================================
// 异常检测 - 使用统计方法（Z-score + IQR）
// ============================================================================

/// 异常检测
pub async fn anomaly_detection(
    State(state): State<AppState>,
    _auth: AuthContext,
    Json(payload): Json<AnomalyDetectionRequest>,
) -> Result<Json<ApiResponse<Vec<AnomalyItem>>>, AppError> {
    let service = AiAnalysisService::new(state.db);

    let days = payload
        .date_range
        .as_ref()
        .and_then(|d| d.parse::<i64>().ok())
        .unwrap_or(30);

    let anomalies = service.detect_anomalies(days).await?;

    let filtered = match payload.data_type.as_str() {
        "sales" => anomalies
            .into_iter()
            .filter(|a| a.entity_type == "SALES")
            .collect::<Vec<_>>(),
        "inventory" => anomalies
            .into_iter()
            .filter(|a| a.entity_type == "INVENTORY")
            .collect::<Vec<_>>(),
        "quality" => anomalies,
        _ => anomalies,
    };

    let items: Vec<AnomalyItem> = filtered
        .into_iter()
        .map(|a| {
            let severity = match a.severity.as_str() {
                "CRITICAL" => "critical",
                "WARNING" | "MEDIUM" => "warning",
                _ => "info",
            };

            let anomaly_type = match a.anomaly_type.as_str() {
                "SPIKE" => "突增",
                "DROP" => "突降",
                "ZERO_STOCK" => "零库存",
                "LOW_STOCK" => "低于安全线",
                "OVERSTOCK" => "库存积压",
                "SLOW_MOVING" => "滞销",
                other => other,
            };

            AnomalyItem {
                item: format!("{} #{}", a.entity_type, a.entity_id),
                anomaly_type: anomaly_type.to_string(),
                description: a.description,
                severity: severity.to_string(),
                detected_at: a.detected_at.to_rfc3339(),
            }
        })
        .collect();

    Ok(Json(ApiResponse::success(items)))
}

// ============================================================================
// 销售合同相关 - 使用真实数据库查询
// ============================================================================

/// 销售合同列表
pub async fn list_sales_contracts(
    State(state): State<AppState>,
) -> Result<Json<ApiResponse<Vec<SalesContract>>>, AppError> {
    let contracts = SalesContractEntity::find()
        .order_by_desc(crate::models::sales_contract::Column::CreatedAt)
        .all(&*state.db)
        .await?;

    let items: Vec<SalesContract> = contracts
        .into_iter()
        .map(|c| SalesContract {
            id: c.id as u32,
            contract_no: c.contract_no,
            customer_name: c
                .customer_name
                .unwrap_or_else(|| format!("客户 #{}", c.customer_id)),
            contract_date: c
                .signed_date
                .map_or_else(|| "".to_string(), |d| d.to_string()),
            total_amount: c.total_amount.and_then(|d| d.to_f64()).unwrap_or(0.0),
            status: c.status,
        })
        .collect();
    Ok(Json(ApiResponse::success(items)))
}

// ============================================================================
// 销售价格相关 - 使用真实数据库查询
// ============================================================================

/// 销售价格列表
pub async fn list_sales_prices(
    State(state): State<AppState>,
) -> Result<Json<ApiResponse<Vec<SalesPrice>>>, AppError> {
    let prices = SalesPriceEntity::find()
        .order_by_desc(crate::models::sales_price::Column::EffectiveDate)
        .all(&*state.db)
        .await?;

    let items: Vec<SalesPrice> = prices
        .into_iter()
        .map(|p| SalesPrice {
            id: p.id as u32,
            product_name: format!("产品 #{}", p.product_id),
            customer_name: p
                .customer_id
                .map_or_else(|| "全部客户".to_string(), |cid| format!("客户 #{}", cid)),
            price: p.price.to_f64().unwrap_or(0.0),
            currency: p.currency,
            unit: p.unit,
            effective_date: p.effective_date.to_string(),
            status: p.status,
        })
        .collect();
    Ok(Json(ApiResponse::success(items)))
}

// ============================================================================
// 租户管理相关 - 使用真实数据库查询
// ============================================================================

/// 租户列表
pub async fn list_tenants(
    State(state): State<AppState>,
) -> Result<Json<ApiResponse<Vec<Tenant>>>, AppError> {
    let tenants = TenantEntity::find()
        .order_by_asc(crate::models::tenant::Column::CreatedAt)
        .all(&*state.db)
        .await?;

    let items: Vec<Tenant> = tenants
        .into_iter()
        .map(|t| Tenant {
            id: t.id as u32,
            tenant_code: t.code,
            tenant_name: t.name,
            domain: t.custom_domain.unwrap_or_else(|| "".to_string()),
            subscription_plan: t
                .plan_id
                .map_or_else(|| "free".to_string(), |pid| format!("plan_{}", pid)),
            current_users: 0,
            max_users: 0,
            status: t.status,
            subscription_start_date: "".to_string(),
            subscription_end_date: t
                .expired_at
                .map_or_else(|| "".to_string(), |d| d.format("%Y-%m-%d").to_string()),
        })
        .collect();
    Ok(Json(ApiResponse::success(items)))
}

/// 创建租户
pub async fn create_tenant(
    State(state): State<AppState>,
    Json(payload): Json<CreateTenantRequest>,
) -> Result<Json<ApiResponse<Tenant>>, AppError> {
    use crate::models::tenant::ActiveModel;

    let now = Utc::now();

    let active = ActiveModel {
        code: sea_orm::ActiveValue::Set(payload.tenant_code.clone()),
        name: sea_orm::ActiveValue::Set(payload.tenant_name.clone()),
        description: sea_orm::ActiveValue::Set(None),
        status: sea_orm::ActiveValue::Set("active".to_string()),
        plan_id: sea_orm::ActiveValue::Set(None),
        admin_user_id: sea_orm::ActiveValue::Set(None),
        db_schema: sea_orm::ActiveValue::Set(None),
        custom_domain: sea_orm::ActiveValue::Set(Some(payload.domain.clone())),
        created_at: sea_orm::ActiveValue::Set(now),
        updated_at: sea_orm::ActiveValue::Set(now),
        expired_at: sea_orm::ActiveValue::Set(Some(now + chrono::Duration::days(365))),
        id: sea_orm::ActiveValue::NotSet,
    };

    let inserted = active.insert(&*state.db).await?;

    Ok(Json(ApiResponse::success(Tenant {
        id: inserted.id as u32,
        tenant_code: inserted.code,
        tenant_name: inserted.name,
        domain: inserted.custom_domain.unwrap_or_else(|| "".to_string()),
        subscription_plan: inserted
            .plan_id
            .map_or_else(|| "free".to_string(), |pid| format!("plan_{}", pid)),
        current_users: 0,
        max_users: 0,
        status: inserted.status,
        subscription_start_date: "".to_string(),
        subscription_end_date: inserted
            .expired_at
            .map_or_else(|| "".to_string(), |d| d.format("%Y-%m-%d").to_string()),
    })))
}

/// 获取租户
pub async fn get_tenant() -> Result<Json<ApiResponse<serde_json::Value>>, AppError> {
    Err(AppError::bad_request("请提供租户ID"))
}

/// 更新租户
pub async fn update_tenant() -> Result<Json<ApiResponse<serde_json::Value>>, AppError> {
    Err(AppError::bad_request("请提供租户ID和更新数据"))
}

// ============================================================================
// 数据结构
// ============================================================================

#[derive(Debug, Serialize, Deserialize)]
pub struct AnomalyDetectionRequest {
    pub data_type: String,
    pub date_range: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct AnomalyItem {
    pub item: String,
    pub anomaly_type: String,
    pub description: String,
    pub severity: String,
    pub detected_at: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct SalesContract {
    pub id: u32,
    pub contract_no: String,
    pub customer_name: String,
    pub contract_date: String,
    pub total_amount: f64,
    pub status: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct SalesPrice {
    pub id: u32,
    pub product_name: String,
    pub customer_name: String,
    pub price: f64,
    pub currency: String,
    pub unit: String,
    pub effective_date: String,
    pub status: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct CreateTenantRequest {
    pub tenant_code: String,
    pub tenant_name: String,
    pub domain: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Tenant {
    pub id: u32,
    pub tenant_code: String,
    pub tenant_name: String,
    pub domain: String,
    pub subscription_plan: String,
    pub current_users: u32,
    pub max_users: u32,
    pub status: String,
    pub subscription_start_date: String,
    pub subscription_end_date: String,
}
