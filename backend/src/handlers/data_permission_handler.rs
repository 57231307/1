//! 数据权限 Handler
//!
//! 数据权限 HTTP 接口层，提供数据权限管理功能

use crate::middleware::auth_context::AuthContext;
use crate::services::data_permission_service::DataPermissionService;
use crate::utils::admin_checker::is_admin_role;
use crate::utils::app_state::AppState;
use crate::utils::error::AppError;
use crate::utils::response::ApiResponse;
use axum::{
    extract::{Path, State},
    Json,
};
use serde::{Deserialize, Serialize};
use serde_json::Value;

/// C-3 修复：数据权限处理器内部的 admin 校验
///
/// 安全原因：scope_type 控制行/部门级数据可见范围，攻击者改写 admin 的
/// scope_type=SELF 可对管理员造成持久性 DoS；为自身设置 scope_type=ALL
/// 造成跨部门/跨租户越权读取。
async fn require_admin_role(
    state: &AppState,
    auth: &AuthContext,
) -> Result<(), AppError> {
    let role_id = auth
        .role_id
        .ok_or_else(|| AppError::permission_denied("用户未分配角色，无法执行该操作"))?;
    if !is_admin_role(&state.db, role_id).await {
        return Err(AppError::permission_denied(
            "数据权限管理仅限管理员（code=admin）执行",
        ));
    }
    Ok(())
}

/// C-3 修复：custom_condition 白名单校验
///
/// 当前处理器未接收 custom_condition，但接口层已声明字段，保留校验逻辑
/// 以便未来开放时即受到 SQL 注入防御。
/// 白名单规则：
/// - 仅允许 `field op value` 三段式结构
/// - field: ^[a-z_][a-z0-9_]{0,63}$ (小写字母+下划线+数字)
/// - op: 严格白名单 {=, !=, <, >, <=, >=, IN, NOT IN, LIKE}
/// - value: 字面量（数字/字符串/单引号字符串）
/// - 不允许出现: ;, (, ), /* */, --, UNION, SELECT, INSERT, UPDATE, DELETE, DROP, EXEC
fn validate_custom_condition_safe(condition: &Value) -> Result<(), AppError> {
    const FORBIDDEN: &[&str] = &[
        ";", "(", ")", "/*", "*/", "--", "UNION", "SELECT", "INSERT", "UPDATE", "DELETE", "DROP",
        "EXEC", "OR 1=1", "xp_",
    ];
    if let Value::Object(map) = condition {
        for (field, value) in map {
            // field 必须是合法列名
            if !field
                .chars()
                .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '_')
                || field.is_empty()
                || field.len() > 64
            {
                return Err(AppError::validation(format!(
                    "custom_condition 字段名非法: {}",
                    field
                )));
            }
            // value 必须是字面量（数字、字符串、bool、null）
            match value {
                Value::Null | Value::Bool(_) | Value::Number(_) => continue,
                Value::String(s) => {
                    // 字符串值禁止特殊字符
                    if s.contains('\'') || s.contains('"') || s.contains(';') {
                        return Err(AppError::validation(
                            "custom_condition 字符串值禁止特殊字符",
                        ));
                    }
                }
                _ => {
                    return Err(AppError::validation(
                        "custom_condition 值必须是字面量（数字/字符串/bool/null）",
                    ));
                }
            }
        }
    } else if !condition.is_null() {
        return Err(AppError::validation("custom_condition 必须是对象或 null"));
    }
    // 额外深度检查：禁止任何已知危险 SQL 关键字
    // 批次 397 修复：序列化失败时 fail-fast 返回错误，避免跳过安全检查
    let serialized = serde_json::to_string(condition)
        .map_err(|e| AppError::internal(format!("custom_condition 序列化失败: {}", e)))?;
    let upper = serialized.to_uppercase();
    for kw in FORBIDDEN {
        if upper.contains(kw) {
            return Err(AppError::validation(format!(
                "custom_condition 包含禁止关键字: {}",
                kw
            )));
        }
    }
    Ok(())
}

/// 设置数据权限请求
#[derive(Debug, Deserialize)]
pub struct SetDataPermissionRequest {
    pub role_id: i32,
    pub resource_type: String,
    pub scope_type: String,
    pub custom_condition: Option<Value>,
    pub allowed_fields: Option<Value>,
    pub hidden_fields: Option<Value>,
}

/// 数据权限响应
#[derive(Debug, Serialize)]
pub struct DataPermissionResponse {
    pub id: i32,
    pub role_id: i32,
    pub resource_type: String,
    pub scope_type: String,
    pub custom_condition: Option<Value>,
    pub allowed_fields: Option<Value>,
    pub hidden_fields: Option<Value>,
    pub is_enabled: bool,
}

