// repository.rs - 数据库访问层
// 关键设计：
// 1. 所有 SQL 强制 WHERE tenant_id = $1（多租户隔离）
// 2. 使用 sqlx 异步查询
// 3. 错误统一为 anyhow::Error

use crate::model::{NewNotification, NotificationRow};
use anyhow::{Context, Result};
use chrono::Utc;
use sqlx::postgres::PgPoolOptions;
use sqlx::PgPool;

/// 创建数据库连接池
pub async fn create_pool(database_url: &str) -> Result<PgPool> {
    let pool = PgPoolOptions::new()
        .max_connections(10)
        .acquire_timeout(std::time::Duration::from_secs(5))
        .connect(database_url)
        .await
        .context("连接 PostgreSQL 失败")?;
    Ok(pool)
}

/// 插入单条通知
/// 多租户隔离：tenant_id 来自请求参数，SQL 中显式使用
pub async fn insert_notification(pool: &PgPool, notif: &NewNotification) -> Result<i64> {
    let row: (i64,) = sqlx::query_as(
        r#"
        INSERT INTO notification_messages
            (tenant_id, user_id, title, content, category, priority, is_read, created_at)
        VALUES ($1, $2, $3, $4, $5, $6, false, $7)
        RETURNING id
        "#,
    )
    .bind(notif.tenant_id)
    .bind(notif.user_id)
    .bind(&notif.title)
    .bind(&notif.content)
    .bind(&notif.category)
    .bind(notif.priority)
    .bind(Utc::now())
    .fetch_one(pool)
    .await
    .context("插入通知失败")?;
    Ok(row.0)
}

/// 批量插入通知
pub async fn batch_insert(pool: &PgPool, notifs: &[NewNotification]) -> Result<i64> {
    let mut count = 0i64;
    for notif in notifs {
        insert_notification(pool, notif).await?;
        count += 1;
    }
    Ok(count)
}

/// 列出用户通知（分页）
/// 多租户隔离：tenant_id + user_id 双条件
pub async fn list_by_user(
    pool: &PgPool,
    tenant_id: i64,
    user_id: i64,
    limit: i32,
    offset: i32,
) -> Result<Vec<NotificationRow>> {
    let rows = sqlx::query_as::<_, NotificationRow>(
        r#"
        SELECT id, tenant_id, user_id, title, content, category, priority, is_read, created_at
        FROM notification_messages
        WHERE tenant_id = $1 AND user_id = $2
        ORDER BY created_at DESC
        LIMIT $3 OFFSET $4
        "#,
    )
    .bind(tenant_id)
    .bind(user_id)
    .bind(limit as i64)
    .bind(offset as i64)
    .fetch_all(pool)
    .await
    .context("查询通知失败")?;
    Ok(rows)
}

/// 标记已读
/// 多租户隔离：tenant_id 必传，避免跨租户误标记
pub async fn mark_as_read(pool: &PgPool, id: i64, tenant_id: i64) -> Result<bool> {
    let result = sqlx::query(
        r#"
        UPDATE notification_messages
        SET is_read = true
        WHERE id = $1 AND tenant_id = $2
        "#,
    )
    .bind(id)
    .bind(tenant_id)
    .execute(pool)
    .await
    .context("标记已读失败")?;
    Ok(result.rows_affected() > 0)
}

/// 统计用户通知总数（多租户隔离）
pub async fn count_by_user(pool: &PgPool, tenant_id: i64, user_id: i64) -> Result<i64> {
    let row: (i64,) = sqlx::query_as(
        r#"
        SELECT COUNT(*)::BIGINT
        FROM notification_messages
        WHERE tenant_id = $1 AND user_id = $2
        "#,
    )
    .bind(tenant_id)
    .bind(user_id)
    .fetch_one(pool)
    .await
    .context("统计通知总数失败")?;
    Ok(row.0)
}
