#![allow(dead_code)]

use serde::{Deserialize, Serialize};
use serde_json::Value;

/// 增强日志服务 - 提供详细的业务日志记录
pub struct EnhancedLogger;

/// 操作上下文
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OperationContext {
    pub trace_id: String,
    pub user_id: i32,
    pub username: String,
    pub tenant_id: Option<i32>,
    pub ip_address: Option<String>,
    pub user_agent: Option<String>,
    pub session_id: Option<String>,
}

/// 数据库操作日志
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DatabaseOperationLog {
    pub operation: String, // INSERT, UPDATE, DELETE, SELECT
    pub table: String,
    pub schema: Option<String>,
    pub sql: Option<String>,
    pub params: Option<Value>,
    pub rows_affected: Option<i64>,
    pub rows_returned: Option<i64>,
    pub duration_ms: i64,
    pub error: Option<DatabaseError>,
    pub context: Option<OperationContext>,
}

/// 数据库错误详情
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DatabaseError {
    pub error_type: String,
    pub code: Option<String>,
    pub message: String,
    pub constraint: Option<String>,
    pub detail: Option<String>,
    pub table: Option<String>,
    pub schema: Option<String>,
    pub suggestion: Option<String>,
}

/// 资金操作日志
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FinancialOperationLog {
    pub operation: String,      // APPROVE, REJECT, CANCEL, CREATE
    pub financial_type: String, // PAYMENT, INVOICE, VOUCHER, FUND
    pub financial_id: i32,
    pub financial_no: String,
    pub amount: f64,
    pub currency: String,
    pub operator: OperatorInfo,
    pub financial_details: FinancialDetails,
    pub approval_info: Option<ApprovalInfo>,
    pub context: Option<Value>,
}

/// 操作者信息
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OperatorInfo {
    pub user_id: i32,
    pub username: String,
    pub ip_address: Option<String>,
    pub department: Option<String>,
}

/// 资金详情
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FinancialDetails {
    pub related_type: Option<String>, // 关联类型（采购订单、销售订单等）
    pub related_id: Option<i32>,      // 关联ID
    pub related_no: Option<String>,   // 关联单号
    pub supplier_id: Option<i32>,
    pub supplier_name: Option<String>,
    pub customer_id: Option<i32>,
    pub customer_name: Option<String>,
    pub payment_method: Option<String>,
    pub bank_account: Option<String>,
    pub due_date: Option<String>,
    pub invoice_ids: Option<Vec<i32>>,
}

/// 审批信息
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApprovalInfo {
    pub before_status: String,
    pub after_status: String,
    pub approval_level: i32,
    pub approver_comments: Option<String>,
    pub approval_time: String,
}

/// 权限变更日志
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PermissionChangeLog {
    pub operation: String, // ASSIGN, REVOKE, UPDATE
    pub operator: OperatorInfo,
    pub target_user: TargetUser,
    pub permission_change: PermissionChange,
    pub context: Option<Value>,
}

/// 目标用户
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TargetUser {
    pub user_id: i32,
    pub username: String,
    pub current_roles: Vec<String>,
}

/// 权限变更详情
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PermissionChange {
    pub before: PermissionSnapshot,
    pub after: PermissionSnapshot,
    pub diff: PermissionDiff,
}

/// 权限快照
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PermissionSnapshot {
    pub roles: Vec<String>,
    pub permissions: Value,
}

/// 权限差异
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PermissionDiff {
    pub roles_added: Vec<String>,
    pub roles_removed: Vec<String>,
    pub permissions_changed: Vec<PermissionItem>,
}

/// 权限项
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PermissionItem {
    pub resource: String,
    pub action: String,
    pub before: bool,
    pub after: bool,
}

/// 登录安全日志
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LoginSecurityLog {
    pub event: String, // LOGIN_SUCCESS, LOGIN_FAILURE, LOGOUT
    pub attempt: LoginAttempt,
    pub failure_info: Option<FailureInfo>,
    pub security_info: SecurityInfo,
    pub geo_info: Option<GeoInfo>,
    pub device_info: DeviceInfo,
}

