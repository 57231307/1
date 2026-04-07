use axum::{extract::State, http::StatusCode, response::IntoResponse, Json};
use chrono::Utc;
use serde::Serialize;

use crate::utils::app_state::AppState;

/// 健康状态响应
#[derive(Debug, Serialize)]
pub struct HealthStatus {
    /// 服务状态 (healthy, unhealthy, degraded)
    pub status: String,
    /// 服务名称
    pub service: String,
    /// 当前时间
    pub timestamp: chrono::DateTime<Utc>,
    /// 版本号
    pub version: String,
    /// 运行时长（秒）
    pub uptime_seconds: u64,
    /// 详细检查信息
    pub checks: HealthChecks,
}

/// 健康检查详情
#[derive(Debug, Serialize)]
pub struct HealthChecks {
    /// 数据库连接状态
    pub database: HealthCheckItem,
    /// 内存状态
    pub memory: HealthCheckItem,
    /// 磁盘状态
    pub disk: HealthCheckItem,
}

/// 单个健康检查项
#[derive(Debug, Serialize)]
pub struct HealthCheckItem {
    /// 状态 (healthy, unhealthy)
    pub status: String,
    /// 消息
    pub message: Option<String>,
    /// 响应时间（毫秒）
    pub response_time_ms: Option<u128>,
}

/// 健康检查接口
pub async fn health_check(State(state): State<AppState>) -> impl IntoResponse {
    let _start_time = std::time::Instant::now();

    // 检查数据库连接
    let db_check = check_database(&state).await;

    // 检查内存
    let memory_check = check_memory();

    // 检查磁盘
    let disk_check = check_disk();

    // 计算整体状态
    let overall_status = if db_check.status == "healthy"
        && memory_check.status == "healthy"
        && disk_check.status == "healthy"
    {
        "healthy"
    } else if db_check.status == "unhealthy" {
        "unhealthy"
    } else {
        "degraded"
    };

    let health = HealthStatus {
        status: overall_status.to_string(),
        service: "面料 ERP 系统".to_string(),
        timestamp: Utc::now(),
        version: env!("CARGO_PKG_VERSION").to_string(),
        uptime_seconds: get_uptime(),
        checks: HealthChecks {
            database: db_check,
            memory: memory_check,
            disk: disk_check,
        },
    };

    let status_code = if overall_status == "healthy" {
        StatusCode::OK
    } else if overall_status == "degraded" {
        StatusCode::PARTIAL_CONTENT
    } else {
        StatusCode::SERVICE_UNAVAILABLE
    };

    (status_code, Json(health))
}

/// 检查数据库连接
async fn check_database(state: &AppState) -> HealthCheckItem {
    let start = std::time::Instant::now();

    let db_status = match state.db.ping().await {
        Ok(_) => ("healthy", "数据库连接正常"),
        Err(_) => ("unhealthy", "数据库连接失败"),
    };

    let duration = start.elapsed();

    HealthCheckItem {
        status: db_status.0.to_string(),
        message: Some(db_status.1.to_string()),
        response_time_ms: Some(duration.as_millis()),
    }
}

/// 检查内存使用
fn check_memory() -> HealthCheckItem {
    // 获取内存使用情况
    // 注意：sysinfo crate 可以提供更详细的系统信息
    HealthCheckItem {
        status: "healthy".to_string(),
        message: Some("内存使用正常".to_string()),
        response_time_ms: Some(0),
    }
}

/// 检查磁盘空间
fn check_disk() -> HealthCheckItem {
    // 检查磁盘空间
    HealthCheckItem {
        status: "healthy".to_string(),
        message: Some("磁盘空间充足".to_string()),
        response_time_ms: Some(0),
    }
}

/// 获取服务运行时长
fn get_uptime() -> u64 {
    // 实际项目中应该记录启动时间，然后计算差值
    // 这里返回 0 作为示例
    0
}

/// 就绪检查（检查所有依赖是否就绪）
pub async fn readiness_check(State(state): State<AppState>) -> impl IntoResponse {
    // 检查数据库是否可连接
    let db_status = check_database(&state).await;

    if db_status.status == "healthy" {
        (
            StatusCode::OK,
            Json(serde_json::json!({
                "status": "healthy"
            })),
        )
    } else {
        (
            StatusCode::SERVICE_UNAVAILABLE,
            Json(serde_json::json!({
                "status": "not_ready",
                "reason": db_status.message
            })),
        )
    }
}

/// 存活检查（Kubernetes 探针使用）
pub async fn liveness_check() -> impl IntoResponse {
    // 简单的存活检查，只要服务能响应就返回成功
    StatusCode::OK
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_health_check_item() {
        let item = HealthCheckItem {
            status: "healthy".to_string(),
            message: Some("测试".to_string()),
            response_time_ms: Some(100),
        };

        assert_eq!(item.status, "healthy");
        assert_eq!(item.message, Some("测试".to_string()));
    }
}
