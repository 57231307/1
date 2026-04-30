use sea_orm::DatabaseConnection;
use std::sync::Arc;

use crate::services::metrics_service::MetricsService;
use crate::utils::cache::AppCache;

/// 应用全局状态
#[derive(Clone)]
pub struct AppState {
    pub db: Arc<DatabaseConnection>,
    pub jwt_secret: String,
    pub cache: Arc<AppCache>,
    pub metrics: Arc<MetricsService>,
}

impl AppState {
    pub fn new(db: Arc<DatabaseConnection>, jwt_secret: String) -> Self {
        let metrics = MetricsService::new().expect("Failed to create metrics service");
        Self {
            db,
            jwt_secret,
            cache: AppCache::arc(),
            metrics: Arc::new(metrics),
        }
    }
}

impl Default for AppState {
    fn default() -> Self {
        let metrics = MetricsService::new().expect("Failed to create metrics service");
        Self {
            db: Arc::new(DatabaseConnection::Disconnected),
            jwt_secret: "default-secret".to_string(),
            cache: AppCache::arc(),
            metrics: Arc::new(metrics),
        }
    }
}