/// 登录尝试
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LoginAttempt {
    pub username: String,
    pub ip_address: String,
    pub user_agent: String,
    pub timestamp: String,
    pub method: String,     // password, sso, api_key
    pub login_type: String, // web, mobile, api
}

/// 失败信息
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FailureInfo {
    pub reason: String,
    pub attempts_today: i32,
    pub attempts_total: i32,
    pub last_success: Option<String>,
    pub last_failure: Option<String>,
}

/// 安全信息
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SecurityInfo {
    pub risk_level: String, // LOW, MEDIUM, HIGH, CRITICAL
    pub risk_factors: Vec<String>,
    pub blocked: bool,
    pub block_reason: Option<String>,
    pub require_captcha: bool,
    pub notify_user: bool,
}

/// 地理位置信息
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GeoInfo {
    pub country: Option<String>,
    pub region: Option<String>,
    pub city: Option<String>,
    pub isp: Option<String>,
    pub latitude: Option<f64>,
    pub longitude: Option<f64>,
}

/// 设备信息
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeviceInfo {
    pub os: Option<String>,
    pub browser: Option<String>,
    pub device_type: String, // desktop, mobile, tablet
    pub is_mobile: bool,
}

/// 性能监控日志
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PerformanceLog {
    pub endpoint: String,
    pub method: String,
    pub performance: PerformanceMetrics,
    pub database: DatabaseMetrics,
    pub cache: CacheMetrics,
    pub memory: Option<MemoryMetrics>,
}

/// 性能指标
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PerformanceMetrics {
    pub total_duration_ms: i64,
    pub db_duration_ms: i64,
    pub cache_duration_ms: i64,
    pub external_duration_ms: i64,
    pub serialization_duration_ms: i64,
    pub middleware_duration_ms: i64,
}

/// 数据库指标
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DatabaseMetrics {
    pub queries: Vec<QueryMetric>,
    pub total_queries: i32,
    pub slow_queries: i32,
    pub connection_pool: ConnectionPoolMetrics,
}

/// 查询指标
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QueryMetric {
    pub sql: String,
    pub duration_ms: i64,
    pub rows_returned: Option<i64>,
    pub rows_affected: Option<i64>,
    pub table: String,
}

/// 连接池指标
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConnectionPoolMetrics {
    pub active: i32,
    pub idle: i32,
    pub max: i32,
    pub waiting: i32,
}

/// 缓存指标
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CacheMetrics {
    pub hits: Vec<String>,
    pub misses: Vec<String>,
    pub hit_rate: f64,
}

/// 内存指标
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MemoryMetrics {
    pub allocated_mb: f64,
    pub peak_mb: f64,
    pub gc_count: i32,
}

/// 业务操作日志
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BusinessOperationLog {
    pub operation: String,
    pub module: String,
    pub resource_type: String,
    pub resource_id: Option<String>,
    pub resource_name: Option<String>,
    pub operator: OperatorInfo,
    pub action_details: Value,
    pub before_data: Option<Value>,
    pub after_data: Option<Value>,
    pub context: Option<Value>,
    pub result: OperationResult,
}

/// 操作结果
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OperationResult {
    pub success: bool,
    pub affected_rows: Option<i64>,
    pub generated_id: Option<i32>,
    pub error_message: Option<String>,
}

/// 系统健康日志
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SystemHealthLog {
    pub timestamp: String,
    pub system: SystemMetrics,
    pub database: DatabaseHealth,
    pub cache: CacheHealth,
    pub application: ApplicationHealth,
}

/// 系统指标
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SystemMetrics {
    pub cpu_usage_percent: f64,
    pub memory_usage_percent: f64,
    pub disk_usage_percent: f64,
    pub load_average: Vec<f64>,
    pub uptime_seconds: i64,
}

/// 数据库健康
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DatabaseHealth {
    pub status: String,
    pub connections: ConnectionPoolMetrics,
    pub replication_lag_ms: Option<i64>,
    pub query_time_avg_ms: f64,
}

