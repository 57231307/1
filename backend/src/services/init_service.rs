//! 系统初始化服务
#![allow(dead_code)]
// TODO(tech-debt): 业务接入或重评估后逐项移除；rustc 1.94+ 编译时由编译器报告具体死代码位置。

use crate::models::department;
use crate::models::role;
use crate::models::user;
use crate::services::auth_service::AuthService;
use crate::utils::error::AppError;
use sea_orm::{
    ActiveModelTrait, ColumnTrait, ConnectOptions, ConnectionTrait, Database, DatabaseConnection,
    EntityTrait, PaginatorTrait, QueryFilter, Set,
};
use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::Mutex;
use tracing::warn;

/// 初始化任务状态
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum InitTaskStatus {
    /// 正在运行
    Running,
    /// 已完成
    Completed,
    /// 失败
    Failed,
}

/// 全局初始化任务状态存储（内存存储，生产环境应改用 Redis）
static INIT_TASKS: std::sync::OnceLock<Arc<Mutex<HashMap<String, InitTaskStatus>>>> =
    std::sync::OnceLock::new();

/// 获取全局初始化任务状态存储
pub fn get_init_tasks() -> &'static Arc<Mutex<HashMap<String, InitTaskStatus>>> {
    INIT_TASKS.get_or_init(|| Arc::new(Mutex::new(HashMap::new())))
}

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
        // Use percent_encoding for url-encoding user/password/name. The host segment
        // of a postgres connection string lives in the URL "authority" position,
        // and its character set is already ASCII-safe (alphanumeric, '.', '-', ':',
        // '[', ']' for IPv6, '%' for already-encoded chars). Encoding '.' or any
        // alphabetic character in the host would break DNS / IP resolution, so we
        // pass the host through verbatim.
        use percent_encoding::{utf8_percent_encode, NON_ALPHANUMERIC};
        let encoded_username = utf8_percent_encode(&self.username, NON_ALPHANUMERIC).to_string();
        let encoded_password = utf8_percent_encode(&self.password, NON_ALPHANUMERIC).to_string();
        let encoded_name = utf8_percent_encode(&self.name, NON_ALPHANUMERIC).to_string();

        format!(
            "postgres://{}:{}@{}:{}/{}?sslmode=disable",
            encoded_username, encoded_password, self.host, self.port, encoded_name
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
        match user::Entity::find().count(self.db.as_ref()).await {
            Ok(count) => {
                if count > 0 {
                    (true, "系统已初始化".to_string())
                } else {
                    (false, "系统未初始化".to_string())
                }
            }
            Err(e) => {
                let err_msg = format!("{}", e);
                if err_msg.contains("does not exist") || err_msg.contains("relation") {
                    // 数据库表不存在，说明系统还未初始化
                    (false, "系统未初始化".to_string())
                } else {
                    (false, format!("检查初始化状态失败: {}", e))
                }
            }
        }
    }

    pub async fn test_database(config: &DatabaseConfig) -> Result<(), InitError> {
        let conn_str = config.to_connection_string();

        let mut opt = ConnectOptions::new(&conn_str);
        opt.max_connections(1)
            .min_connections(0)
            .connect_timeout(Duration::from_secs(3))
            .acquire_timeout(Duration::from_secs(3));

        match Database::connect(opt).await {
            Ok(db) => {
                let query_result = db
                    .query_one_raw(sea_orm::Statement::from_string(
                        sea_orm::DatabaseBackend::Postgres,
                        "SELECT 1 as test".to_string(),
                    ))
                    .await;

                // 测试查询结果
                let _ = query_result
                    .as_ref()
                    .map(|v| {
                        v.as_ref()
                            .map(|row| row.try_get::<i32>("", "test").unwrap_or(1))
                    })
                    .map(|opt| opt.unwrap_or(0));

                query_result
                    .map(|_| ())
                    .map_err(|e| InitError::DatabaseError(format!("数据库测试查询失败: {}", e)))
            }
            Err(e) => Err(InitError::DatabaseError(format!("数据库连接失败: {}", e))),
        }
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

        // Run migrations before creating roles
        self.run_migrations().await?;

        let password_hash = AuthService::hash_password(admin_password)
            .map_err(|e| InitError::HashError(e.to_string()))?;

        // 验证生成的密码哈希长度，确保符合预期
        if password_hash.len() > 512 {
            tracing::warn!("生成的密码哈希长度 {} 超过限制，可能存在问题", password_hash.len());
        }

        // 并行执行独立的初始化操作：创建默认角色和默认部门
        let (admin_role, department_id) = tokio::try_join!(
            self.create_default_roles(),
            self.create_default_departments()
        )?;

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

        // 添加重试机制
        let max_retries = 3;
        let mut last_error: Option<sea_orm::DbErr> = None;

        for attempt in 1..=max_retries {
            match Database::connect(opt.clone()).await {
                Ok(db) => {
                    let db = Arc::new(db);
                    let service = Self::new(db);
                    return service.initialize(admin_username, admin_password).await;
                }
                Err(e) => {
                    last_error = Some(e);
                    if attempt < max_retries {
                        // 等待一段时间后重试
                        tokio::time::sleep(std::time::Duration::from_secs(2 * attempt)).await;
                    }
                }
            }
        }

        Err(InitError::DatabaseError(format!(
            "数据库连接失败: {}",
            last_error
                .map(|e| e.to_string())
                .unwrap_or_else(|| "未知错误".to_string())
        )))
    }

    /// 异步初始化方法（非阻塞）
    ///
    /// 该方法会立即返回任务 ID，然后在后台执行剩余的数据库迁移和默认数据创建。
    /// 可以通过 `get_task_status` 查询任务状态。
    pub async fn initialize_with_db_async(
        db_config: &DatabaseConfig,
        admin_username: &str,
        admin_password: &str,
    ) -> Result<String, InitError> {
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
        let task_id = uuid::Uuid::new_v4().to_string();

        // 存储任务状态
        get_init_tasks()
            .lock()
            .await
            .insert(task_id.clone(), InitTaskStatus::Running);

        // 后台执行剩余迁移
        let db_clone = db.clone();
        let task_id_clone = task_id.clone();
        let admin_username = admin_username.to_string();
        let admin_password = admin_password.to_string();

        tokio::spawn(async move {
            use migration::{Migrator, MigratorTrait};

            // 执行剩余迁移（从第 6 个开始）
            if let Err(e) = Migrator::up(db_clone.as_ref(), None).await {
                tracing::error!("后台迁移失败: {}", e);
                get_init_tasks()
                    .lock()
                    .await
                    .insert(task_id_clone, InitTaskStatus::Failed);
                return;
            }

            // 创建默认数据
            let service = InitService::new(db_clone);
            if let Err(e) = service
                .initialize(&admin_username, &admin_password)
                .await
            {
                tracing::error!("创建默认数据失败: {}", e);
                get_init_tasks()
                    .lock()
                    .await
                    .insert(task_id_clone, InitTaskStatus::Failed);
                return;
            }

            get_init_tasks()
                .lock()
                .await
                .insert(task_id_clone, InitTaskStatus::Completed);
        });

        Ok(task_id)
    }

    async fn run_migrations(&self) -> Result<(), InitError> {
        use migration::{Migrator, MigratorTrait};
        use tracing::info;

        info!("开始执行数据库迁移...");
        Migrator::up(self.db.as_ref(), None)
            .await
            .map_err(|e| InitError::DatabaseError(format!("执行数据库迁移失败: {}", e)))?;

        info!("所有数据库迁移脚本执行完成");
        Ok(())
    }

    async fn create_default_roles(&self) -> Result<role::Model, InitError> {
        // 先检查admin角色是否已存在
        let existing_admin = role::Entity::find()
            .filter(role::Column::Code.eq("admin"))
            .one(self.db.as_ref())
            .await
            .map_err(|e| {
                let err_msg = format!("{}", e);
                if err_msg.contains("does not exist") || err_msg.contains("relation") {
                    InitError::DatabaseError("数据库表不存在，需要先初始化数据库".to_string())
                } else {
                    InitError::DatabaseError(format!("查询角色失败: {}", e))
                }
            })?;

        if let Some(admin_role) = existing_admin {
            return Ok(admin_role);
        }

        // 如果不存在，则创建角色
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

        // 创建其他角色
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

        if let Err(e) = role::Entity::insert_many(vec![manager_role, operator_role])
            .on_conflict(
                sea_orm::sea_query::OnConflict::column(role::Column::Code)
                    .do_nothing()
                    .to_owned(),
            )
            .exec(self.db.as_ref())
            .await
        {
            warn!("批量创建角色失败: {}, 可能部分已存在", e);
        }

        Ok(admin_role)
    }

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

        let dept_models: Vec<department::ActiveModel> = departments
            .into_iter()
            .map(|(name, code, sort)| department::ActiveModel {
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
            id: Set(0),
            username: Set(username.to_string()),
            password_hash: Set(password_hash.to_string()),
            email: Set(Some("admin@example.com".to_string())),
            phone: Set(None),
            role_id: Set(Some(role_id)),
            department_id: Set(Some(department_id)),
            is_active: Set(true),
            totp_secret: Set(None),
            is_totp_enabled: Set(false),
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

        let password_hash = AuthService::hash_password(new_password)
            .map_err(|e| InitError::HashError(e.to_string()))?;

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

impl From<InitError> for AppError {
    fn from(err: InitError) -> Self {
        match err {
            InitError::AlreadyInitialized => AppError::business("系统已经初始化"),
            InitError::HashError(e) => AppError::internal(format!("密码哈希错误: {}", e)),
            InitError::DatabaseError(e) => AppError::database(e),
            InitError::UserNotFound => AppError::not_found("用户不存在"),
            InitError::ConfigError(e) => AppError::bad_request(format!("配置错误: {}", e)),
        }
    }
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn to_connection_string_preserves_ip_host() {
        // 回退测试：确保 host 中合法的 IP 字符（数字、.）不会被错误编码
        let cfg = DatabaseConfig {
            host: "39.99.34.194".to_string(),
            port: "5432".to_string(),
            name: "bingxi".to_string(),
            username: "bingxi".to_string(),
            password: "p@ss word".to_string(),
        };
        let s = cfg.to_connection_string();
        // 关键断言：host 段不应被编码
        assert!(
            s.contains("@39.99.34.194:"),
            "host 不应被 percent-encoding，连接串 = {}",
            s
        );
        // 同时确保 username/password 仍然被正确编码
        assert!(
            s.starts_with("postgres://bingxi:p%40ss%20word@"),
            "s = {}",
            s
        );
        assert!(s.ends_with("/bingxi?sslmode=disable"));
    }

    #[test]
    fn to_connection_string_preserves_dns_host() {
        // DNS 主机名也必须原样保留
        let cfg = DatabaseConfig {
            host: "db.example.com".to_string(),
            port: "5432".to_string(),
            name: "bingxi".to_string(),
            username: "u".to_string(),
            password: "p".to_string(),
        };
        let s = cfg.to_connection_string();
        assert!(s.contains("@db.example.com:5432/"), "s = {}", s);
    }

    #[test]
    fn to_connection_string_preserves_ipv6_host() {
        // IPv6 主机名应保留方括号（注意：这里我们只做 verbatim 透传；
        // 真正使用 IPv6 时应额外处理）
        let cfg = DatabaseConfig {
            host: "[::1]".to_string(),
            port: "5432".to_string(),
            name: "bingxi".to_string(),
            username: "u".to_string(),
            password: "p".to_string(),
        };
        let s = cfg.to_connection_string();
        assert!(s.contains("@[::1]:5432/"), "s = {}", s);
    }
}
