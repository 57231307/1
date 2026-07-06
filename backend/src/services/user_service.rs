//! 用户服务模块
//!
//! 提供用户管理的核心业务逻辑，包括用户CRUD、查询和状态管理。
//!
//! # 主要功能
//! - 用户创建、查询、更新、删除（软删除）
//! - 按用户名/ID查找用户
//! - 用户列表分页查询
//! - 最后登录时间更新
//!
//! # 安全特性
//! - 软删除机制（不物理删除数据）
//! - 密码哈希由调用方处理，本模块不处理明文密码


use crate::models::user;
use crate::utils::error::AppError;
use crate::utils::pagination::paginate_with_total;
use sea_orm::DatabaseConnection;
use sea_orm::{ActiveModelTrait, ColumnTrait, EntityTrait, QueryFilter, Set};
use std::sync::Arc;

/// 用户服务
///
/// 处理用户相关的业务逻辑
#[derive(Debug, Clone)]
pub struct UserService {
    db: Arc<DatabaseConnection>,
}

impl UserService {
    /// 创建新的用户服务实例
    ///
    /// # 参数
    /// - `db`: 数据库连接
    pub fn new(db: Arc<DatabaseConnection>) -> Self {
        Self { db }
    }

    /// 按用户名查找用户
    ///
    /// # 参数
    /// - `username`: 用户名
    ///
    /// # 返回
    /// - `Ok(user)`: 找到用户
    /// - `Err(DbErr::RecordNotFound)`: 用户不存在
    pub async fn find_by_username(&self, username: &str) -> Result<user::Model, AppError> {
        user::Entity::find()
            .filter(user::Column::Username.eq(username))
            .one(self.db.as_ref())
            .await?
            .ok_or_else(|| AppError::not_found(format!("用户 {} 不存在", username)))
    }

    /// 按 ID 查找用户（命中 Redis 时直接返回缓存）
    ///
    /// # 参数
    /// - `id`: 用户 ID
    ///
    /// # 返回
    /// - `Ok(user)`: 找到用户
    /// - `Err(DbErr::RecordNotFound)`: 用户不存在
    pub async fn find_by_id(&self, id: i32) -> Result<user::Model, AppError> {
        user::Entity::find_by_id(id)
            .one(self.db.as_ref())
            .await?
            .ok_or_else(|| AppError::not_found(format!("用户 ID {} 不存在", id)))
    }

    /// 创建新用户
    ///
    /// # 参数
    /// - `username`: 用户名（唯一）
    /// - `password_hash`: 密码哈希值（由 AuthService 生成）
    /// - `email`: 邮箱（可选）
    /// - `phone`: 电话（可选）
    /// - `role_id`: 角色 ID（可选）
    /// - `department_id`: 部门 ID（可选）
    ///
    /// # 返回
    /// - `Ok(user)`: 创建成功，返回用户数据
    /// - `Err(DbErr)`: 创建失败（如用户名已存在）
    ///
    /// # 注意
    /// 密码必须由调用方预先哈希，本方法不处理明文密码
    pub async fn create_user(
        &self,
        username: String,
        password_hash: String,
        email: Option<String>,
        phone: Option<String>,
        role_id: Option<i32>,
        department_id: Option<i32>,
    ) -> Result<user::Model, AppError> {
        // 检查用户名是否已存在
        let existing = user::Entity::find()
            .filter(user::Column::Username.eq(&username))
            .one(self.db.as_ref())
            .await?;
        if existing.is_some() {
            return Err(AppError::business(format!("用户名 '{}' 已存在", username)));
        }

        let active_user = user::ActiveModel {
            id: Default::default(),
            username: Set(username),
            password_hash: Set(password_hash),
            email: Set(email),
            phone: Set(phone),
            role_id: Set(role_id),
            department_id: Set(department_id),
            is_active: Set(true),
            totp_secret: Set(None),
            is_totp_enabled: Set(false),
            // v11 批次 141：2FA 恢复码字段（初始为 None）
            totp_recovery_codes: Set(None),
            last_login_at: Set(None),
            created_at: Set(chrono::Utc::now()),
            updated_at: Set(chrono::Utc::now()),
        };

        active_user
            .insert(self.db.as_ref())
            .await
            .map_err(AppError::from)
    }

