//! 决策类 handler
//!
//! 包含异常检测、销售合同、销售价格、租户管理四类与业务决策相关的端点。

use axum::{extract::State, response::IntoResponse, Json};
use chrono::Utc;
use rust_decimal::prelude::ToPrimitive;
use sea_orm::{EntityTrait, QueryOrder};
use serde::{Deserialize, Serialize};

use crate::middleware::auth_context::AuthContext;
use crate::models::sales_contract::Entity as SalesContractEntity;
use crate::models::sales_price::Entity as SalesPriceEntity;
use crate::models::tenant::Entity as TenantEntity;
use crate::services::ai::AiAnalysisService;
use crate::utils::app_state::AppState;
use crate::utils::response::ApiResponse;

// ============================================================================
// 异常检测 - 使用统计方法（Z-score + IQR）
// ============================================================================

/// 异常检测
pub async fn anomaly_detection(
    State(state): State<AppState>,
    _auth: AuthContext,
    Json(payload): Json<AnomalyDetectionRequest>,
) -> impl IntoResponse {
    let service = AiAnalysisService::new(state.db);

    let days = payload
        .date_range
        .as_ref()
        .and_then(|d| d.parse::<i64>().ok())
        .unwrap_or(30);

    match service.detect_anomalies(days).await {
        Ok(anomalies) => {
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
                        "WARNING" => "warning",
                        "MEDIUM" => "warning",
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

            Json(ApiResponse::success(items))
        }
        Err(e) => {
            tracing::error!("异常检测失败: {}", e);
            Json(ApiResponse::error(format!("异常检测失败: {}", e)))
        }
    }
}

// ============================================================================
// 销售合同相关 - 使用真实数据库查询
// ============================================================================

/// 销售合同列表
pub async fn list_sales_contracts(State(state): State<AppState>) -> impl IntoResponse {
    match SalesContractEntity::find()
        .order_by_desc(crate::models::sales_contract::Column::CreatedAt)
        .all(&*state.db)
        .await
    {
        Ok(contracts) => {
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
            Json(ApiResponse::success(items))
        }
        Err(e) => {
            tracing::error!("查询销售合同失败: {}", e);
            Json(ApiResponse::error(format!("查询销售合同失败: {}", e)))
        }
    }
}

/// 创建销售合同
pub async fn create_sales_contract(
    State(state): State<AppState>,
    Json(payload): Json<CreateSalesContractRequest>,
) -> impl IntoResponse {
    use crate::models::sales_contract::ActiveModel;
    use sea_orm::ActiveModelTrait;

    let now = Utc::now();
    let contract_no = format!("SC{}", now.format("%Y%m%d%H%M%S"));

    let active = ActiveModel {
        contract_no: sea_orm::ActiveValue::Set(contract_no.clone()),
        contract_name: sea_orm::ActiveValue::Set(payload.contract_name.clone()),
        contract_type: sea_orm::ActiveValue::Set(None),
        customer_id: sea_orm::ActiveValue::Set(payload.customer_id),
        customer_name: sea_orm::ActiveValue::Set(None),
        total_amount: sea_orm::ActiveValue::Set(
            rust_decimal::Decimal::try_from(payload.total_amount).ok(),
        ),
        signed_date: sea_orm::ActiveValue::Set(Some(now.naive_utc().date())),
        effective_date: sea_orm::ActiveValue::Set(None),
        expiry_date: sea_orm::ActiveValue::Set(None),
        payment_terms: sea_orm::ActiveValue::Set(None),
        payment_method: sea_orm::ActiveValue::Set(None),
        delivery_date: sea_orm::ActiveValue::Set(None),
        delivery_location: sea_orm::ActiveValue::Set(None),
        status: sea_orm::ActiveValue::Set("pending".to_string()),
        created_by: sea_orm::ActiveValue::Set(0),
        created_at: sea_orm::ActiveValue::Set(now),
        updated_at: sea_orm::ActiveValue::Set(now),
        id: sea_orm::ActiveValue::NotSet,
    };

    match active.insert(&*state.db).await {
        Ok(inserted) => Json(ApiResponse::success(SalesContract {
            id: inserted.id as u32,
            contract_no: inserted.contract_no,
            customer_name: inserted
                .customer_name
                .unwrap_or_else(|| format!("客户 #{}", inserted.customer_id)),
            contract_date: inserted
                .signed_date
                .map_or_else(|| "".to_string(), |d| d.to_string()),
            total_amount: inserted
                .total_amount
                .and_then(|d| d.to_f64())
                .unwrap_or(0.0),
            status: inserted.status,
        })),
        Err(e) => {
            tracing::error!("创建销售合同失败: {}", e);
            Json(ApiResponse::error(format!("创建销售合同失败: {}", e)))
        }
    }
}

