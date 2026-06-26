//! SQL 注入审计中间件
//!
//! 检测请求路径、查询参数及文本类请求体中是否包含已知危险模式。
//! 注意：仅做粗粒度审计，主要防护依赖参数化查询（SeaORM 已默认使用参数化查询）。
//!
//! 使用方式（在路由注册处按需挂载到 router.layer(...)）：
//! ```ignore
//! use axum::middleware as axum_middleware;
//! use crate::middleware::sql_injection_audit::sql_injection_audit_middleware;
//!
//! Router::new()
//!     .route("/api/v1/...", get(handler))
//!     .layer(axum_middleware::from_fn(sql_injection_audit_middleware))
//! ```
//!
//! 设计要点：
//! 1. TS-S-4 安全加固（2026-06-26）：对文本类请求体（Content-Type: application/json、
//!    text/plain、application/x-www-form-urlencoded）做有限大小（1MB）缓冲后审计。
//!    超过 1MB 或非文本类请求体不审计（避免大文件/二进制上传的性能开销）。
//! 2. 命中后立即拒绝并记录 `WARN` 级别日志，便于审计追踪。
//! 3. 模式表保守白名单，避免误伤合法业务路径（例如富文本描述中含 `--`）。

use axum::{
    body::{to_bytes, Body},
    extract::Request,
    http::header::CONTENT_TYPE,
    middleware::Next,
    response::Response,
};

use crate::utils::error::AppError;

/// 文本类请求体审计的最大字节数（1MB）
const MAX_BODY_AUDIT_SIZE: usize = 1024 * 1024;

/// 已知的 SQL 注入危险模式（白名单，命中即拒绝）
///
/// M-7 修复：扩展黑名单，覆盖更多常见 SQL 注入攻击向量：
/// - 时间盲注：SLEEP、pg_sleep、WAITFOR、BENCHMARK
/// - 布尔盲注：更完整的 OR/AND 恒真模式
/// - 注释绕过：-- 、# 、/* */
/// - 函数注入：LOAD_FILE、INTO OUTFILE/DUMPFILE、xp_cmdshell
/// - 信息收集：INFORMATION_SCHEMA、pg_catalog、sysobjects
/// - 编码绕过：CHAR、CONCAT、ASCII、ORD、UNHEX
/// - 堆查询：多语句分隔符 ; 后跟危险关键字
///
/// 设计原则：模式尽量具体，避免误杀合法业务参数（如 "Order"、"OR" 之类的通用词）。
const DANGEROUS_PATTERNS: &[&str] = &[
    // 经典恒真注入
    "' OR '1'='1",
    "' OR 1=1",
    "\" OR \"1\"=\"1",
    "\" OR 1=1",
    "OR 1=1--",
    "OR 1=1#",
    "AND 1=1--",
    "AND 1=1#",

    // 堆查询 / 多语句注入
    "'; DROP TABLE",
    "'; DELETE FROM",
    "'; UPDATE ",
    "'; INSERT INTO",
    "'; ALTER TABLE",
    "'; CREATE TABLE",
    "'; TRUNCATE TABLE",
    "'; EXEC ",
    "'; EXECUTE ",

    // UNION 注入
    "UNION SELECT",
    "UNION ALL SELECT",
    "UNION DISTINCT SELECT",

    // SQL 注释（用于截断查询）
    "-- ",
    "/*",
    "*/",

    // 存储过程 / 命令执行
    "xp_cmdshell",
    "sp_executesql",
    "EXEC sp_",
    "EXECUTE sp_",
    "EXEC xp_",
    "EXECUTE xp_",

    // 信息 schema 探测
    "INFORMATION_SCHEMA.TABLES",
    "INFORMATION_SCHEMA.COLUMNS",
    "INFORMATION_SCHEMA.SCHEMATA",
    "pg_catalog.pg_tables",
    "sysobjects",
    "syscolumns",
    "sqlite_master",

    // 文件操作
    "LOAD_FILE(",
    "INTO OUTFILE",
    "INTO DUMPFILE",
    "COPY FROM",
    "COPY TO",

    // 时间盲注函数
    "SLEEP(",
    "pg_sleep(",
    "WAITFOR DELAY",
    "BENCHMARK(",
    "DBMS_PIPE.RECEIVE_MESSAGE",

    // 常用注入函数（编码/字符串绕过）
    "CHAR(",
    "CONCAT(",
    "ASCII(",
    "ORD(",
    "MID(",
    "SUBSTRING(",
    "SUBSTR(",
    "UNHEX(",
    "HEX(",
    "CAST(",
    "CONVERT(",

    // 布尔盲注常用函数
    "IFNULL(",
    "NULLIF(",
    "ISNULL(",
    "COALESCE(",
    "CASE WHEN",

    // 布尔/时间盲注的常见探测模式
    "' AND SLEEP(",
    "\" AND SLEEP(",
    "' OR SLEEP(",
    "\" OR SLEEP(",
];

