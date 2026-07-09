//! OpenAPI 文档配置
//!
//! 使用 utoipa 生成 OpenAPI/Swagger 文档
//!
//! v14 P0-5 修复（批次 241）：
//! - 原 openapi.rs 是未注册的幽灵文件（无 mod 声明），已删除
//! - 原 docs.rs 是占位文件（ApiDoc 已删除），导致 `#[cfg(feature = "swagger")]` 编译失败
//! - 本文件恢复 ApiDoc，注册当前已添加 `#[utoipa::path]` 注解的 handler
//! - TODO(tech-debt): 后续迭代需为更多 handler 添加 utoipa::path 注解以提升文档覆盖率
//!   当前覆盖率：2/115 handlers（auth/login + health/health_check）

use utoipa::OpenApi;

/// OpenAPI 文档配置
#[derive(OpenApi)]
#[openapi(
    paths(
        // 认证相关（已添加 utoipa::path 注解）
        crate::handlers::auth_handler::login,
        // 健康检查（已添加 utoipa::path 注解）
        crate::handlers::health_handler::health_check,
    ),
    components(
        schemas(
            // 认证相关
            crate::handlers::auth_handler::LoginRequest,
            crate::handlers::auth_handler::LoginResponse,
            crate::handlers::auth_handler::UserInfo,
            // 健康检查
            crate::handlers::health_handler::HealthStatus,
            // 通用响应
            crate::utils::response::ApiResponse<String>,
        )
    ),
    tags(
        (name = "Auth", description = "用户认证和授权"),
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
