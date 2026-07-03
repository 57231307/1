//! BOM物料清单 Handler
//!
//! BOM（Bill of Materials）物料清单API端点

use axum::{
    extract::{Path, Query, State},
    Json,
};
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use validator::Validate;

use crate::middleware::auth_context::AuthContext;
use crate::services::bom_service::{
    BomQuery, BomService, CreateBomItemRequest, CreateBomRequest, UpdateBomRequest,
};
use crate::utils::app_state::AppState;
use crate::utils::error::AppError;
use crate::utils::response::{ApiResponse, PaginatedResponse};

/// 创建BOM请求
#[derive(Debug, Clone, Deserialize, Serialize, Validate)]
pub struct CreateBomPayload {
    pub product_id: i32,
    pub version: Option<i32>,
    pub is_default: Option<bool>,
    pub remarks: Option<String>,
    #[validate(length(min = 1, message = "BOM明细不能为空"))]
    pub items: Vec<CreateBomItemPayload>,
}

/// 创建BOM明细请求
#[derive(Debug, Clone, Deserialize, Serialize, Validate)]
pub struct CreateBomItemPayload {
    pub material_id: i32,
    pub quantity: Decimal,
    pub unit: Option<String>,
    pub scrap_rate: Option<Decimal>,
    pub sort_order: Option<i32>,
}

/// 更新BOM请求
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct UpdateBomPayload {
    pub is_default: Option<bool>,
    pub status: Option<String>,
    pub remarks: Option<String>,
    pub items: Option<Vec<CreateBomItemPayload>>,
}

/// BOM响应
#[derive(Debug, Clone, Serialize)]
pub struct BomResponse {
    pub id: i32,
    pub product_id: i32,
    pub version: i32,
    pub is_default: bool,
    pub status: String,
    pub remarks: Option<String>,
    pub created_by: i32,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub updated_at: chrono::DateTime<chrono::Utc>,
}

/// BOM明细响应
#[derive(Debug, Clone, Serialize)]
pub struct BomItemResponse {
    pub id: i32,
    pub bom_id: i32,
    pub material_id: i32,
    pub quantity: Decimal,
    pub unit: Option<String>,
    pub scrap_rate: Option<Decimal>,
    pub sort_order: Option<i32>,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub updated_at: chrono::DateTime<chrono::Utc>,
}

/// BOM详情响应（含明细）
#[derive(Debug, Clone, Serialize)]
pub struct BomDetailResponse {
    pub bom: BomResponse,
    pub items: Vec<BomItemResponse>,
}

/// BOM列表查询参数
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct ListBomsQuery {
    pub product_id: Option<i32>,
    pub status: Option<String>,
    pub is_default: Option<bool>,
    pub page: Option<u64>,
    pub page_size: Option<u64>,
}

/// 创建BOM
pub async fn create_bom(
    State(state): State<AppState>,
    auth: AuthContext,
    Json(payload): Json<CreateBomPayload>,
) -> Result<Json<ApiResponse<BomDetailResponse>>, AppError> {
    payload
        .validate()
        .map_err(|e| AppError::validation(e.to_string()))?;

    let service = BomService::new(state.db.clone());

    let items: Vec<CreateBomItemRequest> = payload
        .items
        .iter()
        .map(|item| CreateBomItemRequest {
            material_id: item.material_id,
            quantity: item.quantity,
            unit: item.unit.clone(),
            scrap_rate: item.scrap_rate,
            sort_order: item.sort_order,
        })
        .collect();

    let req = CreateBomRequest {
        product_id: payload.product_id,
        version: payload.version,
        is_default: payload.is_default,
        remarks: payload.remarks,
        created_by: auth.user_id,
        items,
    };

    let detail = service.create(req).await?;

    Ok(Json(ApiResponse::success(BomDetailResponse {
        bom: BomResponse {
            id: detail.bom.id,
            product_id: detail.bom.product_id,
            version: detail.bom.version,
            is_default: detail.bom.is_default,
            status: detail.bom.status,
            remarks: detail.bom.remarks,
            created_by: detail.bom.created_by,
            created_at: detail.bom.created_at,
            updated_at: detail.bom.updated_at,
        },
        items: detail
            .items
            .iter()
            .map(|item| BomItemResponse {
                id: item.id,
                bom_id: item.bom_id,
                material_id: item.material_id,
                quantity: item.quantity,
                unit: item.unit.clone(),
                scrap_rate: item.scrap_rate,
                sort_order: item.sort_order,
                created_at: item.created_at,
                updated_at: item.updated_at,
            })
            .collect(),
    })))
}

