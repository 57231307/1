//! 导出合规审查服务（V15 P1 缺陷 10-1 / 10-2）
//!
//! 设计依据：审计报告 batch-11 P1-10-1 / P1-10-2
//!
//! 实现要点：
//! - 每日定时扫描前一天的 print/export 审计日志，识别 6 类异常导出行为；
//! - 6 类异常模式：高频导出 / 大批量导出 / 非工作时间导出 / 离职用户导出 /
//!   跨权限导出 / 敏感数据无审批导出；
//! - 检测到的异常记录为 audit_logs（severity=WARN，resource_type=export_compliance_alert）；
//! - 使用 tokio::spawn + tokio::time::interval 启动定时任务。
//!
//! 环境变量门控：
//! - `EXPORT_COMPLIANCE_CHECK_ENABLED`（默认 "true"）— 设为 "false" / "0" 时跳过启动；
//! - `EXPORT_COMPLIANCE_CHECK_INTERVAL_SECS`（默认 86400=24h）— 扫描间隔。

use std::sync::Arc;

use chrono::{Duration, Utc};
use sea_orm::{ColumnTrait, DatabaseConnection, EntityTrait, QueryFilter, QueryOrder};
use tracing::{info, warn};

use crate::models::audit_log::{self, Entity as AuditLogEntity};
use crate::models::audit_log::{OperationType, Severity};
use crate::services::audit_log_service::{AuditEvent, AuditLogService};
use crate::utils::error::AppError;

/// 默认扫描间隔（秒）— 每 24 小时扫描一次
const DEFAULT_INTERVAL_SECS: u64 = 86400;

/// 启动初始延迟（秒）— 避免与启动初始化争抢数据库连接
const INITIAL_DELAY_SECS: u64 = 180;

/// 高频导出阈值：1 小时内 > 10 次导出
const HIGH_FREQUENCY_THRESHOLD: i64 = 10;
/// 高频导出检测窗口（秒）— 1 小时
const HIGH_FREQUENCY_WINDOW_SECS: i64 = 3600;

/// 大批量导出阈值：导出条数 > 上限的 80%（上限 10000，80% = 8000）
const LARGE_EXPORT_THRESHOLD: i32 = 8000;

/// 非工作时间导出：22:00-06:00（UTC+8 业务时间）
const OFF_HOURS_START: u32 = 22;
const OFF_HOURS_END: u32 = 6;

/// 敏感数据资源类型清单（需二级审批方可导出）
const SENSITIVE_RESOURCE_TYPES: &[&str] = &["dye_recipe", "lab_dip", "production_recipe"];

/// 导出合规审查服务
pub struct ExportComplianceService {
    db: Arc<DatabaseConnection>,
    audit_service: Arc<AuditLogService>,
}

/// 单次审查发现的异常项
#[derive(Debug, Clone)]
struct ComplianceAlert {
    /// 异常类型（high_frequency / large_export / off_hours / resigned_user / cross_permission / sensitive_no_approval）
    alert_type: String,
    /// 相关用户 ID
    user_id: Option<i32>,
    /// 相关用户名
    username: Option<String>,
    /// 相关审计日志 ID
    audit_log_id: Option<i32>,
    /// 异常描述
    description: String,
    /// 严重级别（WARN / CRITICAL）
    severity: Severity,
}

impl ExportComplianceService {
    /// 创建服务实例
    pub fn new(db: Arc<DatabaseConnection>, audit_service: Arc<AuditLogService>) -> Self {
        Self { db, audit_service }
    }

