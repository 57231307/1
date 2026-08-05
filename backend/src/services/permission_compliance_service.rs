//! 权限合规审查服务（V15 P1 缺陷 14.10-B / 14.10-C）
//!
//! 设计依据：审计报告 batch-12 P1-14.10-B / P1-14.10-C
//!
//! 实现要点：
//! - 定期扫描权限变更审计日志，识别 6 类异常权限分配行为（14.10-B）；
//! - 6 类异常模式：非工作时间变更 / 批量权限授予 / 超级权限授予 /
//!   互斥角色分配 / 离职用户权限未撤销 / 权限回滚；
//! - 定期合规审查机制：检测 is_system=true 非 admin 角色 / 互斥角色冲突 /
//!   离职用户权限未撤销等问题（14.10-C）；
//! - 检测到的异常记录为 audit_logs（severity=WARN/CRITICAL，resource_type=permission_compliance_alert）；
//! - 使用 tokio::spawn + tokio::time::interval 启动定时任务。

use std::sync::Arc;

use chrono::{Duration, Utc};
use sea_orm::{
    ColumnTrait, DatabaseConnection, EntityTrait, PaginatorTrait, QueryFilter, QueryOrder,
};
use tracing::{info, warn};

use crate::models::audit_log::{OperationType, Severity};
use crate::models::permission_change_audit::{self, Entity as PermissionChangeAuditEntity};
use crate::models::role::{self, Entity as RoleEntity};
use crate::models::role_permission::{self, Entity as RolePermissionEntity};
use crate::models::user::{self, Entity as UserEntity};
use crate::services::audit_log_service::{AuditEvent, AuditLogService};
use crate::utils::error::AppError;

/// 默认扫描间隔（秒）— 每 7 天扫描一次（合规审查周报）
const DEFAULT_INTERVAL_SECS: u64 = 604800;

/// 启动初始延迟（秒）— 避免与启动初始化争抢数据库连接
const INITIAL_DELAY_SECS: u64 = 300;

/// 非工作时间权限变更：22:00-06:00（UTC+8 业务时间）
const OFF_HOURS_START: u32 = 22;
const OFF_HOURS_END: u32 = 6;

/// 批量权限授予阈值：单次操作（同一 operator_id 在 1 小时内）授予 > 10 条权限
const BULK_GRANT_THRESHOLD: i64 = 10;
/// 批量授予检测窗口（秒）— 1 小时
const BULK_GRANT_WINDOW_SECS: i64 = 3600;

/// 超级权限授予检测：action == "*" 或 resource_type == "*"
fn is_super_permission(resource_type: &str, action: &str) -> bool {
    resource_type == "*" || action == "*"
}

/// 权限合规审查服务
pub struct PermissionComplianceService {
    db: Arc<DatabaseConnection>,
    audit_service: Arc<AuditLogService>,
}

/// 单次审查发现的异常项
#[derive(Debug, Clone)]
struct PermissionComplianceAlert {
    /// 异常类型
    alert_type: String,
    /// 相关操作人 ID
    operator_id: Option<i32>,
    /// 相关用户名
    username: Option<String>,
    /// 相关审计日志 ID
    audit_log_id: Option<i32>,
    /// 异常描述
    description: String,
    /// 严重级别（WARN / CRITICAL）
    severity: Severity,
}

impl PermissionComplianceService {
    /// 创建服务实例
    pub fn new(db: Arc<DatabaseConnection>, audit_service: Arc<AuditLogService>) -> Self {
        Self { db, audit_service }
    }

