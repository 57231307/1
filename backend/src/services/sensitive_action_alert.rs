#![allow(clippy::too_many_arguments)]

use serde::{Deserialize, Serialize};

/// 敏感操作类型
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum SensitiveAction {
    /// 删除操作
    Delete,
    /// 权限变更
    PermissionChange,
    /// 用户管理
    UserManagement,
    /// 角色管理
    RoleManagement,
    /// 系统配置
    SystemConfig,
    /// 数据导出
    DataExport,
    /// 批量操作
    BatchOperation,
    /// 登录失败
    LoginFailure,
    /// 密码变更
    PasswordChange,
    /// 资金操作
    FinancialOperation,
}

impl SensitiveAction {
    /// 获取告警级别
    pub fn alert_level(&self) -> AlertLevel {
        match self {
            SensitiveAction::Delete => AlertLevel::High,
            SensitiveAction::PermissionChange => AlertLevel::Critical,
            SensitiveAction::UserManagement => AlertLevel::High,
            SensitiveAction::RoleManagement => AlertLevel::Critical,
            SensitiveAction::SystemConfig => AlertLevel::Critical,
            SensitiveAction::DataExport => AlertLevel::Medium,
            SensitiveAction::BatchOperation => AlertLevel::Medium,
            SensitiveAction::LoginFailure => AlertLevel::Medium,
            SensitiveAction::PasswordChange => AlertLevel::High,
            SensitiveAction::FinancialOperation => AlertLevel::Critical,
        }
    }

    /// 获取操作描述
    pub fn description(&self) -> &str {
        match self {
            SensitiveAction::Delete => "删除操作",
            SensitiveAction::PermissionChange => "权限变更",
            SensitiveAction::UserManagement => "用户管理",
            SensitiveAction::RoleManagement => "角色管理",
            SensitiveAction::SystemConfig => "系统配置",
            SensitiveAction::DataExport => "数据导出",
            SensitiveAction::BatchOperation => "批量操作",
            SensitiveAction::LoginFailure => "登录失败",
            SensitiveAction::PasswordChange => "密码变更",
            SensitiveAction::FinancialOperation => "资金操作",
        }
    }
}

/// 告警级别
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum AlertLevel {
    Low,
    Medium,
    High,
    Critical,
}

impl AlertLevel {
    pub fn as_str(&self) -> &str {
        match self {
            AlertLevel::Low => "LOW",
            AlertLevel::Medium => "MEDIUM",
            AlertLevel::High => "HIGH",
            AlertLevel::Critical => "CRITICAL",
        }
    }
}

/// 敏感操作告警服务
pub struct SensitiveActionAlert;

impl SensitiveActionAlert {
    /// 检查操作是否敏感并记录告警
    pub fn check_and_alert(
        action: &str,
        resource_type: &str,
        user_id: i32,
        username: &str,
        ip_address: Option<&str>,
    ) -> Option<SensitiveAction> {
        let sensitive_action = Self::classify_action(action, resource_type);

        if let Some(ref sa) = sensitive_action {
            let level = sa.alert_level();
            let timestamp = chrono::Utc::now().format("%Y-%m-%d %H:%M:%S").to_string();

            // 记录告警日志
            match level {
                AlertLevel::Critical => {
                    tracing::error!(
                        "【安全告警-严重】{}! 用户: {}({}), IP: {}, 资源: {}, 操作: {}, 时间: {}",
                        sa.description(),
                        username,
                        user_id,
                        ip_address.unwrap_or("unknown"),
                        resource_type,
                        action,
                        timestamp
                    );
                }
                AlertLevel::High => {
                    tracing::warn!(
                        "【安全告警-高】{}! 用户: {}({}), IP: {}, 资源: {}, 操作: {}, 时间: {}",
                        sa.description(),
                        username,
                        user_id,
                        ip_address.unwrap_or("unknown"),
                        resource_type,
                        action,
                        timestamp
                    );
                }
                AlertLevel::Medium => {
                    tracing::info!(
                        "【安全提醒】{}! 用户: {}({}), IP: {}, 资源: {}, 操作: {}, 时间: {}",
                        sa.description(),
                        username,
                        user_id,
                        ip_address.unwrap_or("unknown"),
                        resource_type,
                        action,
                        timestamp
                    );
                }
                AlertLevel::Low => {
                    // 低级别不记录
                }
            }
        }

        sensitive_action
    }

    /// 根据操作和资源类型分类
    fn classify_action(action: &str, resource_type: &str) -> Option<SensitiveAction> {
        let action_lower = action.to_lowercase();
        let resource_lower = resource_type.to_lowercase();

        // 删除操作
        if action_lower.contains("delete") || action_lower.contains("remove") {
            return Some(SensitiveAction::Delete);
        }

        // 权限变更
        if (resource_lower.contains("permission") || resource_lower.contains("role"))
            && (action_lower.contains("update") || action_lower.contains("assign"))
        {
            return Some(SensitiveAction::PermissionChange);
        }

        // 用户管理
        if resource_lower.contains("user")
            && (action_lower.contains("create")
                || action_lower.contains("update")
                || action_lower.contains("delete"))
        {
            return Some(SensitiveAction::UserManagement);
        }

        // 角色管理
        if resource_lower.contains("role") {
            return Some(SensitiveAction::RoleManagement);
        }

        // 系统配置
        if resource_lower.contains("config") || resource_lower.contains("setting") {
            return Some(SensitiveAction::SystemConfig);
        }

        // 数据导出
        if action_lower.contains("export") || action_lower.contains("download") {
            return Some(SensitiveAction::DataExport);
        }

        // 批量操作
        if action_lower.contains("batch") || action_lower.contains("bulk") {
            return Some(SensitiveAction::BatchOperation);
        }

        // 密码变更
        if resource_lower.contains("password") || action_lower.contains("change_password") {
            return Some(SensitiveAction::PasswordChange);
        }

        // 资金操作
        if (resource_lower.contains("payment")
            || resource_lower.contains("invoice")
            || resource_lower.contains("voucher")
            || resource_lower.contains("fund"))
            && (action_lower.contains("approve") || action_lower.contains("cancel"))
        {
            return Some(SensitiveAction::FinancialOperation);
        }

        None
    }
}
