use sea_orm::{
    ActiveModelTrait, ColumnTrait, DatabaseConnection, EntityTrait, Order, QueryFilter, QueryOrder,
    Set,
};
use serde::{Deserialize, Serialize};
use std::sync::Arc;

use crate::utils::error::AppError;
use crate::models::field_permission::{self, Entity as FieldPermissionEntity};
use crate::models::role;

/// 字段权限详情
#[derive(Debug, Serialize, Deserialize)]
pub struct FieldPermissionDetail {
    pub id: i32,
    pub role_id: i32,
    pub resource_type: String,
    pub field_name: String,
    pub can_read: bool,
    pub can_write: bool,
    pub mask_strategy: String,
    pub is_enabled: bool,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub updated_at: chrono::DateTime<chrono::Utc>,
}

/// 创建字段权限请求
#[derive(Debug, Deserialize)]
pub struct CreateFieldPermissionRequest {
    pub role_id: i32,
    pub resource_type: String,
    pub field_name: String,
    pub can_read: bool,
    pub can_write: bool,
    pub mask_strategy: Option<String>,
}

/// 更新字段权限请求
#[derive(Debug, Deserialize)]
pub struct UpdateFieldPermissionRequest {
    pub can_read: Option<bool>,
    pub can_write: Option<bool>,
    pub mask_strategy: Option<String>,
    pub is_enabled: Option<bool>,
}

/// 字段权限服务
pub struct FieldPermissionService {
    db: Arc<DatabaseConnection>,
}

#[allow(dead_code)]
impl FieldPermissionService {
    pub fn new(db: Arc<DatabaseConnection>) -> Self {
        Self { db }
    }

    /// 获取所有字段权限列表
    pub async fn list_field_permissions(
        &self,
        resource_type: Option<&str>,
        role_id: Option<i32>,
    ) -> Result<Vec<FieldPermissionDetail>, AppError> {
        let mut query = FieldPermissionEntity::find();

        if let Some(rt) = resource_type {
            query = query.filter(field_permission::Column::ResourceType.eq(rt));
        }

        if let Some(rid) = role_id {
            query = query.filter(field_permission::Column::RoleId.eq(rid));
        }

        let permissions = query
            .order_by(field_permission::Column::ResourceType, Order::Asc)
            .order_by(field_permission::Column::FieldName, Order::Asc)
            .all(&*self.db)
            .await?;

        Ok(permissions
            .into_iter()
            .map(|p| FieldPermissionDetail {
                id: p.id,
                role_id: p.role_id,
                resource_type: p.resource_type,
                field_name: p.field_name,
                can_read: p.can_read,
                can_write: p.can_write,
                mask_strategy: p.mask_strategy,
                is_enabled: p.is_enabled,
                created_at: p.created_at,
                updated_at: p.updated_at,
            })
            .collect())
    }

    /// 获取单个字段权限详情
    pub async fn get_field_permission(
        &self,
        id: i32,
    ) -> Result<FieldPermissionDetail, AppError> {
        let perm = FieldPermissionEntity::find_by_id(id)
            .one(&*self.db)
            .await?
            .ok_or_else(|| {
                AppError::ResourceNotFound(format!("字段权限 {} 未找到", id))
            })?;

        Ok(FieldPermissionDetail {
            id: perm.id,
            role_id: perm.role_id,
            resource_type: perm.resource_type,
            field_name: perm.field_name,
            can_read: perm.can_read,
            can_write: perm.can_write,
            mask_strategy: perm.mask_strategy,
            is_enabled: perm.is_enabled,
            created_at: perm.created_at,
            updated_at: perm.updated_at,
        })
    }