    /// V15 P1-14.10-B：异常权限分配识别 — 扫描指定时间范围的权限变更日志，识别 6 类异常行为（返回检测到的异常项数量。）
    pub async fn detect_anomalous_permission_assignments(
        &self,
        start: chrono::DateTime<Utc>,
        end: chrono::DateTime<Utc>,
    ) -> Result<usize, AppError> {
        info!(
            start = %start,
            end = %end,
            "权限合规审查：扫描 {} 到 {} 的权限变更日志",
            start, end
        );

        let change_logs = PermissionChangeAuditEntity::find()
            .filter(permission_change_audit::Column::ChangedAt.gte(start))
            .filter(permission_change_audit::Column::ChangedAt.lt(end))
            .order_by_asc(permission_change_audit::Column::ChangedAt)
            .all(self.db.as_ref())
            .await?;

        let total = change_logs.len();
        info!(total, "权限合规审查：扫描到 {} 条权限变更日志", total);

        let mut alerts = Vec::new();

        // 规则 1：非工作时间权限变更检测（22:00-06:00 UTC+8）
        self.detect_off_hours_changes(&change_logs, &mut alerts);

        // 规则 2：批量权限授予检测（1 小时窗口内 > 10 次）
        self.detect_bulk_grants(&change_logs, &mut alerts);

        // 规则 3：超级权限授予检测（action == "*" 或 resource_type == "*"）
        self.detect_super_permission_grants(&change_logs, &mut alerts);

        // 规则 4：互斥角色分配检测（需查 users 表判断用户是否同时持有互斥角色）
        self.detect_conflicting_role_assignments(&change_logs, &mut alerts)
            .await;

        // 规则 5：离职用户权限未撤销检测
        self.detect_resigned_user_permissions(&change_logs, &mut alerts)
            .await;

        // 规则 6：权限回滚检测（短时间内移除刚授予的权限）
        self.detect_permission_rollback(&change_logs, &mut alerts);

        let alert_count = alerts.len();
        for alert in alerts {
            self.record_compliance_alert(&alert);
        }

        info!(
            total_scanned = total,
            alerts_found = alert_count,
            "权限合规审查：完成，扫描 {} 条记录，发现 {} 项异常",
            total,
            alert_count
        );

        Ok(alert_count)
    }

    /// V15 P1-14.10-C：定期合规审查 — 检测系统级权限配置问题
    /// 检查项：1. is_system=true 的非 admin 角色（违反 is_system 滥用治理）；2. 离职用户仍持有角色权限（user.is_active=false 但 role_permissions 存在）；3. 互斥权限共存（同一角色同时持有 create 和 approve）
    pub async fn periodic_compliance_review(&self) -> Result<usize, AppError> {
        info!("权限合规审查：开始定期合规审查（系统级配置检查）");
        let mut alerts = Vec::new();

        // 检查 1：is_system=true 的非 admin 角色
        self.detect_system_role_abuse(&mut alerts).await;

        // 检查 2：离职用户仍持有角色权限
        self.detect_inactive_user_permissions(&mut alerts).await;

        // 检查 3：互斥权限共存（create + approve）
        self.detect_sod_violations(&mut alerts).await;

        let alert_count = alerts.len();
        for alert in alerts {
            self.record_compliance_alert(&alert);
        }

        info!(
            alerts_found = alert_count,
            "权限合规审查：定期合规审查完成，发现 {} 项系统级问题", alert_count
        );

        Ok(alert_count)
    }

    /// 规则 1：非工作时间权限变更检测（22:00-06:00 UTC+8）
    fn detect_off_hours_changes(
        &self,
        logs: &[permission_change_audit::Model],
        alerts: &mut Vec<PermissionComplianceAlert>,
    ) {
        for log in logs {
            let business_hour = (log.changed_at + Duration::hours(8))
                .format("%H")
                .to_string();
            if let Ok(hour) = business_hour.parse::<u32>() {
                let is_off_hours = !(OFF_HOURS_END..OFF_HOURS_START).contains(&hour);
                if is_off_hours {
                    alerts.push(PermissionComplianceAlert {
                        alert_type: "off_hours_permission_change".to_string(),
                        operator_id: Some(log.operator_id),
                        username: None,
                        audit_log_id: Some(log.id),
                        description: format!(
                            "非工作时间权限变更告警：操作人 {} 在 {:02}:00（业务时间）执行 {} 变更",
                            log.operator_id, hour, log.change_type
                        ),
                        severity: Severity::Warn,
                    });
                }
            }
        }
    }

