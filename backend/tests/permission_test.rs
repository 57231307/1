use sea_orm::{DatabaseBackend, MockDatabase, MockExecResult};
use std::sync::Arc;

use bingxi_backend::services::role_permission_service::RolePermissionService;

#[tokio::test]
async fn test_permission_check() {
    // We use MockDatabase to test the logic
    let db = MockDatabase::new(DatabaseBackend::Postgres)
        .append_query_results([
            // Empty results for check_permission to simulate no permission
            vec![],
        ])
        .into_connection();

    let service = RolePermissionService::new(Arc::new(db));

    // check_permission(role_id, resource_type, action, resource_id)
    let admin_role_id = 1;
    
    // Simulate check
    let has_permission = service
        .check_permission(admin_role_id, "sales_order", "delete", None)
        .await
        .unwrap();

    assert!(!has_permission, "Should default to false when no permission record exists");
}
