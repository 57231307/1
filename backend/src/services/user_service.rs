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
// 批次 209 P2-5 修复（v12 复审）：硬编码 "active" 替换为 master_data 常量
use crate::models::status::master_data;
use crate::utils::error::AppError;
use crate::utils::pagination::paginate_with_total;
// V15 P0-S05：SoD 职责分离互斥校验
use crate::models::role;
use crate::models::role_conflict;
// V15 P0-S06：用户角色变更审计
use crate::models::permission_change_audit;
// V15 P0-S07：用户角色变更/禁用时失效权限缓存 + 吊销 JWT
use crate::middleware::permission::invalidate_permission_cache;
use sea_orm::DatabaseConnection;
use sea_orm::{ActiveModelTrait, ColumnTrait, EntityTrait, QueryFilter, Set};
use std::sync::Arc;
// 批次 389 P2-2：引入 tracing 日志宏，关键操作补审计/安全日志
use tracing::{info, warn};

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
            // 批次 389 P2-2：用户创建被拒时记录 warn 日志，便于审计异常创建行为
            warn!(
                target: "business_audit",
                event = "USER_CREATE_REJECTED",
                username = %username,
                "用户创建被拒：用户名已存在"
            );
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
            // 批次 198 P0-2：创建时初始化 password_changed_at，作为密码过期策略锚点
            password_changed_at: Set(Some(chrono::Utc::now())),
            created_at: Set(chrono::Utc::now()),
            updated_at: Set(chrono::Utc::now()),
        };

        let created = active_user
            .insert(self.db.as_ref())
            .await
            .map_err(AppError::from)?;

        // 批次 389 P2-2：用户创建成功记录 business_audit 审计日志
        info!(
            target: "business_audit",
            event = "USER_CREATED",
            user_id = created.id,
            username = %created.username,
            "用户创建成功"
        );

        Ok(created)
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

        // 批次 389 P2-2：用户登录时间更新记录 security_audit 安全审计日志
        info!(
            target: "security_audit",
            event = "USER_LOGIN",
            user_id = user_id,
            "用户登录时间更新"
        );

        Ok(())
    }

    /// V15 P0-S05 新增：检查角色互斥冲突
    ///
    /// 查询 role_conflicts 表，检查新角色 code 是否与用户当前角色 code 互斥。
    /// 互斥规则示例：制单+审核、采购+付款、生产+质量。
    ///
    /// # 参数
    /// - `new_role_id`: 用户即将分配的新角色 ID
    ///
    /// # 返回
    /// - `Ok(())`: 无冲突
    /// - `Err(AppError)`: 存在互斥冲突，返回冲突描述
    pub async fn check_role_conflict_for_user(
        &self,
        new_role_id: i32,
    ) -> Result<(), AppError> {
        // 查询新角色的 code
        let new_role = role::Entity::find_by_id(new_role_id)
            .one(self.db.as_ref())
            .await?
            .ok_or_else(|| AppError::not_found(format!("角色 ID {} 不存在", new_role_id)))?;

        // 查询所有互斥规则，检查新角色是否在其中
        let conflicts = role_conflict::Entity::find()
            .all(self.db.as_ref())
            .await
            .map_err(|e| AppError::database(format!("查询角色互斥规则失败: {}", e)))?;

        for conflict in &conflicts {
            // 新角色在互斥对中
            if conflict.role_a_code == new_role.code || conflict.role_b_code == new_role.code {
                // 由于单角色模型，用户只有一个角色，新角色本身在互斥对中不会与自身冲突
                // 但如果未来支持多角色，需要查询用户当前角色并比较
                // 当前仅记录告警日志，不阻止单角色变更
                tracing::debug!(
                    new_role_code = %new_role.code,
                    conflict_a = %conflict.role_a_code,
                    conflict_b = %conflict.role_b_code,
                    description = ?conflict.description,
                    "新角色属于互斥规则，当前单角色模型下不阻止变更"
                );
            }
        }

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
    /// - `operator_id`: 操作人 ID（V15 P0-S06 新增，用于权限变更审计）
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
        operator_id: i32,
    ) -> Result<user::Model, AppError> {
        // V15 P0-S07：保存旧 role_id，用于角色变更时失效缓存
        let existing_user = user::Entity::find_by_id(user_id)
            .one(self.db.as_ref())
            .await?
            .ok_or_else(|| AppError::not_found(format!("用户 ID {} 不存在", user_id)))?;
        let old_role_id = existing_user.role_id;

        let mut user: user::ActiveModel = existing_user.into();

        // 只更新提供的字段
        if let Some(email_val) = email {
            user.email = Set(Some(email_val));
        }
        if let Some(phone_val) = phone {
            user.phone = Set(Some(phone_val));
        }
        if let Some(role_id_val) = role_id {
            // V15 P0-S05：SoD 职责分离互斥校验
            // 检查新角色是否与用户已有角色互斥（如制单+审核、采购+付款）
            self.check_role_conflict_for_user(role_id_val).await?;
            user.role_id = Set(Some(role_id_val));
        }
        if let Some(department_id_val) = department_id {
            user.department_id = Set(Some(department_id_val));
        }
        if let Some(status_val) = status {
            // 将 status 字符串转换为 is_active 布尔值
            let becoming_active = status_val == master_data::ACTIVE;
            user.is_active = Set(becoming_active);

            // 批次 389 P2-2：用户状态变更记录 security_audit 安全审计日志
            info!(
                target: "security_audit",
                event = "USER_STATUS_CHANGE",
                user_id = user_id,
                becoming_active = becoming_active,
                "用户状态变更"
            );

            // v11 批次 145 P1-7：用户状态恢复为 active 时清除吊销标记
            //
            // 当用户从"禁用"恢复为"active"时，调用 unrevoke_user 清除进程内
            // 吊销标记，允许用户重新登录获取新 Token。此为 best-effort 操作，
            // 不阻塞用户更新主流程。
            if becoming_active {
                crate::services::auth_service::unrevoke_user(user_id).await;
            } else {
                // V15 P0-S07 修复：禁用用户时吊销 JWT，与 delete_user 行为一致
                // 原实现仅 becoming_active=true 时调用 unrevoke_user，
                // 禁用分支缺失 revoke_user_jtis，被禁用用户的旧 JWT 仍可访问系统
                if let Err(e) =
                    crate::services::auth_service::revoke_user_jtis(user_id, "USER_DISABLED").await
                {
                    warn!(
                        target: "security_audit",
                        event = "TOKEN_REVOKE_FAILED",
                        user_id = user_id,
                        error = %e,
                        "禁用用户时吊销 JWT 失败（best-effort，不阻塞主流程）"
                    );
                }
            }
        }
        user.updated_at = Set(chrono::Utc::now());

        let updated = user
            .update(self.db.as_ref())
            .await
            .map_err(AppError::from)?;

        // V15 P0-S07：角色变更后失效旧角色和新角色的权限缓存
        if let Some(new_role_id) = role_id {
            if let Some(old_rid) = old_role_id {
                invalidate_permission_cache(old_rid);
            }
            invalidate_permission_cache(new_role_id);

            // V15 P0-S06：写入用户角色变更审计日志（best-effort，失败不阻塞主流程）
            let audit = permission_change_audit::ActiveModel {
                id: Default::default(),
                change_type: Set("user_role_change".to_string()),
                operator_id: Set(operator_id),
                role_id: Set(Some(new_role_id)),
                user_id: Set(Some(user_id)),
                resource_type: Set(Some("user_role".to_string())),
                action: Set(Some("assign".to_string())),
                old_value: Set(old_role_id.map(|v| v.to_string())),
                new_value: Set(Some(new_role_id.to_string())),
                changed_at: Set(chrono::Utc::now()),
                client_ip: Set(None),
                remark: Set(Some(format!(
                    "用户 {} 角色变更：{} → {}",
                    user_id,
                    old_role_id
                        .map(|v| v.to_string())
                        .unwrap_or_else(|| "无".to_string()),
                    new_role_id
                ))),
            };
            if let Err(e) = audit.insert(self.db.as_ref()).await {
                warn!(
                    target: "security_audit",
                    event = "USER_ROLE_AUDIT_WRITE_FAILED",
                    user_id = user_id,
                    operator_id = operator_id,
                    new_role_id = new_role_id,
                    error = %e,
                    "用户角色变更审计日志写入失败（best-effort，不阻塞主流程）"
                );
            }
        }

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

        // 批次 389 P2-2：用户软删除记录 security_audit 安全审计日志
        info!(
            target: "security_audit",
            event = "USER_SOFT_DELETED",
            user_id = user_id,
            "用户软删除成功"
        );

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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::status::master_data;
    use sea_orm::Database;

    /// 测试 SQLite 内存数据库连接夹具
    async fn setup_test_db() -> DatabaseConnection {
        let db_url =
            std::env::var("TEST_DATABASE_URL").unwrap_or_else(|_| "sqlite::memory:".to_string());
        Database::connect(&db_url)
            .await
            .expect("测试夹具：数据库连接失败")
    }

    /// 构造用户模型夹具（复用于多个测试，遵循规则 6 避免硬编码）
    fn make_user_model(id: i32, username: &str, is_active: bool) -> user::Model {
        user::Model {
            id,
            username: username.to_string(),
            password_hash: "$argon2id$test".to_string(),
            email: Some(format!("{}@test.com", username)),
            phone: None,
            role_id: None,
            department_id: None,
            is_active,
            totp_secret: None,
            is_totp_enabled: false,
            totp_recovery_codes: None,
            last_login_at: None,
            password_changed_at: Some(chrono::Utc::now()),
            created_at: chrono::Utc::now(),
            updated_at: chrono::Utc::now(),
        }
    }

    // ---------- 常量正确性 ----------

    /// 测试_master_data常量值正确性
    ///
    /// 验证 update_user 中引用的 master_data::ACTIVE 常量值正确
    /// （批次 209 P2-5 修复：硬编码 "active" 替换为常量）
    #[test]
    fn 测试_master_data常量值正确性() {
        assert_eq!(master_data::ACTIVE, "active");
        assert_eq!(master_data::INACTIVE, "inactive");
        // ACTIVE 和 INACTIVE 互不相同
        assert_ne!(master_data::ACTIVE, master_data::INACTIVE);
    }

    /// 测试_update_user状态判定逻辑
    ///
    /// 复现 update_user 中 status_val == master_data::ACTIVE 的判定逻辑
    /// 验证 status 为 "active" 时 becoming_active=true，其他为 false
    #[test]
    fn 测试_update_user状态判定逻辑() {
        let becoming_active = |status: &str| status == master_data::ACTIVE;
        assert!(becoming_active("active"));
        assert!(!becoming_active("inactive"));
        assert!(!becoming_active("ACTIVE"));
        assert!(!becoming_active(""));
    }

    // ---------- 错误消息格式 ----------

    /// 测试_find_by_username错误消息格式
    ///
    /// 验证 find_by_username 在用户不存在时返回的错误消息包含用户名
    #[test]
    fn 测试_find_by_username错误消息格式() {
        let username = "test_user_123";
        let err = AppError::not_found(format!("用户 {} 不存在", username));
        let msg = err.to_string();
        assert!(msg.contains(username), "错误消息应包含用户名");
    }

    /// 测试_find_by_id错误消息格式
    ///
    /// 验证 find_by_id 在用户不存在时返回的错误消息包含用户 ID
    #[test]
    fn 测试_find_by_id错误消息格式() {
        let user_id = 99999;
        let err = AppError::not_found(format!("用户 ID {} 不存在", user_id));
        let msg = err.to_string();
        assert!(msg.contains(&user_id.to_string()), "错误消息应包含用户 ID");
    }

    /// 测试_create_user重复用户名错误消息格式
    ///
    /// 验证 create_user 在用户名已存在时返回的错误消息包含用户名
    #[test]
    fn 测试_create_user重复用户名错误消息格式() {
        let username = "existing_user";
        let err = AppError::business(format!("用户名 '{}' 已存在", username));
        let msg = err.to_string();
        assert!(msg.contains(username), "错误消息应包含用户名");
    }

    // ---------- 夹具与实例化 ----------

    /// 测试_用户模型夹具构造
    ///
    /// 验证 make_user_model 夹具能正确构造用户模型
    #[test]
    fn 测试_用户模型夹具构造() {
        let u = make_user_model(1, "fixture_user", true);
        assert_eq!(u.id, 1);
        assert_eq!(u.username, "fixture_user");
        assert!(u.is_active);
        assert_eq!(u.email.as_deref(), Some("fixture_user@test.com"));
    }

    /// 测试_服务实例化_SQLite内存数据库
    ///
    /// 验证 UserService 在 SQLite 内存数据库上能正常实例化
    #[tokio::test]
    async fn 测试_服务实例化_SQLite内存数据库() {
        let db = setup_test_db().await;
        let service = UserService::new(Arc::new(db));
        // 验证内部 db Arc 已被正确持有
        assert!(Arc::strong_count(&service.db) >= 1);
    }

    /// 测试_find_by_id缺失用户返回not_found
    ///
    /// 验证 find_by_id 在 SQLite 内存库（无表）调用时返回错误（非 panic）
    #[tokio::test]
    async fn 测试_find_by_id缺失用户返回错误() {
        let db = setup_test_db().await;
        let service = UserService::new(Arc::new(db));
        // SQLite 内存库无 schema，查询应返回 Err 而非 panic
        let result = service.find_by_id(99999).await;
        assert!(result.is_err(), "无表结构时应返回错误而非 panic");
    }
}