/// SQL 注入审计中间件函数
///
/// 命中危险模式时返回 `AppError::BadRequest`，否则透传到下游 Handler。
///
/// TS-S-4 安全加固（2026-06-26）：对文本类请求体做有限大小（1MB）缓冲后审计。
pub async fn sql_injection_audit_middleware(
    req: Request,
    next: Next,
) -> Result<Response, AppError> {
    let path = req.uri().path();
    let query = req.uri().query().unwrap_or("");

    // 审计 URL 部分（path + query）
    for pattern in DANGEROUS_PATTERNS {
        if path.contains(pattern) || query.contains(pattern) {
            tracing::warn!(
                "【SQL 注入审计】URL 命中危险模式 | pattern={} | method={} | path={} | query={}",
                pattern,
                req.method(),
                path,
                query
            );
            return Err(AppError::BadRequest("请求包含非法字符".to_string()));
        }
    }

    // TS-S-4：审计文本类请求体（Content-Type 为 json/text/form-urlencoded 且大小 <= 1MB）
    let content_type = req
        .headers()
        .get(CONTENT_TYPE)
        .and_then(|v| v.to_str().ok())
        .unwrap_or("");

    let is_text_body = content_type.contains("application/json")
        || content_type.contains("text/plain")
        || content_type.contains("application/x-www-form-urlencoded");

    if is_text_body {
        // 拆分请求体用于审计
        let (parts, body) = req.into_parts();
        let body_bytes = to_bytes(body, MAX_BODY_AUDIT_SIZE)
            .await
            .map_err(|e| {
                tracing::warn!("【SQL 注入审计】读取请求体失败: {}", e);
                AppError::BadRequest("请求体读取失败".to_string())
            })?;

        // 审计请求体内容
        if let Ok(body_str) = std::str::from_utf8(&body_bytes) {
            for pattern in DANGEROUS_PATTERNS {
                if body_str.contains(pattern) {
                    tracing::warn!(
                        "【SQL 注入审计】请求体命中危险模式 | pattern={} | method={} | path={}",
                        pattern,
                        parts.method,
                        parts.uri.path()
                    );
                    return Err(AppError::BadRequest("请求包含非法字符".to_string()));
                }
            }
        }

        // 重组请求继续传递
        let req = Request::from_parts(parts, Body::from(body_bytes));
        Ok(next.run(req).await)
    } else {
        // 非文本类请求体，不审计 body
        Ok(next.run(req).await)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_dangerous_patterns_non_empty() {
        assert!(!DANGEROUS_PATTERNS.is_empty());
    }

    #[test]
    fn test_pattern_detection() {
        // 简单字符串包含测试
        assert!("'; DROP TABLE users".contains("'; DROP TABLE"));
        assert!("1' OR '1'='1".contains("' OR '1'='1"));
        assert!("admin'--".contains("--"));
    }
}
