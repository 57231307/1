    use bingxi_backend::handlers::health_handler::*;
#[cfg(test)]
mod tests {

    #[test]
    fn test_health_check_item() {
        let item = HealthCheckItem {
            status: "healthy".to_string(),
            message: Some("测试".to_string()),
            response_time_ms: Some(100),
        };

        assert_eq!(item.status, "healthy");
        assert_eq!(item.message, Some("测试".to_string()));
    }
}