    /// 规则 2：批量权限授予检测（1 小时窗口内 > 10 次）
    fn detect_bulk_grants(
        &self,
        logs: &[permission_change_audit::Model],
        alerts: &mut Vec<PermissionComplianceAlert>,
    ) {
        use std::collections::HashMap;

        let mut operator_times: HashMap<i32, Vec<chrono::DateTime<Utc>>> = HashMap::new();
        for log in logs {
            if log.change_type == "role_permission_assign" {
                operator_times
                    .entry(log.operator_id)
                    .or_default()
                    .push(log.changed_at);
            }
        }

        for (operator_id, times) in operator_times {
            if times.len() as i64 <= BULK_GRANT_THRESHOLD {
                continue;
            }
            let mut sorted = times.clone();
            sorted.sort();
            let mut left = 0;
            for right in 0..sorted.len() {
                while sorted[right].timestamp() - sorted[left].timestamp() > BULK_GRANT_WINDOW_SECS
                {
                    left += 1;
                }
                let window_count = (right - left + 1) as i64;
                if window_count > BULK_GRANT_THRESHOLD {
                    alerts.push(PermissionComplianceAlert {
                        alert_type: "bulk_permission_grant".to_string(),
                        operator_id: Some(operator_id),
                        username: None,
                        audit_log_id: None,
                        description: format!(
                            "批量权限授予告警：操作人 {} 在 1 小时窗口内授予 {} 条权限（阈值 {}）",
                            operator_id, window_count, BULK_GRANT_THRESHOLD
                        ),
                        severity: Severity::Warn,
                    });
                    break;
                }
            }
        }
    }

    /// 规则 3：超级权限授予检测（action == "*" 或 resource_type == "*"）
    fn detect_super_permission_grants(
        &self,
        logs: &[permission_change_audit::Model],
        alerts: &mut Vec<PermissionComplianceAlert>,
    ) {
        for log in logs {
            if log.change_type != "role_permission_assign" {
                continue;
            }
            let resource_type = log.resource_type.as_deref().unwrap_or("");
            let action = log.action.as_deref().unwrap_or("");
            if is_super_permission(resource_type, action) {
                let is_grant = log.new_value.as_deref() == Some("true");
                if is_grant {
                    alerts.push(PermissionComplianceAlert {
                        alert_type: "super_permission_grant".to_string(),
                        operator_id: Some(log.operator_id),
                        username: None,
                        audit_log_id: Some(log.id),
                        description: format!(
                            "超级权限授予告警：操作人 {} 授予角色 {:?} 的 {}:{} 通配符权限",
                            log.operator_id, log.role_id, resource_type, action
                        ),
                        severity: Severity::Critical,
                    });
                }
            }
        }
    }

    /// 规则 4：互斥角色分配检测（同一用户被分配互斥角色组合）
    async fn detect_conflicting_role_assignments(
        &self,
        logs: &[permission_change_audit::Model],
        alerts: &mut Vec<PermissionComplianceAlert>,
    ) {
        use std::collections::HashSet;

        let mut checked: HashSet<i32> = HashSet::new();
        for log in logs {
            if log.change_type != "user_role_change" {
                continue;
            }
            let Some(target_user) = log.user_id else {
                continue;
            };
            if !checked.insert(target_user) {
                continue;
            }
            // 查询用户当前角色
            let user = UserEntity::find_by_id(target_user)
                .one(self.db.as_ref())
                .await
                .ok()
                .flatten();
            let Some(role_id) = user.as_ref().and_then(|u| u.role_id) else {
                continue;
            };
            let role = RoleEntity::find_by_id(role_id)
                .one(self.db.as_ref())
                .await
                .ok()
                .flatten();
            let Some(role) = role else {
                continue;
            };
            // 互斥规则：admin 与 auditor 互斥（审计员不应同时是管理员）
            // 财务会计与财务审核互斥（自审自批）
            let role_code = role.code.as_str();
            let is_conflict = matches!(
                role_code,
                "admin" | "auditor" | "finance_accountant" | "finance_reviewer"
            ) && self.has_conflicting_role(target_user, role_code).await;
            if is_conflict {
                alerts.push(PermissionComplianceAlert {
                    alert_type: "conflicting_role_assignment".to_string(),
                    operator_id: Some(log.operator_id),
                    username: user.as_ref().map(|u| u.username.clone()),
                    audit_log_id: Some(log.id),
                    description: format!(
                        "互斥角色分配告警：用户 {}(id={}) 被分配互斥角色 {}",
                        user.as_ref().map(|u| u.username.as_str()).unwrap_or("未知"),
                        target_user,
                        role_code
                    ),
                    severity: Severity::Critical,
                });
            }
        }
    }

