use axum::{extract::State, Json};
use sea_orm::{ActiveModelTrait, ColumnTrait, EntityTrait, QueryFilter, Set};
use serde::Deserialize;

use crate::container::AppState;
use crate::middleware::auth_context::AuthContext;
use crate::models::customer;
use crate::utils::error::AppError;
use crate::utils::response::ApiResponse;

/// 客户合并请求
#[derive(Deserialize)]
pub struct MergeCustomerRequest {
    /// 源客户 ID（将被合并的客户）
    pub source_customer_id: i32,
    /// 目标客户 ID（合并后的主客户）
    pub target_customer_id: i32,
    /// 合并原因
    pub reason: Option<String>,
}

/// POST /api/v1/erp/customers/merge - 客户合并
/// batch-15 P3: 客户合并功能
pub async fn merge_customers(
    State(state): State<AppState>,
    _auth: AuthContext,
    Json(req): Json<MergeCustomerRequest>,
) -> Result<Json<ApiResponse<String>>, AppError> {
    if req.source_customer_id == req.target_customer_id {
        return Err(AppError::bad_request("不能合并同一个客户"));
    }

    // 检查源客户是否存在
    let source = customer::Entity::find_by_id(req.source_customer_id)
        .one(&*state.db)
        .await?
        .ok_or_else(|| AppError::not_found("源客户不存在"))?;

    // 检查目标客户是否存在
    let target = customer::Entity::find_by_id(req.target_customer_id)
        .one(&*state.db)
        .await?
        .ok_or_else(|| AppError::not_found("目标客户不存在"))?;

    // 开始事务
    let txn = state.db.begin().await?;

    // 将源客户的订单转移到目标客户
    // TODO: 需要更新 sales_orders、purchase_orders 等表的 customer_id

    // 将源客户标记为已合并
    let mut source_active: customer::ActiveModel = source.into();
    source_active.status = Set("merged".to_string());
    source_active.notes = Set(Some(format!(
        "已合并到客户 {} (ID: {})。原因: {}",
        target.customer_name,
        req.target_customer_id,
        req.reason.unwrap_or_else(|| "无".to_string())
    )));
    source_active.update(&txn).await?;

    txn.commit().await?;

    Ok(Json(ApiResponse::success(format!(
        "客户 {} 已成功合并到 {}",
        req.source_customer_id, req.target_customer_id
    ))))
}
