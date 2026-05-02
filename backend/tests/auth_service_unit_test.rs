//! AuthService 单元测试

use bingxi_backend::services::auth_service::AuthService;

#[tokio::test]
async fn test_password_hashing_and_verification() {
    let password = "TestPassword123!";
    
    // 1. 测试密码哈希
    let hash_result = AuthService::hash_password(password);
    assert!(hash_result.is_ok());
    let hashed_password = hash_result.unwrap();
    
    // 哈希不应该等于明文
    assert_ne!(password, hashed_password);
    
    // 2. 测试密码验证（正确密码）
    let verify_success = AuthService::verify_password(password, &hashed_password);
    assert!(verify_success.is_ok());
    assert!(verify_success.unwrap());
    
    // 3. 测试密码验证（错误密码）
    let wrong_password = "WrongPassword123!";
    let verify_fail = AuthService::verify_password(wrong_password, &hashed_password);
    assert!(verify_fail.is_ok());
    assert!(!verify_fail.unwrap());
}

#[tokio::test]
async fn test_jwt_token_generation_and_validation() {
    let user_id = 999;
    let username = "test_user";
    let role_id = Some(1);
    let secret = "very-secure-test-secret-key-at-least-32-bytes-long";
    
    // 1. 测试 Token 生成
    let token_result = AuthService::generate_token(user_id, username, role_id, secret);
    assert!(token_result.is_ok());
    let token = token_result.unwrap();
    assert!(!token.is_empty());
    
    // 2. 测试 Token 验证（使用正确的 Secret）
    let claims_result = AuthService::validate_token_static(&token, secret);
    assert!(claims_result.is_ok());
    let claims = claims_result.unwrap();
    
    assert_eq!(claims.sub, user_id);
    assert_eq!(claims.username, username);
    assert_eq!(claims.role_id, role_id);
    
    // 3. 测试 Token 验证（使用错误的 Secret）
    let wrong_secret = "wrong-secure-test-secret-key-at-least-32-bytes-long";
    let bad_claims = AuthService::validate_token_static(&token, wrong_secret);
    assert!(bad_claims.is_err());
}