/// 获取BOM详情
pub async fn get_bom(
    State(state): State<AppState>,
    Path(id): Path<i32>,
) -> Result<Json<ApiResponse<BomDetailResponse>>, AppError> {
    let service = BomService::new(state.db.clone());

    let detail = service
        .get_by_id(id)
        .await?
        .ok_or_else(|| AppError::not_found("BOM不存在"))?;

    Ok(Json(ApiResponse::success(BomDetailResponse {
        bom: BomResponse {
            id: detail.bom.id,
            product_id: detail.bom.product_id,
            version: detail.bom.version,
            is_default: detail.bom.is_default,
            status: detail.bom.status,
            remarks: detail.bom.remarks,
            created_by: detail.bom.created_by,
            created_at: detail.bom.created_at,
            updated_at: detail.bom.updated_at,
        },
        items: detail
            .items
            .iter()
            .map(|item| BomItemResponse {
                id: item.id,
                bom_id: item.bom_id,
                material_id: item.material_id,
                quantity: item.quantity,
                unit: item.unit.clone(),
                scrap_rate: item.scrap_rate,
                sort_order: item.sort_order,
                created_at: item.created_at,
                updated_at: item.updated_at,
            })
            .collect(),
    })))
}

/// 获取BOM列表
pub async fn list_boms(
    State(state): State<AppState>,
    Query(query): Query<ListBomsQuery>,
) -> Result<Json<ApiResponse<PaginatedResponse<BomResponse>>>, AppError> {
    let service = BomService::new(state.db.clone());

    let bom_query = BomQuery {
        product_id: query.product_id,
        status: query.status,
        is_default: query.is_default,
        page: query.page.unwrap_or(1).max(1), // 批次 95 P3-3~8：分页 clamp 防 DoS
        page_size: query.page_size.unwrap_or(20).clamp(1, 100),
    };

    let (boms, total) = service.list(bom_query).await?;

    let response: Vec<BomResponse> = boms
        .iter()
        .map(|bom| BomResponse {
            id: bom.id,
            product_id: bom.product_id,
            version: bom.version,
            is_default: bom.is_default,
            status: bom.status.clone(),
            remarks: bom.remarks.clone(),
            created_by: bom.created_by,
            created_at: bom.created_at,
            updated_at: bom.updated_at,
        })
        .collect();

    Ok(Json(ApiResponse::success_paginated(
        response.clone(),
        total,
        query.page.unwrap_or(1).max(1), // 批次 95 P3-3~8：分页 clamp 防 DoS
        query.page_size.unwrap_or(20).clamp(1, 100),
    )))
}

/// 更新BOM
pub async fn update_bom(
    State(state): State<AppState>,
    Path(id): Path<i32>,
    Json(payload): Json<UpdateBomPayload>,
) -> Result<Json<ApiResponse<BomDetailResponse>>, AppError> {
    let service = BomService::new(state.db.clone());

    let items = payload.items.map(|items| {
        items
            .iter()
            .map(|item| CreateBomItemRequest {
                material_id: item.material_id,
                quantity: item.quantity,
                unit: item.unit.clone(),
                scrap_rate: item.scrap_rate,
                sort_order: item.sort_order,
            })
            .collect()
    });

    let req = UpdateBomRequest {
        is_default: payload.is_default,
        status: payload.status,
        remarks: payload.remarks,
        items,
    };

    let detail = service.update(id, req).await?;

    Ok(Json(ApiResponse::success(BomDetailResponse {
        bom: BomResponse {
            id: detail.bom.id,
            product_id: detail.bom.product_id,
            version: detail.bom.version,
            is_default: detail.bom.is_default,
            status: detail.bom.status,
            remarks: detail.bom.remarks,
            created_by: detail.bom.created_by,
            created_at: detail.bom.created_at,
            updated_at: detail.bom.updated_at,
        },
        items: detail
            .items
            .iter()
            .map(|item| BomItemResponse {
                id: item.id,
                bom_id: item.bom_id,
                material_id: item.material_id,
                quantity: item.quantity,
                unit: item.unit.clone(),
                scrap_rate: item.scrap_rate,
                sort_order: item.sort_order,
                created_at: item.created_at,
                updated_at: item.updated_at,
            })
            .collect(),
    })))
}

/// 删除BOM
pub async fn delete_bom(
    State(state): State<AppState>,
    Path(id): Path<i32>,
) -> Result<Json<ApiResponse<String>>, AppError> {
    let service = BomService::new(state.db.clone());
    service.delete(id).await?;

    Ok(Json(ApiResponse::success_with_message(
        "BOM已删除".to_string(),
        "BOM已删除",
    )))
}

/// 获取BOM版本历史
pub async fn get_bom_versions(
    State(state): State<AppState>,
    Path(product_id): Path<i32>,
) -> Result<Json<ApiResponse<Vec<BomResponse>>>, AppError> {
    let service = BomService::new(state.db.clone());
    let boms = service.get_versions(product_id).await?;

    let response: Vec<BomResponse> = boms
        .iter()
        .map(|bom| BomResponse {
            id: bom.id,
            product_id: bom.product_id,
            version: bom.version,
            is_default: bom.is_default,
            status: bom.status.clone(),
            remarks: bom.remarks.clone(),
            created_by: bom.created_by,
            created_at: bom.created_at,
            updated_at: bom.updated_at,
        })
        .collect();

    Ok(Json(ApiResponse::success(response)))
}

