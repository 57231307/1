//! 系统初始化服务

use crate::models::department;
use crate::models::role;
use crate::models::user;
use crate::services::auth_service::AuthService;
use sea_orm::{
    ActiveModelTrait, ColumnTrait, ConnectOptions, ConnectionTrait, Database, DatabaseConnection,
    EntityTrait, PaginatorTrait, QueryFilter, Set,
};
use std::sync::{Arc, RwLock};
use std::time::Duration;
use tracing::warn;
use once_cell::sync::Lazy;

#[derive(Debug, Clone, serde::Serialize)]
pub struct InitProgress {
    pub status: String, // "idle", "running", "completed", "failed"
    pub progress: u32,
    pub message: String,
    pub error: Option<String>,
}

pub static INIT_PROGRESS: Lazy<RwLock<InitProgress>> = Lazy::new(|| {
    RwLock::new(InitProgress {
        status: "idle".to_string(),
        progress: 0,
        message: "".to_string(),
        error: None,
    })
});

pub fn update_init_progress(status: &str, progress: u32, message: &str, error: Option<String>) {
    if let Ok(mut state) = INIT_PROGRESS.write() {
        state.status = status.to_string();
        if progress > 0 || status == "completed" {
            state.progress = progress;
        }
        state.message = message.to_string();
        if error.is_some() {
            state.error = error;
        }
    }
}
fn split_sql_statements(sql: &str) -> Vec<String> {
    let mut statements = Vec::new();
    let mut current_stmt = String::new();

    let mut in_string = false;
    let mut in_dollar_quote = false;
    let mut dollar_tag = String::new();

    let chars: Vec<char> = sql.chars().collect();
    let mut i = 0;

    while i < chars.len() {
        let c = chars[i];

        // 处理注释
        if !in_string && !in_dollar_quote {
            // 单行注释 --
            if c == '-' && i + 1 < chars.len() && chars[i + 1] == '-' {
                while i < chars.len() && chars[i] != '\n' {
                    current_stmt.push(chars[i]);
                    i += 1;
                }
                if i < chars.len() {
                    current_stmt.push(chars[i]);
                    i += 1;
                }
                continue;
            }
            // 多行注释 /* */
            if c == '/' && i + 1 < chars.len() && chars[i + 1] == '*' {
                current_stmt.push('/');
                current_stmt.push('*');
                i += 2;
                let mut depth = 1;
                while i < chars.len() && depth > 0 {
                    if chars[i] == '/' && i + 1 < chars.len() && chars[i + 1] == '*' {
                        depth += 1;
                        current_stmt.push('/');
                        current_stmt.push('*');
                        i += 2;
                        continue;
                    }
                    if chars[i] == '*' && i + 1 < chars.len() && chars[i + 1] == '/' {
                        depth -= 1;
                        current_stmt.push('*');
                        current_stmt.push('/');
                        i += 2;
                        continue;
                    }
                    current_stmt.push(chars[i]);
                    i += 1;
                }
                continue;
            }
        }

        current_stmt.push(c);

        if in_string {
            if c == '\'' {
                // 检查是否为连续的单引号（转义）
                if i + 1 < chars.len() && chars[i + 1] == '\'' {
                    current_stmt.push('\'');
                    i += 1;
                } else {
                    in_string = false;
                }
            }
        } else if in_dollar_quote {
            if c == '$' {
                // 检查是否匹配当前 dollar tag
                let mut match_tag = true;
                let tag_len = dollar_tag.len();
                if i + tag_len <= chars.len() {
                    for j in 0..tag_len {
                        if chars[i + j] != dollar_tag.chars().nth(j).unwrap() {
                            match_tag = false;
                            break;
                        }
                    }
                    if match_tag {
                        for _ in 1..tag_len {
                            i += 1;
                            current_stmt.push(chars[i]);
                        }
                        in_dollar_quote = false;
                    }
                }
            }
        } else {
            if c == '\'' {
                in_string = true;
            } else if c == '$' {
                // 尝试提取 dollar tag，例如 $$ 或者 $tag$
                let mut j = i + 1;
                let mut tag = String::from("$");
                let mut is_valid = false;
                while j < chars.len() {
                    let next_c = chars[j];
                    if next_c == '$' {
                        tag.push('$');
                        is_valid = true;
                        break;
                    } else if next_c.is_alphanumeric() || next_c == '_' {
                        tag.push(next_c);
                    } else {
                        break;
                    }
                    j += 1;
                }
                if is_valid {
                    in_dollar_quote = true;
                    dollar_tag = tag.clone();
                    for _ in 1..tag.len() {
                        i += 1;
                        current_stmt.push(chars[i]);
                    }
                }
            } else if c == ';' {
                statements.push(current_stmt.clone());
                current_stmt.clear();
            }
        }

        i += 1;
    }

    let final_stmt = current_stmt.trim();
    if !final_stmt.is_empty() {
        statements.push(final_stmt.to_string());
    }

    statements
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
        let encoded_username = urlencoding::encode(&self.username);
        let encoded_password = urlencoding::encode(&self.password);
        let encoded_host = urlencoding::encode(&self.host);
        let encoded_name = urlencoding::encode(&self.name);

        format!(
            "postgres://{}:{}@{}:{}/{}?sslmode=disable",
            encoded_username, encoded_password, encoded_host, self.port, encoded_name
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
            Err(e) => (false, format!("检查初始化状态失败: {}", e)),
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
                    .query_one(sea_orm::Statement::from_string(
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

                return query_result
                    .map(|_| ())
                    .map_err(|e| InitError::DatabaseError(format!("数据库测试查询失败: {}", e)));
            }
            Err(e) => {
                return Err(InitError::DatabaseError(format!("数据库连接失败: {}", e)));
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

        let service = self.clone();
        let username = admin_username.to_string();
        let password = admin_password.to_string();

        update_init_progress("running", 0, "开始初始化...", None);

        tokio::spawn(async move {
            let init_result = service.do_initialize(&username, &password).await;
            
            // 显式关闭数据库连接，避免因为异步上下文中的隐式 Drop 引发 block_on panic
            if let Ok(db_conn) = Arc::try_unwrap(service.db) {
                let _ = db_conn.close().await;
            }

            if let Err(e) = init_result {
                update_init_progress("failed", 0, "初始化失败", Some(e.to_string()));
            } else {
                update_init_progress("completed", 100, "初始化完成，请手动重启后端服务", None);
                
                // 3秒后自动退出进程，以便应用新的数据库配置
                tokio::spawn(async {
                    tokio::time::sleep(std::time::Duration::from_secs(3)).await;
                    tracing::info!("系统初始化完成，正在退出以应用新配置...");
                    std::process::exit(0);
                });
            }
        });

        Ok(InitializationResult {
            success: true,
            message: "Started".to_string(),
            admin_username: admin_username.to_string(),
        })
    }

    async fn do_initialize(
        &self,
        admin_username: &str,
        admin_password: &str,
    ) -> Result<(), InitError> {
        // Run migrations before creating roles
        self.run_migrations().await?;

        update_init_progress("running", 95, "创建默认角色和用户...", None);

        // 使用 spawn_blocking 执行 CPU 密集型的 Hash 操作，防止阻塞 Tokio 调度器引发 panic
        let password = admin_password.to_string();
        let password_hash = tokio::task::spawn_blocking(move || {
            AuthService::hash_password(&password)
        })
        .await
        .map_err(|e| InitError::HashError(e.to_string()))?
        .map_err(|e| InitError::HashError(e.to_string()))?;

        let admin_role = self.create_default_roles().await?;
        let department_id = self.create_default_departments().await?;

        self.create_admin_user(admin_username, &password_hash, admin_role.id, department_id)
            .await?;

        Ok(())
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
                    
                    // 保存配置到 .env 文件
                    // 优先读取系统级配置，以便不丢失其他环境变量
                    let mut env_content = std::fs::read_to_string("/etc/bingxi/.env")
                        .unwrap_or_else(|_| std::fs::read_to_string(".env").unwrap_or_default());
                        
                    if env_content.contains("DATABASE__CONNECTION_STRING") {
                        let lines: Vec<&str> = env_content.lines().collect();
                        let mut new_lines = Vec::new();
                        for line in lines {
                            if line.starts_with("DATABASE__CONNECTION_STRING") {
                                new_lines.push(format!("DATABASE__CONNECTION_STRING=\"{}\"", conn_str));
                            } else {
                                new_lines.push(line.to_string());
                            }
                        }
                        env_content = new_lines.join("\n");
                    } else {
                        env_content.push_str(&format!("\nDATABASE__CONNECTION_STRING=\"{}\"\n", conn_str));
                    }
                    let _ = std::fs::write(".env", env_content.trim_start());
                    // 尝试同步更新系统级的环境配置（生产环境 systemd 部署所需）
                    let _ = std::fs::write("/etc/bingxi/.env", env_content.trim_start());

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
            last_error.unwrap()
        )))
    }

    async fn run_migrations(&self) -> Result<(), InitError> {
        use sea_orm::{ConnectionTrait, DatabaseBackend, Statement};
        use std::path::PathBuf;
        use tracing::{info, warn};

        // 1. 创建迁移记录表
        let create_migration_table_sql = r#"
            CREATE TABLE IF NOT EXISTS __schema_migrations (
                id SERIAL PRIMARY KEY,
                version VARCHAR(255) NOT NULL UNIQUE,
                applied_at TIMESTAMP WITH TIME ZONE DEFAULT CURRENT_TIMESTAMP
            );
        "#;
        self.db
            .execute(Statement::from_string(
                DatabaseBackend::Postgres,
                create_migration_table_sql.to_string(),
            ))
            .await
            .map_err(|e| InitError::DatabaseError(format!("创建迁移记录表失败: {}", e)))?;

        let possible_paths = [
            PathBuf::from("database/migration"),
            PathBuf::from("../database/migration"),
            PathBuf::from("/opt/bingxi-erp/database/migration"),
            PathBuf::from("/opt/bingxi/database/migration"),
        ];

        let mut migration_dir = None;
        for path in &possible_paths {
            if path.exists() && path.is_dir() {
                migration_dir = Some(path.clone());
                break;
            }
        }

        let migration_dir = match migration_dir {
            Some(dir) => dir,
            None => {
                warn!("未找到数据库迁移脚本目录，跳过自动建表");
                return Ok(());
            }
        };

        info!("找到迁移脚本目录: {:?}", migration_dir);
        let mut entries: Vec<_> = std::fs::read_dir(&migration_dir)
            .map_err(|e| InitError::DatabaseError(format!("读取迁移目录失败: {}", e)))?
            .filter_map(Result::ok)
            .collect();

        let mut sql_entries: Vec<_> = entries
            .into_iter()
            .filter(|e| e.path().extension().and_then(|s| s.to_str()) == Some("sql"))
            .collect();

        // 确保按文件名排序执行
        sql_entries.sort_by_key(|e| e.path());
        
        let total_scripts = sql_entries.len();

        for (i, entry) in sql_entries.into_iter().enumerate() {
            let path = entry.path();
            let file_name = path.file_name().unwrap().to_str().unwrap().to_string();

            // 更新进度 (10% ~ 90%)
            let progress = if total_scripts > 0 {
                10 + ((i as f32 / total_scripts as f32) * 80.0) as u32
            } else {
                90
            };
            update_init_progress("running", progress, &format!("Executing {}", file_name), None);

            // 2. 检查是否已经执行过该脚本
            let check_sql = format!(
                    "SELECT COUNT(*) as count FROM __schema_migrations WHERE version = '{}'",
                    file_name
                );
                let query_res = self.db
                    .query_one(Statement::from_string(
                        DatabaseBackend::Postgres,
                        check_sql,
                    ))
                    .await
                    .map_err(|e| InitError::DatabaseError(format!("检查迁移记录失败: {}", e)))?;

                if let Some(row) = query_res {
                    let count: i64 = row.try_get("", "count").unwrap_or(0);
                    if count > 0 {
                        info!("跳过已执行的迁移脚本: {}", file_name);
                        continue;
                    }
                }

                info!("准备执行数据库迁移脚本: {}", file_name);
                
                // 使用 spawn_blocking 读取文件，避免阻塞异步执行器
                let path_clone = path.clone();
                let sql = tokio::task::spawn_blocking(move || std::fs::read_to_string(&path_clone))
                    .await
                    .map_err(|e| InitError::DatabaseError(format!("等待文件读取任务失败: {}", e)))?
                    .map_err(|e| InitError::DatabaseError(format!("读取SQL文件失败 {:?}: {}", path, e)))?;

                // 跳过空的SQL文件
                if sql.trim().is_empty() {
                    continue;
                }

                let statements = split_sql_statements(&sql);
                let total_stmts = statements.len() as u32;

                for (stmt_idx, stmt) in statements.into_iter().enumerate() {
                    let stmt = stmt.trim();
                    if stmt.is_empty() {
                        continue;
                    }

                    // Update fine-grained progress based on statement execution
                    // For example, file progress ranges from current base progress to base + (90/total_files)
                    // If there's only 1 file (our consolidated schema), it ranges from 10 to 90.
                    let file_base_prog = if total_scripts > 0 { 10 + (i as u32 * 80 / total_scripts as u32) } else { 10 };
                    let file_prog_step = if total_scripts > 0 { 80 / total_scripts as u32 } else { 80 };
                    let stmt_prog = file_base_prog + (stmt_idx as u32 * file_prog_step / total_stmts.max(1));
                    
                    update_init_progress("running", stmt_prog, &format!("Executing {} ({}/{})", file_name, stmt_idx + 1, total_stmts), None);

                    self.db
                        .execute(Statement::from_string(
                            DatabaseBackend::Postgres,
                            stmt.to_string(),
                        ))
                        .await
                        .map_err(|e| {
                            InitError::DatabaseError(format!(
                                "执行SQL片段失败 {}: {}\n语句: {}",
                                file_name,
                                e,
                                stmt
                            ))
                        })?;
                }

                // 3. 记录已成功执行的迁移脚本
                let insert_record_sql = format!(
                    "INSERT INTO __schema_migrations (version) VALUES ('{}')",
                    file_name
                );
                self.db
                    .execute(Statement::from_string(
                        DatabaseBackend::Postgres,
                        insert_record_sql,
                    ))
                    .await
                    .map_err(|e| {
                        InitError::DatabaseError(format!(
                            "记录迁移状态失败 {}: {}",
                            file_name, e
                        ))
                    })?;
                
                info!("成功执行脚本: {}", file_name);
        }

        info!("所有数据库迁移脚本执行完成");
        Ok(())
    }

    async fn create_default_roles(&self) -> Result<role::Model, InitError> {
        // 先检查admin角色是否已存在
        let existing_admin = role::Entity::find()
            .filter(
                sea_orm::Condition::any()
                    .add(role::Column::Code.eq("admin"))
                    .add(role::Column::Name.eq("管理员"))
            )
            .one(self.db.as_ref())
            .await
            .map_err(|e| InitError::DatabaseError(format!("查询角色失败: {}", e)))?;

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

        // 创建部门经理角色
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
        // 尝试创建，如果失败则记录但不中断初始化
        if let Err(e) = manager_role.insert(self.db.as_ref()).await {
            warn!("创建部门经理角色失败: {}, 可能已存在", e);
        }

        // 创建操作员角色
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
        // 尝试创建，如果失败则记录但不中断初始化
        if let Err(e) = operator_role.insert(self.db.as_ref()).await {
            warn!("创建操作员角色失败: {}, 可能已存在", e);
        }

        Ok(admin_role)
    }

    async fn create_default_departments(&self) -> Result<i32, InitError> {
        // 先检查总经办是否已存在
        let existing_dept = department::Entity::find()
            .filter(
                sea_orm::Condition::any()
                    .add(department::Column::Code.eq("D001"))
                    .add(department::Column::Name.eq("总经办"))
            )
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

        for (name, code, sort) in departments {
            let dept_model = department::ActiveModel {
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
            };

            // 尝试创建，如果失败则记录但不中断初始化
            if let Err(e) = dept_model.insert(self.db.as_ref()).await {
                warn!(
                    "创建部门 {} ({}): {} 失败: {}, 可能已存在",
                    name, code, sort, e
                );
            }
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
            // 用户已存在，则更新其密码、角色和部门，确保初始化的密码生效
            let mut active_user: user::ActiveModel = user.into();
            active_user.password_hash = Set(password_hash.to_string());
            active_user.role_id = Set(Some(role_id));
            active_user.department_id = Set(Some(department_id));
            active_user.is_active = Set(true);
            return active_user
                .update(self.db.as_ref())
                .await
                .map_err(|e| InitError::DatabaseError(format!("更新管理员用户失败: {}", e)));
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