    /// 执行一次每日合规审查：扫描前一天所有 print/export 操作，识别异常行为
    ///
    /// 返回检测到的异常项数量。
    pub async fn daily_export_compliance_review(&self) -> Result<usize, AppError> {
        let now = Utc::now();
        let start = now - Duration::days(1);
        let end = now;

        info!(
            start = %start,
            end = %end,
            "导出合规审查：开始扫描 {} 到 {} 的 print/export 操作",
            start, end
        );

        // 查询前一天所有 EXPORT/PRINT 操作的审计日志
        let export_logs = AuditLogEntity::find()
            .filter(audit_log::Column::CreatedAt.gte(start))
            .filter(audit_log::Column::CreatedAt.lt(end))
            .filter(
                audit_log::Column::OperationType
                    .eq(OperationType::Export.as_str())
                    .or(audit_log::Column::OperationType.eq(OperationType::Print.as_str())),
            )
            .order_by_asc(audit_log::Column::CreatedAt)
            .all(self.db.as_ref())
            .await?;

        let total = export_logs.len();
        info!(
            total,
            "导出合规审查：扫描到 {} 条 print/export 审计日志", total
        );

        if total == 0 {
            return Ok(0);
        }

        let mut alerts = Vec::new();

        // 规则 1：高频导出检测（1 小时窗口内 > 10 次）
        self.detect_high_frequency_exports(&export_logs, &mut alerts);

        // 规则 2：大批量导出检测（导出条数 > 上限 80%）
        self.detect_large_exports(&export_logs, &mut alerts);

        // 规则 3：非工作时间导出检测（22:00-06:00）
        self.detect_off_hours_exports(&export_logs, &mut alerts);

        // 规则 4：离职用户导出检测（user_id 不在 users 表中或 status=inactive）
        self.detect_resigned_user_exports(&export_logs, &mut alerts)
            .await;

        // 规则 5：跨权限导出检测（非 admin 用户导出敏感资源）
        self.detect_cross_permission_exports(&export_logs, &mut alerts)
            .await;

        // 规则 6：敏感数据无审批导出检测
        self.detect_sensitive_no_approval_exports(&export_logs, &mut alerts);

        // 记录所有检测到的异常为审计日志
        let alert_count = alerts.len();
        for alert in alerts {
            self.record_compliance_alert(&alert);
        }

        info!(
            total_scanned = total,
            alerts_found = alert_count,
            "导出合规审查：完成，扫描 {} 条记录，发现 {} 项异常",
            total,
            alert_count
        );

        Ok(alert_count)
    }

    /// 规则 1：高频导出检测（1 小时窗口内 > 10 次）
    ///
    /// 使用滑动窗口统计每用户的导出频率，超过阈值则告警。
    fn detect_high_frequency_exports(
        &self,
        logs: &[audit_log::Model],
        alerts: &mut Vec<ComplianceAlert>,
    ) {
        use std::collections::HashMap;

        // 按用户分组记录时间戳
        let mut user_export_times: HashMap<i32, Vec<chrono::DateTime<Utc>>> = HashMap::new();
        for log in logs {
            if let (Some(uid), Some(created)) = (log.user_id, log.created_at) {
                user_export_times.entry(uid).or_default().push(created);
            }
        }

        for (user_id, times) in user_export_times {
            if times.len() as i64 <= HIGH_FREQUENCY_THRESHOLD {
                continue;
            }
            // 排序后滑动窗口检测
            let mut sorted = times.clone();
            sorted.sort();
            let mut left = 0;
            for right in 0..sorted.len() {
                while sorted[right].timestamp() - sorted[left].timestamp()
                    > HIGH_FREQUENCY_WINDOW_SECS
                {
                    left += 1;
                }
                let window_count = (right - left + 1) as i64;
                if window_count > HIGH_FREQUENCY_THRESHOLD {
                    let username = logs
                        .iter()
                        .find(|l| l.user_id == Some(user_id))
                        .and_then(|l| l.username.clone());
                    alerts.push(ComplianceAlert {
                        alert_type: "high_frequency".to_string(),
                        user_id: Some(user_id),
                        username: username.clone(),
                        audit_log_id: None,
                        description: format!(
                            "高频导出告警：用户 {:?}({}) 在 1 小时窗口内导出 {} 次（阈值 {}）",
                            username, user_id, window_count, HIGH_FREQUENCY_THRESHOLD
                        ),
                        severity: Severity::Warn,
                    });
                    break;
                }
            }
        }
    }

