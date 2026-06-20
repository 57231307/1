//! 数据库主备隔离实现
//!
//! 提供 FailoverDatabase，支持 PostgreSQL 主库 + PostgreSQL 备库自动切换。
//!
//! # 用法
//!
//! ```ignore
//! use crate::utils::failover::database::FailoverDatabase;
//! use crate::config::failover::DatabaseFailoverConfig;
//!
//! let config = DatabaseFailoverConfig { /* ... */ };
//! let failover = FailoverDatabase::new(primary_conn, backup_conn, config);
//! let result = failover.call().await;
//! ```

use std::sync::Arc;
use std::time::Duration;

use async_trait::async_trait;
use sea_orm::{ConnectionTrait, DatabaseConnection, DbBackend, DbErr, Statement};
use tracing::{info, warn};

use super::circuit_breaker::CircuitBreaker;
use super::{FailoverCall, FailoverError};
use crate::config::failover::DatabaseFailoverConfig;

/// 数据库主备结构体
pub struct FailoverDatabase {
    /// 主库连接
    primary: DatabaseConnection,
    /// 备库连接
    backup: DatabaseConnection,
    /// 熔断器
    circuit: Arc<CircuitBreaker>,
    /// 配置
    config: DatabaseFailoverConfig,
    /// 功能名（监控用）
    function_name: String,
}

impl FailoverDatabase {
    /// 创建数据库主备实例
    pub fn new(
        primary: DatabaseConnection,
        backup: DatabaseConnection,
        config: DatabaseFailoverConfig,
    ) -> Self {
        let circuit = Arc::new(CircuitBreaker::new(
            config.circuit_breaker_threshold,
            Duration::from_secs(config.circuit_breaker_duration_s),
        ));
        Self {
            primary,
            backup,
            circuit,
            config,
            function_name: "database".to_string(),
        }
    }

    /// 获取主库连接（供业务代码使用）
    pub fn primary(&self) -> &DatabaseConnection {
        &self.primary
    }

    /// 获取备库连接（供业务代码使用，仅在切换时使用）
    pub fn backup(&self) -> &DatabaseConnection {
        &self.backup
    }

    /// 获取熔断器引用
    pub fn circuit(&self) -> &Arc<CircuitBreaker> {
        &self.circuit
    }
}

#[async_trait]
impl FailoverCall<bool, DbErr> for FailoverDatabase {
    /// 主库 ping
    async fn primary_call(&self) -> Result<bool, DbErr> {
        ping(&self.primary).await
    }

    /// 备库 ping
    async fn backup_call(&self) -> Result<bool, DbErr> {
        ping(&self.backup).await
    }

    fn primary_timeout(&self) -> Duration {
        Duration::from_millis(self.config.primary_timeout_ms)
    }

    fn backup_timeout(&self) -> Duration {
        Duration::from_millis(self.config.backup_timeout_ms)
    }

    fn function_name(&self) -> &str {
        &self.function_name
    }

    fn circuit_breaker(&self) -> Option<&CircuitBreaker> {
        Some(self.circuit.as_ref())
    }
}

/// 执行 ping 查询
async fn ping(db: &DatabaseConnection) -> Result<bool, DbErr> {
    db.execute(Statement::from_string(
        DbBackend::Postgres,
        "SELECT 1".to_string(),
    ))
    .await?;
    Ok(true)
}

/// 健康检查任务：定期 ping 主库，更新 failover_status 表
pub async fn health_check_task(
    failover: Arc<FailoverDatabase>,
    db: DatabaseConnection,
) {
    use sea_orm::{ActiveModelTrait, Set};
    use crate::models::failover_status::{self, ActiveModel as FailoverStatusActive};

    let mut interval = tokio::time::interval(Duration::from_secs(10));
    loop {
        interval.tick().await;
        match failover.call().await {
            Ok(_) => {
                info!("[数据库主备] 健康检查通过");
                // 更新状态：当前为主调用
                update_status(&db, "database", "primary", failover.circuit().state().as_i64())
                    .await;
            }
            Err(FailoverError::BackupFailed(_)) | Err(FailoverError::BothTimeout) => {
                warn!("[数据库主备] 主备均不可用");
                update_status(&db, "database", "both_down", failover.circuit().state().as_i64())
                    .await;
            }
            Err(_) => {
                // 主调用失败，正在使用备用
                update_status(&db, "database", "backup", failover.circuit().state().as_i64())
                    .await;
            }
        }
    }
}

/// 更新 failover_status 表
async fn update_status(db: &DatabaseConnection, function_name: &str, state: &str, circuit_state: i64) {
    use sea_orm::{ActiveModelTrait, EntityTrait, QueryFilter, ColumnTrait, Set};
    use crate::models::failover_status::{self, ActiveModel as FailoverStatusActive};
    let now = chrono::Utc::now();
    if let Ok(Some(_existing)) = failover_status::Entity::find()
        .filter(failover_status::Column::FunctionName.eq(function_name))
        .one(db)
        .await
    {
        // 更新
        let active = FailoverStatusActive {
            function_name: Set(function_name.to_string()),
            current_state: Set(state.to_string()),
            circuit_state: Set(circuit_state_to_str(circuit_state).to_string()),
            last_success_at: Set(Some(now)),
            updated_at: Set(now),
            ..Default::default()
        };
        let _ = active.update(db).await;
    }
}

fn circuit_state_to_str(state: i64) -> &'static str {
    match state {
        1 => "open",
        2 => "half_open",
        _ => "closed",
    }
}