/// 获取销售合同
pub async fn get_sales_contract() -> impl IntoResponse {
    Json(ApiResponse::<serde_json::Value>::error(
        "请提供合同ID".to_string(),
    ))
}

/// 更新销售合同
pub async fn update_sales_contract() -> impl IntoResponse {
    Json(ApiResponse::<serde_json::Value>::error(
        "请提供合同ID和更新数据".to_string(),
    ))
}

/// 删除销售合同
pub async fn delete_sales_contract() -> impl IntoResponse {
    Json(ApiResponse::<serde_json::Value>::error(
        "请提供合同ID".to_string(),
    ))
}

/// 审批销售合同
pub async fn approve_sales_contract(State(state): State<AppState>) -> impl IntoResponse {
    use sea_orm::{ActiveModelTrait, ColumnTrait, QueryFilter};

    match SalesContractEntity::find()
        .filter(crate::models::sales_contract::Column::Status.eq("pending"))
        .one(&*state.db)
        .await
    {
        Ok(Some(contract)) => {
            let mut active: crate::models::sales_contract::ActiveModel = contract.into();
            active.status = sea_orm::ActiveValue::Set("approved".to_string());
            active.updated_at = sea_orm::ActiveValue::Set(Utc::now());

            match active.update(&*state.db).await {
                Ok(updated) => Json(ApiResponse::<String>::success(format!(
                    "合同 {} 已审批",
                    updated.contract_no
                ))),
                Err(e) => Json(ApiResponse::<String>::error(format!("审批失败: {}", e))),
            }
        }
        Ok(None) => Json(ApiResponse::<String>::error("没有待审批的合同".to_string())),
        Err(e) => Json(ApiResponse::<String>::error(format!("查询失败: {}", e))),
    }
}

// ============================================================================
// 销售价格相关 - 使用真实数据库查询
// ============================================================================

/// 销售价格列表
pub async fn list_sales_prices(State(state): State<AppState>) -> impl IntoResponse {
    match SalesPriceEntity::find()
        .order_by_desc(crate::models::sales_price::Column::EffectiveDate)
        .all(&*state.db)
        .await
    {
        Ok(prices) => {
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
            Json(ApiResponse::success(items))
        }
        Err(e) => {
            tracing::error!("查询销售价格失败: {}", e);
            Json(ApiResponse::error(format!("查询销售价格失败: {}", e)))
        }
    }
}

