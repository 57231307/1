#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Utc;
    use sea_orm::{Database, DatabaseConnection};
    use std::sync::Arc;

    async fn setup_test_db() -> DatabaseConnection {
        let db_url = std::env::var("TEST_DATABASE_URL")
            .unwrap_or_else(|_| "sqlite::memory:".to_string());
        Database::connect(&db_url).await.unwrap()
    }

    #[tokio::test]
    async fn test_user_service_creation() {
        let db = setup_test_db().await;
        let user_service = UserService::new(Arc::new(db));
        
        assert!(user_service.db.lock().await.is_ok());
    }

    #[tokio::test]
    async fn test_create_user() {
        let db = setup_test_db().await;
        let user_service = UserService::new(Arc::new(db.clone()));
        
        let password_hash = AuthService::hash_password("password123").unwrap();
        
        let result = user_service.create_user(
            "test_user".to_string(),
            password_hash,
            Some("test@example.com".to_string()),
            Some("13800138000".to_string()),
            Some(1),
            Some(1),
        ).await;
        
        assert!(result.is_ok());
        let user = result.unwrap();
        assert_eq!(user.username, "test_user");
        assert_eq!(user.email, Some("test@example.com".to_string()));
        assert!(user.is_active);
    }

    #[tokio::test]
    async fn test_find_user_by_username() {
        let db = setup_test_db().await;
        let user_service = UserService::new(Arc::new(db.clone()));
        
        let password_hash = AuthService::hash_password("password123").unwrap();
        
        user_service.create_user(
            "find_test_user".to_string(),
            password_hash,
            None,
            None,
            None,
            None,
        ).await.unwrap();
        
        let result = user_service.find_by_username("find_test_user").await;
        
        assert!(result.is_ok());
        let user = result.unwrap();
        assert_eq!(user.username, "find_test_user");
    }

    #[tokio::test]
    async fn test_find_user_by_id() {
        let db = setup_test_db().await;
        let user_service = UserService::new(Arc::new(db.clone()));
        
        let password_hash = AuthService::hash_password("password123").unwrap();
        
        let user = user_service.create_user(
            "id_test_user".to_string(),
            password_hash,
            None,
            None,
            None,
            None,
        ).await.unwrap();
        
        let result = user_service.find_by_id(user.id).await;
        
        assert!(result.is_ok());
        let found_user = result.unwrap();
        assert_eq!(found_user.id, user.id);
        assert_eq!(found_user.username, "id_test_user");
    }

    #[tokio::test]
    async fn test_list_users() {
        let db = setup_test_db().await;
        let user_service = UserService::new(Arc::new(db.clone()));
        
        let (users, total) = user_service.list_users(0, 20).await.unwrap();
        
        assert!(total >= 0);
        assert!(users.len() as u64 <= total);
    }

    #[tokio::test]
    async fn test_update_last_login() {
        let db = setup_test_db().await;
        let user_service = UserService::new(Arc::new(db.clone()));
        
        let password_hash = AuthService::hash_password("password123").unwrap();
        
        let user = user_service.create_user(
            "login_test_user".to_string(),
            password_hash,
            None,
            None,
            None,
            None,
        ).await.unwrap();
        
        let result = user_service.update_last_login(user.id).await;
        
        assert!(result.is_ok());
        
        let updated_user = user_service.find_by_id(user.id).await.unwrap();
        assert!(updated_user.last_login_at.is_some());
    }
}