    /// 检查用户是否持有与指定角色互斥的角色（简化版：检查角色 code 是否在互斥清单中）
    async fn has_conflicting_role(&self, _user_id: i32, role_code: &str) -> bool {
        // 互斥规则映射：admin ↔ auditor，finance_accountant ↔ finance_reviewer
        // 由于当前用户表只有 role_id 单字段（不支持多角色），同一用户不会同时持有两个角色。
        // 此函数为预留扩展点：未来支持 user_role 多角色表时实现完整互斥检测。
        // 当前实现：检查 role_code 是否属于互斥角色清单（触发告警提醒人工复核）
        matches!(role_code, "auditor" | "finance_reviewer")
    }

    /// 规则 5：离职用户权限未撤销检测（用户 is_active=false 但仍有权限变更记录）
    async fn detect_resigned_user_permissions(
        &self,
        logs: &[permission_change_audit::Model],
        alerts: &mut Vec<PermissionComplianceAlert>,
    ) {
        use std::collections::HashSet;

        let mut checked: HashSet<i32> = HashSet::new();
        for log in logs {
            let Some(target_user) = log.user_id else {
                continue;
            };
            if !checked.insert(target_user) {
                continue;
            }
            let user = UserEntity::find_by_id(target_user)
                .one(self.db.as_ref())
                .await
                .ok()
                .flatten();
            let is_resigned = match &user {
                None => true,
                Some(u) => !u.is_active,
            };
            if is_resigned {
                alerts.push(PermissionComplianceAlert {
                    alert_type: "resigned_user_permission".to_string(),
                    operator_id: Some(log.operator_id),
                    username: user.as_ref().map(|u| u.username.clone()),
                    audit_log_id: Some(log.id),
                    description: format!(
                        "离职用户权限告警：用户 {}(id={}) 状态为 {:?} 但仍有权限变更记录",
                        user.as_ref().map(|u| u.username.as_str()).unwrap_or("未知"),
                        target_user,
                        user.as_ref()
                            .map(|u| if u.is_active { "active" } else { "inactive" })
                            .unwrap_or("not_found")
                    ),
                    severity: Severity::Critical,
                });
            }
        }
    }

