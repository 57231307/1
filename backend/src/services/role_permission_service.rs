use sea_orm::{
    ActiveModelTrait, ColumnTrait, DatabaseConnection, EntityTrait, Order,
    PaginatorTrait, QueryFilter, QueryOrder, TransactionTrait,
};
use std::sync::Arc;

use crate::models::role::{self, Entity as RoleEntity};
use crate::models::role_permission::{self, Entity as RolePermissionEntity};
// V15 P0-S06：权限变更审计
use crate::models::permission_change_audit;
use crate::utils::admin_checker;
use crate::utils::error::AppError;
// V15 P0-S07：权限变更时失效缓存
use crate::middleware::permission::invalidate_permission_cache;
use serde::{Deserialize, Serialize};

/// 角色权限详情
#[derive(Debug, Serialize, Deserialize)]
pub struct RolePermissionDetail {
    pub id: i32,
    pub role_id: i32,
    pub resource_type: String,
    pub resource_id: Option<i32>,
    pub action: String,
    pub allowed: bool,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub updated_at: chrono::DateTime<chrono::Utc>,
}

/// 角色详情
#[derive(Debug, Serialize, Deserialize)]
pub struct RoleDetail {
    pub id: i32,
    pub name: String,
    pub code: String,
    pub description: Option<String>,
    pub permissions: Option<String>,
    pub is_system: bool,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub updated_at: chrono::DateTime<chrono::Utc>,
    pub permission_list: Option<Vec<RolePermissionDetail>>,
}

/// 创建角色请求
#[derive(Debug, Deserialize)]
pub struct CreateRoleRequest {
    pub name: String,
    pub code: String,
    pub description: Option<String>,
    pub is_system: Option<bool>,
}

/// 更新角色请求
#[derive(Debug, Deserialize)]
pub struct UpdateRoleRequest {
    pub name: Option<String>,
    pub code: Option<String>,
    pub description: Option<String>,
    pub is_system: Option<bool>,
}

/// 分配权限请求
#[derive(Debug, Deserialize)]
pub struct AssignPermissionRequest {
    pub role_id: i32,
    pub resource_type: String,
    pub resource_id: Option<i32>,
    pub action: String,
    pub allowed: bool,
}

/// 角色权限服务
pub struct RolePermissionService {
    db: Arc<DatabaseConnection>,
}

impl RolePermissionService {
    pub fn new(db: Arc<DatabaseConnection>) -> Self {
        Self { db }
    }

    /// 获取角色列表
    pub async fn list_roles(&self) -> Result<Vec<RoleDetail>, AppError> {
        let roles = RoleEntity::find()
            .order_by(role::Column::Code, Order::Asc)
            .all(&*self.db)
            .await?;

        let role_details: Vec<RoleDetail> = roles
            .into_iter()
            .map(|role| RoleDetail {
                id: role.id,
                name: role.name,
                code: role.code,
                description: role.description,
                permissions: role.permissions,
                is_system: role.is_system,
                created_at: role.created_at,
                updated_at: role.updated_at,
                permission_list: None,
            })
            .collect();

        Ok(role_details)
    }

    /// 获取角色详情（包含权限列表）
    pub async fn get_role_detail(&self, role_id: i32) -> Result<RoleDetail, AppError> {
        let role = RoleEntity::find_by_id(role_id)
            .one(&*self.db)
            .await?
            .ok_or_else(|| AppError::not_found(format!("角色 {} 未找到", role_id)))?;

        // 获取角色权限列表
        let permissions = RolePermissionEntity::find()
            .filter(role_permission::Column::RoleId.eq(role_id))
            .order_by(role_permission::Column::ResourceType, Order::Asc)
            .all(&*self.db)
            .await?;

        let permission_list: Vec<RolePermissionDetail> = permissions
            .into_iter()
            .map(|perm| RolePermissionDetail {
                id: perm.id,
                role_id: perm.role_id,
                resource_type: perm.resource_type,
                resource_id: perm.resource_id,
                action: perm.action,
                allowed: perm.allowed,
                created_at: perm.created_at,
                updated_at: perm.updated_at,
            })
            .collect();

        Ok(RoleDetail {
            id: role.id,
            name: role.name,
            code: role.code,
            description: role.description,
            permissions: role.permissions,
            is_system: role.is_system,
            created_at: role.created_at,
            updated_at: role.updated_at,
            permission_list: Some(permission_list),
        })
    }

