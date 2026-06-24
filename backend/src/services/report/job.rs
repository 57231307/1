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

impl ReportEngineService {
    /// 根据 cron 表达式推断订阅频率
    #[allow(dead_code)] // TODO(tech-debt): 创建报表订阅接入后移除
    fn infer_frequency(cron: &str) -> String {
        let parts: Vec<&str> = cron.split_whitespace().collect();
        if parts.len() != 5 {
            return "DAILY".to_string();
        }
        // 简单判断：
        // - 每天: 0 0 * * *
        // - 每周: 0 0 * * 1
        // - 每月: 0 0 1 * *
        let day = parts[2];
        let month = parts[3];
        let weekday = parts[4];
        if day == "1" && month == "*" && weekday == "*" {
            "MONTHLY".to_string()
        } else if weekday != "*" {
            "WEEKLY".to_string()
        } else {
            "DAILY".to_string()
        }
    }

    /// 计算下次运行时间
    ///
    /// 支持标准 5 字段 cron 表达式（minute hour day-of-month month day-of-week）。
    /// 简化实现：精确计算到分钟级匹配的下一次执行时间。
    pub fn calculate_next_run(
        &self,
        cron_expression: &str,
    ) -> Result<Option<chrono::DateTime<Utc>>, AppError> {
        let parts: Vec<&str> = cron_expression.split_whitespace().collect();
        if parts.len() != 5 {
            return Err(AppError::bad_request(format!(
                "无效的cron表达式: {}（需5段：分 时 日 月 周）",
                cron_expression
            )));
        }

        let minute = Self::parse_cron_field(parts[0], 0, 59)?;
        let hour = Self::parse_cron_field(parts[1], 0, 23)?;
        let day = Self::parse_cron_field(parts[2], 1, 31)?;
        let month = Self::parse_cron_field(parts[3], 1, 12)?;
        let weekday = Self::parse_cron_field(parts[4], 0, 6)?;

        // 简化实现: 使用 cron 库或者基础算法
        // 这里采用基础实现：遍历未来 366 天，找到第一个匹配的时间
        let now = Utc::now();
        let start = now + chrono::Duration::minutes(1);

        for day_offset in 0..366i64 {
            let candidate_date = (start.date_naive() + chrono::Duration::days(day_offset))
                .and_hms_opt(0, 0, 0)
                .unwrap()
                .and_utc();

            // 解析时间字段（chrono 不会失败，0 仅为防御默认值）
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

            if !month.contains(&m) {
                continue;
            }
            if !day.contains(&d) {
                continue;
            }
            if !weekday.contains(&dow) {
                continue;
            }

            for h in 0..24 {
                if !hour.contains(&(h as u32)) {
                    continue;
                }
                for mn in 0..60 {
                    if !minute.contains(&(mn as u32)) {
                        continue;
                    }

                    let candidate = candidate_date
                        .date_naive()
                        .and_hms_opt(h as u32, mn as u32, 0)
                        .unwrap()
                        .and_utc();

                    if candidate > now {
                        return Ok(Some(candidate));
                    }
                }
            }
        }

        Err(AppError::business("无法计算下次运行时间".to_string()))
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
