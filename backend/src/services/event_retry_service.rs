//! 事件重试与死信队列服务
//!
//! B-P1-7 修复（批次 384 v13 复审）：
//! 事件处理失败后采用指数退避重试，超过最大重试次数后入死信队列并告警。
//! 复用现有 EventIdempotencyService 进行幂等去重，避免重试导致重复副作用。

use crate::models::event_dead_letter;
use crate::utils::error::AppError;
use sea_orm::{
    ActiveModelTrait, ColumnTrait, DatabaseConnection, EntityTrait, PaginatorTrait, QueryFilter,
    QueryOrder, QuerySelect, Set,
};
use std::sync::Arc;
use std::time::Duration;

/// 最大重试次数（超过后入死信队列）
pub const MAX_RETRY_COUNT: i32 = 5;

/// 指数退避基础延迟（秒），实际延迟 = BASE_DELAY * 2^retry_count
const BASE_DELAY_SECS: u64 = 1;

/// 事件重试与死信服务
pub struct EventRetryService {
    db: Arc<DatabaseConnection>,
}

impl EventRetryService {
    pub fn new(db: Arc<DatabaseConnection>) -> Self {
        Self { db }
    }

    /// 处理事件失败：记录失败信息，超过最大重试次数后入死信队列
    ///
    /// 参数：
    /// - `event_type`: 事件类型（BusinessEvent 变体名）
    /// - `event_payload`: 事件 payload 的 JSON 序列化
    /// - `failure_reason`: 失败原因摘要
    /// - `full_error`: 完整错误信息
    /// - `retry_count`: 当前已重试次数（从 0 开始）
    ///
    /// 返回值：是否已入死信队列（true 表示已入死信，不再重试）
    pub async fn handle_failure(
        &self,
        event_type: &str,
        event_payload: serde_json::Value,
        failure_reason: &str,
        full_error: &str,
        retry_count: i32,
    ) -> Result<bool, AppError> {
        if retry_count >= MAX_RETRY_COUNT {
            // 超过最大重试次数，入死信队列
            self.write_to_dead_letter(
                event_type,
                event_payload,
                failure_reason,
                full_error,
                retry_count,
            )
            .await?;

            // 告警（日志级别 error，便于监控系统采集）
            tracing::error!(
                "事件已入死信队列：type={}, retry_count={}, reason={}",
                event_type,
                retry_count,
                failure_reason
            );
            Ok(true)
        } else {
            // 未超过最大重试次数，记录 warn 日志
            tracing::warn!(
                "事件处理失败（将重试）：type={}, retry_count={}/{}, reason={}",
                event_type,
                retry_count,
                MAX_RETRY_COUNT,
                failure_reason
            );
            Ok(false)
        }
    }

    /// 计算指数退避延迟
    ///
    /// 公式：delay = BASE_DELAY * 2^retry_count，上限 60 秒
    pub fn calculate_backoff_delay(retry_count: i32) -> Duration {
        let exponent = retry_count.max(0) as u32;
        let delay_secs = BASE_DELAY_SECS
            .saturating_mul(2u64.saturating_pow(exponent))
            .min(60);
        Duration::from_secs(delay_secs)
    }

    /// 将失败事件写入死信队列表
    async fn write_to_dead_letter(
        &self,
        event_type: &str,
        event_payload: serde_json::Value,
        failure_reason: &str,
        full_error: &str,
        retry_count: i32,
    ) -> Result<(), AppError> {
        let now = chrono::Utc::now();
        let dead_letter = event_dead_letter::ActiveModel {
            event_type: Set(event_type.to_string()),
            event_payload: Set(event_payload),
            failure_reason: Set(failure_reason.to_string()),
            last_error: Set(Some(full_error.to_string())),
            retry_count: Set(retry_count),
            max_retries: Set(MAX_RETRY_COUNT),
            status: Set(event_dead_letter::status::DEAD.to_string()),
            first_failed_at: Set(now),
            last_retry_at: Set(Some(now)),
            resolved_at: Set(None),
            resolved_by: Set(None),
            created_at: Set(now),
            updated_at: Set(now),
            ..Default::default()
        };
        dead_letter.insert(&*self.db).await?;
        Ok(())
    }

    /// 查询死信队列（分页）
    pub async fn list_dead_letters(
        &self,
        status_filter: Option<&str>,
        limit: u64,
    ) -> Result<Vec<event_dead_letter::Model>, AppError> {
        let mut query = event_dead_letter::Entity::find()
            .order_by_desc(event_dead_letter::Column::CreatedAt)
            .limit(limit);
        if let Some(status) = status_filter {
            query = query.filter(event_dead_letter::Column::Status.eq(status));
        }
        let result = query.all(&*self.db).await?;
        Ok(result)
    }

    /// 标记死信为已处理
    pub async fn resolve_dead_letter(
        &self,
        dead_letter_id: i32,
        resolved_by: i32,
    ) -> Result<event_dead_letter::Model, AppError> {
        let dead_letter = event_dead_letter::Entity::find_by_id(dead_letter_id)
            .one(&*self.db)
            .await?
            .ok_or_else(|| AppError::not_found(format!("死信记录 {}", dead_letter_id)))?;

        let mut active: event_dead_letter::ActiveModel = dead_letter.into();
        active.status = Set(event_dead_letter::status::RESOLVED.to_string());
        active.resolved_at = Set(Some(chrono::Utc::now()));
        active.resolved_by = Set(Some(resolved_by));
        active.updated_at = Set(chrono::Utc::now());
        let updated = active.update(&*self.db).await?;
        Ok(updated)
    }

    /// 统计死信队列数量（按状态）
    pub async fn count_by_status(
        &self,
        status_filter: Option<&str>,
    ) -> Result<u64, AppError> {
        let mut query = event_dead_letter::Entity::find();
        if let Some(status) = status_filter {
            query = query.filter(event_dead_letter::Column::Status.eq(status));
        }
        let count = query.count(&*self.db).await?;
        Ok(count)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn 测试指数退避延迟计算() {
        // retry_count=0: 1 * 2^0 = 1 秒
        assert_eq!(EventRetryService::calculate_backoff_delay(0), Duration::from_secs(1));
        // retry_count=1: 1 * 2^1 = 2 秒
        assert_eq!(EventRetryService::calculate_backoff_delay(1), Duration::from_secs(2));
        // retry_count=2: 1 * 2^2 = 4 秒
        assert_eq!(EventRetryService::calculate_backoff_delay(2), Duration::from_secs(4));
        // retry_count=3: 1 * 2^3 = 8 秒
        assert_eq!(EventRetryService::calculate_backoff_delay(3), Duration::from_secs(8));
        // retry_count=10: 上限 60 秒
        assert_eq!(EventRetryService::calculate_backoff_delay(10), Duration::from_secs(60));
        // 负数退化为 0
        assert_eq!(EventRetryService::calculate_backoff_delay(-1), Duration::from_secs(1));
    }

    #[test]
    fn 测试最大重试次数常量() {
        assert_eq!(MAX_RETRY_COUNT, 5);
    }
}