    /// 创建角色
    pub async fn create_role(&self, request: CreateRoleRequest) -> Result<RoleDetail, AppError> {
        // 检查角色编码是否已存在
        let existing = RoleEntity::find()
            .filter(role::Column::Code.eq(&request.code))
            .one(&*self.db)
            .await?;

        if existing.is_some() {
            return Err(AppError::business("角色编码已存在"));
        }

        let role = role::ActiveModel {
            id: Default::default(),
            name: sea_orm::ActiveValue::Set(request.name),
            code: sea_orm::ActiveValue::Set(request.code),
            description: sea_orm::ActiveValue::Set(request.description),
            permissions: sea_orm::ActiveValue::NotSet,
            is_system: sea_orm::ActiveValue::Set(request.is_system.unwrap_or(false)),
            // V15 P0-S01：新角色默认 data_scope=self（最小权限原则）
            data_scope: sea_orm::ActiveValue::Set("self".to_string()),
            created_at: sea_orm::ActiveValue::Set(chrono::Utc::now()),
            updated_at: sea_orm::ActiveValue::Set(chrono::Utc::now()),
        };

        let role_entity = role.insert(&*self.db).await?;

        self.get_role_detail(role_entity.id).await
    }

    /// 更新角色
    pub async fn update_role(
        &self,
        role_id: i32,
        request: UpdateRoleRequest,
        user_id: i32,
    ) -> Result<RoleDetail, AppError> {
        let role = RoleEntity::find_by_id(role_id)
            .one(&*self.db)
            .await?
            .ok_or_else(|| AppError::not_found(format!("角色 {} 未找到", role_id)))?;

        // 系统角色不允许修改
        if role.is_system {
            return Err(AppError::business("系统角色不允许修改"));
        }

        // 如果修改了编码，检查新编码是否已存在
        if let Some(ref new_code) = request.code {
            if new_code != &role.code {
                let existing = RoleEntity::find()
                    .filter(role::Column::Code.eq(new_code))
                    .one(&*self.db)
                    .await?;

                if existing.is_some() {
                    return Err(AppError::business("角色编码已存在"));
                }
            }
        }

        let mut role_update: role::ActiveModel = role.into();
        if let Some(name) = request.name {
            role_update.name = sea_orm::ActiveValue::Set(name);
        }
        if let Some(code) = request.code {
            role_update.code = sea_orm::ActiveValue::Set(code);
        }
        if let Some(description) = request.description {
            role_update.description = sea_orm::ActiveValue::Set(Some(description));
        }
        if let Some(is_system) = request.is_system {
            role_update.is_system = sea_orm::ActiveValue::Set(is_system);
        }
        role_update.updated_at = sea_orm::ActiveValue::Set(chrono::Utc::now());

        let role_entity = crate::services::audit_log_service::AuditLogService::update_with_audit(
            &*self.db,
            "auto_audit",
            role_update,
            // 批次 94 P2-10：原 Some(0) 占位改为真实操作人 user_id，便于审计追踪
            Some(user_id),
        )
        .await?;

        self.get_role_detail(role_entity.id).await
    }

