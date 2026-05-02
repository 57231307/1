//! 密码强度验证单元测试

use bingxi_backend::handlers::user_handler::CreateUserRequest;
use validator::Validate;

#[test]
fn test_password_strength_validation() {
    // 1. 测试太短的密码
    let req1 = CreateUserRequest {
        username: "test_user".to_string(),
        password: "Short1!".to_string(),
        email: None,
        phone: None,
        role_id: None,
        department_id: None,
        is_active: None,
    };
    assert!(req1.validate().is_err());

    // 2. 测试没有特殊字符的密码
    let req2 = CreateUserRequest {
        username: "test_user".to_string(),
        password: "NoSpecialChar123".to_string(),
        email: None,
        phone: None,
        role_id: None,
        department_id: None,
        is_active: None,
    };
    assert!(req2.validate().is_err());

    // 3. 测试没有数字的密码
    let req3 = CreateUserRequest {
        username: "test_user".to_string(),
        password: "NoDigitSpecial!".to_string(),
        email: None,
        phone: None,
        role_id: None,
        department_id: None,
        is_active: None,
    };
    assert!(req3.validate().is_err());

    // 4. 测试合规的密码
    let req4 = CreateUserRequest {
        username: "test_user".to_string(),
        password: "ValidPassword123!".to_string(),
        email: None,
        phone: None,
        role_id: None,
        department_id: None,
        is_active: None,
    };
    assert!(req4.validate().is_ok());
}