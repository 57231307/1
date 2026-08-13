use bingxi_backend::container::AppState;
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
#[tokio::test]
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