    /// 规则 6：权限回滚检测（短时间内移除刚授予的权限）
    fn detect_permission_rollback(
        &self,
        logs: &[permission_change_audit::Model],
        alerts: &mut Vec<PermissionComplianceAlert>,
    ) {
        // 检测模式：同一 operator 对同一 role+resource+action 先 assign 后 remove（或反之）
        // 时间窗口内（1 小时）反复变更视为回滚
        use std::collections::HashMap;

        let mut key_changes: HashMap<String, Vec<&permission_change_audit::Model>> = HashMap::new();
        for log in logs {
            if log.change_type != "role_permission_assign"
                && log.change_type != "role_permission_remove"
            {
                continue;
            }
            let key = format!(
                "{}:{}:{}",
                log.role_id.unwrap_or(0),
                log.resource_type.as_deref().unwrap_or(""),
                log.action.as_deref().unwrap_or("")
            );
            key_changes.entry(key).or_default().push(log);
        }

        for (_key, entries) in key_changes {
            if entries.len() < 2 {
                continue;
            }
            // 检查是否有 assign 和 remove 交替（回滚模式）
            let has_assign = entries
                .iter()
                .any(|e| e.change_type == "role_permission_assign");
            let has_remove = entries
                .iter()
                .any(|e| e.change_type == "role_permission_remove");
            if has_assign && has_remove {
                let first = entries.first().unwrap();
                let last = entries.last().unwrap();
                let duration = last.changed_at - first.changed_at;
                if duration.num_seconds() <= BULK_GRANT_WINDOW_SECS {
                    alerts.push(PermissionComplianceAlert {
                        alert_type: "permission_rollback".to_string(),
                        operator_id: Some(first.operator_id),
                        username: None,
                        audit_log_id: Some(last.id),
                        description: format!(
                            "权限回滚告警：操作人 {} 在 {} 秒内对角色 {:?} 的 {}:{} 先授予后移除（或反之）",
                            first.operator_id,
                            duration.num_seconds(),
                            first.role_id,
                            first.resource_type.as_deref().unwrap_or(""),
                            first.action.as_deref().unwrap_or("")
                        ),
                        severity: Severity::Warn,
                    });
                }
            }
        }
    }

    /// 合规检查 1：is_system=true 的非 admin 角色（违反 is_system 滥用治理）
    async fn detect_system_role_abuse(&self, alerts: &mut Vec<PermissionComplianceAlert>) {
        let roles = RoleEntity::find()
            .filter(role::Column::IsSystem.eq(true))
            .all(self.db.as_ref())
            .await
            .unwrap_or_default();

        for r in roles {
            if r.code != "admin" {
                alerts.push(PermissionComplianceAlert {
                    alert_type: "system_role_abuse".to_string(),
                    operator_id: None,
                    username: None,
                    audit_log_id: None,
                    description: format!(
                        "is_system 滥用告警：角色 {}(code={}, id={}) 的 is_system=true 但 code 不为 admin，违反 is_system 仅限 admin 角色原则",
                        r.name, r.code, r.id
                    ),
                    severity: Severity::Critical,
                });
            }
        }
    }

    /// 合规检查 2：离职用户仍持有角色权限
    async fn detect_inactive_user_permissions(&self, alerts: &mut Vec<PermissionComplianceAlert>) {
        let inactive_users = UserEntity::find()
            .filter(user::Column::IsActive.eq(false))
            .all(self.db.as_ref())
            .await
            .unwrap_or_default();

        for u in inactive_users {
            if let Some(role_id) = u.role_id {
                let perm_count = RolePermissionEntity::find()
                    .filter(role_permission::Column::RoleId.eq(role_id))
                    .filter(role_permission::Column::Allowed.eq(true))
                    .count(self.db.as_ref())
                    .await
                    .unwrap_or(0);
                if perm_count > 0 {
                    alerts.push(PermissionComplianceAlert {
                        alert_type: "inactive_user_has_permissions".to_string(),
                        operator_id: None,
                        username: Some(u.username.clone()),
                        audit_log_id: None,
                        description: format!(
                            "离职用户权限未撤销告警：用户 {}(id={}, role_id={}) 已停用但仍持有 {} 条有效权限",
                            u.username, u.id, role_id, perm_count
                        ),
                        severity: Severity::Critical,
                    });
                }
            }
        }
    }

