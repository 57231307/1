//! 报表调度器服务（report/job）
//!
//! 包含报表订阅与定时调度：
//! - `create_subscription`  创建报表订阅（含 cron 表达式校验与 next_run 计算）
//! - `calculate_next_run`   cron 表达式计算下次运行时间
//!
//! 拆分自原 `report_engine_service.rs` 的"报表订阅管理"段。

use chrono::Utc;

use crate::utils::error::AppError;

use super::ReportEngineService;

/// cron 表达式 5 字段解析结果（分/时/日/月/周）
struct CronFields {
    minute: std::collections::HashSet<u32>,
    hour: std::collections::HashSet<u32>,
    day: std::collections::HashSet<u32>,
    month: std::collections::HashSet<u32>,
    weekday: std::collections::HashSet<u32>,
}

impl ReportEngineService {
    /// 计算下次运行时间（支持 5 字段 cron：分 时 日 月 周，遍历未来 366 天精确到分钟）
    pub fn calculate_next_run(
        &self,
        cron_expression: &str,
    ) -> Result<Option<chrono::DateTime<Utc>>, AppError> {
        let fields = Self::parse_cron_expression(cron_expression)?;

        let now = Utc::now();
        let start = now + chrono::Duration::minutes(1);

        // 遍历未来 366 天，找到第一个匹配的执行时间
        for day_offset in 0..366i64 {
            let candidate_date = (start.date_naive() + chrono::Duration::days(day_offset))
                // P1-3 修复：expect 改为 ok_or_else 返回 AppError，保持防御性
                .and_hms_opt(0, 0, 0)
                .ok_or_else(|| AppError::internal("cron 候选日期时分秒非法"))?
                .and_utc();

            if !Self::date_matches_cron(&candidate_date, &fields) {
                continue;
            }

            if let Some(candidate) = Self::find_next_time_in_day(candidate_date, now, &fields)? {
                return Ok(Some(candidate));
            }
        }

        Err(AppError::business("无法计算下次运行时间".to_string()))
    }

    /// 解析 cron 表达式为 5 字段集合（分 时 日 月 周）
    fn parse_cron_expression(cron_expression: &str) -> Result<CronFields, AppError> {
        let parts: Vec<&str> = cron_expression.split_whitespace().collect();
        if parts.len() != 5 {
            return Err(AppError::bad_request(format!(
                "无效的cron表达式: {}（需5段：分 时 日 月 周）",
                cron_expression
            )));
        }
        Ok(CronFields {
            minute: Self::parse_cron_field(parts[0], 0, 59)?,
            hour: Self::parse_cron_field(parts[1], 0, 23)?,
            day: Self::parse_cron_field(parts[2], 1, 31)?,
            month: Self::parse_cron_field(parts[3], 1, 12)?,
            weekday: Self::parse_cron_field(parts[4], 0, 6)?,
        })
    }

    /// 校验候选日期的月/日/周是否匹配 cron 字段
    fn date_matches_cron(candidate_date: &chrono::DateTime<Utc>, fields: &CronFields) -> bool {
        // chrono 解析不会失败，unwrap_or_default 仅为防御
        let m = candidate_date
            .format("%m")
            .to_string()
            .parse::<u32>()
            .unwrap_or_default();
        let d = candidate_date
            .format("%d")
            .to_string()
            .parse::<u32>()
            .unwrap_or_default();
        let dow = candidate_date
            .format("%w")
            .to_string()
            .parse::<u32>()
            .unwrap_or_default();
        fields.month.contains(&m) && fields.day.contains(&d) && fields.weekday.contains(&dow)
    }

    /// 在指定日期内查找第一个晚于 now 的匹配时间（小时/分钟级）
    fn find_next_time_in_day(
        candidate_date: chrono::DateTime<Utc>,
        now: chrono::DateTime<Utc>,
        fields: &CronFields,
    ) -> Result<Option<chrono::DateTime<Utc>>, AppError> {
        for h in 0..24 {
            if !fields.hour.contains(&(h as u32)) {
                continue;
            }
            for mn in 0..60 {
                if !fields.minute.contains(&(mn as u32)) {
                    continue;
                }
                // P1-3 修复：expect 改为 ok_or_else 返回 AppError，保持防御性
                let candidate = candidate_date
                    .date_naive()
                    .and_hms_opt(h as u32, mn as u32, 0)
                    .ok_or_else(|| AppError::internal("cron 候选时间时分秒非法"))?
                    .and_utc();
                if candidate > now {
                    return Ok(Some(candidate));
                }
            }
        }
        Ok(None)
    }

    /// 解析 cron 字段
    /// 支持：* / , - 语法
    fn parse_cron_field(
        field: &str,
        min: u32,
        max: u32,
    ) -> Result<std::collections::HashSet<u32>, AppError> {
        let mut values = std::collections::HashSet::new();

        if field == "*" {
            for v in min..=max {
                values.insert(v);
            }
            return Ok(values);
        }

        for part in field.split(',') {
            if part.contains('/') {
                // step: a/b
                let step_parts: Vec<&str> = part.split('/').collect();
                if step_parts.len() != 2 {
                    return Err(AppError::bad_request(format!("无效的cron步长: {}", part)));
                }
                let step: u32 = step_parts[1].parse().map_err(|_| {
                    AppError::bad_request(format!("无效的步长值: {}", step_parts[1]))
                })?;
                let range_start = if step_parts[0] == "*" {
                    min
                } else {
                    step_parts[0].parse().map_err(|_| {
                        AppError::bad_request(format!("无效的起始值: {}", step_parts[0]))
                    })?
                };
                let mut v = range_start;
                while v <= max {
                    values.insert(v);
                    v += step;
                }
            } else if part.contains('-') {
                // range: a-b
                let range_parts: Vec<&str> = part.split('-').collect();
                if range_parts.len() != 2 {
                    return Err(AppError::bad_request(format!("无效的cron范围: {}", part)));
                }
                let start: u32 = range_parts[0].parse().map_err(|_| {
                    AppError::bad_request(format!("无效的起始值: {}", range_parts[0]))
                })?;
                let end: u32 = range_parts[1].parse().map_err(|_| {
                    AppError::bad_request(format!("无效的结束值: {}", range_parts[1]))
                })?;
                if start > end || start < min || end > max {
                    return Err(AppError::bad_request(format!("无效的范围: {}", part)));
                }
                for v in start..=end {
                    values.insert(v);
                }
            } else {
                // single value
                let v: u32 = part
                    .parse()
                    .map_err(|_| AppError::bad_request(format!("无效的cron值: {}", part)))?;
                if v < min || v > max {
                    return Err(AppError::bad_request(format!("cron值超出范围: {}", v)));
                }
                values.insert(v);
            }
        }

        Ok(values)
    }
}
