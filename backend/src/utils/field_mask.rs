use serde_json::Value;
use crate::middleware::auth_context::AuthContext;

/// 脱敏敏感字段（如成本价、敏感金额）
pub fn mask_sensitive_fields(mut value: Value, auth: &AuthContext) -> Value {
    // 假设 role_id = 1 是超级管理员，其他角色脱敏
    // 实际项目中可以根据权限表动态判断 `has_permission(user_id, "view_cost_price")`
    if auth.role_id != Some(1) {
        if value.is_object() {
            let obj = value.as_object_mut().unwrap();
            
            // 移除或掩码成本价
            if obj.contains_key("cost_price") {
                obj.insert("cost_price".to_string(), Value::Null);
            }
            
            // 可以递归脱敏
            for (_, v) in obj.iter_mut() {
                *v = mask_sensitive_fields(v.clone(), auth);
            }
        } else if value.is_array() {
            let arr = value.as_array_mut().unwrap();
            for item in arr.iter_mut() {
                *item = mask_sensitive_fields(item.clone(), auth);
            }
        }
    }
    value
}
