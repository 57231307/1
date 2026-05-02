use crate::models::user;
use sea_orm::DatabaseConnection;
use sea_orm::{ActiveModelTrait, ColumnTrait, EntityTrait, QueryFilter, Set};
use std::sync::Arc;

#[derive(Debug, Clone)]
pub struct UserService {
    db: Arc<DatabaseConnection>,
}

impl UserService {
    pub fn new(db: Arc<DatabaseConnection>) -> Self {
        Self { db }
    }

    pub async fn find_by_username(&self, username: &str) -> Result<user::Model, sea_orm::DbErr> {
        user::Entity::find()
            .filter(user::Column::Username.eq(username))
            .one(self.db.as_ref())
            .await?
            .ok_or_else(|| sea_orm::DbErr::RecordNotFound(format!("用户 {} 不存在", username)))
    }

    pub async fn find_by_id(&self, id: i32) -> Result<user::Model, sea_orm::DbErr> {
        user::Entity::find_by_id(id)
            .one(self.db.as_ref())
            .await?
            .ok_or_else(|| sea_orm::DbErr::RecordNotFound(format!("用户 ID {} 不存在", id)))
    }

    pub async fn create_user(
        &self,
        username: String,
        password_hash: String,
        email: Option<String>,
        phone: Option<String>,
        role_id: Option<i32>,
        department_id: Option<i32>,
    ) -> Result<user::Model, sea_orm::DbErr> {
        let active_user = user::ActiveModel {
            id: Set(0),
            username: Set(username),
            password_hash: Set(password_hash),
            email: Set(email),
            phone: Set(phone),
            role_id: Set(role_id),
            department_id: Set(department_id),
            is_active: Set(true),
            totp_secret: Set(None),
            is_totp_enabled: Set(false),
            last_login_at: Set(None),
            created_at: Set(chrono::Utc::now()),
            updated_at: Set(chrono::Utc::now()),
        };

        active_user.insert(self.db.as_ref()).await
    }

    pub async fn update_last_login(&self, user_id: i32) -> Result<(), sea_orm::DbErr> {
        let mut user: user::ActiveModel = user::Entity::find_by_id(user_id)
            .one(self.db.as_ref())
            .await?
            .ok_or_else(|| sea_orm::DbErr::RecordNotFound(format!("用户 ID {} 不存在", user_id)))?
            .into();

        user.last_login_at = Set(Some(chrono::Utc::now()));
        user.update(self.db.as_ref()).await?;

        Ok(())
    }

    pub async fn list_users(
        &self,
        page: u64,
        page_size: u64,
    ) -> Result<(Vec<user::Model>, u64), sea_orm::DbErr> {
        use sea_orm::PaginatorTrait;

        let paginator = user::Entity::find().paginate(self.db.as_ref(), page_size);

        let total = paginator.num_items().await?;
        let users = paginator.fetch_page(if page > 0 { page - 1 } else { 0 }).await?;

        Ok((users, total))
    }

    /// 更新用户信息
    pub async fn update_user(
        &self,
        user_id: i32,
        email: Option<String>,
        phone: Option<String>,
        role_id: Option<i32>,
        department_id: Option<i32>,
        status: Option<String>,
    ) -> Result<user::Model, sea_orm::DbErr> {
        let mut user: user::ActiveModel = user::Entity::find_by_id(user_id)
            .one(self.db.as_ref())
            .await?
            .ok_or_else(|| sea_orm::DbErr::RecordNotFound(format!("用户 ID {} 不存在", user_id)))?
            .into();

        // 只更新提供的字段
        if let Some(email_val) = email {
            user.email = Set(Some(email_val));
        }
        if let Some(phone_val) = phone {
            user.phone = Set(Some(phone_val));
        }
        if let Some(role_id_val) = role_id {
            user.role_id = Set(Some(role_id_val));
        }
        if let Some(department_id_val) = department_id {
            user.department_id = Set(Some(department_id_val));
        }
        if let Some(status_val) = status {
            // 将 status 字符串转换为 is_active 布尔值
            user.is_active = Set(status_val == "active");
        }
        user.updated_at = Set(chrono::Utc::now());

        user.update(self.db.as_ref()).await
    }

    /// 删除用户（软删除，设置为非激活状态）
    pub async fn delete_user(&self, user_id: i32) -> Result<(), sea_orm::DbErr> {
        let mut user: user::ActiveModel = user::Entity::find_by_id(user_id)
            .one(self.db.as_ref())
            .await?
            .ok_or_else(|| sea_orm::DbErr::RecordNotFound(format!("用户 ID {} 不存在", user_id)))?
            .into();

        // 软删除：只设置为非激活状态
        user.is_active = Set(false);
        user.updated_at = Set(chrono::Utc::now());
        user.update(self.db.as_ref()).await?;

        Ok(())
    }
}
