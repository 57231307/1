#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Utc;
    use sea_orm::{Database, DatabaseConnection};
    use std::sync::Arc;

    async fn setup_test_db() -> DatabaseConnection {
        // 使用测试数据库
        let db_url = std::env::var("TEST_DATABASE_URL")
            .unwrap_or_else(|_| "sqlite::memory:".to_string());
        Database::connect(&db_url).await.unwrap()
    }

    #[tokio::test]
    async fn test_auth_service_creation() {
        let db = setup_test_db().await;
        let secret = "test_secret".to_string();
        
        let auth_service = AuthService::new(Arc::new(db), secret);
        
        assert!(auth_service.get_secret().len() > 0);
    }

    #[tokio::test]
    async fn test_password_hashing() {
        let password = "test_password_123";
        
        let hash_result = AuthService::hash_password(password);
        
        assert!(hash_result.is_ok());
        let hash = hash_result.unwrap();
        assert_ne!(password, hash);
        assert!(hash.len() > 50);
    }

    #[tokio::test]
    async fn test_password_verification() {
        let password = "test_password_123";
        let hash = AuthService::hash_password(password).unwrap();
        
        let db = setup_test_db().await;
        let secret = "test_secret".to_string();
        let auth_service = AuthService::new(Arc::new(db), secret);
        
        let result = auth_service.verify_password(password, &hash);
        assert!(result);
        
        let wrong_result = auth_service.verify_password("wrong_password", &hash);
        assert!(!wrong_result);
    }

    #[tokio::test]
    async fn test_token_generation() {
        let db = setup_test_db().await;
        let secret = "test_secret".to_string();
        let auth_service = AuthService::new(Arc::new(db), secret.clone());
        
        let user = user::Model {
            id: 1,
            username: "test_user".to_string(),
            password_hash: "hash".to_string(),
            email: Some("test@example.com".to_string()),
            phone: None,
            role_id: Some(1),
            department_id: None,
            is_active: true,
            last_login_at: None,
            created_at: Utc::now(),
            updated_at: Utc::now(),
        };
        
        let token_result = auth_service.generate_token(
            user.id,
            &user.username,
            user.role_id,
        );
        
        assert!(token_result.is_ok());
        let token = token_result.unwrap();
        assert!(token.len() > 100);
        
        let validation_result = auth_service.validate_token(&token);
        assert!(validation_result.is_ok());
        
        let claims = validation_result.unwrap();
        assert_eq!(claims.sub, 1);
        assert_eq!(claims.username, "test_user");
    }

    #[tokio::test]
    async fn test_invalid_token() {
        let db = setup_test_db().await;
        let secret = "test_secret".to_string();
        let auth_service = AuthService::new(Arc::new(db), secret);
        
        let invalid_token = "invalid.token.here";
        let result = auth_service.validate_token(invalid_token);
        
        assert!(result.is_err());
    }
}
