//! 管理服务集成测试
//!
//! 测试采购合同、销售合同、固定资产、预算管理四个服务的 API 端点

use axum::{
    body::Body,
    http::{Method, Request, StatusCode},
    Router,
};
use sea_orm::Database;
use serde_json::json;
use std::sync::Arc;
use tower::ServiceExt;

// 导入后端的路由创建函数
use bingxi_backend::routes::create_router;

/// 设置测试应用
async fn setup_app() -> Router {
    let db = Database::connect("sqlite::memory:").await.unwrap();
    create_router(Arc::new(db))
}

// ===================== 采购合同测试 =====================

/// 测试获取采购合同列表 - 未授权访问
#[tokio::test]
async fn test_get_purchase_contracts_unauthorized() {
    let app = setup_app().await;

    let response = app
        .oneshot(
            Request::builder()
                .method(Method::GET)
                .uri("/api/v1/erp/purchase-contracts")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
}

/// 测试创建采购合同 - 未授权访问
#[tokio::test]
async fn test_create_purchase_contract_unauthorized() {
    let app = setup_app().await;

    let response = app
        .oneshot(
            Request::builder()
                .method(Method::POST)
                .uri("/api/v1/erp/purchase-contracts")
                .header("Content-Type", "application/json")
                .body(Body::from(
                    json!({
                        "contract_no": "PC20260316001",
                        "contract_name": "测试采购合同",
                        "supplier_id": 1,
                        "total_amount": "10000.00",
                        "delivery_date": "2026-04-01"
                    })
                    .to_string(),
                ))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
}

/// 测试获取单个采购合同 - 未授权访问
#[tokio::test]
async fn test_get_purchase_contract_unauthorized() {
    let app = setup_app().await;

    let response = app
        .oneshot(
            Request::builder()
                .method(Method::GET)
                .uri("/api/v1/erp/purchase-contracts/1")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
}

// ===================== 销售合同测试 =====================

/// 测试获取销售合同列表 - 未授权访问
#[tokio::test]
async fn test_get_sales_contracts_unauthorized() {
    let app = setup_app().await;

    let response = app
        .oneshot(
            Request::builder()
                .method(Method::GET)
                .uri("/api/v1/erp/sales-contracts")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
}

/// 测试创建销售合同 - 未授权访问
#[tokio::test]
async fn test_create_sales_contract_unauthorized() {
    let app = setup_app().await;

    let response = app
        .oneshot(
            Request::builder()
                .method(Method::POST)
                .uri("/api/v1/erp/sales-contracts")
                .header("Content-Type", "application/json")
                .body(Body::from(
                    json!({
                        "contract_no": "SC20260316001",
                        "contract_name": "测试销售合同",
                        "customer_id": 1,
                        "total_amount": "15000.00",
                        "delivery_date": "2026-04-01"
                    })
                    .to_string(),
                ))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
}

/// 测试获取单个销售合同 - 未授权访问
#[tokio::test]
async fn test_get_sales_contract_unauthorized() {
    let app = setup_app().await;

    let response = app
        .oneshot(
            Request::builder()
                .method(Method::GET)
                .uri("/api/v1/erp/sales-contracts/1")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
}

// ===================== 固定资产测试 =====================

/// 测试获取固定资产列表 - 未授权访问
#[tokio::test]
async fn test_get_fixed_assets_unauthorized() {
    let app = setup_app().await;

    let response = app
        .oneshot(
            Request::builder()
                .method(Method::GET)
                .uri("/api/v1/erp/fixed-assets")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
}

/// 测试创建固定资产 - 未授权访问
#[tokio::test]
async fn test_create_fixed_asset_unauthorized() {
    let app = setup_app().await;

    let response = app
        .oneshot(
            Request::builder()
                .method(Method::POST)
                .uri("/api/v1/erp/fixed-assets")
                .header("Content-Type", "application/json")
                .body(Body::from(
                    json!({
                        "asset_no": "FA20260316001",
                        "asset_name": "测试设备",
                        "asset_category": "equipment",
                        "original_value": "50000.00",
                        "useful_life": 60,
                        "depreciation_method": "straight_line",
                        "purchase_date": "2026-03-16",
                        "put_in_date": "2026-03-16"
                    })
                    .to_string(),
                ))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
}

/// 测试获取单个固定资产 - 未授权访问
#[tokio::test]
async fn test_get_fixed_asset_unauthorized() {
    let app = setup_app().await;

    let response = app
        .oneshot(
            Request::builder()
                .method(Method::GET)
                .uri("/api/v1/erp/fixed-assets/1")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
}

// ===================== 预算管理测试 =====================

/// 测试获取预算科目列表 - 未授权访问
#[tokio::test]
async fn test_get_budget_items_unauthorized() {
    let app = setup_app().await;

    let response = app
        .oneshot(
            Request::builder()
                .method(Method::GET)
                .uri("/api/v1/erp/budget-items")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
}

/// 测试创建预算科目 - 未授权访问
#[tokio::test]
async fn test_create_budget_item_unauthorized() {
    let app = setup_app().await;

    let response = app
        .oneshot(
            Request::builder()
                .method(Method::POST)
                .uri("/api/v1/erp/budget-items")
                .header("Content-Type", "application/json")
                .body(Body::from(
                    json!({
                        "item_code": "BI001",
                        "item_name": "测试预算科目",
                        "item_type": "expense",
                        "level": 1
                    })
                    .to_string(),
                ))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
}

/// 测试获取单个预算科目 - 未授权访问
#[tokio::test]
async fn test_get_budget_item_unauthorized() {
    let app = setup_app().await;

    let response = app
        .oneshot(
            Request::builder()
                .method(Method::GET)
                .uri("/api/v1/erp/budget-items/1")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
}

// ===================== 404 路由测试 =====================

/// 测试采购合同 404 路由
#[tokio::test]
async fn test_purchase_contract_404() {
    let app = setup_app().await;

    let response = app
        .oneshot(
            Request::builder()
                .method(Method::GET)
                .uri("/api/v1/erp/purchase-contracts/nonexistent")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::NOT_FOUND);
}

/// 测试销售合同 404 路由
#[tokio::test]
async fn test_sales_contract_404() {
    let app = setup_app().await;

    let response = app
        .oneshot(
            Request::builder()
                .method(Method::GET)
                .uri("/api/v1/erp/sales-contracts/nonexistent")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::NOT_FOUND);
}

/// 测试固定资产 404 路由
#[tokio::test]
async fn test_fixed_asset_404() {
    let app = setup_app().await;

    let response = app
        .oneshot(
            Request::builder()
                .method(Method::GET)
                .uri("/api/v1/erp/fixed-assets/nonexistent")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::NOT_FOUND);
}

/// 测试预算管理 404 路由
#[tokio::test]
async fn test_budget_management_404() {
    let app = setup_app().await;

    let response = app
        .oneshot(
            Request::builder()
                .method(Method::GET)
                .uri("/api/v1/erp/budget-items/nonexistent")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::NOT_FOUND);
}