    /// 更新用户最后登录时间
    ///
    /// # 参数
    /// - `user_id`: 用户 ID
    ///
    /// # 返回
    /// - `Ok(())`: 更新成功
    /// - `Err(DbErr::RecordNotFound)`: 用户不存在
    pub async fn update_last_login(&self, user_id: i32) -> Result<(), AppError> {
        let mut user: user::ActiveModel = user::Entity::find_by_id(user_id)
            .one(self.db.as_ref())
            .await?
            .ok_or_else(|| AppError::not_found(format!("用户 ID {} 不存在", user_id)))?
            .into();

        user.last_login_at = Set(Some(chrono::Utc::now()));
        user.update(self.db.as_ref()).await?;

        Ok(())
    }

    /// 查询用户列表（分页）
    ///
    /// # 参数
    /// - `page`: 页码（从0开始）
    /// - `page_size`: 每页数量
    ///
    /// # 返回
    /// - `Ok((users, total))`: 用户列表和总数量
    pub async fn list_users(
        &self,
        page: u64,
        page_size: u64,
    ) -> Result<(Vec<user::Model>, u64), AppError> {
        use sea_orm::PaginatorTrait;

        let paginator = user::Entity::find().paginate(self.db.as_ref(), page_size);

        // 使用统一分页辅助函数，并行执行分页查询与总数统计
        let (users, total) = paginate_with_total(paginator, page).await?;

        Ok((users, total))
    }

    /// 更新用户信息
    ///
    /// 只更新提供的字段，未提供的字段保持不变
    ///
    /// # 参数
    /// - `user_id`: 用户 ID
    /// - `email`: 新邮箱（可选）
    /// - `phone`: 新电话（可选）
    /// - `role_id`: 新角色 ID（可选）
    /// - `department_id`: 新部门 ID（可选）
    /// - `status`: 状态字符串，`"active"` 表示激活，其他表示禁用（可选）
    ///
    /// # 返回
    /// - `Ok(user)`: 更新成功
    /// - `Err(DbErr::RecordNotFound)`: 用户不存在
    pub async fn update_user(
        &self,
        user_id: i32,
        email: Option<String>,
        phone: Option<String>,
        role_id: Option<i32>,
        department_id: Option<i32>,
        status: Option<String>,
    ) -> Result<user::Model, AppError> {
        let mut user: user::ActiveModel = user::Entity::find_by_id(user_id)
            .one(self.db.as_ref())
            .await?
            .ok_or_else(|| AppError::not_found(format!("用户 ID {} 不存在", user_id)))?
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

        let updated = user
            .update(self.db.as_ref())
            .await
            .map_err(AppError::from)?;

        Ok(updated)
    }

    /// 删除用户（软删除）
    ///
    /// 将用户设置为非激活状态，不物理删除数据
    /// 保留用户历史记录和关联数据
    ///
    /// # 安全
    /// 软删除成功后立即吊销该用户的所有活跃 JWT，防止被删除用户的旧 Token
    /// 在剩余有效期（最长 2 小时）内继续访问系统。吊销属 best-effort，失败仅记录
    /// warn 日志，不阻塞删除主流程。
    ///
    /// # 参数
    /// - `user_id`: 用户 ID
    ///
    /// # 返回
    /// - `Ok(())`: 删除成功
    /// - `Err(DbErr::RecordNotFound)`: 用户不存在
    ///
    /// # 注意
    /// 软删除后用户无法登录，但数据仍保留在数据库中
    pub async fn delete_user(&self, user_id: i32) -> Result<(), AppError> {
        let mut user: user::ActiveModel = user::Entity::find_by_id(user_id)
            .one(self.db.as_ref())
            .await?
            .ok_or_else(|| AppError::not_found(format!("用户 ID {} 不存在", user_id)))?
            .into();

        // 软删除：只设置为非激活状态
        user.is_active = Set(false);
        user.updated_at = Set(chrono::Utc::now());
        user.update(self.db.as_ref()).await?;

        // P0 7-3 修复：吊销该用户的所有活跃 JWT
        //    将吊销逻辑下沉到 service 层作为单一真相源，保证任何调用方
        //    （handler / 定时任务 / 其他 service）都能自动获得吊销保护。
        //    auth_middleware 会拒绝该用户 iat < revoked_at 的 Token。
        if let Err(e) =
            crate::services::auth_service::revoke_user_jtis(user_id, "USER_DELETED").await
        {
            tracing::warn!(
                target: "security_audit",
                event = "TOKEN_REVOKE_FAILED",
                user_id = user_id,
                error = %e,
                "[SECURITY] 吊销已删除用户 {} 的活跃 JWT 失败",
                user_id
            );
        }

        Ok(())
    }
}
