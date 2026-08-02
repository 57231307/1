//! 色卡发放统计服务
//! V15 P2 类九 10.5-3：发放统计（每日 23:00 执行，生成色卡发放日报）
//!
//! 统计口径：以发放日期（issued_at 落在指定日期）为基数，统计各状态记录数：
//! - total_issued：当日发放记录总数
//! - total_received：当日归还（status='returned'）
//! - total_used：当日遗失/损坏（status='lost'/'damaged'，视为使用后损耗）
//! - total_expired：当日发放且已超期未归还（status='issued' 且 expected_return_date < 当日）
//! - total_cancelled：当日取消（status='cancelled'）

use crate::models::color_card_issue::{self, Entity as IssueEntity};
use crate::utils::error::AppError;
use sea_orm::*;
use serde::Serialize;
use std::sync::Arc;

/// 单日发放统计结果
#[derive(Debug, Clone, Serialize)]
pub struct DailyStats {
    pub date: chrono::NaiveDate,
    pub total_issued: i32,
    pub total_received: i32,
    pub total_used: i32,
    pub total_expired: i32,
    pub total_cancelled: i32,
}

/// 色卡发放统计服务
pub struct ColorCardIssueStatisticsService {
    db: Arc<DatabaseConnection>,
}

impl ColorCardIssueStatisticsService {
    pub fn new(db: Arc<DatabaseConnection>) -> Self {
        Self { db }
    }

    /// 生成日统计（按发放日期 issued_at 落在指定日期内统计）
    pub async fn generate_daily_stats(
        &self,
        date: chrono::NaiveDate,
    ) -> Result<DailyStats, AppError> {
        let start_dt = date
            .and_hms_opt(0, 0, 0)
            .ok_or_else(|| AppError::validation("日期不合法"))?;
        let start =
            chrono::DateTime::<chrono::Utc>::from_naive_utc_and_offset(start_dt, chrono::Utc);
        let end_dt = date
            .and_hms_opt(23, 59, 59)
            .ok_or_else(|| AppError::validation("日期不合法"))?;
        let end = chrono::DateTime::<chrono::Utc>::from_naive_utc_and_offset(end_dt, chrono::Utc);

        let rows: Vec<color_card_issue::Model> = IssueEntity::find()
            .filter(color_card_issue::Column::IsDeleted.eq(false))
            .filter(color_card_issue::Column::IssuedAt.gte(start))
            .filter(color_card_issue::Column::IssuedAt.lte(end))
            .all(&*self.db)
            .await?;

        let mut stats = DailyStats {
            date,
            total_issued: 0,
            total_received: 0,
            total_used: 0,
            total_expired: 0,
            total_cancelled: 0,
        };
        for row in rows {
            stats.total_issued += 1;
            match row.status.as_str() {
                "returned" => stats.total_received += 1,
                "lost" | "damaged" => stats.total_used += 1,
                "cancelled" => stats.total_cancelled += 1,
                "issued" => {
                    if row.expected_return_date.is_some_and(|d| d < date) {
                        stats.total_expired += 1;
                    }
                }
                _ => {}
            }
        }
        Ok(stats)
    }
}