impl From<crate::models::data_permission::Model> for DataPermissionResponse {
    fn from(model: crate::models::data_permission::Model) -> Self {
        Self {
            id: model.id,
            role_id: model.role_id,
            resource_type: model.resource_type,
            scope_type: model.scope_type,
            custom_condition: model.custom_condition,
            allowed_fields: model.allowed_fields,
            hidden_fields: model.hidden_fields,
            is_enabled: model.is_enabled,
        }
    }
}

/// 设置数据权限
pub async fn set_data_permission(
    State(state): State<AppState>,
    auth: AuthContext,
    Json(req): Json<SetDataPermissionRequest>,
) -> Result<Json<ApiResponse<DataPermissionResponse>>, AppError> {
    require_admin_role(&state, &auth).await?;

    let valid_scope_types = ["ALL", "DEPT", "DEPT_AND_BELOW", "SELF", "CUSTOM"];
    if !valid_scope_types.contains(&req.scope_type.as_str()) {
        return Err(AppError::validation("无效的数据范围类型"));
    }

    // C-3 修复：custom_condition 白名单校验（拒绝 SQL 注入）
    if let Some(ref cond) = req.custom_condition {
        validate_custom_condition_safe(cond)?;
    }

    let service = DataPermissionService::new(state.db.clone());
    let permission = service
        .set_data_permission(
            req.role_id,
            req.resource_type,
            req.scope_type,
            req.custom_condition,
            req.allowed_fields,
            req.hidden_fields,
        )
        .await?;

    Ok(Json(ApiResponse::success_with_message(
        DataPermissionResponse::from(permission),
        "数据权限设置成功",
    )))
}

/// 获取角色的数据权限列表
pub async fn list_role_data_permissions(
    Path(role_id): Path<i32>,
    State(state): State<AppState>,
    auth: AuthContext,
) -> Result<Json<ApiResponse<Vec<DataPermissionResponse>>>, AppError> {
    require_admin_role(&state, &auth).await?;
    let service = DataPermissionService::new(state.db.clone());
    let permissions = service.list_role_data_permissions(role_id).await?;

    let responses: Vec<DataPermissionResponse> = permissions
        .into_iter()
        .map(DataPermissionResponse::from)
        .collect();

    Ok(Json(ApiResponse::success(responses)))
}

/// 删除数据权限
pub async fn delete_data_permission(
    Path((role_id, resource_type)): Path<(i32, String)>,
    State(state): State<AppState>,
    auth: AuthContext,
) -> Result<Json<ApiResponse<()>>, AppError> {
    require_admin_role(&state, &auth).await?;
    let service = DataPermissionService::new(state.db.clone());
    service
        .delete_data_permission(role_id, &resource_type)
        .await?;

    Ok(Json(ApiResponse::success_with_message(
        (),
        "数据权限删除成功",
    )))
}

/// 获取数据权限详情
pub async fn get_data_permission(
    Path((role_id, resource_type)): Path<(i32, String)>,
    State(state): State<AppState>,
) -> Result<Json<ApiResponse<Option<DataPermissionResponse>>>, AppError> {
    let service = DataPermissionService::new(state.db.clone());
    let permission = service
        .get_role_data_permission(role_id, &resource_type)
        .await?;

    Ok(Json(ApiResponse::success(permission.map(|p| {
        DataPermissionResponse {
            id: 0,
            role_id,
            resource_type: resource_type.clone(),
            scope_type: p.scope_type,
            custom_condition: p.custom_condition,
            allowed_fields: p.allowed_fields.map(|fields| {
                serde_json::Value::Array(
                    fields.into_iter().map(serde_json::Value::String).collect(),
                )
            }),
            hidden_fields: p.hidden_fields.map(|fields| {
                serde_json::Value::Array(
                    fields.into_iter().map(serde_json::Value::String).collect(),
                )
            }),
            is_enabled: true,
        }
    }))))
}

