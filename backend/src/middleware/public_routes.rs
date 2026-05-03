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
];

pub fn is_public_path(path: &str) -> bool {
    PUBLIC_PATHS.iter().any(|p| path.starts_with(p))
}

#[cfg(test)]
mod tests {
    use super::is_public_path;

    #[test]
    fn dashboard_should_not_be_public() {
        assert!(!is_public_path("/api/v1/erp/dashboard/overview"));
        assert!(!is_public_path("/api/v1/erp/dashboard/sales-stats"));
    }

    #[test]
    fn auth_login_should_be_public() {
        assert!(is_public_path("/api/v1/erp/auth/login"));
    }
}
