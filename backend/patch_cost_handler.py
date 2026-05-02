import re

with open('backend/src/handlers/cost_collection_handler.rs', 'r') as f:
    content = f.read()

new_funcs = '''
use axum::extract::Path;
use serde_json::Value as JsonValue;
use crate::services::cost_collection_service::UpdateCostCollectionRequest;

/// 获取成本归集详情
pub async fn get_collection(
    Path(id): Path<i32>,
    State(state): State<AppState>,
    auth: AuthContext,
) -> Result<Json<ApiResponse<JsonValue>>, AppError> {
    let service = CostCollectionService::new(state.db.clone());
    let collection = service.get_by_id(id).await?;
    Ok(Json(ApiResponse::success(serde_json::to_value(collection).map_err(|_| AppError::InternalError("序列化失败".into()))?)))
}

/// 更新成本归集
pub async fn update_collection(
    Path(id): Path<i32>,
    State(state): State<AppState>,
    auth: AuthContext,
    Json(req): Json<UpdateCostCollectionRequest>,
) -> Result<Json<ApiResponse<JsonValue>>, AppError> {
    let service = CostCollectionService::new(state.db.clone());
    let collection = service.update(id, req, auth.user_id).await?;
    Ok(Json(ApiResponse::success_with_message(
        serde_json::to_value(collection).map_err(|_| AppError::InternalError("序列化失败".into()))?,
        "成本归集更新成功",
    )))
}

/// 删除成本归集
pub async fn delete_collection(
    Path(id): Path<i32>,
    State(state): State<AppState>,
    auth: AuthContext,
) -> Result<Json<ApiResponse<()>>, AppError> {
    let service = CostCollectionService::new(state.db.clone());
    service.delete(id, auth.user_id).await?;
    Ok(Json(ApiResponse::success_with_message((), "成本归集删除成功")))
}
'''

if 'pub async fn get_collection' not in content:
    content += new_funcs

with open('backend/src/handlers/cost_collection_handler.rs', 'w') as f:
    f.write(content)