    /// 合规检查 3：互斥权限共存（同一角色同时持有 create 和 approve）
    async fn detect_sod_violations(&self, alerts: &mut Vec<PermissionComplianceAlert>) {
        let roles = RoleEntity::find()
            .filter(role::Column::IsSystem.eq(false))
            .all(self.db.as_ref())
            .await
            .unwrap_or_default();

        for r in roles {
            let perms = RolePermissionEntity::find()
                .filter(role_permission::Column::RoleId.eq(r.id))
                .filter(role_permission::Column::Allowed.eq(true))
                .all(self.db.as_ref())
                .await
                .unwrap_or_default();

            // 按 resource_type 分组，检查 create + approve 是否共存
            use std::collections::HashMap;
            let mut resource_actions: HashMap<String, Vec<String>> = HashMap::new();
            for p in &perms {
                resource_actions
                    .entry(p.resource_type.clone())
                    .or_default()
                    .push(p.action.clone());
            }

            for (resource_type, actions) in resource_actions {
                let has_create = actions.iter().any(|a| a == "create" || a == "*");
                let has_approve = actions.iter().any(|a| a == "approve" || a == "*");
                if has_create && has_approve {
                    alerts.push(PermissionComplianceAlert {
                        alert_type: "sod_violation".to_string(),
                        operator_id: None,
                        username: None,
                        audit_log_id: None,
                        description: format!(
                            "SoD 职责分离违规告警：角色 {}(code={}, id={}) 同时持有 {} 的 create 和 approve 权限",
                            r.name, r.code, r.id, resource_type
                        ),
                        severity: Severity::Critical,
                    });
                }
            }
        }
    }

    /// 记录合规告警为审计日志（best-effort，异步不阻塞）
    fn record_compliance_alert(&self, alert: &PermissionComplianceAlert) {
        let event = AuditEvent {
            user_id: alert.operator_id,
            username: alert.username.clone(),
            operation_type: OperationType::Other,
            severity: alert.severity.clone(),
            resource_type: Some("permission_compliance_alert".to_string()),
            resource_id: alert.audit_log_id.map(|id| id.to_string()),
            resource_name: Some(alert.alert_type.clone()),
            description: Some(alert.description.clone()),
            request_method: None,
            request_path: None,
            before_snapshot: None,
            after_snapshot: Some(serde_json::json!({
                "alert_type": alert.alert_type,
                "severity": format!("{:?}", alert.severity),
                "source": "permission_compliance_review",
            })),
        };
        self.audit_service.clone().record_async(event, None);
    }

    /// 14.10-C：启动权限合规审查定时任务（受 CancellationToken 控制，支持 graceful shutdown）
    pub fn start_periodic_review(
        self: &Arc<Self>,
        token: tokio_util::sync::CancellationToken,
    ) -> tokio::task::JoinHandle<()> {
        let service = self.clone();
        tokio::spawn(async move {
            let enabled = std::env::var("PERMISSION_COMPLIANCE_CHECK_ENABLED")
                .unwrap_or_else(|_| "true".to_string());
            if enabled == "false" || enabled == "0" {
                info!("权限合规审查（14.10-C）：环境变量禁用，跳过启动");
                return;
            }

            let interval_secs = std::env::var("PERMISSION_COMPLIANCE_CHECK_INTERVAL_SECS")
                .ok()
                .and_then(|s| s.parse::<u64>().ok())
                .unwrap_or(DEFAULT_INTERVAL_SECS);

            tokio::time::sleep(std::time::Duration::from_secs(INITIAL_DELAY_SECS)).await;

            let interval = std::time::Duration::from_secs(interval_secs);
            info!(
                interval_secs,
                "权限合规审查（14.10-C）：定时任务已启动（每 {} 秒执行一次）", interval_secs
            );

            loop {
                tokio::select! {
                    _ = tokio::time::sleep(interval) => {
                        let now = Utc::now();
                        let start = now - Duration::days(7);

                        if let Err(e) = service
                            .detect_anomalous_permission_assignments(start, now)
                            .await
                        {
                            warn!(error = %e, "权限合规审查：异常权限分配识别失败，下次循环继续");
                        }

                        if let Err(e) = service.periodic_compliance_review().await {
                            warn!(error = %e, "权限合规审查：定期合规审查失败，下次循环继续");
                        }
                    }
                    _ = token.cancelled() => {
                        info!("权限合规审查（14.10-C）：收到取消信号，优雅退出");
                        break;
                    }
                }
            }
        })
    }
}