    /// 规则 2：大批量导出检测（导出条数 > 上限 80%）
    fn detect_large_exports(&self, logs: &[audit_log::Model], alerts: &mut Vec<ComplianceAlert>) {
        for log in logs {
            if let Some(count) = log.export_record_count {
                if count > LARGE_EXPORT_THRESHOLD {
                    alerts.push(ComplianceAlert {
                        alert_type: "large_export".to_string(),
                        user_id: log.user_id,
                        username: log.username.clone(),
                        audit_log_id: Some(log.id),
                        description: format!(
                            "大批量导出告警：用户 {:?} 导出 {} 条记录（阈值 {}，占上限 {:.0}%）",
                            log.username,
                            count,
                            LARGE_EXPORT_THRESHOLD,
                            (count as f64 / 10000.0) * 100.0
                        ),
                        severity: Severity::Warn,
                    });
                }
            }
        }
    }

    /// 规则 3：非工作时间导出检测（22:00-06:00 UTC+8）
    fn detect_off_hours_exports(
        &self,
        logs: &[audit_log::Model],
        alerts: &mut Vec<ComplianceAlert>,
    ) {
        for log in logs {
            if let Some(created) = log.created_at {
                // 转换为 UTC+8 业务时间
                let business_hour = (created + Duration::hours(8)).format("%H").to_string();
                if let Ok(hour) = business_hour.parse::<u32>() {
                    // 非工作时间 = 22:00-06:00（跨午夜），即不在 06:00-22:00 范围内
                    let is_off_hours = !(OFF_HOURS_END..OFF_HOURS_START).contains(&hour);
                    if is_off_hours {
                        alerts.push(ComplianceAlert {
                            alert_type: "off_hours".to_string(),
                            user_id: log.user_id,
                            username: log.username.clone(),
                            audit_log_id: Some(log.id),
                            description: format!(
                                "非工作时间导出告警：用户 {:?} 在 {:02}:00（业务时间）执行导出操作",
                                log.username, hour
                            ),
                            severity: Severity::Warn,
                        });
                    }
                }
            }
        }
    }

    /// 规则 4：离职用户导出检测（用户状态为 inactive 或不存在）
    async fn detect_resigned_user_exports(
        &self,
        logs: &[audit_log::Model],
        alerts: &mut Vec<ComplianceAlert>,
    ) {
        use std::collections::HashSet;

        let mut checked: HashSet<i32> = HashSet::new();
        for log in logs {
            if let Some(uid) = log.user_id {
                if !checked.insert(uid) {
                    continue;
                }
                // 查询用户状态
                let user = crate::models::user::Entity::find_by_id(uid)
                    .one(self.db.as_ref())
                    .await
                    .ok()
                    .flatten();
                let is_resigned = match &user {
                    None => true,
                    Some(u) => !u.is_active,
                };
                if is_resigned {
                    alerts.push(ComplianceAlert {
                        alert_type: "resigned_user".to_string(),
                        user_id: Some(uid),
                        username: log.username.clone(),
                        audit_log_id: Some(log.id),
                        description: format!(
                            "离职用户导出告警：用户 {:?}(id={}) 状态为 {:?}，但仍有导出操作",
                            log.username,
                            uid,
                            user.as_ref()
                                .map(|u| if u.is_active { "active" } else { "inactive" })
                                .unwrap_or("not_found")
                        ),
                        severity: Severity::Critical,
                    });
                }
            }
        }
    }

    /// 规则 5：跨权限导出检测（非 admin 用户导出敏感资源）
    async fn detect_cross_permission_exports(
        &self,
        logs: &[audit_log::Model],
        alerts: &mut Vec<ComplianceAlert>,
    ) {
        use std::collections::HashSet;

        let mut checked: HashSet<i32> = HashSet::new();
        for log in logs {
            let is_sensitive = log
                .resource_type
                .as_ref()
                .map(|rt| SENSITIVE_RESOURCE_TYPES.contains(&rt.as_str()))
                .unwrap_or(false);
            if !is_sensitive {
                continue;
            }
            if let Some(uid) = log.user_id {
                if !checked.insert(uid) {
                    // 同一用户只告警一次
                    continue;
                }
                // 查询用户角色
                let user = crate::models::user::Entity::find_by_id(uid)
                    .one(self.db.as_ref())
                    .await
                    .ok()
                    .flatten();
                let role_id = user.as_ref().and_then(|u| u.role_id);
                let is_admin = if let Some(rid) = role_id {
                    crate::utils::admin_checker::is_admin_role(self.db.as_ref(), rid).await
                } else {
                    false
                };
                if !is_admin {
                    alerts.push(ComplianceAlert {
                        alert_type: "cross_permission".to_string(),
                        user_id: Some(uid),
                        username: log.username.clone(),
                        audit_log_id: Some(log.id),
                        description: format!(
                            "跨权限导出告警：非 admin 用户 {:?}(id={}) 导出敏感资源 {:?}",
                            log.username, uid, log.resource_type
                        ),
                        severity: Severity::Critical,
                    });
                }
            }
        }
    }

