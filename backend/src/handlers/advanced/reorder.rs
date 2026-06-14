//! 补货与采购 handler
//!
//! 包含采购合同、采购价格、销售退货三类与库存调整相关的端点。

use axum::{extract::State, response::IntoResponse, Json};
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

// ============================================================================
// 数据结构
// ============================================================================

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
