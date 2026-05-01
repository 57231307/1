use bingxi_erp_backend::handlers::user_handler::CreateUserRequest;
use bingxi_erp_backend::handlers::warehouse_handler::CreateWarehouseRequest;
use validator::Validate;

#[test]
fn test_create_user_validation() {
    // 边界条件：太短的用户名
    let req1 = CreateUserRequest {
        username: "ab".to_string(), // < 3
        password: "password123".to_string(),
        email: None,
        phone: Some("123456789".to_string()),
        role_id: None,
        department_id: None,
    };
    assert!(req1.validate().is_err());

    // 边界条件：弱密码
    let req2 = CreateUserRequest {
        username: "admin".to_string(),
        password: "short".to_string(), // < 8
        email: None,
        phone: Some("123456789".to_string()),
        role_id: None,
        department_id: None,
    };
    assert!(req2.validate().is_err());

    // 边界条件：非法邮箱
    let req3 = CreateUserRequest {
        username: "admin".to_string(),
        password: "password123".to_string(),
        email: Some("invalid-email".to_string()),
        phone: Some("123456789".to_string()),
        role_id: None,
        department_id: None,
    };
    assert!(req3.validate().is_err());

    // 合法请求
    let valid_req = CreateUserRequest {
        username: "admin_user".to_string(),
        password: "password123".to_string(),
        email: Some("admin@example.com".to_string()),
        phone: Some("123456789".to_string()),
        role_id: None,
        department_id: None,
    };
    assert!(valid_req.validate().is_ok());
}

#[test]
fn test_warehouse_validation() {
    let req = CreateWarehouseRequest {
        name: "".to_string(), // empty
        code: "WH01".to_string(),
        address: None,
        manager: None,
        phone: None,
        capacity: None,
        description: None,
    };
    assert!(req.validate().is_err());
}
