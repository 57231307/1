//! 系统初始化服务

use crate::models::department;
use crate::models::role;
use crate::models::user;
use crate::services::auth_service::AuthService;
use sea_orm::{ActiveModelTrait, ConnectOptions, Database, DatabaseConnection, DbErr, EntityTrait, PaginatorTrait, Set, ConnectionTrait};
use std::sync::Arc;
use std::time::Duration;

#[derive(Debug, Clone, serde::Deserialize, serde::Serialize)]
pub struct DatabaseConfig {
    pub host: String,
    pub port: String,
    pub name: String,
    pub username: String,
    pub password: String,
}

impl DatabaseConfig {
    pub fn to_connection_string(&self) -> String {
        format!(
            "postgres://{}:{}@{}:{}/{}",
            self.username, self.password, self.host, self.port, self.name
        )
    }
}

#[derive(Debug, Clone)]
pub struct InitService {
    db: Arc<DatabaseConnection>,
}

impl InitService {
    pub fn new(db: Arc<DatabaseConnection>) -> Self {
        Self { db }
    }

    pub async fn check_initialized(&self) -> (bool, String) {
        match user::Entity::find()
            .count(self.db.as_ref())
            .await
        {
            Ok(count) => {
                if count > 0 {
                    (true, "系统已初始化".to_string())
                } else {
                    (false, "系统未初始化".to_string())
                }
            }
            Err(e) => {
                (false, format!("检查初始化状态失败: {}", e))
            }
        }
    }

    pub async fn test_database(config: &DatabaseConfig) -> Result<(), InitError> {
        let conn_str = config.to_connection_string();
        
        let mut opt = ConnectOptions::new(&conn_str);
        opt.max_connections(1)
            .min_connections(0)
            .connect_timeout(Duration::from_secs(5))
            .acquire_timeout(Duration::from_secs(5));

        let db = Database::connect(opt)
            .await
            .map_err(|e| InitError::DatabaseError(format!("数据库连接失败: {}", e)))?;

        let result: Result<i32, DbErr> = db
            .query_one(sea_orm::Statement::from_string(
                sea_orm::DatabaseBackend::Postgres,
                "SELECT 1 as test".to_string(),
            ))
            .await
            .map(|v: Option<sea_orm::QueryResult>| {
                v.map(|row: sea_orm::QueryResult| row.try_get::<i32>("", "test").unwrap_or(1))
            })
            .map(|opt: Option<i32>| opt.unwrap_or(0));

        result.map(|_| ()).map_err(|e| {
            InitError::DatabaseError(format!("数据库测试查询失败: {}", e))
        })
    }

    pub async fn initialize(
        &self,
        admin_username: &str,
        admin_password: &str,
    ) -> Result<InitializationResult, InitError> {
        let (initialized, _) = self.check_initialized().await;
        if initialized {
            return Err(InitError::AlreadyInitialized);
        }

        let password_hash =
            AuthService::hash_password(admin_password).map_err(|e| InitError::HashError(e.to_string()))?;

        let admin_role = self.create_default_roles().await?;
        let department_id = self.create_default_departments().await?;

        self.create_admin_user(admin_username, &password_hash, admin_role.id, department_id)
            .await?;

        Ok(InitializationResult {
            success: true,
            message: "系统初始化成功".to_string(),
            admin_username: admin_username.to_string(),
        })
    }

    pub async fn initialize_with_db(
        db_config: &DatabaseConfig,
        admin_username: &str,
        admin_password: &str,
    ) -> Result<InitializationResult, InitError> {
        Self::test_database(db_config).await?;

        let conn_str = db_config.to_connection_string();
        
        let mut opt = ConnectOptions::new(&conn_str);
        opt.max_connections(10)
            .min_connections(1)
            .connect_timeout(Duration::from_secs(10))
            .acquire_timeout(Duration::from_secs(10));

        let db = Database::connect(opt)
            .await
            .map_err(|e| InitError::DatabaseError(format!("数据库连接失败: {}", e)))?;

        let db = Arc::new(db);
        let service = Self::new(db);

        service.initialize(admin_username, admin_password).await
    }

