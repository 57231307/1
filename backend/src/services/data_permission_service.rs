//! 数据权限服务
//!
//! 提供数据范围控制和字段级权限管理功能

use crate::models::data_permission::{self, Entity as DataPermissionEntity};
use crate::utils::admin_checker;
use crate::utils::error::AppError;
use chrono::Utc;
use sea_orm::{
    ActiveModelTrait, ColumnTrait, DatabaseConnection, EntityTrait, PaginatorTrait, QueryFilter,
    QuerySelect, Set, TransactionTrait,
};
use serde_json::Value;
use std::sync::Arc;

/// 数据范围类型常量
pub mod data_scope {
    pub const ALL: &str = "ALL";
    #[allow(dead_code)] // TODO(tech-debt): 数据权限范围常量接入 handler/校验后移除
    pub const DEPT: &str = "DEPT";
    #[allow(dead_code)] // TODO(tech-debt): 数据权限范围常量接入 handler/校验后移除
    pub const DEPT_AND_BELOW: &str = "DEPT_AND_BELOW";
    #[allow(dead_code)] // TODO(tech-debt): 数据权限范围常量接入 handler/校验后移除
    pub const SELF: &str = "SELF";
    #[allow(dead_code)] // TODO(tech-debt): 数据权限范围常量接入 handler/校验后移除
    pub const CUSTOM: &str = "CUSTOM";
}

/// 数据权限查询结果
#[derive(Debug, Clone)]
pub struct DataPermissionResult {
    /// 数据范围类型
    pub scope_type: String,
    /// 自定义条件
    pub custom_condition: Option<Value>,
    /// 允许的字段
    pub allowed_fields: Option<Vec<String>>,
    /// 隐藏的字段
    pub hidden_fields: Option<Vec<String>>,
}

/// 数据权限服务
pub struct DataPermissionService {
    db: Arc<DatabaseConnection>,
}

impl DataPermissionService {
    /// 创建服务实例
    pub fn new(db: Arc<DatabaseConnection>) -> Self {
        Self { db }
    }

    /// 获取角色的数据权限
    pub async fn get_role_data_permission(
        &self,
        role_id: i32,
        resource_type: &str,
    ) -> Result<Option<DataPermissionResult>, AppError> {
        // Admin 角色拥有全部权限（从数据库查询角色编码）
        if self.is_admin_role(role_id).await? {
            return Ok(Some(DataPermissionResult {
                scope_type: data_scope::ALL.to_string(),
                custom_condition: None,
                allowed_fields: None,
                hidden_fields: None,
            }));
        }

        let permission = DataPermissionEntity::find()
            .filter(data_permission::Column::RoleId.eq(role_id))
            .filter(data_permission::Column::ResourceType.eq(resource_type))
            .filter(data_permission::Column::IsEnabled.eq(true))
            .one(&*self.db)
            .await?;

        Ok(permission.map(|p| DataPermissionResult {
            scope_type: p.scope_type,
            custom_condition: p.custom_condition,
            allowed_fields: p.allowed_fields.and_then(|f| {
                f.as_array().map(|arr| {
                    arr.iter()
                        .filter_map(|v| v.as_str().map(|s| s.to_string()))
                        .collect()
                })
            }),
            hidden_fields: p.hidden_fields.and_then(|f| {
                f.as_array().map(|arr| {
                    arr.iter()
                        .filter_map(|v| v.as_str().map(|s| s.to_string()))
                        .collect()
                })
            }),
        }))
    }

    /// 检查用户是否有数据权限
    #[allow(dead_code)] // TODO(tech-debt): 数据权限检查接口接入业务后移除
    pub async fn check_data_permission(
        &self,
        role_id: i32,
        resource_type: &str,
    ) -> Result<bool, AppError> {
        // Admin 角色拥有全部权限（从数据库查询角色编码）
        if self.is_admin_role(role_id).await? {
            return Ok(true);
        }

        let count = DataPermissionEntity::find()
            .filter(data_permission::Column::RoleId.eq(role_id))
            .filter(data_permission::Column::ResourceType.eq(resource_type))
            .filter(data_permission::Column::IsEnabled.eq(true))
            .count(&*self.db)
            .await?;

        Ok(count > 0)
    }

    /// 检查角色是否为管理员角色（带缓存）
    async fn is_admin_role(&self, role_id: i32) -> Result<bool, AppError> {
        Ok(admin_checker::is_admin_role(&self.db, role_id).await)
    }

