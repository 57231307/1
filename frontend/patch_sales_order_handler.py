with open("/home/root0/桌面/121/1/backend/src/handlers/sales_order_handler.rs", "r") as f:
    content = f.read()

# Modify get_order to include AuthContext and sanitize
get_order_old = """pub async fn get_order(
    State(state): State<AppState>,
    Path(id): Path<i32>,
) -> Result<Json<ApiResponse<serde_json::Value>>, AppError> {
    let sales_service = SalesService::new(state.db.clone());
    let order = sales_service.get_order_detail(id).await?;
    let order_json = serde_json::to_value(order).map_err(|e| AppError::InternalError(format!("序列化失败: {}", e)))?;
    Ok(Json(ApiResponse::success(order_json)))
}"""

get_order_new = """pub async fn get_order(
    auth: AuthContext,
    State(state): State<AppState>,
    Path(id): Path<i32>,
) -> Result<Json<ApiResponse<serde_json::Value>>, AppError> {
    let sales_service = SalesService::new(state.db.clone());
    let order = sales_service.get_order_detail(id).await?;
    let mut order_json = serde_json::to_value(order).map_err(|e| AppError::InternalError(format!("序列化失败: {}", e)))?;
    
    // 字段级权限控制：如果角色不是管理员 (假设 ID 为 1)，则隐藏敏感的财务字段
    if auth.role_id != Some(1) {
        if let Some(obj) = order_json.as_object_mut() {
            obj.remove("subtotal");
            obj.remove("tax_amount");
            obj.remove("discount_amount");
            obj.remove("shipping_cost");
            obj.remove("total_amount");
            obj.remove("paid_amount");
            obj.remove("balance_amount");
            
            if let Some(items) = obj.get_mut("items").and_then(|i| i.as_array_mut()) {
                for item in items {
                    if let Some(item_obj) = item.as_object_mut() {
                        item_obj.remove("unit_price");
                        item_obj.remove("tax_rate");
                        item_obj.remove("total_price");
                    }
                }
            }
        }
    }
    
    Ok(Json(ApiResponse::success(order_json)))
}"""

content = content.replace(get_order_old, get_order_new)

with open("/home/root0/桌面/121/1/backend/src/handlers/sales_order_handler.rs", "w") as f:
    f.write(content)
