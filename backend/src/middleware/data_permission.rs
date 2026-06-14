//! 数据权限中间件
//!
//! 提供统一的数据权限过滤功能，避免在各 handler 中重复实现
//! 主要功能：
//! - 从认证上下文和应用状态创建数据权限上下文
//! - 对 JSON 数据应用字段过滤（允许字段 / 隐藏字段）
//! - 支持管理员跳过过滤

use crate::middleware::auth_context::AuthContext;
use crate::services::data_permission_service::DataPermissionService;
use crate::utils::app_state::AppState;
use crate::utils::error::AppError;
use serde_json::Value;
use std::sync::Arc;

/// 数据权限上下文
///
/// 封装了角色数据权限信息，提供统一的字段过滤方法
pub struct DataPermissionContext {
    /// 角色 ID
    pub role_id: Option<i32>,
    /// 允许的字段列表（如果为 None，则不过滤）
    pub allowed_fields: Option<Vec<String>>,
    /// 隐藏的字段列表
    pub hidden_fields: Option<Vec<String>>,
    /// 是否为管理员角色
    pub is_admin: bool,
    /// 数据权限服务引用
    permission_service: Arc<DataPermissionService>,
}

impl DataPermissionContext {
    /// 从认证上下文创建数据权限上下文
    ///
    /// # 参数
    /// - `auth`: 认证上下文
    /// - `state`: 应用状态
    /// - `resource_type`: 资源类型（如 "customer", "sales_order" 等）
    ///
    /// # 返回
    /// 返回数据权限上下文实例
    pub async fn from_auth(
        auth: &AuthContext,
        state: &AppState,
        resource_type: &str,
    ) -> Result<Self, AppError> {
        let role_id = auth.role_id;
        let is_admin = role_id == Some(1);

        let (allowed_fields, hidden_fields) = if let Some(rid) = role_id {
            if let Some(permission) = state
                .data_permission_service
                .get_role_data_permission(rid, resource_type)
                .await?
            {
                (permission.allowed_fields, permission.hidden_fields)
            } else {
                (None, None)
            }
        } else {
            (None, None)
        };

        Ok(Self {
            role_id,
            allowed_fields,
            hidden_fields,
            is_admin,
            permission_service: state.data_permission_service.clone(),
        })
    }

    /// 对单个 JSON 值应用字段过滤
    ///
    /// # 参数
    /// - `data`: 需要过滤的 JSON 数据
    ///
    /// # 说明
    /// - 管理员角色不会应用任何过滤
    /// - 如果配置了允许字段，则只保留允许的字段
    /// - 如果配置了隐藏字段，则移除隐藏的字段
    pub fn filter_single(&self, data: &mut Value) {
        // 管理员不进行过滤
        if self.is_admin {
            return;
        }

        self.permission_service
            .filter_fields(data, &self.allowed_fields, &self.hidden_fields);
    }

    /// 对 JSON 数组应用字段过滤
    ///
    /// # 参数
    /// - `items`: 需要过滤的 JSON 数组
    ///
    /// # 说明
    /// - 管理员不进行过滤
    /// - 对数组中的每个元素调用 `filter_single` 方法
    pub fn filter_batch(&self, items: &mut [Value]) {
        // 管理员不进行过滤
        if self.is_admin {
            return;
        }

        self.permission_service.filter_fields_batch(
            items,
            &self.allowed_fields,
            &self.hidden_fields,
        );
    }

    /// 对嵌套结构中的列表应用字段过滤
    ///
    /// # 参数
    /// - `data`: 包含列表的 JSON 对象
    /// - `list_keys`: 可能的列表键名（按优先级尝试，如 ["list", "data"]）
    ///
    /// # 返回
    /// 返回是否成功找到并过滤了列表
    pub fn filter_nested_list(&self, data: &mut Value, list_keys: &[&str]) -> bool {
        // 管理员不进行过滤
        if self.is_admin {
            return false;
        }

        // 尝试从可能的键名中获取列表
        // 在循环内完成借用到 filter_batch 调用的全生命周期，避免外层 NLL 误判
        for key in list_keys {
            if let Some(arr) = data.get_mut(*key).and_then(|v| v.as_array_mut()) {
                self.filter_batch(arr);
                return true;
            }
        }

        false
    }

    /// 检查是否应该应用默认字段隐藏
    ///
    /// # 返回
    /// 如果没有配置数据权限且不是管理员，返回 true
    pub fn should_apply_default_hidden(&self) -> bool {
        !self.is_admin && self.allowed_fields.is_none() && self.hidden_fields.is_none()
    }

    /// 应用默认字段隐藏（当没有配置数据权限时使用）
    ///
    /// # 参数
    /// - `data`: 需要过滤的 JSON 数据
    /// - `default_hidden_fields`: 默认需要隐藏的字段列表
    pub fn apply_default_hidden(&self, data: &mut Value, default_hidden_fields: &[&str]) {
        if self.should_apply_default_hidden() {
            if let Some(obj) = data.as_object_mut() {
                for field in default_hidden_fields {
                    obj.remove(*field);
                }
            }
        }
    }

    /// 批量应用默认字段隐藏
    ///
    /// # 参数
    /// - `items`: 需要过滤的 JSON 数组
    /// - `default_hidden_fields`: 默认需要隐藏的字段列表
    pub fn apply_default_hidden_batch(&self, items: &mut [Value], default_hidden_fields: &[&str]) {
        if self.should_apply_default_hidden() {
            for item in items {
                self.apply_default_hidden(item, default_hidden_fields);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_admin_should_not_filter() {
        let ctx = DataPermissionContext {
            role_id: Some(1),
            allowed_fields: None,
            hidden_fields: None,
            is_admin: true,
            // 测试中不会实际调用服务
            permission_service: Arc::new(DataPermissionService::new(Arc::new(
                sea_orm::DatabaseConnection::default(),
            ))),
        };

        let mut data = serde_json::json!({
            "id": 1,
            "secret_field": "should_not_be_removed"
        });

        ctx.filter_single(&mut data);

        assert!(data.get("secret_field").is_some());
    }

    #[test]
    fn test_should_apply_default_hidden() {
        let ctx = DataPermissionContext {
            role_id: Some(2),
            allowed_fields: None,
            hidden_fields: None,
            is_admin: false,
            permission_service: Arc::new(DataPermissionService::new(Arc::new(
                sea_orm::DatabaseConnection::default(),
            ))),
        };

        assert!(ctx.should_apply_default_hidden());
    }

    #[test]
    fn test_should_not_apply_default_hidden_for_admin() {
        let ctx = DataPermissionContext {
            role_id: Some(1),
            allowed_fields: None,
            hidden_fields: None,
            is_admin: true,
            permission_service: Arc::new(DataPermissionService::new(Arc::new(
                sea_orm::DatabaseConnection::default(),
            ))),
        };

        assert!(!ctx.should_apply_default_hidden());
    }
}
