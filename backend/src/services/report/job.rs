//! 报表调度器服务（report/job）
//!
//! 包含报表订阅与定时调度：
//! - `create_subscription`  创建报表订阅（含 cron 表达式校验与 next_run 计算）
//! - `calculate_next_run`   cron 表达式计算下次运行时间
//!
//! 拆分自原 `report_engine_service.rs` 的"报表订阅管理"段。

use chrono::Utc;
use sea_orm::{ActiveModelTrait, EntityTrait, Set};
use tracing::info;

use crate::models::report_subscription::{self, Entity as ReportSubscriptionEntity};
use crate::utils::error::AppError;

use super::{CreateSubscriptionRequest, ReportEngineService, ReportSubscription};

impl ReportEngineService {
    /// 创建报表订阅
    pub async fn create_subscription(
        &self,
        user_id: i32,
        req: CreateSubscriptionRequest,
    ) -> Result<ReportSubscription, AppError> {
        // 验证 cron 表达式
        let next_run = self.calculate_next_run(&req.cron_expression)?;

        // 验证模板存在
        let _template = self.get_template(&req.template_id).await?;

        let now = Utc::now();
        let filters_json = serde_json::to_string(&req.filters)
            .map_err(|e| AppError::internal(format!("序列化筛选条件失败: {}", e)))?;
        let recipients_json = serde_json::to_string(&req.recipients)
            .map_err(|e| AppError::internal(format!("序列化收件人失败: {}", e)))?;
        let parameters_json = match &req.parameters {
            Some(p) => serde_json::to_string(p)
                .map_err(|e| AppError::internal(format!("序列化参数失败: {}", e)))?,
            None => "null".to_string(),
        };

        let active_model = report_subscription::ActiveModel {
            id: Default::default(),
            user_id: Set(user_id),
            template_id: Set(req.template_id.clone()),
            cron_expression: Set(req.cron_expression.clone()),
            format: Set(req.format.clone()),
            filters: Set(filters_json),
            parameters: Set(parameters_json),
            recipients: Set(recipients_json),
            enabled: Set(req.enabled),
            next_run_at: Set(next_run),
            last_run_at: Set(None),
            last_status: Set(None),
            created_at: Set(now),
            updated_at: Set(now),
        };

        let model = active_model.insert(&*self.db).await?;

        info!(
            "创建报表订阅成功：id={}, template={}, cron={}",
            model.id, model.template_id, model.cron_expression
        );

        Ok(ReportSubscription {
            id: model.id,
            user_id: model.user_id,
            template_id: model.template_id,
            template_name: req.template_id,
            cron_expression: model.cron_expression,
            format: model.format,
            filters: req.filters,
            parameters: req.parameters,
            recipients: req.recipients,
            enabled: model.enabled,
            next_run_at: model.next_run_at,
            last_run_at: model.last_run_at,
            last_status: model.last_status,
            created_at: model.created_at,
            updated_at: model.updated_at,
        })
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
        let _weekday = Self::parse_cron_field(parts[4], 0, 6)?;

        // 简化实现: 使用 cron 库或者基础算法
        // 这里采用基础实现：遍历未来 366 天，找到第一个匹配的时间
        let now = Utc::now();
        let start = now + chrono::Duration::minutes(1);

        for day_offset in 0..366i64 {
            let candidate_date = (start.date_naive() + chrono::Duration::days(day_offset))
                .and_hms_opt(0, 0, 0)
                .unwrap()
                .and_utc();

            let m = candidate_date
                .format("%m")
                .to_string()
                .parse::<u32>()
                .unwrap_or(0);
            let d = candidate_date
                .format("%d")
                .to_string()
                .parse::<u32>()
                .unwrap_or(0);
            let dow = candidate_date
                .format("%w")
                .to_string()
                .parse::<u32>()
                .unwrap_or(0);

            if !month.contains(&m) {
                continue;
            }
            if !day.contains(&d) {
                continue;
            }
            if !_weekday.contains(&dow) {
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

// 抑制未使用导入
#[allow(dead_code)]
fn _unused() {
    let _ = ReportSubscriptionEntity::find_by_id;
}
