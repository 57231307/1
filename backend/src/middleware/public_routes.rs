pub const PUBLIC_PATHS: &[&str] = &[
    "/health",
    "/ready",
    "/live",
    "/init",
    "/api/v1/erp/health",
    "/api/v1/erp/ready",
    "/api/v1/erp/live",
    "/api/v1/erp/init",
    "/api/v1/erp/auth/login",
    "/api/v1/erp/auth/refresh",
    "/api/v1/erp/auth/logout",
    "/api/v1/erp/dashboard",
];

pub fn is_public_path(path: &str) -> bool {
    PUBLIC_PATHS.iter().any(|p| path.starts_with(p))
}