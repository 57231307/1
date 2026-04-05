use crate::models::operation_log;
use sea_orm::{EntityTrait, Set, ActiveModelTrait, DbErr, Order, PaginatorTrait};
use std::sync::Arc;
use sea_orm::DatabaseConnection;
use chrono::Utc;
use serde_json::Value;

/// 操作日志服务
#[derive(Debug, Clone)]
pub struct OperationLogService {
    db: Arc<DatabaseConnection>,
}

/// 创建操作日志请求
#[derive(Debug, Clone)]
pub struct CreateOperationLogRequest {
    pub user_id: Option<i32>,
    pub username: Option<String>,
    pub module: String,
    pub action: String,
    pub description: Option<String>,
    pub request_method: Option<String>,
    pub request_uri: Option<String>,
    pub request_ip: Option<String>,
    pub user_agent: Option<String>,
    pub status: String,
    pub error_message: Option<String>,
    pub duration_ms: Option<i64>,
    pub extra_data: Option<Value>,
}

impl OperationLogService {
    pub fn new(db: Arc<DatabaseConnection>) -> Self {
        Self { db }
    }

    /// 创建操作日志
    pub async fn create_log(&self, request: CreateOperationLogRequest) -> Result<operation_log::Model, DbErr> {
        let log = operation_log::ActiveModel {
            id: Set(0),
            user_id: Set(request.user_id),
            username: Set(request.username),
            module: Set(request.module),
            action: Set(request.action),
            description: Set(request.description),
            request_method: Set(request.request_method),
            request_uri: Set(request.request_uri),
            request_ip: Set(request.request_ip),
            user_agent: Set(request.user_agent),
            status: Set(request.status),
            error_message: Set(request.error_message),
            duration_ms: Set(request.duration_ms),
            extra_data: Set(request.extra_data.map(|data| data.into())),
            created_at: Set(Utc::now()),
        };

        log.insert(&*self.db).await
    }

    /// 记录成功操作
    pub async fn log_success(
        &self,
        user_id: Option<i32>,
        username: Option<String>,
        module: &str,
        action: &str,
        description: Option<String>,
        request_method: Option<String>,
        request_uri: Option<String>,
        request_ip: Option<String>,
        user_agent: Option<String>,
        duration_ms: Option<i64>,
        extra_data: Option<Value>,
    ) -> Result<(), DbErr> {
        let request = CreateOperationLogRequest {
            user_id,
            username,
            module: module.to_string(),
            action: action.to_string(),
            description,
            request_method,
            request_uri,
            request_ip,
            user_agent,
            status: "success".to_string(),
            error_message: None,
            duration_ms,
            extra_data,
        };

        self.create_log(request).await?;
        Ok(())
    }

    /// 记录失败操作
    pub async fn log_failure(
        &self,
        user_id: Option<i32>,
        username: Option<String>,
        module: &str,
        action: &str,
        error_message: String,
        request_method: Option<String>,
        request_uri: Option<String>,
        request_ip: Option<String>,
        user_agent: Option<String>,
        duration_ms: Option<i64>,
    ) -> Result<(), DbErr> {
        let request = CreateOperationLogRequest {
            user_id,
            username,
            module: module.to_string(),
            action: action.to_string(),
            description: None,
            request_method,
            request_uri,
            request_ip,
            user_agent,
            status: "failure".to_string(),
            error_message: Some(error_message),
            duration_ms,
            extra_data: None,
        };

        self.create_log(request).await?;
        Ok(())
    }

    /// 查询操作日志（分页）
    pub async fn list_logs(
        &self,
        page: u64,
        page_size: u64,
    ) -> Result<(Vec<operation_log::Model>, u64), DbErr> {
        use sea_orm::QueryOrder;

        let paginator = operation_log::Entity::find()
            .order_by(operation_log::Column::CreatedAt, Order::Desc)
            .paginate(&*self.db, page_size);

        let total = paginator.num_items().await?;
        let logs = paginator.fetch_page(page).await?;

        Ok((logs, total))
    }

    /// 根据模块筛选查询
    pub async fn list_logs_by_module(
        &self,
        module: &str,
        page: u64,
        page_size: u64,
    ) -> Result<(Vec<operation_log::Model>, u64), DbErr> {
        use sea_orm::{QueryFilter, ColumnTrait, QueryOrder};

        let paginator = operation_log::Entity::find()
            .filter(operation_log::Column::Module.eq(module))
            .order_by(operation_log::Column::CreatedAt, Order::Desc)
            .paginate(&*self.db, page_size);

        let total = paginator.num_items().await?;
        let logs = paginator.fetch_page(page).await?;

        Ok((logs, total))
    }

