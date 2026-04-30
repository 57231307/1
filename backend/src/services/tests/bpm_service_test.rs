//! BPM 流程引擎服务测试
//! 测试 BPM 流程引擎的核心功能

#[cfg(test)]
mod tests {
    use super::*;
    use crate::services::BpmService;
    use sea_orm::{Database, DatabaseConnection, JsonValue};
    use std::sync::Arc;

    async fn setup_test_db() -> DatabaseConnection {
        let db_url = std::env::var("TEST_DATABASE_URL")
            .unwrap_or_else(|_| "sqlite::memory:".to_string());
        Database::connect(&db_url).await.unwrap()
    }

    #[tokio::test]
    async fn test_start_process() {
        let db = setup_test_db().await;
        let service = BpmService::new(Arc::new(db));
        
        let req = crate::services::bpm_service::StartProcessRequest {
            process_key: "purchase_approval".to_string(),
            business_type: "purchase_order".to_string(),
            business_id: 1,
            title: "采购订单审批".to_string(),
            initiator_id: 1,
            initiator_name: "张三".to_string(),
            initiator_department_id: Some(1),
            priority: Some("normal".to_string()),
            form_data: Some(JsonValue::String("{}".to_string())),
            variables: Some(JsonValue::String("{}".to_string())),
        };
        
        let result = service.start_process(req).await;
        
        assert!(result.is_ok());
        let start_result = result.unwrap();
        assert!(start_result.instance_id > 0);
        assert!(start_result.instance_no.starts_with("BPM"));
        assert!(!start_result.task_ids.is_empty());
    }

    #[tokio::test]
    async fn test_approve_task() {
        let db = setup_test_db().await;
        let service = BpmService::new(Arc::new(db));
        
        let req = crate::services::bpm_service::ApproveTaskRequest {
            task_id: 1,
            handler_id: 2,
            handler_name: "李四".to_string(),
            action: "approve".to_string(),
            approval_opinion: Some("同意".to_string()),
            attachment_urls: None,
        };
        
        let result = service.approve_task(req).await;
        
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_query_user_tasks() {
        let db = setup_test_db().await;
        let service = BpmService::new(Arc::new(db));
        
        let result = service.query_user_tasks(
            1,
            Some("pending".to_string()),
            1,
            10
        ).await;
        
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_query_instances() {
        let db = setup_test_db().await;
        let service = BpmService::new(Arc::new(db));
        
        let result = service.query_instances(
            Some("purchase_approval".to_string()),
            Some(1),
            Some("running".to_string()),
            1,
            10
        ).await;
        
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_process_lifecycle() {
        let db = setup_test_db().await;
        let service = BpmService::new(Arc::new(db));
        
        let start_req = crate::services::bpm_service::StartProcessRequest {
            process_key: "purchase_approval".to_string(),
            business_type: "purchase_order".to_string(),
            business_id: 1,
            title: "采购订单审批".to_string(),
            initiator_id: 1,
            initiator_name: "张三".to_string(),
            initiator_department_id: Some(1),
            priority: Some("normal".to_string()),
            form_data: Some(JsonValue::String("{}".to_string())),
            variables: Some(JsonValue::String("{}".to_string())),
        };
        
        let start_result = service.start_process(start_req).await;
        assert!(start_result.is_ok());
        
        let instance_no = start_result.unwrap().instance_no;
        
        let instances = service.query_instances(
            Some("purchase_approval".to_string()),
            None,
            None,
            1,
            10
        ).await;
        
        assert!(instances.is_ok());
        let instances_list = instances.unwrap();
        assert!(!instances_list.is_empty());
        
        let found = instances_list.iter().any(|i| i.instance_no == instance_no);
        assert!(found);
    }
}