    async fn create_default_roles(&self) -> Result<role::Model, InitError> {
        let admin_role = role::ActiveModel {
            id: Set(0),
            name: Set("管理员".to_string()),
            code: Set("admin".to_string()),
            description: Set(Some("系统管理员".to_string())),
            permissions: Set(Some("[\"*\"]".to_string())),
            is_system: Set(true),
            created_at: Set(chrono::Utc::now()),
            updated_at: Set(chrono::Utc::now()),
        };
        let admin_role = admin_role
            .insert(self.db.as_ref())
            .await
            .map_err(|e| InitError::DatabaseError(format!("创建管理员角色失败: {}", e)))?;

        let manager_role = role::ActiveModel {
            id: Set(0),
            name: Set("部门经理".to_string()),
            code: Set("manager".to_string()),
            description: Set(Some("部门经理".to_string())),
            permissions: Set(Some(
                "[\"user:view\", \"product:*\", \"inventory:*\", \"sales:*\"]".to_string(),
            )),
            is_system: Set(true),
            created_at: Set(chrono::Utc::now()),
            updated_at: Set(chrono::Utc::now()),
        };
        manager_role
            .insert(self.db.as_ref())
            .await
            .map_err(|e| InitError::DatabaseError(format!("创建经理角色失败: {}", e)))?;

        let operator_role = role::ActiveModel {
            id: Set(0),
            name: Set("操作员".to_string()),
            code: Set("operator".to_string()),
            description: Set(Some("操作员".to_string())),
            permissions: Set(Some(
                "[\"product:view\", \"inventory:view\", \"sales:view\"]".to_string(),
            )),
            is_system: Set(true),
            created_at: Set(chrono::Utc::now()),
            updated_at: Set(chrono::Utc::now()),
        };
        operator_role
            .insert(self.db.as_ref())
            .await
            .map_err(|e| InitError::DatabaseError(format!("创建操作员角色失败: {}", e)))?;

        Ok(admin_role)
    }

    async fn create_default_departments(&self) -> Result<i32, InitError> {
        let dept = department::ActiveModel {
            id: Set(0),
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

        for (name, code, sort) in departments {
            let _ = department::ActiveModel {
                id: Set(0),
                name: Set(name.to_string()),
                code: Set(code.to_string()),
                parent_id: Set(None),
                manager_id: Set(None),
                description: Set(Some(format!("{}部门", name))),
                sort_order: Set(sort),
                is_active: Set(true),
                created_at: Set(chrono::Utc::now()),
                updated_at: Set(chrono::Utc::now()),
            }
            .insert(self.db.as_ref())
            .await;
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
        let user = user::ActiveModel {
            id: Set(0),
            username: Set(username.to_string()),
            password_hash: Set(password_hash.to_string()),
            email: Set(Some("admin@example.com".to_string())),
            phone: Set(None),
            role_id: Set(Some(role_id)),
            department_id: Set(Some(department_id)),
            is_active: Set(true),
            last_login_at: Set(None),
            created_at: Set(chrono::Utc::now()),
            updated_at: Set(chrono::Utc::now()),
        };

        user.insert(self.db.as_ref())
            .await
            .map_err(|e| InitError::DatabaseError(format!("创建管理员用户失败: {}", e)))
    }

    pub async fn reset_password(
        &self,
        username: &str,
        new_password: &str,
    ) -> Result<(), InitError> {
        let user_service = crate::services::user_service::UserService::new(self.db.clone());
        let user = user_service
            .find_by_username(username)
            .await
            .map_err(|_| InitError::UserNotFound)?;

        let password_hash =
            AuthService::hash_password(new_password).map_err(|e| InitError::HashError(e.to_string()))?;

        let mut user_model: user::ActiveModel = user.into();
        user_model.password_hash = Set(password_hash);
        user_model.updated_at = Set(chrono::Utc::now());

        user_model
            .update(self.db.as_ref())
            .await
            .map_err(|e| InitError::DatabaseError(format!("更新密码失败: {}", e)))?;

        Ok(())
    }
}

#[derive(Debug, thiserror::Error)]
pub enum InitError {
    #[error("系统已经初始化")]
    AlreadyInitialized,
    #[error("密码哈希错误：{0}")]
    HashError(String),
    #[error("数据库错误：{0}")]
    DatabaseError(String),
    #[error("用户不存在")]
    UserNotFound,
    #[error("配置错误：{0}")]
    ConfigError(String),
}

#[derive(Debug, serde::Serialize)]
pub struct InitializationResult {
    pub success: bool,
    pub message: String,
    pub admin_username: String,
}

#[derive(Debug, serde::Deserialize)]
pub struct InitRequest {
    pub admin_username: String,
    pub admin_password: String,
}

#[derive(Debug, serde::Serialize)]
pub struct InitStatus {
    pub initialized: bool,
    pub message: String,
}