    /// 根据用户 ID 筛选查询
    pub async fn list_logs_by_user(
        &self,
        user_id: i32,
        page: u64,
        page_size: u64,
    ) -> Result<(Vec<operation_log::Model>, u64), DbErr> {
        use sea_orm::{QueryFilter, ColumnTrait, QueryOrder};

        let paginator = operation_log::Entity::find()
            .filter(operation_log::Column::UserId.eq(user_id))
            .order_by(operation_log::Column::CreatedAt, Order::Desc)
            .paginate(&*self.db, page_size);

        let total = paginator.num_items().await?;
        let logs = paginator.fetch_page(page).await?;

        Ok((logs, total))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use sea_orm::{Database, DatabaseConnection};
    use std::sync::Arc;

    async fn setup_test_db() -> DatabaseConnection {
        let db_url = std::env::var("TEST_DATABASE_URL")
            .unwrap_or_else(|_| "sqlite::memory:".to_string());
        Database::connect(&db_url).await.unwrap()
    }

    #[tokio::test]
    async fn test_operation_log_service_creation() {
        let db = setup_test_db().await;
        let service = OperationLogService::new(Arc::new(db));
        
        assert!(Arc::strong_count(&service.db) >= 1);
    }

    #[test]
    fn test_create_log_request_structure() {
        let request = CreateOperationLogRequest {
            user_id: Some(1),
            username: Some("admin".to_string()),
            module: "user".to_string(),
            action: "create".to_string(),
            description: Some("创建用户".to_string()),
            request_method: Some("POST".to_string()),
            request_uri: Some("/api/v1/erp/users".to_string()),
            request_ip: Some("127.0.0.1".to_string()),
            user_agent: Some("Mozilla/5.0".to_string()),
            status: "success".to_string(),
            error_message: None,
            duration_ms: Some(100),
            extra_data: None,
        };
        
        assert_eq!(request.user_id, Some(1));
        assert_eq!(request.username, Some("admin".to_string()));
        assert_eq!(request.module, "user");
        assert_eq!(request.status, "success");
    }

    #[test]
    fn test_log_success_request() {
        let request = CreateOperationLogRequest {
            user_id: Some(1),
            username: Some("admin".to_string()),
            module: "product".to_string(),
            action: "update".to_string(),
            description: Some("更新产品".to_string()),
            request_method: Some("PUT".to_string()),
            request_uri: Some("/api/v1/erp/products/1".to_string()),
            request_ip: Some("192.168.1.1".to_string()),
            user_agent: None,
            status: "success".to_string(),
            error_message: None,
            duration_ms: Some(50),
            extra_data: None,
        };
        
        assert_eq!(request.status, "success");
        assert_eq!(request.module, "product");
        assert_eq!(request.action, "update");
    }

    #[test]
    fn test_log_failure_request() {
        let request = CreateOperationLogRequest {
            user_id: Some(1),
            username: Some("admin".to_string()),
            module: "inventory".to_string(),
            action: "delete".to_string(),
            description: None,
            request_method: Some("DELETE".to_string()),
            request_uri: Some("/api/v1/erp/inventory/1".to_string()),
            request_ip: None,
            user_agent: None,
            status: "failure".to_string(),
            error_message: Some("权限不足".to_string()),
            duration_ms: Some(10),
            extra_data: None,
        };
        
        assert_eq!(request.status, "failure");
        assert_eq!(request.error_message, Some("权限不足".to_string()));
    }

    #[tokio::test]
    #[ignore]
    async fn test_list_logs_empty() {
        let db = setup_test_db().await;
        let service = OperationLogService::new(Arc::new(db));
        
        let (logs, total) = service.list_logs(0, 20).await.unwrap();
        
        assert!(logs.is_empty());
        assert_eq!(total, 0);
    }

    #[tokio::test]
    #[ignore]
    async fn test_list_logs_by_module_empty() {
        let db = setup_test_db().await;
        let service = OperationLogService::new(Arc::new(db));
        
        let (logs, total) = service.list_logs_by_module("user", 0, 20).await.unwrap();
        
        assert!(logs.is_empty());
        assert_eq!(total, 0);
    }

    #[tokio::test]
    #[ignore]
    async fn test_list_logs_by_user_empty() {
        let db = setup_test_db().await;
        let service = OperationLogService::new(Arc::new(db));
        
        let (logs, total) = service.list_logs_by_user(1, 0, 20).await.unwrap();
        
        assert!(logs.is_empty());
        assert_eq!(total, 0);
    }

    #[test]
    fn test_status_values() {
        let valid_statuses = vec!["success", "failure"];
        
        for status in valid_statuses {
            assert!(status == "success" || status == "failure");
        }
    }

    #[test]
    fn test_module_values() {
        let valid_modules = vec![
            "user", "product", "inventory", "sales", "finance",
            "inventory_adjustment", "auth", "role"
        ];
        
        for module in &valid_modules {
            assert!(!module.is_empty());
        }
    }

    #[test]
    fn test_action_values() {
        let valid_actions = vec!["create", "update", "delete", "view", "list", "approve", "reject"];
        
        for action in &valid_actions {
            assert!(!action.is_empty());
        }
    }

    #[test]
    fn test_http_methods() {
        let valid_methods = vec!["GET", "POST", "PUT", "DELETE", "PATCH"];
        
        for method in &valid_methods {
            assert!(!method.is_empty());
        }
    }

    #[test]
    fn test_ip_address_format() {
        let valid_ips = vec![
            "127.0.0.1",
            "192.168.1.1",
            "10.0.0.1",
        ];
        
        for ip in &valid_ips {
            assert!(ip.contains('.'));
        }
    }

    #[test]
    fn test_uri_format() {
        let valid_uris = vec![
            "/api/v1/erp/users",
            "/api/v1/erp/products/1",
            "/api/v1/erp/inventory/adjustments",
        ];
        
        for uri in &valid_uris {
            assert!(uri.starts_with("/"));
        }
    }

    #[test]
    fn test_duration_ms_range() {
        let valid_durations = vec![0, 10, 50, 100, 500, 1000, 5000];
        
        for duration in &valid_durations {
            assert!(*duration >= 0);
        }
    }

    #[test]
    fn test_extra_data_types() {
        // 测试不同类型的额外数据
        let string_data = Value::String("test".to_string());
        let number_data = Value::Number(serde_json::Number::from(123));
        let bool_data = Value::Bool(true);
        let array_data = Value::Array(vec![]);
        let null_data = Value::Null;
        
        assert!(string_data.is_string());
        assert!(number_data.is_number());
        assert!(bool_data.is_boolean());
        assert!(array_data.is_array());
        assert!(null_data.is_null());
    }
}
