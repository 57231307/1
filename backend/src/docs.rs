//! OpenAPI 文档配置
//!
//! 使用 utoipa 生成 OpenAPI/Swagger 文档
//!
//! v14 P0-5 修复（批次 241）：
//! - 原 openapi.rs 是未注册的幽灵文件（无 mod 声明），已删除
//! - 原 docs.rs 是占位文件（ApiDoc 已删除），导致 `#[cfg(feature = "swagger")]` 编译失败
//! - 本文件恢复 ApiDoc，注册当前已添加 `#[utoipa::path]` 注解的 handler
//! - 文档覆盖率按模块增量补全（每模块补注解时同步在此注册），不作为技术债挂起；
//!   B02-P2-4 评估：为 100+ handler 补注解属持续性文档增强，按模块迭代推进而非独立修复项。
//!   当前覆盖率：14/115 handlers（auth 8 + user 5 + health 1，~12%）
//!   其余域（role/inventory/sales/purchase/finance/production/crm）已加 utoipa::path 注解，
//!   待各 struct 加 ToSchema derive 后注册到 paths。

use utoipa::OpenApi;

/// OpenAPI 文档配置
#[derive(OpenApi)]
#[openapi(
    paths(
        // 认证相关（A.25.1 auth 域补全，struct 已加 ToSchema）
        crate::handlers::auth_handler::login,
        crate::handlers::auth_handler_session::logout,
        crate::handlers::auth_handler_misc::refresh_token,
        crate::handlers::auth_handler_misc::setup_totp,
        crate::handlers::auth_handler_misc::enable_totp,
        crate::handlers::auth_handler_misc::get_current_user,
        crate::handlers::auth_handler_misc::agree_to_terms,
        // 健康检查
        crate::handlers::health_handler::health_check,
        // 用户管理（A.25.2，struct 已加 ToSchema）
        crate::handlers::user_handler::get_user,
        crate::handlers::user_handler::create_user,
        crate::handlers::user_handler::list_users,
        crate::handlers::user_handler::update_user,
        crate::handlers::user_handler::delete_user,
    ),
    components(
        schemas(
            // 认证相关
            crate::handlers::auth_handler::LoginRequest,
            crate::handlers::auth_handler::LoginResponse,
            crate::handlers::auth_handler::UserInfo,
            crate::handlers::auth_handler_misc::TotpSetupResponse,
            crate::handlers::auth_handler_misc::TotpVerifyRequest,
            crate::handlers::auth_handler_misc::AgreeToTermsRequest,
            // 健康检查
            crate::handlers::health_handler::HealthStatus,
            // 通用响应
            crate::utils::response::ApiResponse<String>,
        )
    ),
    tags(
        (name = "Auth", description = "用户认证和授权"),
        (name = "User", description = "用户管理"),
        (name = "health", description = "健康检查与服务状态")
    ),
    info(
        title = "面料管理 API",
        description = "面料管理的 RESTful API 文档\n\n主要功能模块：\n- 用户认证与授权\n- 采购合同管理\n- 销售合同管理\n- 固定资产管理\n- 预算管理\n- 质量标准与审批流程\n- 资金账户与转账\n\n注：当前仅注册已添加 utoipa::path 注解的接口，后续迭代逐步补全。",
        version = "1.0.0",
        contact(
            name = "面料管理团队",
            email = "support@bingxi.com"
        )
    ),
    servers(
        (url = "/api/v1/erp", description = "生产环境"),
        (url = "http://localhost:8082/api/v1/erp", description = "本地开发")
    )
)]
#[allow(dead_code, reason = "预留：OpenAPI 文档定义，待接入")]
pub struct ApiDoc;

impl ApiDoc {
    /// 创建 OpenAPI 文档
    pub fn new() -> Self {
        Self
    }
}

impl Default for ApiDoc {
    fn default() -> Self {
        Self::new()
    }
}
