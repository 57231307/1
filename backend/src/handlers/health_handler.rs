use axum::{extract::State, http::StatusCode, response::IntoResponse, Json};
use chrono::Utc;
use serde::Serialize;
use std::sync::OnceLock;
use std::time::Instant;
use sysinfo::{Disks, System};

use crate::utils::app_state::AppState;

use utoipa::ToSchema;

/// 进程启动时间（OnceLock 保证只初始化一次）
static START_TIME: OnceLock<Instant> = OnceLock::new();

/// 初始化进程启动时间（main 入口调用一次）
///
/// OnceLock 首次写入即锁定，确保 uptime 反映真实的进程启动时间而非首次健康检查时间。
pub fn start_time_init() -> Instant {
    *START_TIME.get_or_init(Instant::now)
}

/// 获取进程启动时间（若未初始化则当场初始化）
fn start_time() -> Instant {
    start_time_init()
}

/// 健康状态响应
#[derive(Debug, Serialize, ToSchema)]
pub struct HealthStatus {
    /// 服务状态 (healthy, unhealthy, degraded)
    pub status: String,
    /// 服务名称
    pub service: String,
    /// 当前时间
    #[schema(value_type = String, format = DateTime)]
    pub timestamp: chrono::DateTime<Utc>,
    /// 版本号
    pub version: String,
    /// 运行时长（秒）
    pub uptime_seconds: u64,
    /// 详细检查信息
    pub checks: HealthChecks,
}

/// 健康检查详情
#[derive(Debug, Serialize, ToSchema)]
pub struct HealthChecks {
    /// 数据库连接状态
    pub database: HealthCheckItem,
    /// 内存状态
    pub memory: HealthCheckItem,
    /// 磁盘状态
    pub disk: HealthCheckItem,
}

/// 单个健康检查项
#[derive(Debug, Serialize, ToSchema)]
pub struct HealthCheckItem {
    /// 状态 (healthy, unhealthy)
    pub status: String,
    /// 消息
    pub message: Option<String>,
    /// 响应时间（毫秒）
    pub response_time_ms: Option<u128>,
}

/// 健康检查接口
#[utoipa::path(
    get,
    path = "/api/v1/erp/init/health",
    responses(
        (status = 200, description = "服务完全健康", body = HealthStatus),
        (status = 206, description = "服务部分降级", body = HealthStatus),
        (status = 503, description = "服务不可用", body = HealthStatus)
    ),
    tag = "health"
)]
pub async fn health_check(State(state): State<AppState>) -> impl IntoResponse {
    let _start_time = std::time::Instant::now();

    // 确保启动时间已初始化（首次调用健康检查时记录）
    let _ = start_time();

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

    // 实际检查数据库连接池状态
    let is_connected = state.db.ping().await.is_ok();

    let duration = start.elapsed();

    if is_connected {
        HealthCheckItem {
            status: "healthy".to_string(),
            message: Some("数据库连接正常".to_string()),
            response_time_ms: Some(duration.as_millis()),
        }
    } else {
        HealthCheckItem {
            status: "unhealthy".to_string(),
            message: Some("数据库连接失败".to_string()),
            response_time_ms: Some(duration.as_millis()),
        }
    }
}

/// 检查内存使用
///
/// 基于 sysinfo 读取真实内存数据：
/// - used / total 比例 > 90% → unhealthy
/// - 比例 > 75% → degraded（但整体 status 只区分 healthy/unhealthy，故归 healthy）
fn check_memory() -> HealthCheckItem {
    let start = std::time::Instant::now();
    let mut sys = System::new_all();
    sys.refresh_memory();

    let total = sys.total_memory();
    let used = sys.used_memory();

    if total == 0 {
        return HealthCheckItem {
            status: "unhealthy".to_string(),
            message: Some("无法读取内存信息".to_string()),
            response_time_ms: Some(start.elapsed().as_millis()),
        };
    }

    let used_ratio = used as f64 / total as f64;
    let used_mb = used / 1024 / 1024;
    let total_mb = total / 1024 / 1024;
    let message = format!("内存使用 {}/{} MB ({:.0}%)", used_mb, total_mb, used_ratio * 100.0);

    let status = if used_ratio > 0.90 {
        "unhealthy"
    } else {
        "healthy"
    };

    HealthCheckItem {
        status: status.to_string(),
        message: Some(message),
        response_time_ms: Some(start.elapsed().as_millis()),
    }
}

/// 检查磁盘空间
///
/// 基于 sysinfo 读取根分区真实剩余空间：
/// - used / total 比例 > 90% → unhealthy
/// - 否则 → healthy
fn check_disk() -> HealthCheckItem {
    let start = std::time::Instant::now();
    let disks = Disks::new_with_refreshed_list();
    let root_disk = disks
        .list()
        .iter()
        .find(|d| d.mount_point() == std::path::Path::new("/"))
        .or_else(|| disks.list().first());

    let Some(disk) = root_disk else {
        return HealthCheckItem {
            status: "unhealthy".to_string(),
            message: Some("无法读取磁盘信息".to_string()),
            response_time_ms: Some(start.elapsed().as_millis()),
        };
    };

    let total = disk.total_space();
    let available = disk.available_space();
    if total == 0 {
        return HealthCheckItem {
            status: "unhealthy".to_string(),
            message: Some("磁盘总空间为 0".to_string()),
            response_time_ms: Some(start.elapsed().as_millis()),
        };
    }

    let used = total.saturating_sub(available);
    let used_ratio = used as f64 / total as f64;
    let used_gb = used / 1024 / 1024 / 1024;
    let total_gb = total / 1024 / 1024 / 1024;
    let message = format!(
        "磁盘使用 {}/{} GB ({:.0}%)",
        used_gb, total_gb, used_ratio * 100.0
    );

    let status = if used_ratio > 0.90 {
        "unhealthy"
    } else {
        "healthy"
    };

    HealthCheckItem {
        status: status.to_string(),
        message: Some(message),
        response_time_ms: Some(start.elapsed().as_millis()),
    }
}

/// 获取服务运行时长（秒）
///
/// 基于 OnceLock 记录的进程启动时间计算真实 uptime。
/// 首次调用（通常在 health_check 入口）会初始化 START_TIME。
fn get_uptime() -> u64 {
    start_time().elapsed().as_secs()
}

/// 就绪检查（检查所有依赖是否就绪）
///
/// P3 2-13 说明：本接口刻意不使用 `ApiResponse` 包装，原因是：
/// 1. K8s readinessProbe 仅依赖 HTTP 状态码（200/503），不需要业务层 envelope；
/// 2. 探针响应需保持简洁结构（仅 `status`/`reason`），避免暴露内部细节；
/// 3. 与 liveness_check 保持一致的轻量化响应风格。
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
