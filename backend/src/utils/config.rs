//! 应用配置工具模块
//!
//! 提供统一的生产环境判断函数，供错误响应脱敏（漏洞 #11）、
//! Cookie `Secure` 标志（auth_handler_*）等需要"开发/生产"二元判断的代码共用。
//!
//! ## 设计原则
//!
//! - **单一来源**：统一从 `APP_ENV` 环境变量读取，避免此前 `ENV` / `cfg!(debug_assertions)` 多源不一致
//! - **保守策略**：未设置 `APP_ENV` 时按开发环境处理（暴露更多 detail 便于排错）
//! - **大小写不敏感**：`APP_ENV=production` / `APP_ENV=PRODUCTION` 均视为生产环境
//! - **config.yaml 同步**（批次 398 修复）：`AppSettings::new()` 启动时若 `APP_ENV` 未设置，
//!   会将 `config.yaml` 的 `env` 字段同步到 `APP_ENV` 环境变量，消除部署陷阱
//!
//! ## 使用示例
//!
//! ```rust
//! use crate::utils::config::is_production;
//!
//! if is_production() {
//!     // 生产环境：脱敏错误响应、设置 Cookie Secure
//! } else {
//!     // 开发环境：暴露 detail、关闭 Cookie Secure
//! }
//! ```
//!
//! ## 与其他文件的关系
//!
//! - 依赖关系：仅依赖 `std::env`，无业务依赖，避免循环依赖
//! - 被引用方：
//!   - `utils/error.rs` `IntoResponse::into_response`（漏洞 #11 修复）
//!   - `utils/error.rs::AppError::to_response`（统一 message 脱敏判断）
//!   - `handlers/auth_handler.rs`（登录 Cookie Secure 标志）
//!   - `handlers/auth_handler_misc.rs`（refresh_token Cookie Secure 标志）
//!   - `handlers/auth_handler_session.rs`（logout Cookie Secure 标志）
//! - **不修改**：`utils/audit.rs`（按用户规则保留）

/// 判断当前是否为生产环境（从 APP_ENV 环境变量读取，production 视为生产环境）
pub fn is_production() -> bool {
    std::env::var("APP_ENV")
        .map(|v| v.eq_ignore_ascii_case("production"))
        .unwrap_or(false)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::env;

    /// 测试 `APP_ENV=production` 时识别为生产环境
    #[test]
    fn test_is_production_with_production_value() {
        // 设置为 production
        env::set_var("APP_ENV", "production");
        assert!(
            is_production(),
            "APP_ENV=production 时应判定为生产环境"
        );
        // 清理环境变量，避免污染后续测试
        env::remove_var("APP_ENV");
    }

    /// 测试 `APP_ENV=development` 时识别为开发环境
    #[test]
    fn test_is_production_with_development_value() {
        env::set_var("APP_ENV", "development");
        assert!(
            !is_production(),
            "APP_ENV=development 时应判定为开发环境"
        );
        env::remove_var("APP_ENV");
    }

    /// 测试 `APP_ENV` 未设置时识别为开发环境（保守策略）
    #[test]
    fn test_is_production_with_unset() {
        env::remove_var("APP_ENV");
        assert!(
            !is_production(),
            "APP_ENV 未设置时应判定为开发环境（保守策略）"
        );
    }

    /// 测试 `APP_ENV=PRODUCTION` 大小写不敏感识别
    #[test]
    fn test_is_production_case_insensitive() {
        env::set_var("APP_ENV", "PRODUCTION");
        assert!(
            is_production(),
            "APP_ENV=PRODUCTION 大写也应判定为生产环境"
        );
        env::remove_var("APP_ENV");
    }
}
