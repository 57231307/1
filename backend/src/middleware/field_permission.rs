use crate::middleware::auth_context::AuthContext;
use crate::models::role;
use crate::services::field_permission_service::FieldPermissionService;
use crate::utils::app_state::AppState;
use axum::{
    body::Body,
    extract::State,
    http::{Request, Response},
    middleware::Next,
};
use sea_orm::EntityTrait;
use serde_json::Value;

/// 检查角色是否是管理员角色
#[allow(dead_code)]
async fn is_admin_role(db: &sea_orm::DatabaseConnection, role_id: i32) -> bool {
    // 从数据库查询角色，检查code是否为"admin"
    match role::Entity::find_by_id(role_id).one(db).await {
        Ok(Some(role)) => role.code == "admin",
        Ok(None) => false,
        Err(e) => {
            // 如果是表不存在的错误，说明系统还未初始化，允许访问
            let err_msg = format!("{}", e);
            if err_msg.contains("does not exist") || err_msg.contains("relation") {
                tracing::warn!("数据库表不存在，系统可能未初始化，允许访问: {}", e);
                true // 系统未初始化时允许所有操作
            } else {
                tracing::warn!("查询角色失败: {}", e);
                false
            }
        }
    }
}

/// 字段权限中间件
///
/// 在响应返回前自动过滤或掩码处理无权限的字段
#[allow(dead_code)]
pub async fn field_permission_middleware(
    State(state): State<AppState>,
    request: Request<Body>,
    next: Next,
) -> Response<Body> {
    // 获取认证上下文
    let auth = request.extensions().get::<AuthContext>().cloned();

    // 如果没有认证上下文或没有角色 ID，直接放行
    let auth = match auth {
        Some(a) => a,
        None => return next.run(request).await,
    };

    let role_id = match auth.role_id {
        Some(id) => id,
        None => return next.run(request).await,
    };

    // 检查是否是管理员角色（从数据库查询，而非硬编码）
    if is_admin_role(&state.db, role_id).await {
        return next.run(request).await;
    }

    // 从请求路径提取资源类型
    let path = request.uri().path();
    let resource_type = extract_resource_type(path);

    // 如果没有匹配的资源类型，直接放行
    if resource_type.is_none() {
        return next.run(request).await;
    }

    let resource_type = resource_type.unwrap();

    // 执行后续 handler
    let response = next.run(request).await;

    // 只处理 JSON 响应
    let content_type = response
        .headers()
        .get(axum::http::header::CONTENT_TYPE)
        .and_then(|v| v.to_str().ok());

    if !content_type
        .map(|ct| ct.contains("application/json"))
        .unwrap_or(false)
    {
        return response;
    }

    // 获取响应体
    let body_bytes = match axum::body::to_bytes(response.into_body(), usize::MAX).await {
        Ok(bytes) => bytes,
        Err(_) => {
            return Response::builder()
                .status(axum::http::StatusCode::INTERNAL_SERVER_ERROR)
                .body(Body::from("Failed to read response body"))
                .unwrap()
        }
    };

    // 尝试解析 JSON
    let mut json_value: Value = match serde_json::from_slice(&body_bytes) {
        Ok(v) => v,
        Err(_) => {
            // 不是有效 JSON，原样返回
            return Response::builder()
                .header(axum::http::header::CONTENT_TYPE, "application/json")
                .body(Body::from(body_bytes))
                .unwrap();
        }
    };

    // 获取字段权限
    let service = FieldPermissionService::new(state.db.clone());
    let permissions = match service
        .get_role_field_permissions(role_id, &resource_type)
        .await
    {
        Ok(perms) => perms,
        Err(e) => {
            tracing::warn!("获取字段权限失败: {}", e);
            return Response::builder()
                .header(axum::http::header::CONTENT_TYPE, "application/json")
                .body(Body::from(body_bytes))
                .unwrap();
        }
    };

    // 如果没有权限规则，直接返回
    if permissions.is_empty() {
        return Response::builder()
            .header(axum::http::header::CONTENT_TYPE, "application/json")
            .body(Body::from(body_bytes))
            .unwrap();
    }

    // 处理 JSON 数据
    process_response_json(&mut json_value, &permissions, &service);

    // 序列化回 JSON
    let new_body = match serde_json::to_vec(&json_value) {
        Ok(bytes) => bytes,
        Err(_) => body_bytes.to_vec(),
    };

    Response::builder()
        .header(axum::http::header::CONTENT_TYPE, "application/json")
        .body(Body::from(new_body))
        .unwrap()
}

