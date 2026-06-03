//! 补货与采购 handler
//!
//! 包含采购合同、采购价格、销售退货三类与库存调整相关的端点。

use axum::{extract::State, response::IntoResponse, Json};
use chrono::Utc;
use rust_decimal::prelude::ToPrimitive;
use sea_orm::{EntityTrait, QueryOrder};
use serde::{Deserialize, Serialize};

use crate::models::purchase_contract::Entity as PurchaseContractEntity;
use crate::models::purchase_price::Entity as PurchasePriceEntity;
use crate::models::sales_return::Entity as SalesReturnEntity;
use crate::utils::app_state::AppState;
use crate::utils::response::ApiResponse;

// ============================================================================
// 采购合同相关 - 使用真实数据库查询
// ============================================================================

/// 采购合同列表
pub async fn list_purchase_contracts(State(state): State<AppState>) -> impl IntoResponse {
    match PurchaseContractEntity::find()
        .order_by_desc(crate::models::purchase_contract::Column::CreatedAt)
        .all(&*state.db)
        .await
    {
        Ok(contracts) => {
            let items: Vec<PurchaseContract> = contracts
                .into_iter()
                .map(|c| PurchaseContract {
                    id: c.id as u32,
                    contract_no: c.contract_no,
                    supplier_name: c
                        .supplier_name
                        .unwrap_or_else(|| format!("供应商 #{}", c.supplier_id)),
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
            tracing::error!("查询采购合同失败: {}", e);
            Json(ApiResponse::error(format!("查询采购合同失败: {}", e)))
        }
    }
}

/// 创建采购合同
pub async fn create_purchase_contract(
    State(state): State<AppState>,
    Json(payload): Json<CreatePurchaseContractRequest>,
) -> impl IntoResponse {
    use crate::models::purchase_contract::ActiveModel;
    use sea_orm::ActiveModelTrait;

    let now = Utc::now();
    let contract_no = format!("PC{}", now.format("%Y%m%d%H%M%S"));

    let active = ActiveModel {
        contract_no: sea_orm::ActiveValue::Set(contract_no.clone()),
        contract_name: sea_orm::ActiveValue::Set(payload.contract_name.clone()),
        contract_type: sea_orm::ActiveValue::Set(None),
        supplier_id: sea_orm::ActiveValue::Set(payload.supplier_id),
        supplier_name: sea_orm::ActiveValue::Set(None),
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
        Ok(inserted) => Json(ApiResponse::success(PurchaseContract {
            id: inserted.id as u32,
            contract_no: inserted.contract_no,
            supplier_name: inserted
                .supplier_name
                .unwrap_or_else(|| format!("供应商 #{}", inserted.supplier_id)),
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
            tracing::error!("创建采购合同失败: {}", e);
            Json(ApiResponse::error(format!("创建采购合同失败: {}", e)))
        }
    }
}

/// 获取采购合同
pub async fn get_purchase_contract() -> impl IntoResponse {
    Json(ApiResponse::<serde_json::Value>::error(
        "请提供合同ID".to_string(),
    ))
}

/// 更新采购合同
pub async fn update_purchase_contract() -> impl IntoResponse {
    Json(ApiResponse::<serde_json::Value>::error(
        "请提供合同ID和更新数据".to_string(),
    ))
}

/// 删除采购合同
pub async fn delete_purchase_contract() -> impl IntoResponse {
    Json(ApiResponse::<serde_json::Value>::error(
        "请提供合同ID".to_string(),
    ))
}

/// 审批采购合同
pub async fn approve_purchase_contract(State(state): State<AppState>) -> impl IntoResponse {
    use sea_orm::{ActiveModelTrait, ColumnTrait, QueryFilter};

    match PurchaseContractEntity::find()
        .filter(crate::models::purchase_contract::Column::Status.eq("pending"))
        .one(&*state.db)
        .await
    {
        Ok(Some(contract)) => {
            let mut active: crate::models::purchase_contract::ActiveModel = contract.into();
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

/// 执行采购合同
pub async fn execute_purchase_contract() -> impl IntoResponse {
    Json(ApiResponse::<serde_json::Value>::error(
        "请提供合同ID".to_string(),
    ))
}

// ============================================================================
// 采购价格相关 - 使用真实数据库查询
// ============================================================================

/// 采购价格列表
pub async fn list_purchase_prices(State(state): State<AppState>) -> impl IntoResponse {
    match PurchasePriceEntity::find()
        .order_by_desc(crate::models::purchase_price::Column::EffectiveDate)
        .all(&*state.db)
        .await
    {
        Ok(prices) => {
            let items: Vec<PurchasePrice> = prices
                .into_iter()
                .map(|p| PurchasePrice {
                    id: p.id as u32,
                    product_name: format!("产品 #{}", p.product_id),
                    supplier_name: format!("供应商 #{}", p.supplier_id),
                    price: p.price.to_f64().unwrap_or(0.0),
                    currency: p.currency,
                    unit: p.unit,
                    effective_date: p.effective_date.to_string(),
                    expiry_date: p
                        .expiry_date
                        .map_or_else(|| "".to_string(), |d| d.to_string()),
                    status: p.status,
                })
                .collect();
            Json(ApiResponse::success(items))
        }
        Err(e) => {
            tracing::error!("查询采购价格失败: {}", e);
            Json(ApiResponse::error(format!("查询采购价格失败: {}", e)))
        }
    }
}

/// 创建采购价格
pub async fn create_purchase_price(
    State(state): State<AppState>,
    Json(payload): Json<CreatePurchasePriceRequest>,
) -> impl IntoResponse {
    use crate::models::purchase_price::ActiveModel;
    use sea_orm::ActiveModelTrait;

    let now = Utc::now();

    let active = ActiveModel {
        product_id: sea_orm::ActiveValue::Set(payload.product_id),
        supplier_id: sea_orm::ActiveValue::Set(payload.supplier_id),
        price: sea_orm::ActiveValue::Set(
            rust_decimal::Decimal::try_from(payload.price).unwrap_or(rust_decimal::Decimal::ZERO),
        ),
        currency: sea_orm::ActiveValue::Set(payload.currency),
        unit: sea_orm::ActiveValue::Set(payload.unit),
        min_order_qty: sea_orm::ActiveValue::Set(rust_decimal::Decimal::ONE),
        price_type: sea_orm::ActiveValue::Set("fixed".to_string()),
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
        Ok(inserted) => Json(ApiResponse::success(PurchasePrice {
            id: inserted.id as u32,
            product_name: format!("产品 #{}", inserted.product_id),
            supplier_name: format!("供应商 #{}", inserted.supplier_id),
            price: inserted.price.to_f64().unwrap_or(0.0),
            currency: inserted.currency,
            unit: inserted.unit,
            effective_date: inserted.effective_date.to_string(),
            expiry_date: inserted
                .expiry_date
                .map_or_else(|| "".to_string(), |d| d.to_string()),
            status: inserted.status,
        })),
        Err(e) => {
            tracing::error!("创建采购价格失败: {}", e);
            Json(ApiResponse::error(format!("创建采购价格失败: {}", e)))
        }
    }
}

/// 更新采购价格
pub async fn update_purchase_price() -> impl IntoResponse {
    Json(ApiResponse::<serde_json::Value>::error(
        "请提供价格ID和更新数据".to_string(),
    ))
}

/// 删除采购价格
pub async fn delete_purchase_price() -> impl IntoResponse {
    Json(ApiResponse::<serde_json::Value>::error(
        "请提供价格ID".to_string(),
    ))
}

/// 审批采购价格
pub async fn approve_purchase_price() -> impl IntoResponse {
    Json(ApiResponse::<serde_json::Value>::error(
        "请提供价格ID".to_string(),
    ))
}

// ============================================================================
// 销售退货相关 - 使用真实数据库查询
// ============================================================================

/// 销售退货列表
pub async fn list_sales_returns(State(state): State<AppState>) -> impl IntoResponse {
    match SalesReturnEntity::find()
        .order_by_desc(crate::models::sales_return::Column::CreatedAt)
        .all(&*state.db)
        .await
    {
        Ok(returns) => {
            let items: Vec<SalesReturn> = returns
                .into_iter()
                .map(|r| SalesReturn {
                    id: r.id as u32,
                    return_no: r.return_no,
                    customer_name: format!("客户 #{}", r.customer_id),
                    order_no: r
                        .sales_order_id
                        .map_or_else(|| "".to_string(), |oid| format!("SO{}", oid)),
                    return_date: r.return_date.to_string(),
                    total_amount: r.total_amount.to_f64().unwrap_or(0.0),
                    reason: r.reason,
                    status: r.status,
                })
                .collect();
            Json(ApiResponse::success(items))
        }
        Err(e) => {
            tracing::error!("查询销售退货失败: {}", e);
            Json(ApiResponse::error(format!("查询销售退货失败: {}", e)))
        }
    }
}

/// 创建销售退货
pub async fn create_sales_return(
    State(state): State<AppState>,
    Json(payload): Json<CreateSalesReturnRequest>,
) -> impl IntoResponse {
    use crate::models::sales_return::ActiveModel;
    use sea_orm::ActiveModelTrait;

    let now = Utc::now();
    let return_no = format!("SR{}", now.format("%Y%m%d%H%M%S"));

    let active = ActiveModel {
        return_no: sea_orm::ActiveValue::Set(return_no.clone()),
        sales_order_id: sea_orm::ActiveValue::Set(Some(payload.order_id)),
        customer_id: sea_orm::ActiveValue::Set(payload.customer_id),
        return_date: sea_orm::ActiveValue::Set(now.naive_utc().date()),
        warehouse_id: sea_orm::ActiveValue::Set(1),
        reason: sea_orm::ActiveValue::Set(payload.reason.clone()),
        status: sea_orm::ActiveValue::Set("pending".to_string()),
        total_amount: sea_orm::ActiveValue::Set(
            rust_decimal::Decimal::try_from(payload.total_amount)
                .unwrap_or(rust_decimal::Decimal::ZERO),
        ),
        remarks: sea_orm::ActiveValue::Set(None),
        approved_by: sea_orm::ActiveValue::Set(None),
        approved_at: sea_orm::ActiveValue::Set(None),
        rejected_reason: sea_orm::ActiveValue::Set(None),
        created_by: sea_orm::ActiveValue::Set(0),
        created_at: sea_orm::ActiveValue::Set(now),
        updated_at: sea_orm::ActiveValue::Set(now),
        id: sea_orm::ActiveValue::NotSet,
    };

    match active.insert(&*state.db).await {
        Ok(inserted) => Json(ApiResponse::success(SalesReturn {
            id: inserted.id as u32,
            return_no: inserted.return_no,
            customer_name: format!("客户 #{}", inserted.customer_id),
            order_no: inserted
                .sales_order_id
                .map_or_else(|| "".to_string(), |oid| format!("SO{}", oid)),
            return_date: inserted.return_date.to_string(),
            total_amount: inserted.total_amount.to_f64().unwrap_or(0.0),
            reason: inserted.reason,
            status: inserted.status,
        })),
        Err(e) => {
            tracing::error!("创建销售退货失败: {}", e);
            Json(ApiResponse::error(format!("创建销售退货失败: {}", e)))
        }
    }
}

/// 获取销售退货
pub async fn get_sales_return() -> impl IntoResponse {
    Json(ApiResponse::<serde_json::Value>::error(
        "请提供退货ID".to_string(),
    ))
}

/// 更新销售退货
pub async fn update_sales_return() -> impl IntoResponse {
    Json(ApiResponse::<serde_json::Value>::error(
        "请提供退货ID和更新数据".to_string(),
    ))
}

/// 删除销售退货
pub async fn delete_sales_return() -> impl IntoResponse {
    Json(ApiResponse::<serde_json::Value>::error(
        "请提供退货ID".to_string(),
    ))
}

// ============================================================================
// 数据结构
// ============================================================================

#[derive(Debug, Serialize, Deserialize)]
pub struct CreatePurchaseContractRequest {
    pub contract_name: String,
    pub supplier_id: i32,
    pub total_amount: f64,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct PurchaseContract {
    pub id: u32,
    pub contract_no: String,
    pub supplier_name: String,
    pub contract_date: String,
    pub total_amount: f64,
    pub status: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct CreatePurchasePriceRequest {
    pub product_id: i32,
    pub supplier_id: i32,
    pub price: f64,
    pub currency: String,
    pub unit: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct PurchasePrice {
    pub id: u32,
    pub product_name: String,
    pub supplier_name: String,
    pub price: f64,
    pub currency: String,
    pub unit: String,
    pub effective_date: String,
    pub expiry_date: String,
    pub status: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct CreateSalesReturnRequest {
    pub order_id: i32,
    pub customer_id: i32,
    pub total_amount: f64,
    pub reason: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct SalesReturn {
    pub id: u32,
    pub return_no: String,
    pub customer_name: String,
    pub order_no: String,
    pub return_date: String,
    pub total_amount: f64,
    pub reason: String,
    pub status: String,
}
