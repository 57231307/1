use bingxi_backend::container::AppState;
use bingxi_backend::handlers::init_handler::require_admin_role;
use bingxi_backend::middleware::auth_context::AuthContext;
use bingxi_backend::utils::error::AppError;

/// V15 P2 14.11-H：require_admin_role 二次校验测试
/// 场景 1：缺角色用户（role_id=None）→ PermissionDenied（fail-closed，不依赖 DB）
#[tokio::test]
async fn test_require_admin_role_rejects_missing_role() {
    let state = AppState::default();
    let auth = AuthContext {
        user_id: 42,
        username: "no_role_user".to_string(),
        role_id: None,
        department_id: None,
        data_scope: None,
    };
    let result = require_admin_role(&state, &auth).await;
    assert!(
        matches!(result, Err(AppError::PermissionDenied(_))),
        "缺角色用户调用 require_admin_role 应返回 PermissionDenied，实际: {:?}",
        result
    );
}

/// 场景 2：非 admin 角色（role_id=999，mock DB 无角色记录 fail-closed）→ PermissionDenied
/// 注：需要真实的 PostgreSQL 连接（AppState::default() 的 Disconnected 连接会导致 sea-orm panic）
#[tokio::test]
#[ignore = "需要真实的 PostgreSQL 连接（is_admin_role 查询 DB）"]
async fn test_require_admin_role_rejects_non_admin() {
    let state = AppState::default();
    let auth = AuthContext {
        user_id: 43,
        username: "operator_user".to_string(),
        role_id: Some(999),
        department_id: None,
        data_scope: None,
    };
    let result = require_admin_role(&state, &auth).await;
    assert!(
        matches!(result, Err(AppError::PermissionDenied(_))),
        "非 admin 用户调用 require_admin_role 应返回 PermissionDenied，实际: {:?}",
        result
    );
}