    /// 删除角色
    pub async fn delete_role(&self, role_id: i32, user_id: i32) -> Result<(), AppError> {
        let role = RoleEntity::find_by_id(role_id)
            .one(&*self.db)
            .await?
            .ok_or_else(|| AppError::not_found(format!("角色 {} 未找到", role_id)))?;

        // 系统角色不允许删除
        if role.is_system {
            return Err(AppError::business("系统角色不允许删除"));
        }

        // 检查是否有用户关联此角色
        let user_count = crate::models::user::Entity::find()
            .filter(crate::models::user::Column::RoleId.eq(role_id))
            .count(&*self.db)
            .await?;

        if user_count > 0 {
            return Err(AppError::business(format!(
                "该角色下有 {} 个用户，请先移除用户的角色关联后再删除",
                user_count
            )));
        }

        // 开启事务
        let txn = (*self.db).begin().await?;

        // 删除角色权限
        RolePermissionEntity::delete_many()
            .filter(role_permission::Column::RoleId.eq(role_id))
            .exec(&txn)
            .await?;

        // 删除角色（P0 8-3 修复：补审计日志）
        // 批次 94 P2-10：原 Some(0) 占位改为真实操作人 user_id，便于审计追踪
        crate::services::audit_log_service::AuditLogService::delete_with_audit::<
            RoleEntity,
            _,
        >(&txn, "role", role_id, Some(user_id))
        .await?;

        // 提交事务
        txn.commit().await?;

        Ok(())
    }

    /// 分配权限
    pub async fn assign_permission(
        &self,
        request: AssignPermissionRequest,
        user_id: i32,
    ) -> Result<RolePermissionDetail, AppError> {
        // 检查角色是否存在
        let role = RoleEntity::find_by_id(request.role_id)
            .one(&*self.db)
            .await?
            .ok_or_else(|| AppError::not_found(format!("角色 {} 未找到", request.role_id)))?;

        // 系统角色不允许修改权限
        if role.is_system {
            return Err(AppError::business("系统角色不允许修改权限".to_string()));
        }

        // 检查权限是否已存在
        let mut query = RolePermissionEntity::find()
            .filter(role_permission::Column::RoleId.eq(request.role_id))
            .filter(role_permission::Column::ResourceType.eq(&request.resource_type))
            .filter(role_permission::Column::Action.eq(&request.action));

        if let Some(resource_id) = request.resource_id {
            query = query.filter(role_permission::Column::ResourceId.eq(resource_id));
        } else {
            query = query.filter(role_permission::Column::ResourceId.is_null());
        }

        let existing = query.one(&*self.db).await?;

        if let Some(perm) = existing {
            // V15 P0-S06：保存旧 allowed 值用于审计日志
            let old_allowed = perm.allowed;
            // 更新现有权限
            let mut perm_update: role_permission::ActiveModel = perm.into();
            perm_update.allowed = sea_orm::ActiveValue::Set(request.allowed);
            perm_update.updated_at = sea_orm::ActiveValue::Set(chrono::Utc::now());
            let perm_entity =
                crate::services::audit_log_service::AuditLogService::update_with_audit(
                    &*self.db,
                    "auto_audit",
                    perm_update,
                    // 批次 94 P2-10：原 Some(0) 占位改为真实操作人 user_id，便于审计追踪
                    Some(user_id),
                )
                .await?;

            // V15 P0-S07：权限变更后失效该角色的权限缓存
            invalidate_permission_cache(request.role_id);

            // V15 P0-S06：写入权限变更审计日志（best-effort，失败不阻塞主流程）
            self.write_permission_audit(
                "role_permission_assign",
                user_id,
                request.role_id,
                &request.resource_type,
                &request.action,
                Some(old_allowed.to_string()),
                Some(request.allowed.to_string()),
            )
            .await;

            Ok(RolePermissionDetail {
                id: perm_entity.id,
                role_id: perm_entity.role_id,
                resource_type: perm_entity.resource_type,
                resource_id: perm_entity.resource_id,
                action: perm_entity.action,
                allowed: perm_entity.allowed,
                created_at: perm_entity.created_at,
                updated_at: perm_entity.updated_at,
            })
        } else {
            // 创建新权限
            let permission = role_permission::ActiveModel {
                id: Default::default(),
                role_id: sea_orm::ActiveValue::Set(request.role_id),
                resource_type: sea_orm::ActiveValue::Set(request.resource_type),
                resource_id: sea_orm::ActiveValue::Set(request.resource_id),
                action: sea_orm::ActiveValue::Set(request.action),
                allowed: sea_orm::ActiveValue::Set(request.allowed),
                created_at: sea_orm::ActiveValue::Set(chrono::Utc::now()),
                updated_at: sea_orm::ActiveValue::Set(chrono::Utc::now()),
            };

            let perm_entity = permission.insert(&*self.db).await?;

            // V15 P0-S07：权限变更后失效该角色的权限缓存
            invalidate_permission_cache(perm_entity.role_id);

            // V15 P0-S06：写入权限变更审计日志（新建权限，old_value=None）
            // 注意：request.resource_type/action 已在 ActiveModel 构造时 move，
            // 这里使用 perm_entity 的字段值（实际插入数据库的值）
            self.write_permission_audit(
                "role_permission_assign",
                user_id,
                perm_entity.role_id,
                &perm_entity.resource_type,
                &perm_entity.action,
                None,
                Some(perm_entity.allowed.to_string()),
            )
            .await;

            Ok(RolePermissionDetail {
                id: perm_entity.id,
                role_id: perm_entity.role_id,
                resource_type: perm_entity.resource_type,
                resource_id: perm_entity.resource_id,
                action: perm_entity.action,
                allowed: perm_entity.allowed,
                created_at: perm_entity.created_at,
                updated_at: perm_entity.updated_at,
            })
        }
    }

