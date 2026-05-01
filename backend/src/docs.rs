use utoipa::OpenApi;

#[derive(OpenApi)]
#[openapi(
    paths(
        crate::handlers::health_handler::health_check,
    ),
    components(
        schemas(
            crate::handlers::health_handler::HealthStatus,
            crate::handlers::health_handler::HealthChecks,
            crate::handlers::health_handler::HealthCheckItem,
        )
    ),
    tags(
        (name = "health", description = "健康检查及系统状态接口")
    ),
    info(
        title = "Bingxi ERP API",
        version = "1.0.0",
        description = "秉羲面料管理系统 (Bingxi ERP) 官方 API 文档"
    )
)]
pub struct ApiDoc;
