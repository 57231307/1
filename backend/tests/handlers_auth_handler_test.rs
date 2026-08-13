use bingxi_backend::handlers::auth_handler::*;
use std::collections::HashSet;


/// 构造测试用的 LoginResponse 实例
fn build_test_login_response() -> LoginResponse {
    LoginResponse {
        csrf_token: "csrf-token-uuid".to_string(),
        user: UserInfo {
            id: 42,
            username: "test_user".to_string(),
            email: Some("test@example.com".to_string()),
            role_id: Some(1),
            role_name: Some("admin".to_string()),
            permissions: vec![
                "user.list:read".to_string(),
                "user.list:write".to_string(),
                "order:read".to_string(),
            ],
            // 批次 29 v7 P0-4+5：补全新增的 6 个字段（与生产构造保持一致）
            phone: Some("13800000000".to_string()),
            department_id: Some(1),
            department_name: Some("研发部".to_string()),
            is_totp_enabled: false,
            real_name: Some("测试用户".to_string()),
            avatar: None,
            agreed_to_terms_at: None,
        },
        permissions: vec![
            "user.list:read".to_string(),
            "user.list:write".to_string(),
            "order:read".to_string(),
        ],
        password_expired: false,
    }
}

/// 测试 #10：LoginResponse JSON 序列化结果不含 `token` 字段 原因：access_token
/// 已通过 httpOnly Cookie 写入响应，响应体再含 token 字段会增加 XSS/中间人/前端日志泄露的攻击面
#[test]
fn test_login_response_omits_token_field() {
    let response = build_test_login_response();
    let json = serde_json::to_value(&response).expect("LoginResponse 序列化失败");

    // 响应体不应包含 `token` 字段
    assert!(
        json.get("token").is_none(),
        "LoginResponse 序列化结果不应包含 `token` 字段，实际 JSON = {}",
        json
    );
}

/// 测试 #13：LoginResponse JSON 序列化结果不含 `refresh_token` 字段 原因：refresh_token
/// 已通过 httpOnly Cookie 写入响应，响应体再含 refresh_token 字段 同样会增加泄露风险
#[test]
fn test_login_response_omits_refresh_token_field() {
    let response = build_test_login_response();
    let json = serde_json::to_value(&response).expect("LoginResponse 序列化失败");

    // 响应体不应包含 `refresh_token` 字段
    assert!(
        json.get("refresh_token").is_none(),
        "LoginResponse 序列化结果不应包含 `refresh_token` 字段，实际 JSON = {}",
        json
    );
}

/// 测试 #14：LoginResponse 的 `permissions` 字段是 `Vec<String>` 类型
/// 验证资源标识符格式 `"{resource}:{action}"`，且不暴露内部 `resource_id` 主键
#[test]
fn test_login_response_permissions_is_string_array() {
    let response = build_test_login_response();
    let json = serde_json::to_value(&response).expect("LoginResponse 序列化失败");

    // 验证 permissions 字段存在
    let permissions = json
        .get("permissions")
        .expect("LoginResponse 应包含 `permissions` 字段")
        .as_array()
        .expect("`permissions` 字段类型应为 JSON 数组");

    // 验证数组元素全部为字符串（不是对象）
    assert_eq!(permissions.len(), 3, "测试数据应包含 3 个权限项");
    for (i, perm) in permissions.iter().enumerate() {
        assert!(
            perm.is_string(),
            "`permissions[{}]` 必须是字符串，实际类型 = {:?}",
            i,
            perm
        );
    }

    // 验证资源标识符格式 `"{resource}:{action}"`
    assert_eq!(permissions[0].as_str(), Some("user.list:read"));
    assert_eq!(permissions[1].as_str(), Some("user.list:write"));
    assert_eq!(permissions[2].as_str(), Some("order:read"));

    // 验证 permissions 元素是对象时不存在（防止回归到 `Vec<UserPermissionDto>` 形态）
    assert!(
        permissions[0].as_object().is_none(),
        "`permissions` 元素不应为对象，回归到 `Vec<UserPermissionDto>` 形态"
    );
}

/// 综合测试：LoginResponse 序列化结果的字段白名单
/// 只允许包含 `csrf_token` / `user` / `permissions` 三个字段
#[test]
fn test_login_response_field_whitelist() {
    let response = build_test_login_response();
    let json = serde_json::to_value(&response).expect("LoginResponse 序列化失败");
    let obj = json
        .as_object()
        .expect("LoginResponse 应序列化为 JSON 对象");

    let actual_fields: std::collections::HashSet<&String> = obj.keys().collect();
    let expected_fields: std::collections::HashSet<&str> =
        ["csrf_token", "user", "permissions"].into_iter().collect();

    let extra: Vec<&&String> = actual_fields
        .iter()
        .filter(|f| !expected_fields.contains(f.as_str()))
        .collect();
    assert!(
        extra.is_empty(),
        "LoginResponse 应仅包含白名单字段，发现额外字段: {:?}",
        extra
    );
}