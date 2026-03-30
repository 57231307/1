use sea_orm::DatabaseConnection;
use std::sync::Arc;

/// 应用全局状态
#[derive(Clone)]
#[allow(dead_code)]
pub struct AppState {
    pub db: Arc<DatabaseConnection>,
    pub jwt_secret: String,
}

impl AppState {
    pub fn new(db: Arc<DatabaseConnection>, jwt_secret: String) -> Self {
        Self { db, jwt_secret }
    }
}

impl Default for AppState {
    fn default() -> Self {
        Self {
            db: Arc::new(DatabaseConnection::Disconnected),
            jwt_secret: "default-secret".to_string(),
        }
    }
}