    /// 创建字段权限
    pub async fn create_field_permission(
        &self,
        request: CreateFieldPermissionRequest,
    ) -> Result<FieldPermissionDetail, AppError> {
        // 检查是否已存在相同的权限规则
        let existing = FieldPermissionEntity::find()
            .filter(field_permission::Column::RoleId.eq(request.role_id))
            .filter(field_permission::Column::ResourceType.eq(&request.resource_type))
            .filter(field_permission::Column::FieldName.eq(&request.field_name))
            .one(&*self.db)
            .await?;

        if existing.is_some() {
            return Err(AppError::BusinessError(
                "该角色对该资源类型的字段权限已存在".to_string(),
            ));
        }

        let mask_strategy = request.mask_strategy.unwrap_or_else(|| "NONE".to_string());

        let permission = field_permission::ActiveModel {
            id: Default::default(),
            role_id: Set(request.role_id),
            resource_type: Set(request.resource_type),
            field_name: Set(request.field_name),
            can_read: Set(request.can_read),
            can_write: Set(request.can_write),
            mask_strategy: Set(mask_strategy),
            is_enabled: Set(true),
            created_at: Set(chrono::Utc::now()),
            updated_at: Set(chrono::Utc::now()),
        };

        let entity = permission.insert(&*self.db).await?;

        Ok(FieldPermissionDetail {
            id: entity.id,
            role_id: entity.role_id,
            resource_type: entity.resource_type,
            field_name: entity.field_name,
            can_read: entity.can_read,
            can_write: entity.can_write,
            mask_strategy: entity.mask_strategy,
            is_enabled: entity.is_enabled,
            created_at: entity.created_at,
            updated_at: entity.updated_at,
        })
    }

    /// 更新字段权限
    pub async fn update_field_permission(
        &self,
        id: i32,
        request: UpdateFieldPermissionRequest,
    ) -> Result<FieldPermissionDetail, AppError> {
        let perm = FieldPermissionEntity::find_by_id(id)
            .one(&*self.db)
            .await?
            .ok_or_else(|| {
                AppError::ResourceNotFound(format!("字段权限 {} 未找到", id))
            })?;

        let mut active: field_permission::ActiveModel = perm.into();

        if let Some(can_read) = request.can_read {
            active.can_read = Set(can_read);
        }
        if let Some(can_write) = request.can_write {
            active.can_write = Set(can_write);
        }
        if let Some(mask_strategy) = request.mask_strategy {
            active.mask_strategy = Set(mask_strategy);
        }
        if let Some(is_enabled) = request.is_enabled {
            active.is_enabled = Set(is_enabled);
        }
        active.updated_at = Set(chrono::Utc::now());

        let entity = active.update(&*self.db).await?;

        Ok(FieldPermissionDetail {
            id: entity.id,
            role_id: entity.role_id,
            resource_type: entity.resource_type,
            field_name: entity.field_name,
            can_read: entity.can_read,
            can_write: entity.can_write,
            mask_strategy: entity.mask_strategy,
            is_enabled: entity.is_enabled,
            created_at: entity.created_at,
            updated_at: entity.updated_at,
        })
    }

    /// 删除字段权限
    pub async fn delete_field_permission(&self, id: i32) -> Result<(), AppError> {
        let perm = FieldPermissionEntity::find_by_id(id)
            .one(&*self.db)
            .await?
            .ok_or_else(|| {
                AppError::ResourceNotFound(format!("字段权限 {} 未找到", id))
            })?;

        // 软删除：禁用而不是真正删除
        let mut active: field_permission::ActiveModel = perm.into();
        active.is_enabled = Set(false);
        active.updated_at = Set(chrono::Utc::now());
        active.update(&*self.db).await?;

        Ok(())
    }

    /// 检查角色是否为管理员角色（从数据库查询角色编码）
    async fn is_admin_role(&self, role_id: i32) -> Result<bool, AppError> {
        use sea_orm::EntityTrait;
        match role::Entity::find_by_id(role_id)
            .one(&*self.db)
            .await?
        {
            Some(r) => Ok(r.code == "admin"),
            None => Ok(false),
        }
    }