    /// 移除权限
    pub async fn remove_permission(
        &self,
        permission_id: i32,
        user_id: i32,
    ) -> Result<(), AppError> {
        let permission = RolePermissionEntity::find_by_id(permission_id)
            .one(&*self.db)
            .await?
            .ok_or_else(|| AppError::not_found(format!("权限 {} 未找到", permission_id)))?;

        // 检查是否为系统角色的权限
        let role = RoleEntity::find_by_id(permission.role_id)
            .one(&*self.db)
            .await?;

        if let Some(r) = role {
            if r.is_system {
                return Err(AppError::business("系统角色的权限不允许删除".to_string()));
            }
        }

        // V15 P0-S06：保存被删除权限的信息用于审计日志（删除后无法再获取）
        let audit_role_id = permission.role_id;
        let audit_resource_type = permission.resource_type.clone();
        let audit_action = permission.action.clone();
        let audit_old_allowed = permission.allowed;

        // P0 8-3 修复：delete 操作补审计日志
        // 批次 94 P2-10：原 Some(0) 占位改为真实操作人 user_id，便于审计追踪
        let result = crate::services::audit_log_service::AuditLogService::delete_with_audit::<
            RolePermissionEntity,
            _,
        >(&*self.db, "role_permission", permission_id, Some(user_id))
        .await;

        // V15 P0-S07：权限删除后失效该角色的权限缓存
        if result.is_ok() {
            invalidate_permission_cache(audit_role_id);

            // V15 P0-S06：写入权限删除审计日志（best-effort，失败不阻塞主流程）
            self.write_permission_audit(
                "role_permission_remove",
                user_id,
                audit_role_id,
                &audit_resource_type,
                &audit_action,
                Some(audit_old_allowed.to_string()),
                None,
            )
            .await;
        }

        result
    }

