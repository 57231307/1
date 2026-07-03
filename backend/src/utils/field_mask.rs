use crate::middleware::auth_context::AuthContext;
use serde_json::Value;

/// 脱敏敏感字段（如成本价、敏感金额）
pub fn mask_sensitive_fields(mut value: Value, auth: &AuthContext) -> Value {
    // 假设 role_id = 1 是超级管理员，其他角色脱敏
    // 实际项目中可以根据权限表动态判断 `has_permission(user_id, "view_cost_price")`
    if auth.role_id != Some(1) {
        // P3 维度 3 修复（批次 87）：消除 unwrap，改用 if let 显式模式匹配
        if let Some(obj) = value.as_object_mut() {
            // 移除或掩码成本价
            if obj.contains_key("cost_price") {
                obj.insert("cost_price".to_string(), Value::Null);
            }

            // 可以递归脱敏
            for (_, v) in obj.iter_mut() {
                *v = mask_sensitive_fields(v.clone(), auth);
            }
        } else if let Some(arr) = value.as_array_mut() {
            for item in arr.iter_mut() {
                *item = mask_sensitive_fields(item.clone(), auth);
            }
        }
    }
    value
}