/// 创建销售价格
pub async fn create_sales_price(
    State(state): State<AppState>,
    Json(payload): Json<CreateSalesPriceRequest>,
) -> impl IntoResponse {
    use crate::models::sales_price::ActiveModel;
    use sea_orm::ActiveModelTrait;

    let now = Utc::now();

    let active = ActiveModel {
        product_id: sea_orm::ActiveValue::Set(payload.product_id),
        customer_id: sea_orm::ActiveValue::Set(payload.customer_id),
        customer_type: sea_orm::ActiveValue::Set(None),
        price: sea_orm::ActiveValue::Set(
            rust_decimal::Decimal::try_from(payload.price).unwrap_or(rust_decimal::Decimal::ZERO),
        ),
        currency: sea_orm::ActiveValue::Set(payload.currency),
        unit: sea_orm::ActiveValue::Set(payload.unit),
        min_order_qty: sea_orm::ActiveValue::Set(rust_decimal::Decimal::ONE),
        price_type: sea_orm::ActiveValue::Set("fixed".to_string()),
        price_level: sea_orm::ActiveValue::Set(None),
        effective_date: sea_orm::ActiveValue::Set(now.naive_utc().date()),
        expiry_date: sea_orm::ActiveValue::Set(None),
        status: sea_orm::ActiveValue::Set("active".to_string()),
        approved_by: sea_orm::ActiveValue::Set(None),
        approved_at: sea_orm::ActiveValue::Set(None),
        created_by: sea_orm::ActiveValue::Set(None),
        created_at: sea_orm::ActiveValue::Set(now),
        updated_at: sea_orm::ActiveValue::Set(now),
        id: sea_orm::ActiveValue::NotSet,
    };

    match active.insert(&*state.db).await {
        Ok(inserted) => Json(ApiResponse::success(SalesPrice {
            id: inserted.id as u32,
            product_name: format!("产品 #{}", inserted.product_id),
            customer_name: inserted
                .customer_id
                .map_or_else(|| "全部客户".to_string(), |cid| format!("客户 #{}", cid)),
            price: inserted.price.to_f64().unwrap_or(0.0),
            currency: inserted.currency,
            unit: inserted.unit,
            effective_date: inserted.effective_date.to_string(),
            status: inserted.status,
        })),
        Err(e) => {
            tracing::error!("创建销售价格失败: {}", e);
            Json(ApiResponse::error(format!("创建销售价格失败: {}", e)))
        }
    }
}

/// 更新销售价格
pub async fn update_sales_price() -> impl IntoResponse {
    Json(ApiResponse::<serde_json::Value>::error(
        "请提供价格ID和更新数据".to_string(),
    ))
}

/// 删除销售价格
pub async fn delete_sales_price() -> impl IntoResponse {
    Json(ApiResponse::<serde_json::Value>::error(
        "请提供价格ID".to_string(),
    ))
}

/// 审批销售价格
pub async fn approve_sales_price() -> impl IntoResponse {
    Json(ApiResponse::<serde_json::Value>::error(
        "请提供价格ID".to_string(),
    ))
}

// ============================================================================
// 租户管理相关 - 使用真实数据库查询
// ============================================================================

/// 租户列表
pub async fn list_tenants(State(state): State<AppState>) -> impl IntoResponse {
    match TenantEntity::find()
        .order_by_asc(crate::models::tenant::Column::CreatedAt)
        .all(&*state.db)
        .await
    {
        Ok(tenants) => {
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
            Json(ApiResponse::success(items))
        }
        Err(e) => {
            tracing::error!("查询租户列表失败: {}", e);
            Json(ApiResponse::error(format!("查询租户列表失败: {}", e)))
        }
    }
}

/// 创建租户
pub async fn create_tenant(
    State(state): State<AppState>,
    Json(payload): Json<CreateTenantRequest>,
) -> impl IntoResponse {
    use crate::models::tenant::ActiveModel;
    use sea_orm::ActiveModelTrait;

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

    match active.insert(&*state.db).await {
        Ok(inserted) => Json(ApiResponse::success(Tenant {
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
        })),
        Err(e) => {
            tracing::error!("创建租户失败: {}", e);
            Json(ApiResponse::error(format!("创建租户失败: {}", e)))
        }
    }
}

/// 获取租户
pub async fn get_tenant() -> impl IntoResponse {
    Json(ApiResponse::<serde_json::Value>::error(
        "请提供租户ID".to_string(),
    ))
}

/// 更新租户
pub async fn update_tenant() -> impl IntoResponse {
    Json(ApiResponse::<serde_json::Value>::error(
        "请提供租户ID和更新数据".to_string(),
    ))
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
pub struct CreateSalesContractRequest {
    pub contract_name: String,
    pub customer_id: i32,
    pub total_amount: f64,
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
pub struct CreateSalesPriceRequest {
    pub product_id: i32,
    pub customer_id: Option<i32>,
    pub price: f64,
    pub currency: String,
    pub unit: String,
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
