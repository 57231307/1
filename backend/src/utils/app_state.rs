use sea_orm::DatabaseConnection;
use std::sync::Arc;

use crate::services::metrics_service::MetricsService;
use crate::utils::cache::AppCache;

/// 应用全局状态
#[derive(Clone)]
pub struct AppState {
    pub db: Arc<DatabaseConnection>,
    pub jwt_secret: String,
    pub previous_jwt_secret: Option<String>,
    pub cookie_secret: String,
    pub cache: Arc<AppCache>,
    pub metrics: Arc<MetricsService>,
}

impl AppState {
    pub fn new(db: Arc<DatabaseConnection>, jwt_secret: String) -> Self {
        Self::with_secrets(db, jwt_secret.clone(), None, jwt_secret)
    }

    pub fn with_secrets(db: Arc<DatabaseConnection>, jwt_secret: String, previous_jwt_secret: Option<String>, cookie_secret: String) -> Self {
        let mut final_cookie_secret = cookie_secret;
        if final_cookie_secret.len() < 32 {
            tracing::warn!(
                "配置警告: cookie_secret 长度不足 32 字节 (当前长度: {})。这会降低系统的加密安全性。已自动为您填充为 32 字节以保证服务启动，请尽快在生产环境更换！",
                final_cookie_secret.len()
            );
            final_cookie_secret.push_str(&"0".repeat(32 - final_cookie_secret.len()));
        }
        
        let metrics = MetricsService::new().expect("Failed to create metrics service");
        Self {
            db,
            jwt_secret,
            previous_jwt_secret,
            cookie_secret: final_cookie_secret,
            cache: AppCache::arc(),
            metrics: Arc::new(metrics),
        }
    }
}

impl Default for AppState {
    fn default() -> Self {
        // 在 default 初始化中强制使用 32 字节的安全测试密钥，避免 panic
        let metrics = MetricsService::new().expect("Failed to create metrics service");
        Self {
            db: Arc::new(DatabaseConnection::Disconnected),
            jwt_secret: "default-secret-key-for-test-environments-only-32-bytes".to_string(),
            previous_jwt_secret: None,
            cookie_secret: "default-cookie-secret-key-for-test-environments-only-32-bytes".to_string(),
            cache: AppCache::arc(),
            metrics: Arc::new(metrics),
        }
    }
}
