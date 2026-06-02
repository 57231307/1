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

/// 敏感操作详情
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SensitiveActionDetail {
    /// 操作类型
    pub action_type: SensitiveAction,
    /// 告警级别
    pub alert_level: AlertLevel,
    /// 操作描述
    pub description: String,
    /// 用户ID
    pub user_id: i32,
    /// 用户名
    pub username: String,
    /// IP地址
    pub ip_address: Option<String>,
    /// 操作时间
    pub timestamp: String,
    /// 资源类型
    pub resource_type: String,
    /// 操作内容
    pub action: String,
    /// 详细信息（JSON格式）
    pub detail: Option<serde_json::Value>,
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

    /// 记录删除操作详情
    pub fn log_delete(
        user_id: i32,
        username: &str,
        ip_address: Option<&str>,
        resource_type: &str,
        resource_id: &str,
        resource_name: Option<&str>,
        deleted_data: Option<&serde_json::Value>,
    ) -> SensitiveActionDetail {
        let detail = serde_json::json!({
            "resource_id": resource_id,
            "resource_name": resource_name,
            "deleted_data": deleted_data,
        });

        tracing::warn!(
            "【删除操作】用户: {}({}), IP: {}, 删除了 {} [ID: {}, 名称: {:?}]",
            username,
            user_id,
            ip_address.unwrap_or("unknown"),
            resource_type,
            resource_id,
            resource_name
        );

        SensitiveActionDetail {
            action_type: SensitiveAction::Delete,
            alert_level: AlertLevel::High,
            description: format!("删除了 {} [ID: {}]", resource_type, resource_id),
            user_id,
            username: username.to_string(),
            ip_address: ip_address.map(|s| s.to_string()),
            timestamp: chrono::Utc::now().format("%Y-%m-%d %H:%M:%S").to_string(),
            resource_type: resource_type.to_string(),
            action: "DELETE".to_string(),
            detail: Some(detail),
        }
    }

    /// 记录权限变更详情
    pub fn log_permission_change(
        user_id: i32,
        username: &str,
        ip_address: Option<&str>,
        target_user: &str,
        old_permissions: &serde_json::Value,
        new_permissions: &serde_json::Value,
    ) -> SensitiveActionDetail {
        let detail = serde_json::json!({
            "target_user": target_user,
            "old_permissions": old_permissions,
            "new_permissions": new_permissions,
        });

        tracing::error!(
            "【权限变更】用户: {}({}), IP: {}, 修改了 {} 的权限\n变更前: {}\n变更后: {}",
            username,
            user_id,
            ip_address.unwrap_or("unknown"),
            target_user,
            old_permissions,
            new_permissions
        );

        SensitiveActionDetail {
            action_type: SensitiveAction::PermissionChange,
            alert_level: AlertLevel::Critical,
            description: format!("修改了 {} 的权限", target_user),
            user_id,
            username: username.to_string(),
            ip_address: ip_address.map(|s| s.to_string()),
            timestamp: chrono::Utc::now().format("%Y-%m-%d %H:%M:%S").to_string(),
            resource_type: "permission".to_string(),
            action: "UPDATE".to_string(),
            detail: Some(detail),
        }
    }

    /// 记录用户管理操作详情
    pub fn log_user_management(
        user_id: i32,
        username: &str,
        ip_address: Option<&str>,
        action: &str,
        target_user_id: i32,
        target_username: &str,
        user_data: Option<&serde_json::Value>,
    ) -> SensitiveActionDetail {
        let detail = serde_json::json!({
            "target_user_id": target_user_id,
            "target_username": target_username,
            "user_data": user_data,
        });

        let action_desc = match action {
            "CREATE" => "创建了新用户",
            "UPDATE" => "修改了用户信息",
            "DELETE" => "删除了用户",
            "DISABLE" => "禁用了用户",
            "ENABLE" => "启用了用户",
            "RESET_PASSWORD" => "重置了用户密码",
            _ => "操作了用户",
        };

        tracing::warn!(
            "【用户管理】用户: {}({}), IP: {}, {} {}({})",
            username,
            user_id,
            ip_address.unwrap_or("unknown"),
            action_desc,
            target_username,
            target_user_id
        );

        SensitiveActionDetail {
            action_type: SensitiveAction::UserManagement,
            alert_level: AlertLevel::High,
            description: format!("{} {}({})", action_desc, target_username, target_user_id),
            user_id,
            username: username.to_string(),
            ip_address: ip_address.map(|s| s.to_string()),
            timestamp: chrono::Utc::now().format("%Y-%m-%d %H:%M:%S").to_string(),
            resource_type: "user".to_string(),
            action: action.to_string(),
            detail: Some(detail),
        }
    }

    /// 记录角色管理操作详情
    pub fn log_role_management(
        user_id: i32,
        username: &str,
        ip_address: Option<&str>,
        action: &str,
        role_id: i32,
        role_name: &str,
        role_data: Option<&serde_json::Value>,
    ) -> SensitiveActionDetail {
        let detail = serde_json::json!({
            "role_id": role_id,
            "role_name": role_name,
            "role_data": role_data,
        });

        let action_desc = match action {
            "CREATE" => "创建了新角色",
            "UPDATE" => "修改了角色信息",
            "DELETE" => "删除了角色",
            "ASSIGN_PERMISSION" => "分配了角色权限",
            _ => "操作了角色",
        };

        tracing::error!(
            "【角色管理】用户: {}({}), IP: {}, {} {}({})\n角色详情: {}",
            username,
            user_id,
            ip_address.unwrap_or("unknown"),
            action_desc,
            role_name,
            role_id,
            role_data.map(|d| d.to_string()).unwrap_or_default()
        );

        SensitiveActionDetail {
            action_type: SensitiveAction::RoleManagement,
            alert_level: AlertLevel::Critical,
            description: format!("{} {}({})", action_desc, role_name, role_id),
            user_id,
            username: username.to_string(),
            ip_address: ip_address.map(|s| s.to_string()),
            timestamp: chrono::Utc::now().format("%Y-%m-%d %H:%M:%S").to_string(),
            resource_type: "role".to_string(),
            action: action.to_string(),
            detail: Some(detail),
        }
    }

    /// 记录系统配置变更详情
    pub fn log_system_config(
        user_id: i32,
        username: &str,
        ip_address: Option<&str>,
        config_key: &str,
        old_value: &str,
        new_value: &str,
    ) -> SensitiveActionDetail {
        let detail = serde_json::json!({
            "config_key": config_key,
            "old_value": old_value,
            "new_value": new_value,
        });

        tracing::error!(
            "【系统配置变更】用户: {}({}), IP: {}, 修改了配置 {}\n变更前: {}\n变更后: {}",
            username,
            user_id,
            ip_address.unwrap_or("unknown"),
            config_key,
            old_value,
            new_value
        );

        SensitiveActionDetail {
            action_type: SensitiveAction::SystemConfig,
            alert_level: AlertLevel::Critical,
            description: format!("修改了系统配置 {}", config_key),
            user_id,
            username: username.to_string(),
            ip_address: ip_address.map(|s| s.to_string()),
            timestamp: chrono::Utc::now().format("%Y-%m-%d %H:%M:%S").to_string(),
            resource_type: "system_config".to_string(),
            action: "UPDATE".to_string(),
            detail: Some(detail),
        }
    }

    /// 记录数据导出详情
    pub fn log_data_export(
        user_id: i32,
        username: &str,
        ip_address: Option<&str>,
        export_type: &str,
        record_count: i64,
        file_name: &str,
        query_params: Option<&serde_json::Value>,
    ) -> SensitiveActionDetail {
        let detail = serde_json::json!({
            "export_type": export_type,
            "record_count": record_count,
            "file_name": file_name,
            "query_params": query_params,
        });

        tracing::info!(
            "【数据导出】用户: {}({}), IP: {}, 导出了 {} 条 {} 数据，文件: {}",
            username,
            user_id,
            ip_address.unwrap_or("unknown"),
            record_count,
            export_type,
            file_name
        );

        SensitiveActionDetail {
            action_type: SensitiveAction::DataExport,
            alert_level: AlertLevel::Medium,
            description: format!("导出了 {} 条 {} 数据", record_count, export_type),
            user_id,
            username: username.to_string(),
            ip_address: ip_address.map(|s| s.to_string()),
            timestamp: chrono::Utc::now().format("%Y-%m-%d %H:%M:%S").to_string(),
            resource_type: export_type.to_string(),
            action: "EXPORT".to_string(),
            detail: Some(detail),
        }
    }

    /// 记录批量操作详情
    pub fn log_batch_operation(
        user_id: i32,
        username: &str,
        ip_address: Option<&str>,
        operation_type: &str,
        resource_type: &str,
        affected_count: i64,
        resource_ids: &[i32],
    ) -> SensitiveActionDetail {
        let detail = serde_json::json!({
            "operation_type": operation_type,
            "resource_type": resource_type,
            "affected_count": affected_count,
            "resource_ids": resource_ids,
        });

        tracing::info!(
            "【批量操作】用户: {}({}), IP: {}, 对 {} 条 {} 执行了 {} 操作",
            username,
            user_id,
            ip_address.unwrap_or("unknown"),
            affected_count,
            resource_type,
            operation_type
        );

        SensitiveActionDetail {
            action_type: SensitiveAction::BatchOperation,
            alert_level: AlertLevel::Medium,
            description: format!(
                "对 {} 条 {} 执行了 {} 操作",
                affected_count, resource_type, operation_type
            ),
            user_id,
            username: username.to_string(),
            ip_address: ip_address.map(|s| s.to_string()),
            timestamp: chrono::Utc::now().format("%Y-%m-%d %H:%M:%S").to_string(),
            resource_type: resource_type.to_string(),
            action: operation_type.to_string(),
            detail: Some(detail),
        }
    }

    /// 记录登录失败详情
    pub fn log_login_failure(
        attempted_username: &str,
        ip_address: Option<&str>,
        failure_reason: &str,
        user_agent: Option<&str>,
    ) -> SensitiveActionDetail {
        let detail = serde_json::json!({
            "attempted_username": attempted_username,
            "failure_reason": failure_reason,
            "user_agent": user_agent,
        });

        tracing::warn!(
            "【登录失败】用户名: {}, IP: {}, 原因: {}, 时间: {}",
            attempted_username,
            ip_address.unwrap_or("unknown"),
            failure_reason,
            chrono::Utc::now().format("%Y-%m-%d %H:%M:%S")
        );

        SensitiveActionDetail {
            action_type: SensitiveAction::LoginFailure,
            alert_level: AlertLevel::Medium,
            description: format!("用户 {} 登录失败: {}", attempted_username, failure_reason),
            user_id: 0,
            username: attempted_username.to_string(),
            ip_address: ip_address.map(|s| s.to_string()),
            timestamp: chrono::Utc::now().format("%Y-%m-%d %H:%M:%S").to_string(),
            resource_type: "auth".to_string(),
            action: "LOGIN_FAILED".to_string(),
            detail: Some(detail),
        }
    }

    /// 记录密码变更详情
    pub fn log_password_change(
        user_id: i32,
        username: &str,
        ip_address: Option<&str>,
        changed_by: &str,
    ) -> SensitiveActionDetail {
        let detail = serde_json::json!({
            "changed_by": changed_by,
        });

        tracing::warn!(
            "【密码变更】用户: {}({}), IP: {}, 操作人: {}, 时间: {}",
            username,
            user_id,
            ip_address.unwrap_or("unknown"),
            changed_by,
            chrono::Utc::now().format("%Y-%m-%d %H:%M:%S")
        );

        SensitiveActionDetail {
            action_type: SensitiveAction::PasswordChange,
            alert_level: AlertLevel::High,
            description: format!("用户 {} 的密码被 {} 修改", username, changed_by),
            user_id,
            username: username.to_string(),
            ip_address: ip_address.map(|s| s.to_string()),
            timestamp: chrono::Utc::now().format("%Y-%m-%d %H:%M:%S").to_string(),
            resource_type: "user".to_string(),
            action: "CHANGE_PASSWORD".to_string(),
            detail: Some(detail),
        }
    }

    /// 记录资金操作详情
    pub fn log_financial_operation(
        user_id: i32,
        username: &str,
        ip_address: Option<&str>,
        operation_type: &str,
        resource_type: &str,
        resource_id: &str,
        amount: Option<f64>,
        operation_data: Option<&serde_json::Value>,
    ) -> SensitiveActionDetail {
        let detail = serde_json::json!({
            "operation_type": operation_type,
            "resource_type": resource_type,
            "resource_id": resource_id,
            "amount": amount,
            "operation_data": operation_data,
        });

        let amount_str = amount
            .map(|a| format!("，金额: {:.2}", a))
            .unwrap_or_default();

        tracing::error!(
            "【资金操作】用户: {}({}), IP: {}, {} 了 {} [ID: {}]{}",
            username,
            user_id,
            ip_address.unwrap_or("unknown"),
            operation_type,
            resource_type,
            resource_id,
            amount_str
        );

        SensitiveActionDetail {
            action_type: SensitiveAction::FinancialOperation,
            alert_level: AlertLevel::Critical,
            description: format!(
                "{} 了 {} [ID: {}]{}",
                operation_type, resource_type, resource_id, amount_str
            ),
            user_id,
            username: username.to_string(),
            ip_address: ip_address.map(|s| s.to_string()),
            timestamp: chrono::Utc::now().format("%Y-%m-%d %H:%M:%S").to_string(),
            resource_type: resource_type.to_string(),
            action: operation_type.to_string(),
            detail: Some(detail),
        }
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
        if resource_lower.contains("permission") || resource_lower.contains("role") {
            if action_lower.contains("update") || action_lower.contains("assign") {
                return Some(SensitiveAction::PermissionChange);
            }
        }

        // 用户管理
        if resource_lower.contains("user") {
            if action_lower.contains("create")
                || action_lower.contains("update")
                || action_lower.contains("delete")
            {
                return Some(SensitiveAction::UserManagement);
            }
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
        if resource_lower.contains("payment")
            || resource_lower.contains("invoice")
            || resource_lower.contains("voucher")
            || resource_lower.contains("fund")
        {
            if action_lower.contains("approve") || action_lower.contains("cancel") {
                return Some(SensitiveAction::FinancialOperation);
            }
        }

        None
    }
}