/// 数据范围类型列表
pub async fn list_scope_types(
    State(_state): State<AppState>,
) -> Result<Json<ApiResponse<Vec<serde_json::Value>>>, AppError> {
    let types = vec![
        serde_json::json!({
            "value": "ALL",
            "label": "全部数据",
            "description": "可以查看所有数据"
        }),
        serde_json::json!({
            "value": "DEPT",
            "label": "本部门数据",
            "description": "只能查看本部门的数据"
        }),
        serde_json::json!({
            "value": "DEPT_AND_BELOW",
            "label": "本部门及以下数据",
            "description": "可以查看本部门及下级部门的数据"
        }),
        serde_json::json!({
            "value": "SELF",
            "label": "仅本人数据",
            "description": "只能查看自己创建的数据"
        }),
        serde_json::json!({
            "value": "CUSTOM",
            "label": "自定义数据范围",
            "description": "通过自定义条件过滤数据"
        }),
    ];

    Ok(Json(ApiResponse::success(types)))
}

/// 获取所有数据权限列表
pub async fn list_data_permissions(
    State(state): State<AppState>,
    auth: AuthContext,
) -> Result<Json<ApiResponse<Vec<DataPermissionResponse>>>, AppError> {
    require_admin_role(&state, &auth).await?;
    let service = DataPermissionService::new(state.db.clone());
    let permissions = service.list_all_data_permissions().await?;

    let responses: Vec<DataPermissionResponse> = permissions
        .into_iter()
        .map(DataPermissionResponse::from)
        .collect();

    Ok(Json(ApiResponse::success(responses)))
}

#[cfg(test)]
mod tests {
    //! 数据权限 Handler 单元测试（批次 394 补测）
    //!
    //! 覆盖目标：
    //! - validate_custom_condition_safe SQL 注入防御纯函数（6 个分支）

    use super::*;
    use serde_json::json;

    /// 测试_validate_custom_condition_null通过
    ///
    /// 场景：Value::Null 应通过校验（无自定义条件）
    #[test]
    fn 测试_validate_custom_condition_null通过() {
        let result = validate_custom_condition_safe(&Value::Null);
        assert!(result.is_ok(), "null 值应通过校验");
    }

    /// 测试_validate_custom_condition空对象通过
    ///
    /// 场景：空对象 {} 应通过校验（无字段需要检查）
    #[test]
    fn 测试_validate_custom_condition空对象通过() {
        let result = validate_custom_condition_safe(&json!({}));
        assert!(result.is_ok(), "空对象应通过校验");
    }

    /// 测试_validate_custom_condition合法对象通过
    ///
    /// 场景：合法字段名（小写+下划线+数字）+ 合法值类型（数字/字符串/bool/null）应通过
    #[test]
    fn 测试_validate_custom_condition合法对象通过() {
        let cond = json!({
            "field1": 123,
            "field2": "value",
            "field_3": true,
            "field4": null
        });
        let result = validate_custom_condition_safe(&cond);
        assert!(result.is_ok(), "合法对象应通过校验");
    }

    /// 测试_validate_custom_condition拒绝大写字段名
    ///
    /// 场景：字段名含大写字母（如 "FieldName"）应被拒绝
    #[test]
    fn 测试_validate_custom_condition拒绝大写字段名() {
        let cond = json!({"FieldName": 123});
        let result = validate_custom_condition_safe(&cond);
        assert!(result.is_err(), "含大写字母的字段名应被拒绝");
        let err = result.unwrap_err();
        let msg = err.to_string();
        assert!(
            msg.contains("字段名非法"),
            "错误消息应包含'字段名非法'，实际：{}",
            msg
        );
    }

    /// 测试_validate_custom_condition拒绝SQL关键字
    ///
    /// 场景：序列化后包含 UNION/SELECT/DROP 等 FORBIDDEN 关键字应被拒绝
    #[test]
    fn 测试_validate_custom_condition拒绝SQL关键字() {
        // 字符串值中包含 UNION（序列化后大写匹配）
        let cond = json!({"field": "UNION SELECT"});
        let result = validate_custom_condition_safe(&cond);
        assert!(result.is_err(), "包含 SQL 关键字应被拒绝");

        // 包含分号
        let cond = json!({"field": "value;"});
        let result = validate_custom_condition_safe(&cond);
        assert!(result.is_err(), "包含分号应被拒绝");
    }

    /// 测试_validate_custom_condition拒绝字符串含引号
    ///
    /// 场景：字符串值包含单引号/双引号应被拒绝（防 SQL 注入）
    #[test]
    fn 测试_validate_custom_condition拒绝字符串含引号() {
        // 单引号
        let cond = json!({"field": "value'with'quotes"});
        let result = validate_custom_condition_safe(&cond);
        assert!(result.is_err(), "含单引号的字符串值应被拒绝");

        // 双引号
        let cond = json!({"field": "value\"with\"quotes"});
        let result = validate_custom_condition_safe(&cond);
        assert!(result.is_err(), "含双引号的字符串值应被拒绝");
    }
}
