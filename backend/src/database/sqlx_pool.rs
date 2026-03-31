//! SQLx 数据库连接池模块
//!
//! 提供 PostgreSQL 数据库连接池的初始化和管理
//! 支持 SSL 加密连接

use crate::config::settings::AppSettings;
use sqlx::postgres::{PgConnectOptions, PgPool, PgPoolOptions};
use std::sync::Arc;
use tracing::info;

/// SQLx 数据库连接池包装
#[derive(Clone)]
pub struct SqlxDatabase {
    pool: Arc<PgPool>,
}

impl SqlxDatabase {
    /// 创建新的 SQLx 数据库连接池
    ///
    /// # 参数
    /// * `settings` - 应用配置
    ///
    /// # 返回
    /// * `Result<Self, sqlx::Error>` - 成功返回数据库连接池，失败返回错误
    pub async fn new(settings: &AppSettings) -> Result<Self, sqlx::Error> {
        info!("初始化 SQLx 数据库连接池...");

        let mut options = PgConnectOptions::new()
            .host(&settings.database.host)
            .port(settings.database.port)
            .database(&settings.database.name)
            .username(&settings.database.username)
            .password(&settings.database.password);

        // 配置 SSL 模式
        options = match settings.database.ssl_mode.as_str() {
            "disable" => options.ssl_mode(sqlx::postgres::PgSslMode::Disable),
            "prefer" => options.ssl_mode(sqlx::postgres::PgSslMode::Prefer),
            "require" => options.ssl_mode(sqlx::postgres::PgSslMode::Require),
            "verify-ca" => options.ssl_mode(sqlx::postgres::PgSslMode::VerifyCa),
            "verify-full" => options.ssl_mode(sqlx::postgres::PgSslMode::VerifyFull),
            _ => options.ssl_mode(sqlx::postgres::PgSslMode::Prefer),
        };

        // 配置 SSL CA 证书（如果提供）
        if let Some(ssl_ca) = &settings.database.ssl_ca {
            if !ssl_ca.is_empty() {
                options = options.ssl_root_cert(ssl_ca);
            }
        }

        info!(
            "数据库连接配置 - 主机: {}:{}, 数据库: {}, SSL模式: {}",
            settings.database.host,
            settings.database.port,
            settings.database.name,
            settings.database.ssl_mode
        );

        // 配置连接池选项
        let pool_options = PgPoolOptions::new()
            .max_connections(settings.database.max_connections)
            .min_connections(5)
            .idle_timeout(std::time::Duration::from_secs(300))
            .connect_timeout(std::time::Duration::from_secs(10))
            .acquire_timeout(std::time::Duration::from_secs(5));

        // 添加重试机制
        let max_retries = 3;
        let mut last_error: Option<sqlx::Error> = None;

        for attempt in 1..=max_retries {
            match pool_options.connect_with(options.clone()).await {
                Ok(pool) => {
                    info!("SQLx 数据库连接池初始化成功（尝试 {}）", attempt);
                    return Ok(Self {
                        pool: Arc::new(pool),
                    });
                }
                Err(e) => {
                    last_error = Some(e);
                    warn!("数据库连接尝试 {} 失败: {}", attempt, last_error.as_ref().unwrap());
                    if attempt < max_retries {
                        // 等待一段时间后重试
                        tokio::time::sleep(std::time::Duration::from_secs(2 * attempt)).await;
                    }
                }
            }
        }

        // 所有重试都失败
        Err(last_error.unwrap_or_else(|| sqlx::Error::Configuration("数据库连接失败".into())))
    }

    /// 获取数据库连接池引用
    pub fn pool(&self) -> &PgPool {
        &self.pool
    }

    /// 获取 Arc 包装的数据库连接池
    pub fn pool_arc(&self) -> Arc<PgPool> {
        self.pool.clone()
    }

    /// 测试数据库连接
    pub async fn test_connection(&self) -> Result<(), sqlx::Error> {
        sqlx::query("SELECT 1")
            .execute(self.pool())
            .await?;
        Ok(())
    }
}
