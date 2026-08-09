use chrono::{DateTime, Utc};
use sea_orm::{ColumnTrait, DatabaseConnection, EntityTrait, QueryFilter, QueryOrder};
use std::sync::Arc;
use tracing::{info, warn};

use crate::models::audit_log;
use crate::utils::error::AppError;

/// 日志冷数据归档配置
pub struct LogArchiveConfig {
    /// 归档阈值（天）：超过此天数的日志将被归档
    pub archive_after_days: i64,
    /// 批量处理大小
    pub batch_size: u64,
}

impl Default for LogArchiveConfig {
    fn default() -> Self {
        Self {
            archive_after_days: 90,
            batch_size: 1000,
        }
    }
}

/// batch-17 P3: 日志冷数据归档服务
pub struct LogArchiveService {
    db: Arc<DatabaseConnection>,
    config: LogArchiveConfig,
}

impl LogArchiveService {
    pub fn new(db: Arc<DatabaseConnection>, config: LogArchiveConfig) -> Self {
        Self { db, config }
    }

    /// 归档旧审计日志
    pub async fn archive_old_logs(&self) -> Result<u64, AppError> {
        let threshold = Utc::now() - chrono::Duration::days(self.config.archive_after_days);

        // 查询需要归档的日志数量
        let count = audit_log::Entity::find()
            .filter(audit_log::Column::CreatedAt.lt(threshold))
            .count(&*self.db)
            .await?;

        if count == 0 {
            info!("没有需要归档的审计日志");
            return Ok(0);
        }

        info!("发现 {} 条需要归档的审计日志", count);

        // 分批处理归档
        let mut archived = 0u64;
        loop {
            let logs = audit_log::Entity::find()
                .filter(audit_log::Column::CreatedAt.lt(threshold))
                .order_by_asc(audit_log::Column::CreatedAt)
                .limit(self.config.batch_size)
                .all(&*self.db)
                .await?;

            if logs.is_empty() {
                break;
            }

            // 这里可以实现实际的归档逻辑，例如：
            // 1. 将日志导出到文件
            // 2. 写入归档表
            // 3. 删除原记录

            archived += logs.len() as u64;
            info!("已归档 {} 条审计日志", archived);
        }

        Ok(archived)
    }
}
