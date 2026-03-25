use chrono::Utc;
use crate::models::role;
use sea_orm::{EntityTrait, Set, QueryFilter, ColumnTrait, ActiveModelTrait, PaginatorTrait};
use std::sync::Arc;
use sea_orm::DatabaseConnection;

#[derive(Debug, Clone)]
pub struct RoleService {
    db: Arc<DatabaseConnection>,
}

impl RoleService {
    pub fn new(db: Arc<DatabaseConnection>) -> Self {
        Self { db }
    }

    /// 根据 ID 查找角色
    pub async fn find_by_id(&self, id: i32) -> Result<role::Model, sea_orm::DbErr> {
        role::Entity::find_by_id(id)
            .one(&*self.db)
            .await?
            .ok_or_else(|| sea_orm::DbErr::RecordNotFound(format!("角色 ID {} 不存在", id)))
    }

    /// 根据编码查找角色
    pub async fn find_by_code(&self, code: &str) -> Result<role::Model, sea_orm::DbErr> {
        role::Entity::find()
            .filter(role::Column::Code.eq(code))
            .one(&*self.db)
            .await?
            .ok_or_else(|| sea_orm::DbErr::RecordNotFound(format!("角色编码 {} 不存在", code)))
    }

    /// 创建角色
    pub async fn create_role(
        &self,
        name: String,
        code: String,
        description: Option<String>,
        permissions: Option<String>,
        is_system: bool,
    ) -> Result<role::Model, sea_orm::DbErr> {
        let active_role = role::ActiveModel {
            id: Set(0),
            name: Set(name),
            code: Set(code),
            description: Set(description),
            permissions: Set(permissions),
            is_system: Set(is_system),
            created_at: Set(Utc::now()),
            updated_at: Set(Utc::now()),
        };

        active_role.insert(&*self.db).await
    }

    /// 更新角色信息
    pub async fn update_role(
        &self,
        role_id: i32,
        name: Option<String>,
        code: Option<String>,
        description: Option<String>,
        permissions: Option<String>,
        is_system: Option<bool>,
    ) -> Result<role::Model, sea_orm::DbErr> {
        let mut role_active: role::ActiveModel = role::Entity::find_by_id(role_id)
            .one(&*self.db)
            .await?
            .ok_or_else(|| sea_orm::DbErr::RecordNotFound(format!("角色 ID {} 不存在", role_id)))?
            .into();

        if let Some(name) = name {
            role_active.name = Set(name);
        }
        if let Some(code) = code {
            role_active.code = Set(code);
        }
        if let Some(description) = description {
            role_active.description = Set(Some(description));
        }
        if let Some(permissions) = permissions {
            role_active.permissions = Set(Some(permissions));
        }
        if let Some(is_system) = is_system {
            role_active.is_system = Set(is_system);
        }

        role_active.updated_at = Set(Utc::now());
        role_active.update(&*self.db).await
    }

    /// 删除角色
    pub async fn delete_role(&self, role_id: i32) -> Result<(), sea_orm::DbErr> {
        let role_active: role::ActiveModel = role::Entity::find_by_id(role_id)
            .one(&*self.db)
            .await?
            .ok_or_else(|| sea_orm::DbErr::RecordNotFound(format!("角色 ID {} 不存在", role_id)))?
            .into();

        role_active.delete(&*self.db).await?;
        Ok(())
    }

    /// 获取角色列表（分页）
    pub async fn list_roles(
        &self,
        page: u64,
        page_size: u64,
    ) -> Result<(Vec<role::Model>, u64), sea_orm::DbErr> {
        let paginator = role::Entity::find()
            .paginate(&*self.db, page_size);

        let total = paginator.num_items().await?;
        let roles = paginator.fetch_page(page).await?;

        Ok((roles, total))
    }

    /// 获取所有角色（不分页）
    pub async fn get_all_roles(&self) -> Result<Vec<role::Model>, sea_orm::DbErr> {
        role::Entity::find()
            .all(&*self.db)
            .await
    }
}
