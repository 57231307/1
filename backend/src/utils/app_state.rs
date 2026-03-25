use sea_orm::DatabaseConnection;
use std::sync::Arc;

/// 应用全局状态
#[derive(Clone)]
#[allow(dead_code)]
pub struct AppState {
    pub db: Arc<DatabaseConnection>,
}

impl Default for AppState {
    fn default() -> Self {
        Self {
            db: Arc::new(DatabaseConnection::Disconnected),
        }
    }
}