    /// 规则 6：敏感数据无审批导出检测（敏感资源导出无 approval_token）
    fn detect_sensitive_no_approval_exports(
        &self,
        logs: &[audit_log::Model],
        alerts: &mut Vec<ComplianceAlert>,
    ) {
        for log in logs {
            let is_sensitive = log
                .resource_type
                .as_ref()
                .map(|rt| SENSITIVE_RESOURCE_TYPES.contains(&rt.as_str()))
                .unwrap_or(false);
            if !is_sensitive {
                continue;
            }
            let has_approval = log
                .export_approval_token
                .as_ref()
                .map(|t| !t.is_empty())
                .unwrap_or(false);
            if !has_approval {
                alerts.push(ComplianceAlert {
                    alert_type: "sensitive_no_approval".to_string(),
                    user_id: log.user_id,
                    username: log.username.clone(),
                    audit_log_id: Some(log.id),
                    description: format!(
                        "敏感数据无审批导出告警：用户 {:?} 导出敏感资源 {:?} 但无审批 token",
                        log.username, log.resource_type
                    ),
                    severity: Severity::Critical,
                });
            }
        }
    }

    /// 记录合规告警为审计日志（best-effort，异步不阻塞）
    fn record_compliance_alert(&self, alert: &ComplianceAlert) {
        let event = AuditEvent {
            user_id: alert.user_id,
            username: alert.username.clone(),
            operation_type: OperationType::Other,
            severity: alert.severity.clone(),
            resource_type: Some("export_compliance_alert".to_string()),
            resource_id: alert.audit_log_id.map(|id| id.to_string()),
            resource_name: Some(alert.alert_type.clone()),
            description: Some(alert.description.clone()),
            request_method: None,
            request_path: None,
            before_snapshot: None,
            after_snapshot: Some(serde_json::json!({
                "alert_type": alert.alert_type,
                "severity": format!("{:?}", alert.severity),
                "source": "daily_export_compliance_review",
            })),
        };
        self.audit_service.clone().record_async(event, None);
    }

    /// 启动后台定时任务（每 24 小时执行一次合规审查）
    ///
    /// 返回 JoinHandle 供 shutdown 时 abort。
    pub fn start_background_task(self: &Arc<Self>) -> tokio::task::JoinHandle<()> {
        let service = self.clone();
        tokio::spawn(async move {
            // 读取环境变量门控
            let enabled = std::env::var("EXPORT_COMPLIANCE_CHECK_ENABLED")
                .unwrap_or_else(|_| "true".to_string());
            if enabled == "false" || enabled == "0" {
                info!("导出合规审查：环境变量禁用，跳过启动");
                return;
            }

            let interval_secs = std::env::var("EXPORT_COMPLIANCE_CHECK_INTERVAL_SECS")
                .ok()
                .and_then(|s| s.parse::<u64>().ok())
                .unwrap_or(DEFAULT_INTERVAL_SECS);

            // 启动初始延迟
            tokio::time::sleep(std::time::Duration::from_secs(INITIAL_DELAY_SECS)).await;

            let interval = std::time::Duration::from_secs(interval_secs);
            info!(
                interval_secs,
                "导出合规审查：后台任务已启动（每 {} 秒执行一次合规审查）", interval_secs
            );

            loop {
                match service.daily_export_compliance_review().await {
                    Ok(alert_count) if alert_count > 0 => {
                        warn!(
                            alert_count,
                            "导出合规审查：本轮发现 {} 项异常导出行为", alert_count
                        );
                    }
                    Ok(_) => {
                        info!("导出合规审查：本轮无异常");
                    }
                    Err(e) => {
                        warn!(error = %e, "导出合规审查：本轮扫描失败，下次循环继续");
                    }
                }
                tokio::time::sleep(interval).await;
            }
        })
    }
}
