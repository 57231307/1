//! 默认部门与管理员用户子模块（init_service_ops/dept_user）
//!
//! 从原 `init_service.rs` 迁移 3 个方法：
//! - create_default_departments：创建默认部门（总经办 + 财务/销售/仓储/生产部）
//! - create_admin_user：创建管理员用户
//! - reset_password：重置用户密码（密码强度校验 + Argon2id 哈希 + JWT 吊销）

use crate::models::{department, user};
use crate::services::auth_service::AuthService;
use crate::services::init_service::{InitError, InitService};
use sea_orm::{ActiveModelTrait, ColumnTrait, EntityTrait, QueryFilter, Set};
use tracing::warn;

impl InitService {
    async fn create_default_departments(&self) -> Result<i32, InitError> {
        // 先检查总经办是否已存在
        let existing_dept = department::Entity::find()
            .filter(department::Column::Code.eq("D001"))
            .one(self.db.as_ref())
            .await
            .map_err(|e| InitError::DatabaseError(format!("查询部门失败: {}", e)))?;

        if let Some(dept) = existing_dept {
            return Ok(dept.id);
        }

        // 如果不存在，则创建部门
        let dept = department::ActiveModel {
            id: Default::default(),
            name: Set("总经办".to_string()),
            code: Set("D001".to_string()),
            parent_id: Set(None),
            manager_id: Set(None),
            description: Set(Some("公司管理层".to_string())),
            sort_order: Set(1),
            is_active: Set(true),
            created_at: Set(chrono::Utc::now()),
            updated_at: Set(chrono::Utc::now()),
        };
        let dept = dept
            .insert(self.db.as_ref())
            .await
            .map_err(|e| InitError::DatabaseError(format!("创建部门失败: {}", e)))?;

        let departments = vec![
            ("财务部", "D002", 2),
            ("销售部", "D003", 3),
            ("仓储部", "D004", 4),
            ("生产部", "D005", 5),
        ];

        let dept_models: Vec<department::ActiveModel> = departments
            .into_iter()
            .map(|(name, code, sort)| department::ActiveModel {
                id: Default::default(),
                name: Set(name.to_string()),
                code: Set(code.to_string()),
                parent_id: Set(None),
                manager_id: Set(None),
                description: Set(Some(format!("{}部门", name))),
                sort_order: Set(sort),
                is_active: Set(true),
                created_at: Set(chrono::Utc::now()),
                updated_at: Set(chrono::Utc::now()),
            })
            .collect();

        if let Err(e) = department::Entity::insert_many(dept_models)
            .on_conflict(
                sea_orm::sea_query::OnConflict::column(department::Column::Code)
                    .do_nothing()
                    .to_owned(),
            )
            .exec(self.db.as_ref())
            .await
        {
            warn!("批量创建部门失败: {}, 可能部分已存在", e);
        }

        Ok(dept.id)
    }

    async fn create_admin_user(
        &self,
        username: &str,
        password_hash: &str,
        role_id: i32,
        department_id: i32,
    ) -> Result<user::Model, InitError> {
        // 先检查用户是否已存在
        let existing_user = user::Entity::find()
            .filter(user::Column::Username.eq(username))
            .one(self.db.as_ref())
            .await
            .map_err(|e| InitError::DatabaseError(format!("查询用户失败: {}", e)))?;

        if let Some(user) = existing_user {
            return Ok(user);
        }

        let user = user::ActiveModel {
            id: Default::default(),
            username: Set(username.to_string()),
            password_hash: Set(password_hash.to_string()),
            email: Set(Some("admin@example.com".to_string())),
            phone: Set(None),
            role_id: Set(Some(role_id)),
            department_id: Set(Some(department_id)),
            is_active: Set(true),
            totp_secret: Set(None),
            is_totp_enabled: Set(false),
            // v11 批次 141：2FA 恢复码字段（初始为 None）
            totp_recovery_codes: Set(None),
            last_login_at: Set(None),
            // 批次 198 P0-2：初始化 password_changed_at，作为密码过期策略锚点
            password_changed_at: Set(Some(chrono::Utc::now())),
            created_at: Set(chrono::Utc::now()),
            updated_at: Set(chrono::Utc::now()),
        };

        user.insert(self.db.as_ref())
            .await
            .map_err(|e| InitError::DatabaseError(format!("创建管理员用户失败: {}", e)))
    }

    /// 重置用户密码（密码强度校验 + 存在性二次校验 + Argon2id 哈希 + JWT 吊销）
    pub async fn reset_password(
        &self,
        username: &str,
        new_password: &str,
    ) -> Result<(), InitError> {
        // 1) 密码强度校验（与 AuthService::hash_password 行为对齐，复用 password_validator 模块）
        let password_check =
            crate::utils::password_validator::validate_password(new_password);
        if !password_check.is_valid {
            return Err(InitError::ValidationError(
                crate::utils::password_validator::get_password_feedback(&password_check),
            ));
        }

        // 2) 二次校验：用户必须存在（精确区分 NotFound / DatabaseError，避免把 DB 错误误报为用户不存在）
        let user_service = crate::services::user_service::UserService::new(self.db.clone());
        let user = user_service.find_by_username(username).await.map_err(|e| {
            use crate::utils::error::AppError;
            match e {
                AppError::NotFound(_) => InitError::UserNotFound,
                AppError::DatabaseError(msg) => InitError::DatabaseError(msg),
                other => {
                    InitError::DatabaseError(format!("查询用户失败: {}", other))
                }
            }
        })?;

        // 3) Argon2id 哈希
        // v14 P0-1 修复：使用 spawn_blocking 包装 Argon2id 哈希计算，避免阻塞 tokio worker
        let password_hash = AuthService::hash_password_async(new_password.to_string())
            .await
            .map_err(|e| InitError::HashError(e.to_string()))?;

        // 4) 更新密码 + 写日志（service 层不持有 actor 信息，handler 层已记录 actor+target 全量审计）
        // 注意：需在吊销 JWT 之前完成密码更新，确保吊销时用户已存在且密码已变更
        let user_id = user.id; // 保存 user_id 供后续吊销使用
        let mut user_model: user::ActiveModel = user.into();
        user_model.password_hash = Set(password_hash);
        user_model.updated_at = Set(chrono::Utc::now());
        // 批次 198 P0-2：重置密码时同步更新 password_changed_at，作为密码过期策略锚点
        user_model.password_changed_at = Set(Some(chrono::Utc::now()));

        user_model
            .update(self.db.as_ref())
            .await
            .map_err(|e| InitError::DatabaseError(format!("更新密码失败: {}", e)))?;

        // P1 7-2 修复：管理员重置密码后吊销目标用户所有活跃 JWT
        // 修复背景：原 reset_password 成功后未调 revoke_user_jtis，旧 JWT 最长 2 小时仍可用，
        // 被重置密码的账号在密码变更后仍可用旧密码登录态访问系统。
        // 修复方案：与 change_password 对齐，调用 revoke_user_jtis 吊销该用户所有 JTI。
        if let Err(e) =
            crate::services::auth_service::revoke_user_jtis(user_id, "PASSWORD_RESET_BY_ADMIN")
                .await
        {
            tracing::warn!(
                "[SECURITY] password reset succeeded for user_id={} but failed to revoke active JWTs: {}",
                user_id,
                e
            );
        }

        // 安全审计：service 层落库成功时记录日志，便于运维排查（handler 层已异步写入 audit_log 表）
        tracing::info!(
            "[SECURITY] password reset succeeded for username={} (user_id={}, JWTs revoked)",
            username,
            user_id
        );

        Ok(())
    }
}