    /// V15 P0-S06 新增：写入权限变更审计日志（best-effort，失败仅记录 warn 不阻塞主流程）
    ///
    /// 记录角色权限的分配/移除变更，用于合规审查和安全追溯。
    /// 审计日志写入失败不影响业务主流程，仅记录 warn 日志。
    ///
    /// # 参数
    /// - `change_type`：变更类型（role_permission_assign / role_permission_remove）
    /// - `operator_id`：操作人 ID
    /// - `role_id`：受影响角色 ID
    /// - `resource_type`：资源类型
    /// - `action`：操作权限码
    /// - `old_value`：旧值（如旧 allowed）
    /// - `new_value`：新值（如新 allowed）
    async fn write_permission_audit(
        &self,
        change_type: &str,
        operator_id: i32,
        role_id: i32,
        resource_type: &str,
        action: &str,
        old_value: Option<String>,
        new_value: Option<String>,
    ) {
        let audit = permission_change_audit::ActiveModel {
            id: Default::default(),
            change_type: sea_orm::ActiveValue::Set(change_type.to_string()),
            operator_id: sea_orm::ActiveValue::Set(operator_id),
            role_id: sea_orm::ActiveValue::Set(Some(role_id)),
            user_id: sea_orm::ActiveValue::Set(None),
            resource_type: sea_orm::ActiveValue::Set(Some(resource_type.to_string())),
            action: sea_orm::ActiveValue::Set(Some(action.to_string())),
            old_value: sea_orm::ActiveValue::Set(old_value),
            new_value: sea_orm::ActiveValue::Set(new_value),
            changed_at: sea_orm::ActiveValue::Set(chrono::Utc::now()),
            client_ip: sea_orm::ActiveValue::Set(None),
            remark: sea_orm::ActiveValue::Set(None),
        };
        if let Err(e) = audit.insert(&*self.db).await {
            tracing::warn!(
                target: "security_audit",
                event = "PERMISSION_AUDIT_WRITE_FAILED",
                change_type = change_type,
                role_id = role_id,
                operator_id = operator_id,
                error = %e,
                "权限变更审计日志写入失败（best-effort，不阻塞主流程）"
            );
        }
    }

    /// 检查角色是否为管理员角色（带缓存）
    async fn is_admin_role(&self, role_id: i32) -> Result<bool, AppError> {
        Ok(admin_checker::is_admin_role(&self.db, role_id).await)
    }

    /// 获取角色的所有权限
    pub async fn get_role_permissions(
        &self,
        role_id: i32,
    ) -> Result<Vec<RolePermissionDetail>, AppError> {
        let permissions = RolePermissionEntity::find()
            .filter(role_permission::Column::RoleId.eq(role_id))
            .order_by(role_permission::Column::ResourceType, Order::Asc)
            .all(&*self.db)
            .await?;

        let permission_list: Vec<RolePermissionDetail> = permissions
            .into_iter()
            .map(|perm| RolePermissionDetail {
                id: perm.id,
                role_id: perm.role_id,
                resource_type: perm.resource_type,
                resource_id: perm.resource_id,
                action: perm.action,
                allowed: perm.allowed,
                created_at: perm.created_at,
                updated_at: perm.updated_at,
            })
            .collect();

        Ok(permission_list)
    }

    /// 检查权限
    pub async fn check_permission(
        &self,
        role_id: i32,
        resource_type: &str,
        action: &str,
        resource_id: Option<i32>,
    ) -> Result<bool, AppError> {
        // Admin 角色绕过所有权限检查（从数据库查询角色编码，而非硬编码 ID）
        if self.is_admin_role(role_id).await? {
            return Ok(true);
        }

        // 查询角色权限
        let mut query = RolePermissionEntity::find()
            .filter(role_permission::Column::RoleId.eq(role_id))
            .filter(role_permission::Column::ResourceType.eq(resource_type))
            .filter(role_permission::Column::Action.eq(action));

        // 如果有具体的资源 ID，优先匹配
        if let Some(rid) = resource_id {
            query = query.filter(
                role_permission::Column::ResourceId
                    .eq(rid)
                    .or(role_permission::Column::ResourceId.is_null()),
            );
        }

        let permissions = query
            .order_by(role_permission::Column::ResourceId, Order::Desc)
            .all(&*self.db)
            .await?;

        // 返回第一个匹配的权限结果
        if let Some(perm) = permissions.into_iter().next() {
            Ok(perm.allowed)
        } else {
            // 默认拒绝
            Ok(false)
        }
    }
}
