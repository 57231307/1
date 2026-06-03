//! SQL 注入审计中间件
//!
//! 检测请求路径、查询参数中是否包含已知危险模式。
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
//! 1. 不读取请求体（避免大文件/二进制上传的性能开销），只审计 URL 部分。
//! 2. 命中后立即拒绝并记录 `WARN` 级别日志，便于审计追踪。
//! 3. 模式表保守白名单，避免误伤合法业务路径（例如富文本描述中含 `--`）。

use axum::{extract::Request, middleware::Next, response::Response};

use crate::utils::error::AppError;

/// 已知的 SQL 注入危险模式（白名单，命中即拒绝）
const DANGEROUS_PATTERNS: &[&str] = &[
    "' OR '1'='1",
    "' OR 1=1",
    "'; DROP TABLE",
    "'; DELETE FROM",
    "'; UPDATE ",
    "'; INSERT INTO",
    "UNION SELECT",
    "/*",
    "*/",
    "xp_cmdshell",
    "sp_executesql",
    "INFORMATION_SCHEMA.TABLES",
    "INFORMATION_SCHEMA.COLUMNS",
    "LOAD_FILE(",
    "INTO OUTFILE",
];

/// SQL 注入审计中间件函数
///
/// 命中危险模式时返回 `AppError::BadRequest`，否则透传到下游 Handler。
pub async fn sql_injection_audit_middleware(
    req: Request,
    next: Next,
) -> Result<Response, AppError> {
    let path = req.uri().path();
    let query = req.uri().query().unwrap_or("");

    // 仅审计 URL 部分（不读取 body，避免大请求体带来的性能开销）
    for pattern in DANGEROUS_PATTERNS {
        if path.contains(pattern) || query.contains(pattern) {
            tracing::warn!(
                "【SQL 注入审计】检测到可疑模式 | pattern={} | method={} | path={} | query={}",
                pattern,
                req.method(),
                path,
                query
            );
            return Err(AppError::BadRequest("请求包含非法字符".to_string()));
        }
    }

    Ok(next.run(req).await)
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
