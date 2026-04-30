//! 日志管理服务测试
//! 测试日志管理的核心功能

#[cfg(test)]
mod tests {
    use super::*;
    use crate::services::LogService;
    use sea_orm::{Database, DatabaseConnection};
    use std::sync::Arc;

    async fn setup_test_db() -> DatabaseConnection {
        let db_url = std::env::var("TEST_DATABASE_URL")
            .unwrap_or_else(|_| "sqlite::memory:".to_string());
        Database::connect(&db_url).await.unwrap()
    }

    #[tokio::test]
    async fn test_log_operation() {
        let db = setup_test_db().await;
        let service = LogService::new(Arc::new(db));
        
        let req = crate::services::log_service::LogOperationRequest {
            module: "采购管理".to_string(),
            operation_type: "创建".to_string(),
            operation_desc: "创建采购订单".to_string(),
            business_type: Some("purchase_order".to_string()),
            business_id: Some(1),
            user_id: 1,
            username: "zhangsan".to_string(),
            real_name: Some("张三".to_string()),
            request_method: Some("POST".to_string()),
            request_url: Some("/api/v1/erp/purchase/orders".to_string()),
            ip_address: Some("192.168.1.100".to_string()),
            duration_ms: Some(150),
        };
        
        let result = service.log_operation(req).await;
        
        assert!(result.is_ok());
        let log_no = result.unwrap();
        assert!(log_no.starts_with("OP"));
    }

    #[tokio::test]
    async fn test_log_system() {
        let db = setup_test_db().await;
        let service = LogService::new(Arc::new(db));
        
        let req = crate::services::log_service::LogSystemRequest {
            log_level: "ERROR".to_string(),
            module: "数据库".to_string(),
            message: "数据库连接失败".to_string(),
            stack_trace: Some("at com.example...".to_string()),
            server_name: Some("server-01".to_string()),
        };
        
        let result = service.log_system(req).await;
        
        assert!(result.is_ok());
        let log_no = result.unwrap();
        assert!(log_no.starts_with("SYS"));
    }

    #[tokio::test]
    async fn test_log_login() {
        let db = setup_test_db().await;
        let service = LogService::new(Arc::new(db));
        
        let req = crate::services::log_service::LogLoginRequest {
            username: "zhangsan".to_string(),
            login_result: "success".to_string(),
            ip_address: "192.168.1.100".to_string(),
            user_agent: Some("Mozilla/5.0".to_string()),
            failure_reason: None,
        };
        
        let result = service.log_login(req).await;
        
        assert!(result.is_ok());
        let log_no = result.unwrap();
        assert!(log_no.starts_with("LOGIN"));
    }

    #[tokio::test]
    async fn test_log_api_access() {
        let db = setup_test_db().await;
        let service = LogService::new(Arc::new(db));
        
        let req = crate::services::log_service::LogApiAccessRequest {
            request_id: "req-001".to_string(),
            request_method: "GET".to_string(),
            request_url: "/api/v1/erp/purchase/orders".to_string(),
            request_path: "/api/v1/erp/purchase/orders".to_string(),
            response_status: 200,
            duration_ms: 150,
            client_ip: "192.168.1.100".to_string(),
            client_type: Some("Web".to_string()),
            user_id: Some(1),
            username: Some("zhangsan".to_string()),
        };
        
        let result = service.log_api_access(req).await;
        
        assert!(result.is_ok());
        let log_no = result.unwrap();
        assert!(log_no.starts_with("API"));
    }

    #[tokio::test]
    async fn test_query_operation_logs() {
        let db = setup_test_db().await;
        let service = LogService::new(Arc::new(db));
        
        let result = service.query_operation_logs(
            Some("采购管理".to_string()),
            Some("创建".to_string()),
            Some(1),
            None,
            None,
            1,
            10
        ).await;
        
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_query_system_logs() {
        let db = setup_test_db().await;
        let service = LogService::new(Arc::new(db));
        
        let result = service.query_system_logs(
            Some("ERROR".to_string()),
            Some("数据库".to_string()),
            None,
            None,
            1,
            10
        ).await;
        
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_query_login_logs() {
        let db = setup_test_db().await;
        let service = LogService::new(Arc::new(db));
        
        let result = service.query_login_logs(
            Some("zhangsan".to_string()),
            Some("success".to_string()),
            None,
            None,
            1,
            10
        ).await;
        
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_query_api_logs() {
        let db = setup_test_db().await;
        let service = LogService::new(Arc::new(db));
        
        let result = service.query_api_logs(
            Some("GET".to_string()),
            Some(200),
            None,
            None,
            1,
            10
        ).await;
        
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_log_workflow() {
        let db = setup_test_db().await;
        let service = LogService::new(Arc::new(db));
        
        let op_req = crate::services::log_service::LogOperationRequest {
            module: "采购管理".to_string(),
            operation_type: "创建".to_string(),
            operation_desc: "创建采购订单".to_string(),
            business_type: Some("purchase_order".to_string()),
            business_id: Some(1),
            user_id: 1,
            username: "zhangsan".to_string(),
            real_name: Some("张三".to_string()),
            request_method: Some("POST".to_string()),
            request_url: Some("/api/v1/erp/purchase/orders".to_string()),
            ip_address: Some("192.168.1.100".to_string()),
            duration_ms: Some(150),
        };
        
        let op_result = service.log_operation(op_req).await;
        assert!(op_result.is_ok());
        
        let login_req = crate::services::log_service::LogLoginRequest {
            username: "zhangsan".to_string(),
            login_result: "success".to_string(),
            ip_address: "192.168.1.100".to_string(),
            user_agent: Some("Mozilla/5.0".to_string()),
            failure_reason: None,
        };
        
        let login_result = service.log_login(login_req).await;
        assert!(login_result.is_ok());
        
        let api_req = crate::services::log_service::LogApiAccessRequest {
            request_id: "req-001".to_string(),
            request_method: "POST".to_string(),
            request_url: "/api/v1/erp/purchase/orders".to_string(),
            request_path: "/api/v1/erp/purchase/orders".to_string(),
            response_status: 201,
            duration_ms: 150,
            client_ip: "192.168.1.100".to_string(),
            client_type: Some("Web".to_string()),
            user_id: Some(1),
            username: Some("zhangsan".to_string()),
        };
        
        let api_result = service.log_api_access(api_req).await;
        assert!(api_result.is_ok());
    }
}
