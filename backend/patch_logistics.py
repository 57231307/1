import re

with open('backend/src/handlers/logistics_handler.rs', 'r') as f:
    content = f.read()

new_funcs = '''
pub async fn get_waybill(
    Path(id): Path<i32>,
    State(state): State<AppState>,
) -> Result<Json<ApiResponse<logistics_waybill::Model>>, AppError> {
    let waybill = logistics_waybill::Entity::find_by_id(id)
        .one(&*state.db)
        .await?
        .ok_or_else(|| AppError::NotFound("运单不存在".to_string()))?;
    
    Ok(Json(ApiResponse::success(waybill)))
}

pub async fn delete_waybill(
    Path(id): Path<i32>,
    State(state): State<AppState>,
) -> Result<Json<ApiResponse<()>>, AppError> {
    let waybill = logistics_waybill::Entity::find_by_id(id)
        .one(&*state.db)
        .await?
        .ok_or_else(|| AppError::NotFound("运单不存在".to_string()))?;
    
    // 检查是否可以删除（例如：未发货的运单才能删除）
    if waybill.status == Some("IN_TRANSIT".to_string()) || waybill.status == Some("DELIVERED".to_string()) {
        return Err(AppError::BadRequest("运输中或已送达的运单不能删除".to_string()));
    }
    
    logistics_waybill::Entity::delete_by_id(id)
        .exec(&*state.db)
        .await?;
    
    Ok(Json(ApiResponse::success_with_message((), "运单删除成功")))
}
'''

content += new_funcs

with open('backend/src/handlers/logistics_handler.rs', 'w') as f:
    f.write(content)