/// 缓存健康
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CacheHealth {
    pub status: String,
    pub memory_used_mb: f64,
    pub memory_max_mb: f64,
    pub hit_rate: f64,
    pub evictions: i64,
}

/// 应用健康
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApplicationHealth {
    pub version: String,
    pub environment: String,
    pub active_users: i32,
    pub requests_per_minute: i32,
    pub error_rate_percent: f64,
}

impl EnhancedLogger {
    /// 记录资金操作日志
    pub fn log_financial_operation(log: &FinancialOperationLog) {
        tracing::info!(
            target: "financial_audit",
            "[资金操作] 操作: {} | 类型: {} | 单号: {} | 金额: {} {} | 操作人: {}({}) | IP: {}",
            log.operation,
            log.financial_type,
            log.financial_no,
            log.amount,
            log.currency,
            log.operator.username,
            log.operator.user_id,
            log.operator.ip_address.as_deref().unwrap_or("unknown")
        );

        // 详细日志
        tracing::info!(
            target: "financial_audit_detail",
            "{}",
            serde_json::to_string(log).unwrap_or_default()
        );
    }

    /// 记录权限变更日志
    pub fn log_permission_change(log: &PermissionChangeLog) {
        tracing::warn!(
            target: "permission_audit",
            "[权限变更] 操作: {} | 操作人: {}({}) | 目标用户: {}({}) | 变更角色: +{:?} -{:?}",
            log.operation,
            log.operator.username,
            log.operator.user_id,
            log.target_user.username,
            log.target_user.user_id,
            log.permission_change.diff.roles_added,
            log.permission_change.diff.roles_removed
        );

        // 详细日志
        tracing::info!(
            target: "permission_audit_detail",
            "{}",
            serde_json::to_string(log).unwrap_or_default()
        );
    }

    /// 记录登录安全日志
    pub fn log_login_security(log: &LoginSecurityLog) {
        let level = match log.security_info.risk_level.as_str() {
            "CRITICAL" | "HIGH" => tracing::Level::WARN,
            "MEDIUM" => tracing::Level::INFO,
            _ => tracing::Level::DEBUG,
        };

        match level {
            tracing::Level::WARN => {
                tracing::warn!(
                    target: "security_audit",
                    "[安全事件] 事件: {} | 用户: {} | IP: {} | 风险级别: {} | 风险因素: {:?} | 已封禁: {}",
                    log.event,
                    log.attempt.username,
                    log.attempt.ip_address,
                    log.security_info.risk_level,
                    log.security_info.risk_factors,
                    log.security_info.blocked
                );
            }
            tracing::Level::INFO => {
                tracing::info!(
                    target: "security_audit",
                    "[安全事件] 事件: {} | 用户: {} | IP: {} | 风险级别: {}",
                    log.event,
                    log.attempt.username,
                    log.attempt.ip_address,
                    log.security_info.risk_level
                );
            }
            _ => {
                tracing::debug!(
                    target: "security_audit",
                    "[安全事件] 事件: {} | 用户: {} | IP: {}",
                    log.event,
                    log.attempt.username,
                    log.attempt.ip_address
                );
            }
        }

        // 详细日志
        tracing::info!(
            target: "security_audit_detail",
            "{}",
            serde_json::to_string(log).unwrap_or_default()
        );
    }

    /// 记录数据库操作日志
    pub fn log_database_operation(log: &DatabaseOperationLog) {
        if let Some(ref error) = log.error {
            tracing::error!(
                target: "database_audit",
                "[数据库错误] 操作: {} | 表: {} | 耗时: {}ms | 错误类型: {} | 错误: {} | 建议: {}",
                log.operation,
                log.table,
                log.duration_ms,
                error.error_type,
                error.message,
                error.suggestion.as_deref().unwrap_or("无")
            );
        } else if log.duration_ms > 1000 {
            tracing::warn!(
                target: "database_audit",
                "[慢查询] 操作: {} | 表: {} | 耗时: {}ms | 返回行数: {:?} | 影响行数: {:?}",
                log.operation,
                log.table,
                log.duration_ms,
                log.rows_returned,
                log.rows_affected
            );
        } else {
            tracing::debug!(
                target: "database_audit",
                "[数据库操作] 操作: {} | 表: {} | 耗时: {}ms",
                log.operation,
                log.table,
                log.duration_ms
            );
        }

        // 详细日志
        tracing::info!(
            target: "database_audit_detail",
            "{}",
            serde_json::to_string(log).unwrap_or_default()
        );
    }