/// 从路径中提取资源类型
#[allow(dead_code)]
fn extract_resource_type(path: &str) -> Option<String> {
    let path_parts: Vec<&str> = path.split('/').filter(|p| !p.is_empty()).collect();

    if path_parts.len() < 4
        || path_parts[0] != "api"
        || path_parts[1] != "v1"
        || path_parts[2] != "erp"
    {
        return None;
    }

    // 跳过需要排除的路径（权限管理自身的路由）
    if path_parts[3] == "permissions" {
        return None;
    }

    // 处理嵌套路径，如 /api/v1/erp/sales/orders
    let resource_idx = if path_parts.len() >= 5 && is_module_prefix(path_parts[3]) {
        4
    } else {
        3
    };

    if resource_idx < path_parts.len() {
        // 将连字符转换为下划线（如 sales-orders -> sales_orders）
        Some(path_parts[resource_idx].replace('-', "_"))
    } else {
        None
    }
}

/// 判断是否为模块前缀
#[allow(dead_code)]
fn is_module_prefix(part: &str) -> bool {
    matches!(
        part,
        "sales"
            | "purchases"
            | "finance"
            | "inventory"
            | "gl"
            | "ap"
            | "ar"
            | "bpm"
            | "crm"
            | "ai"
            | "reports"
            | "tenants"
            | "webhooks"
            | "supplier-evaluation"
            | "customer-credits"
            | "financial-analysis"
            | "fund-management"
            | "quality-inspection"
            | "cost-collections"
            | "sales-analysis"
            | "sales-prices"
            | "purchase-prices"
            | "sales-returns"
            | "ar-reconciliations"
            | "exchange-rates"
    )
}

/// 处理响应 JSON，应用字段过滤和掩码
#[allow(dead_code)]
fn process_response_json(
    value: &mut Value,
    permissions: &[crate::services::field_permission_service::FieldPermissionDetail],
    service: &FieldPermissionService,
) {
    // 处理 ApiResponse 格式: { "code": 200, "data": ... }
    if let Some(obj) = value.as_object_mut() {
        if let Some(data_field) = obj.get_mut("data") {
            process_data_value(data_field, permissions, service);
        }
    } else {
        // 直接是数组或对象
        process_data_value(value, permissions, service);
    }
}

/// 处理 data 字段的值
#[allow(dead_code)]
fn process_data_value(
    data: &mut Value,
    permissions: &[crate::services::field_permission_service::FieldPermissionDetail],
    service: &FieldPermissionService,
) {
    match data {
        Value::Array(arr) => {
            for item in arr.iter_mut() {
                service.filter_fields_by_read_permission(item, permissions);
                service.mask_fields(item, permissions);
            }
        }
        Value::Object(_) => {
            // 检查是否为分页响应 { "items": [...], "total": ... }
            if let Some(obj) = data.as_object_mut() {
                if let Some(Value::Array(arr)) = obj.get_mut("items") {
                    for item in arr.iter_mut() {
                        service.filter_fields_by_read_permission(item, permissions);
                        service.mask_fields(item, permissions);
                    }
                }
                // 也处理顶层对象本身
                service.filter_fields_by_read_permission(data, permissions);
                service.mask_fields(data, permissions);
            }
        }
        _ => {}
    }
}
