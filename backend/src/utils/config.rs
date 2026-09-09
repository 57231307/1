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

/// 按请求实际协议判断登录会话 Cookie 的 Secure 标志。
///
/// 背景：deploy 包将 nginx 调整为 80 端口 HTTP 直接服务（443 为可选），
/// 而 config.yaml 的 env 固定为 "production" → is_production() 恒 true →
/// 登录 Cookie 带 Secure → HTTP 访问下浏览器拒绝存储 → 登录成功但会话
/// 无法建立（"登录异常"）。反向场景（HTTPS 部署 + env 误配 development）
/// 若仅依赖 env 会下发非 Secure Cookie，同样不可接受。
///
/// 判定规则：
/// 1. 请求经 nginx 代理时带 `X-Forwarded-Proto`（deploy/nginx.conf 已配置
///    `proxy_set_header X-Forwarded-Proto $scheme`）——以最外层代理声明的
///    请求协议为准：https → Secure，http → 非 Secure。
/// 2. 无该头（直连后端/本地开发）→ 回退 is_production()（保持既有语义）。
///
/// 安全边界：Secure=false 仅意味着 Cookie 可经 HTTP 发送。同站 HTTP 明文
/// 传输的会话令牌本身可被链路窃听，这是 HTTP 部署模式的固有属性；生产
/// 建议按 nginx.conf 尾注启用 443。直连后端伪造 X-Forwarded-Proto 的场景
/// 由部署形态排除（后端仅监听 127.0.0.1:8082，仅 nginx 可达）。
pub fn cookie_secure_for_request(headers: &axum::http::HeaderMap) -> bool {
    match headers
        .get("x-forwarded-proto")
        .and_then(|v| v.to_str().ok())
        .map(|v| v.trim().to_ascii_lowercase())
    {
        Some(proto) => proto == "https",
        None => is_production(),
    }
}