    /// 记录性能监控日志
    pub fn log_performance(log: &PerformanceLog) {
        let is_slow = log.performance.total_duration_ms > 1000;
        let has_slow_queries = log.database.slow_queries > 0;

        if is_slow || has_slow_queries {
            tracing::warn!(
                target: "performance_audit",
                "[性能告警] 接口: {} {} | 总耗时: {}ms | DB耗时: {}ms | 查询数: {} | 慢查询: {} | 缓存命中率: {:.2}%",
                log.method,
                log.endpoint,
                log.performance.total_duration_ms,
                log.performance.db_duration_ms,
                log.database.total_queries,
                log.database.slow_queries,
                log.cache.hit_rate * 100.0
            );
        } else {
            tracing::debug!(
                target: "performance_audit",
                "[性能监控] 接口: {} {} | 总耗时: {}ms | DB耗时: {}ms | 查询数: {}",
                log.method,
                log.endpoint,
                log.performance.total_duration_ms,
                log.performance.db_duration_ms,
                log.database.total_queries
            );
        }

        // 详细日志
        tracing::info!(
            target: "performance_audit_detail",
            "{}",
            serde_json::to_string(log).unwrap_or_default()
        );
    }

    /// 记录业务操作日志
    pub fn log_business_operation(log: &BusinessOperationLog) {
        if log.result.success {
            tracing::info!(
                target: "business_audit",
                "[业务操作] 模块: {} | 操作: {} | 资源: {} {} | 操作人: {}({}) | 结果: 成功 | 影响行数: {:?}",
                log.module,
                log.operation,
                log.resource_type,
                log.resource_id.as_deref().unwrap_or("-"),
                log.operator.username,
                log.operator.user_id,
                log.result.affected_rows
            );
        } else {
            tracing::warn!(
                target: "business_audit",
                "[业务操作] 模块: {} | 操作: {} | 资源: {} {} | 操作人: {}({}) | 结果: 失败 | 错误: {}",
                log.module,
                log.operation,
                log.resource_type,
                log.resource_id.as_deref().unwrap_or("-"),
                log.operator.username,
                log.operator.user_id,
                log.result.error_message.as_deref().unwrap_or("未知错误")
            );
        }

        // 详细日志
        tracing::info!(
            target: "business_audit_detail",
            "{}",
            serde_json::to_string(log).unwrap_or_default()
        );
    }

    /// 记录系统健康日志
    pub fn log_system_health(log: &SystemHealthLog) {
        let is_healthy = log.system.cpu_usage_percent < 80.0
            && log.system.memory_usage_percent < 80.0
            && log.database.status == "healthy"
            && log.cache.status == "healthy";

        if is_healthy {
            tracing::debug!(
                target: "system_health",
                "[系统健康] CPU: {:.1}% | 内存: {:.1}% | 磁盘: {:.1}% | 数据库: {} | 缓存: {} | 活跃用户: {}",
                log.system.cpu_usage_percent,
                log.system.memory_usage_percent,
                log.system.disk_usage_percent,
                log.database.status,
                log.cache.status,
                log.application.active_users
            );
        } else {
            tracing::warn!(
                target: "system_health",
                "[系统告警] CPU: {:.1}% | 内存: {:.1}% | 磁盘: {:.1}% | 数据库: {} | 缓存: {} | 错误率: {:.2}%",
                log.system.cpu_usage_percent,
                log.system.memory_usage_percent,
                log.system.disk_usage_percent,
                log.database.status,
                log.cache.status,
                log.application.error_rate_percent
            );
        }

        // 详细日志
        tracing::info!(
            target: "system_health_detail",
            "{}",
            serde_json::to_string(log).unwrap_or_default()
        );
    }
}