/// 复制BOM
pub async fn copy_bom(
    State(state): State<AppState>,
    auth: AuthContext,
    Path(id): Path<i32>,
) -> Result<Json<ApiResponse<BomDetailResponse>>, AppError> {
    let service = BomService::new(state.db.clone());
    let detail = service.copy(id, auth.user_id).await?;

    Ok(Json(ApiResponse::success(BomDetailResponse {
        bom: BomResponse {
            id: detail.bom.id,
            product_id: detail.bom.product_id,
            version: detail.bom.version,
            is_default: detail.bom.is_default,
            status: detail.bom.status,
            remarks: detail.bom.remarks,
            created_by: detail.bom.created_by,
            created_at: detail.bom.created_at,
            updated_at: detail.bom.updated_at,
        },
        items: detail
            .items
            .iter()
            .map(|item| BomItemResponse {
                id: item.id,
                bom_id: item.bom_id,
                material_id: item.material_id,
                quantity: item.quantity,
                unit: item.unit.clone(),
                scrap_rate: item.scrap_rate,
                sort_order: item.sort_order,
                created_at: item.created_at,
                updated_at: item.updated_at,
            })
            .collect(),
    })))
}

/// 设置默认BOM
pub async fn set_default_bom(
    State(state): State<AppState>,
    Path(id): Path<i32>,
) -> Result<Json<ApiResponse<BomResponse>>, AppError> {
    let service = BomService::new(state.db.clone());
    let bom = service.set_default(id).await?;

    Ok(Json(ApiResponse::success(BomResponse {
        id: bom.id,
        product_id: bom.product_id,
        version: bom.version,
        is_default: bom.is_default,
        status: bom.status,
        remarks: bom.remarks,
        created_by: bom.created_by,
        created_at: bom.created_at,
        updated_at: bom.updated_at,
    })))
}

/// BOM树形结构查询参数
#[derive(Debug, Deserialize)]
pub struct BomTreeQuery {
    pub max_depth: Option<i32>,
}

/// GET /api/v1/erp/boms/:id/tree - 获取BOM树形结构
pub async fn get_bom_tree(
    State(state): State<AppState>,
    Path(id): Path<i32>,
    Query(query): Query<BomTreeQuery>,
) -> Result<Json<ApiResponse<serde_json::Value>>, AppError> {
    let service = BomService::new(state.db.clone());
    let tree = service.get_bom_tree(id, query.max_depth).await?;
    Ok(Json(ApiResponse::success(serde_json::to_value(tree)?)))
}

/// BOM需求计算请求
#[derive(Debug, Deserialize)]
pub struct BomRequirementRequest {
    pub quantity: rust_decimal::Decimal,
}

/// POST /api/v1/erp/boms/:id/requirements - 计算BOM需求
pub async fn calculate_bom_requirements(
    State(state): State<AppState>,
    Path(id): Path<i32>,
    Json(req): Json<BomRequirementRequest>,
) -> Result<Json<ApiResponse<serde_json::Value>>, AppError> {
    let service = BomService::new(state.db.clone());
    let requirements = service.calculate_bom_requirements(id, req.quantity).await?;
    Ok(Json(ApiResponse::success(serde_json::json!({
        "requirements": requirements,
        "total_items": requirements.len(),
    }))))
}

/// PUT /api/v1/erp/boms/:id/submit - 提交BOM审核
pub async fn submit_bom(
    State(state): State<AppState>,
    auth: AuthContext,
    Path(id): Path<i32>,
) -> Result<Json<ApiResponse<BomResponse>>, AppError> {
    let service = BomService::new(state.db.clone());
    // 批次 94 P2-10：注入真实操作人 user_id 用于审计日志
    let bom = service.submit(id, auth.user_id).await?;

    Ok(Json(ApiResponse::success_with_message(
        BomResponse {
            id: bom.id,
            product_id: bom.product_id,
            version: bom.version,
            is_default: bom.is_default,
            status: bom.status,
            remarks: bom.remarks,
            created_by: bom.created_by,
            created_at: bom.created_at,
            updated_at: bom.updated_at,
        },
        "BOM已提交审核",
    )))
}

/// 审核BOM请求
#[derive(Debug, Deserialize)]
pub struct ApproveBomRequest {
    pub approved: bool,
    pub remark: Option<String>,
}

/// PUT /api/v1/erp/boms/:id/approve - 审核BOM
pub async fn approve_bom(
    State(state): State<AppState>,
    auth: AuthContext,
    Path(id): Path<i32>,
    Json(req): Json<ApproveBomRequest>,
) -> Result<Json<ApiResponse<BomResponse>>, AppError> {
    let service = BomService::new(state.db.clone());
    // 批次 94 P2-10：注入真实操作人 user_id 用于审计日志
    let bom = service
        .approve(id, req.approved, req.remark, auth.user_id)
        .await?;

    Ok(Json(ApiResponse::success_with_message(
        BomResponse {
            id: bom.id,
            product_id: bom.product_id,
            version: bom.version,
            is_default: bom.is_default,
            status: bom.status,
            remarks: bom.remarks,
            created_by: bom.created_by,
            created_at: bom.created_at,
            updated_at: bom.updated_at,
        },
        if req.approved {
            "BOM已审核通过"
        } else {
            "BOM已驳回"
        },
    )))
}
