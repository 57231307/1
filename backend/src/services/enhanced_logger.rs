use serde::{Deserialize, Serialize};

/// 增强日志服务 - 提供详细的业务日志记录
pub struct EnhancedLogger;

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

impl EnhancedLogger {
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
}
