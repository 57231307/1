//! 初始化入口与数据库连接子模块（init_service_ops/setup）
//!
//! 从原 `init_service.rs` 迁移 10 个方法：
//! - check_initialized / test_database / initialize / initialize_with_db / initialize_with_db_async
//! - connect_database / mark_task_running / spawn_background_init / resolve_final_status / run_migrations

use crate::models::user;
use crate::services::auth_service::AuthService;
use crate::services::init_service::{
    get_init_tasks, DatabaseConfig, InitError, InitService, InitTaskStatus, InitializationResult,
};
use futures::FutureExt;
use sea_orm::{
    ConnectOptions, ConnectionTrait, Database, DatabaseConnection, EntityTrait, PaginatorTrait,
};
use std::panic::AssertUnwindSafe;
use std::sync::Arc;
use std::time::Duration;
use tracing::{info, warn};

impl InitService {
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

        // V15 P0-S23 修复：初始化默认角色互斥规则（SoD 职责分离）
        self.create_default_role_conflicts().await?;

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

    /// 异步初始化方法（非阻塞），立即返回任务 ID，后台执行迁移与默认数据创建
    pub async fn initialize_with_db_async(
        db_config: &DatabaseConfig,
        admin_username: &str,
        admin_password: &str,
    ) -> Result<String, InitError> {
        Self::test_database(db_config).await?;
        let db = Self::connect_database(db_config).await?;
        let task_id = uuid::Uuid::new_v4().to_string();
        Self::mark_task_running(&task_id).await;
        Self::spawn_background_init(
            db,
            task_id.clone(),
            admin_username.to_string(),
            admin_password.to_string(),
        );
        Ok(task_id)
    }

    /// 构建 ConnectOptions 并连接数据库
    async fn connect_database(
        db_config: &DatabaseConfig,
    ) -> Result<Arc<DatabaseConnection>, InitError> {
        let conn_str = db_config.to_connection_string();
        let mut opt = ConnectOptions::new(&conn_str);
        opt.max_connections(10)
            .min_connections(1)
            .connect_timeout(Duration::from_secs(10))
            .acquire_timeout(Duration::from_secs(10));
        let db = Database::connect(opt)
            .await
            .map_err(|e| InitError::DatabaseError(format!("数据库连接失败: {}", e)))?;
        Ok(Arc::new(db))
    }

    /// 标记任务为 Running
    async fn mark_task_running(task_id: &str) {
        get_init_tasks()
            .lock()
            .await
            .insert(task_id.to_string(), InitTaskStatus::Running);
    }

    /// 后台 spawn 迁移与默认数据创建任务，panic 隔离并更新最终状态
    fn spawn_background_init(
        db: Arc<DatabaseConnection>,
        task_id: String,
        admin_username: String,
        admin_password: String,
    ) {
        let db_clone = db.clone();
        let task_id_clone = task_id.clone();
        tokio::spawn(async move {
            use migration::{Migrator, MigratorTrait};
            // 批次 7（2026-06-28）：一次性 spawn 任务 panic 隔离
            // 后台迁移任务 panic 会导致 task_id 永远停留在 Running，
            // 前端永远显示"初始化中"且无人能再次触发迁移。
            // 用 catch_unwind 包裹整个 async 块，panic 时也更新 task 状态为 Failed。
            let result = AssertUnwindSafe(async {
                if let Err(e) = Migrator::up(db_clone.as_ref(), None).await {
                    tracing::error!("后台迁移失败: {}", e);
                    return InitTaskStatus::Failed;
                }
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
            let final_status = Self::resolve_final_status(result);
            get_init_tasks()
                .lock()
                .await
                .insert(task_id_clone, final_status);
        });
    }

    /// 将 catch_unwind 结果归一为 InitTaskStatus（业务失败或 panic 均视为 Failed）
    fn resolve_final_status(
        result: Result<InitTaskStatus, Box<dyn std::any::Any + Send>>,
    ) -> InitTaskStatus {
        match result {
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
        }
    }

    async fn run_migrations(&self) -> Result<(), InitError> {
        use migration::{Migrator, MigratorTrait};

        info!("开始执行数据库迁移...");
        Migrator::up(self.db.as_ref(), None)
            .await
            .map_err(|e| InitError::DatabaseError(format!("执行数据库迁移失败: {}", e)))?;

        info!("所有数据库迁移脚本执行完成");
        Ok(())
    }
}
