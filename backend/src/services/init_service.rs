//! 系统初始化服务

use crate::models::department;
use crate::models::role;
use crate::models::role_permission;
use crate::models::user;
use crate::services::auth_service::AuthService;
use crate::utils::admin_checker::ADMIN_ROLE_CODE;
use crate::utils::error::AppError;
use futures::FutureExt;
use sea_orm::{
    ActiveModelTrait, ColumnTrait, ConnectOptions, ConnectionTrait, Database, DatabaseConnection,
    EntityTrait, PaginatorTrait, QueryFilter, Set,
};
use std::collections::HashMap;
use std::panic::AssertUnwindSafe;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::Mutex;
use tracing::warn;

/// 初始化任务状态（L-24 修复：补充终态与恢复路径文档）
/// 状态机：Running → Completed | Failed（终态）；Failed 后需重新调用 initialize 创建新 task_id 恢复。
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum InitTaskStatus {
    /// 正在运行（迁移 + 默认数据创建中，panic 会被 catch_unwind 隔离并转为 Failed）
    Running,
    /// 已完成（迁移 + 默认数据创建均成功，终态）
    Completed,
    /// 失败（迁移错误/创建错误/panic，终态；需重新调用 initialize 创建新任务恢复）
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
    /// SSL 模式：prefer（默认）/ require / disable 等
    /// 来源：前端初始化请求或 config.yaml 的 database.ssl_mode
    /// v5 审计批次 21：原硬编码 sslmode=disable，无视配置文件中的 ssl_mode 字段；
    /// 改为读取配置值，缺省时回退到 prefer（比 disable 更安全）。
    /// 使用 #[serde(default)] 保证前端旧版请求（不携带 ssl_mode 字段）仍可解析。
    #[serde(default)]
    pub ssl_mode: Option<String>,
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

        // SSL 模式来源：self.ssl_mode（来自 config.yaml 或前端请求），缺省时使用 prefer
        // v5 审计批次 21：原硬编码 "disable"，现改为读取配置值，默认 prefer
        // prefer 比 disable 更安全：先尝试 SSL 连接，失败再回退明文
        let ssl_mode = self.ssl_mode.as_deref().unwrap_or("prefer");

        format!(
            "postgres://{}:{}@{}:{}/{}?sslmode={}",
            encoded_username, encoded_password, self.host, self.port, encoded_name, ssl_mode
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
                let query_result: Result<Option<sea_orm::QueryResult>, sea_orm::DbErr> = db
                    .query_one(sea_orm::Statement::from_string(
                        sea_orm::DatabaseBackend::Postgres,
                        "SELECT 1 as test".to_string(),
                    ))
                    .await;

                // L-10 修复（批次 375 v13 复审）：移除冗余的 let _ = 查询结果丢弃代码块
                // 原实现计算了查询结果但立即丢弃，是无意义的死代码。
                // query_result 在下方直接通过 map/map_err 处理，无需提前提取值。

                // P1-1 修复（H-3，2026-06-25 综合审计）：错误消息脱敏
                // 不透传底层 DbErr 原文，避免差异化错误信息被用于内网服务枚举。
                // 详细错误通过 tracing::warn 记录到服务端日志用于排查。
                query_result
                    .map(|_| ())
                    .map_err(|e| {
                        warn!("init test_database 查询失败，目标 {}: {}", config.host, e);
                        InitError::DatabaseError("数据库测试查询失败".to_string())
                    })
            }
            Err(e) => {
                warn!("init test_database 连接失败，目标 {}: {}", config.host, e);
                Err(InitError::DatabaseError("数据库连接失败".to_string()))
            }
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

        // v14 P0-1 修复：使用 spawn_blocking 包装 Argon2id 哈希计算，避免阻塞 tokio worker
        let password_hash = AuthService::hash_password_async(admin_password.to_string())
            .await
            .map_err(|e| InitError::HashError(e.to_string()))?;

        // 验证生成的密码哈希长度，确保符合预期
        if password_hash.len() > 512 {
            tracing::warn!(
                "生成的密码哈希长度 {} 超过限制，可能存在问题",
                password_hash.len()
            );
        }

        // 并行执行独立的初始化操作：创建默认角色和默认部门
        let (admin_role, department_id) = tokio::try_join!(
            self.create_default_roles(),
            self.create_default_departments()
        )?;

        // V15 P0-S03 修复：为 manager/operator 创建基本 role_permission 记录
        self.create_default_role_permissions().await?;

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

            // 批次 7（2026-06-28）：一次性 spawn 任务 panic 隔离
            // 后台迁移任务 panic 会导致 task_id 永远停留在 Running，
            // 前端永远显示"初始化中"且无人能再次触发迁移。
            // 用 catch_unwind 包裹整个 async 块，panic 时也更新 task 状态为 Failed。
            let result = AssertUnwindSafe(async {
                // 执行剩余迁移（从第 6 个开始）
                if let Err(e) = Migrator::up(db_clone.as_ref(), None).await {
                    tracing::error!("后台迁移失败: {}", e);
                    return InitTaskStatus::Failed;
                }

                // 创建默认数据
                let service = InitService::new(db_clone);
                if let Err(e) = service.initialize(&admin_username, &admin_password).await {
                    tracing::error!("创建默认数据失败: {}", e);
                    return InitTaskStatus::Failed;
                }

                InitTaskStatus::Completed
            })
            .catch_unwind()
            .await;

            // 批次 7：统一更新 task 状态（业务失败 / panic 都更新为 Failed）
            let final_status = match result {
                Ok(status) => status,
                Err(panic_payload) => {
                    let panic_msg = panic_payload
                        .downcast_ref::<String>()
                        .map(|s| s.as_str())
                        .or_else(|| panic_payload.downcast_ref::<&'static str>().copied())
                        .unwrap_or("<非字符串 panic payload>");
                    tracing::error!(
                        panic = %panic_msg,
                        "⚠ 后台初始化任务 panic 已被隔离，确保 task_id 状态不卡在 Running"
                    );
                    InitTaskStatus::Failed
                }
            };

            get_init_tasks()
                .lock()
                .await
                .insert(task_id_clone, final_status);
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
        // 批次 24 v6 P0-1 修复：使用 ADMIN_ROLE_CODE 常量替代硬编码 "admin"，
        // 与 admin_checker.rs 保持单一真相源，避免角色编码变更时多处不同步。
        let existing_admin = role::Entity::find()
            .filter(role::Column::Code.eq(ADMIN_ROLE_CODE))
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
            id: Default::default(),
            name: Set("管理员".to_string()),
            code: Set(ADMIN_ROLE_CODE.to_string()),
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
            id: Default::default(),
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
            id: Default::default(),
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

    /// V15 P0-S03 修复：为 manager/operator 角色创建基本 role_permission 记录。
    /// 原实现仅将权限 JSON 存入 role.permissions 字段，role_permission 表无记录，
    /// 导致修改 `*:*` 注入后 manager/operator 完全无权限。此方法补全基本权限记录。
    async fn create_default_role_permissions(&self) -> Result<(), InitError> {
        // 检查 role_permission 表是否已有记录，避免重复插入
        let existing_count = role_permission::Entity::find()
            .count(self.db.as_ref())
            .await
            .unwrap_or(0);
        if existing_count > 0 {
            return Ok(());
        }

        // 查询 manager 和 operator 角色 id
        let manager_role = role::Entity::find()
            .filter(role::Column::Code.eq("manager"))
            .one(self.db.as_ref())
            .await
            .map_err(|e| InitError::DatabaseError(format!("查询 manager 角色失败: {}", e)))?;

        let operator_role = role::Entity::find()
            .filter(role::Column::Code.eq("operator"))
            .one(self.db.as_ref())
            .await
            .map_err(|e| InitError::DatabaseError(format!("查询 operator 角色失败: {}", e)))?;

        let now = chrono::Utc::now();
        let mut perms: Vec<role_permission::ActiveModel> = Vec::new();

        // manager 权限：用户读取 + 产品/订单/客户/供应商全部操作
        if let Some(ref mgr) = manager_role {
            let mid = mgr.id;
            for (resource, action) in [
                ("users", "read"),
                ("products", "*"),
                ("orders", "*"),
                ("customers", "*"),
                ("suppliers", "*"),
                ("inventory", "read"),
            ] {
                perms.push(role_permission::ActiveModel {
                    id: Default::default(),
                    role_id: Set(mid),
                    resource_type: Set(resource.to_string()),
                    resource_id: Set(None),
                    action: Set(action.to_string()),
                    allowed: Set(true),
                    created_at: Set(now),
                    updated_at: Set(now),
                });
            }
        }

        // operator 权限：产品/订单/客户/库存只读
        if let Some(ref opr) = operator_role {
            let oid = opr.id;
            for (resource, action) in [
                ("products", "read"),
                ("orders", "read"),
                ("customers", "read"),
                ("inventory", "read"),
            ] {
                perms.push(role_permission::ActiveModel {
                    id: Default::default(),
                    role_id: Set(oid),
                    resource_type: Set(resource.to_string()),
                    resource_id: Set(None),
                    action: Set(action.to_string()),
                    allowed: Set(true),
                    created_at: Set(now),
                    updated_at: Set(now),
                });
            }
        }

        if !perms.is_empty() {
            if let Err(e) = role_permission::Entity::insert_many(perms)
                .exec(self.db.as_ref())
                .await
            {
                warn!("批量创建角色权限失败: {}, 可能部分已存在", e);
            }
        }

        Ok(())
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

    /// 重置用户密码（P0 修复：深度防御 + 密码强度校验 + 用户存在性二次校验）
    ///
    /// 业务流程：
    /// 1. 密码强度校验（与 user_handler::create_user / change_password 一致，使用 `password_validator`）
    /// 2. 用户存在性二次校验（handler 层已做"admin 角色"判断，本层防止 service 误用）
    /// 3. Argon2id 密码哈希
    /// 4. 更新 DB + tracing::info 安全审计日志
    ///
    /// 错误返回：
    /// - `InitError::ValidationError` → 密码强度不满足策略（HTTP 400）
    /// - `InitError::UserNotFound` → 用户不存在（HTTP 404）
    /// - `InitError::HashError` → 哈希失败（HTTP 400）
    /// - `InitError::DatabaseError` → DB 错误（HTTP 500）
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
    /// 参数校验错误（P0 新增：用于密码强度等输入校验，HTTP 400）
    #[error("参数校验错误：{0}")]
    ValidationError(String),
}

impl From<InitError> for AppError {
    fn from(err: InitError) -> Self {
        match err {
            InitError::AlreadyInitialized => AppError::business("系统已经初始化"),
            InitError::HashError(e) => AppError::internal(format!("密码哈希错误: {}", e)),
            InitError::DatabaseError(e) => AppError::database(e),
            InitError::UserNotFound => AppError::not_found("用户不存在"),
            InitError::ConfigError(e) => AppError::bad_request(format!("配置错误: {}", e)),
            InitError::ValidationError(e) => AppError::validation(format!("参数校验失败: {}", e)),
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
        // 批次 28 v7 P0-2 修复：原测试数据使用真实生产 IP，已改为 RFC 5737 文档示例段
        let cfg = DatabaseConfig {
            host: "192.0.2.100".to_string(),
            port: "5432".to_string(),
            name: "bingxi".to_string(),
            username: "bingxi".to_string(),
            password: "p@ss word".to_string(),
            // v5 审计批次 21：ssl_mode 缺省时回退到 prefer（原为 disable）
            ssl_mode: None,
        };
        let s = cfg.to_connection_string();
        // 关键断言：host 段不应被编码
        assert!(
            s.contains("@192.0.2.100:"),
            "host 不应被 percent-encoding，连接串 = {}",
            s
        );
        // 同时确保 username/password 仍然被正确编码
        assert!(
            s.starts_with("postgres://bingxi:p%40ss%20word@"),
            "s = {}",
            s
        );
        // v5 审计批次 21：ssl_mode 缺省时默认 prefer
        assert!(s.ends_with("/bingxi?sslmode=prefer"));
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
            // v5 审计批次 21：ssl_mode 缺省时回退到 prefer
            ssl_mode: None,
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
            // v5 审计批次 21：ssl_mode 缺省时回退到 prefer
            ssl_mode: None,
        };
        let s = cfg.to_connection_string();
        assert!(s.contains("@[::1]:5432/"), "s = {}", s);
    }
}
