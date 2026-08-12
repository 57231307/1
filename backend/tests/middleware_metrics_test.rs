#[cfg(test)]
mod tests {
use bingxi_backend::middleware::metrics::*;


    #[test]
    fn test_truncate_route_short() {
        assert_eq!(truncate_route("/api/v1/erp/users"), "/api/v1/erp/users");
    }

    #[test]
    fn test_truncate_route_long() {
        let long_path = format!("/{}", "a".repeat(200));
        let truncated = truncate_route(&long_path);
        assert!(truncated.len() <= 128 + 32);
        assert!(truncated.contains("_trunc_"));
    }
}