    /// 检查角色对某资源的字段读权限
    pub async fn check_read_permission(
        &self,
        role_id: i32,
        resource_type: &str,
        field_name: &str,
    ) -> Result<bool, AppError> {
        // Admin 角色默认拥有全部权限（从数据库查询角色编码）
        if self.is_admin_role(role_id).await? {
            return Ok(true);
        }

        let perm = FieldPermissionEntity::find()
            .filter(field_permission::Column::RoleId.eq(role_id))
            .filter(field_permission::Column::ResourceType.eq(resource_type))
            .filter(field_permission::Column::FieldName.eq(field_name))
            .filter(field_permission::Column::IsEnabled.eq(true))
            .one(&*self.db)
            .await?;

        match perm {
            Some(p) => Ok(p.can_read),
            None => Ok(false), // 默认拒绝读取（未配置权限规则时）
        }
    }

    /// 检查角色对某资源的字段写权限
    pub async fn check_write_permission(
        &self,
        role_id: i32,
        resource_type: &str,
        field_name: &str,
    ) -> Result<bool, AppError> {
        // Admin 角色默认拥有全部权限（从数据库查询角色编码）
        if self.is_admin_role(role_id).await? {
            return Ok(true);
        }

        let perm = FieldPermissionEntity::find()
            .filter(field_permission::Column::RoleId.eq(role_id))
            .filter(field_permission::Column::ResourceType.eq(resource_type))
            .filter(field_permission::Column::FieldName.eq(field_name))
            .filter(field_permission::Column::IsEnabled.eq(true))
            .one(&*self.db)
            .await?;

        match perm {
            Some(p) => Ok(p.can_write),
            None => Ok(false), // 默认拒绝写入（未配置权限规则时）
        }
    }

    /// 获取角色对某资源的所有字段权限
    pub async fn get_role_field_permissions(
        &self,
        role_id: i32,
        resource_type: &str,
    ) -> Result<Vec<FieldPermissionDetail>, AppError> {
        let permissions = FieldPermissionEntity::find()
            .filter(field_permission::Column::RoleId.eq(role_id))
            .filter(field_permission::Column::ResourceType.eq(resource_type))
            .filter(field_permission::Column::IsEnabled.eq(true))
            .order_by(field_permission::Column::FieldName, Order::Asc)
            .all(&*self.db)
            .await?;

        Ok(permissions
            .into_iter()
            .map(|p| FieldPermissionDetail {
                id: p.id,
                role_id: p.role_id,
                resource_type: p.resource_type,
                field_name: p.field_name,
                can_read: p.can_read,
                can_write: p.can_write,
                mask_strategy: p.mask_strategy,
                is_enabled: p.is_enabled,
                created_at: p.created_at,
                updated_at: p.updated_at,
            })
            .collect())
    }

    /// 过滤 JSON 数据中的字段（根据读权限）
    pub fn filter_fields_by_read_permission(
        &self,
        data: &mut serde_json::Value,
        permissions: &[FieldPermissionDetail],
    ) {
        if let Some(obj) = data.as_object_mut() {
            let perm_map: std::collections::HashMap<&str, &FieldPermissionDetail> = permissions
                .iter()
                .map(|p| (p.field_name.as_str(), p))
                .collect();

            let keys_to_remove: Vec<String> = obj
                .keys()
                .filter_map(|key| {
                    if let Some(perm) = perm_map.get(key.as_str()) {
                        if !perm.can_read {
                            return Some(key.clone());
                        }
                    }
                    None
                })
                .collect();

            for key in keys_to_remove {
                obj.remove(&key);
            }
        }
    }

    /// 掩码处理 JSON 数据中的字段（无读权限时显示为 "***"）
    pub fn mask_fields(
        &self,
        data: &mut serde_json::Value,
        permissions: &[FieldPermissionDetail],
    ) {
        if let Some(obj) = data.as_object_mut() {
            for perm in permissions {
                if !perm.can_read && perm.mask_strategy == "MASK" {
                    if let Some(val) = obj.get_mut(&perm.field_name) {
                        *val = serde_json::Value::String("***".to_string());
                    }
                }
            }
        }
    }

    /// 批量处理 JSON 数组
    pub fn process_json_array(
        &self,
        data_list: &mut [serde_json::Value],
        permissions: &[FieldPermissionDetail],
    ) {
        for data in data_list {
            self.filter_fields_by_read_permission(data, permissions);
            self.mask_fields(data, permissions);
        }
    }
}