    /// 设置数据权限
    ///
    /// 批次 85 v2 复审 P1-8 修复：find + update/insert 移入单一事务 + lock_exclusive 串行化
    /// 原实现 find + update/insert 在 self.db 上分别执行，无 txn 无 lock，并发设置相同权限会基于过期状态 upsert
    pub async fn set_data_permission(
        &self,
        role_id: i32,
        resource_type: String,
        scope_type: String,
        custom_condition: Option<Value>,
        allowed_fields: Option<Value>,
        hidden_fields: Option<Value>,
    ) -> Result<data_permission::Model, AppError> {
        let txn = (*self.db).begin().await?;

        // 加 lock_exclusive 串行化并发 upsert
        let existing = DataPermissionEntity::find()
            .filter(data_permission::Column::RoleId.eq(role_id))
            .filter(data_permission::Column::ResourceType.eq(&resource_type))
            .lock_exclusive()
            .one(&txn)
            .await?;

        let permission = if let Some(existing) = existing {
            let mut active_model: data_permission::ActiveModel = existing.into();
            active_model.scope_type = Set(scope_type);
            active_model.custom_condition = Set(custom_condition);
            active_model.allowed_fields = Set(allowed_fields);
            active_model.hidden_fields = Set(hidden_fields);
            active_model.is_enabled = Set(true);
            active_model.updated_at = Set(Utc::now());
            active_model.update(&txn).await?
        } else {
            let active_model = data_permission::ActiveModel {
                id: Default::default(),
                role_id: Set(role_id),
                resource_type: Set(resource_type),
                scope_type: Set(scope_type),
                custom_condition: Set(custom_condition),
                allowed_fields: Set(allowed_fields),
                hidden_fields: Set(hidden_fields),
                is_enabled: Set(true),
                created_at: Set(Utc::now()),
                updated_at: Set(Utc::now()),
            };
            active_model.insert(&txn).await?
        };

        txn.commit().await?;
        Ok(permission)
    }

    /// 删除数据权限
    pub async fn delete_data_permission(
        &self,
        role_id: i32,
        resource_type: &str,
    ) -> Result<(), AppError> {
        let existing = DataPermissionEntity::find()
            .filter(data_permission::Column::RoleId.eq(role_id))
            .filter(data_permission::Column::ResourceType.eq(resource_type))
            .one(&*self.db)
            .await?;

        if let Some(existing) = existing {
            let mut active_model: data_permission::ActiveModel = existing.into();
            active_model.is_enabled = Set(false);
            active_model.updated_at = Set(Utc::now());
            active_model.update(&*self.db).await?;
        }

        Ok(())
    }

    /// 获取角色的所有数据权限
    pub async fn list_role_data_permissions(
        &self,
        role_id: i32,
    ) -> Result<Vec<data_permission::Model>, AppError> {
        let permissions = DataPermissionEntity::find()
            .filter(data_permission::Column::RoleId.eq(role_id))
            .filter(data_permission::Column::IsEnabled.eq(true))
            .all(&*self.db)
            .await?;

        Ok(permissions)
    }

    /// 获取所有数据权限列表
    pub async fn list_all_data_permissions(&self) -> Result<Vec<data_permission::Model>, AppError> {
        let permissions = DataPermissionEntity::find()
            .filter(data_permission::Column::IsEnabled.eq(true))
            .all(&*self.db)
            .await?;

        Ok(permissions)
    }

    /// 过滤字段（根据字段权限）
    pub fn filter_fields(
        &self,
        data: &mut serde_json::Value,
        allowed_fields: &Option<Vec<String>>,
        hidden_fields: &Option<Vec<String>>,
    ) {
        if let Some(obj) = data.as_object_mut() {
            // 如果有允许的字段列表，只保留允许的字段
            if let Some(allowed) = allowed_fields {
                let allowed_set: std::collections::HashSet<_> = allowed.iter().cloned().collect();
                obj.retain(|key, _| allowed_set.contains(key));
            }

            // 移除隐藏的字段
            if let Some(hidden) = hidden_fields {
                for field in hidden {
                    obj.remove(field);
                }
            }
        }
    }

    /// 批量过滤字段
    pub fn filter_fields_batch(
        &self,
        data_list: &mut [serde_json::Value],
        allowed_fields: &Option<Vec<String>>,
        hidden_fields: &Option<Vec<String>>,
    ) {
        for data in data_list {
            self.filter_fields(data, allowed_fields, hidden_fields);
        }
    }
}
