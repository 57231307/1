use config::{Config, ConfigError, File};
use serde::Deserialize;

#[derive(Debug, Clone, Deserialize)]
pub struct AppSettings {
    pub server: ServerConfig,
    pub database: DatabaseConfig,
    pub auth: AuthConfig,
    pub grpc: GrpcConfig,
    pub log: LogConfig,
    pub cors: CorsConfig,
    pub env: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct ServerConfig {
    pub host: String,
    pub port: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct DatabaseConfig {
    pub connection_string: String,
    #[allow(dead_code)]
    pub host: String,
    #[allow(dead_code)]
    pub port: u16,
    #[allow(dead_code)]
    pub name: String,
    #[allow(dead_code)]
    pub username: String,
    #[allow(dead_code)]
    pub password: String,
    #[allow(dead_code)]
    pub max_connections: u32,
    #[allow(dead_code)]
    pub ssl_mode: String,
    #[allow(dead_code)]
    pub ssl_ca: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct AuthConfig {
    pub jwt_secret: String,
    #[allow(dead_code)]
    pub token_expiry_hours: i64,
}

#[derive(Debug, Clone, Deserialize)]
pub struct GrpcConfig {
    pub host: String,
    pub port: u16,
}

#[derive(Debug, Clone, Deserialize)]
pub struct LogConfig {
    pub level: String,
    pub dir: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct CorsConfig {
    pub allowed_origins: Vec<String>,
}

impl AppSettings {
    pub fn new() -> Result<Self, ConfigError> {
        let settings = Config::builder()
            .add_source(File::with_name("config").required(false))
            .add_source(File::with_name(".env").required(false))
            .add_source(config::Environment::default().separator("__"))
            .build()?;

        let mut app_settings: AppSettings = settings.try_deserialize()?;

        // 处理 CORS 允许来源的环境变量（逗号分隔列表）
        if let Ok(origins_str) = std::env::var("CORS__ALLOWED_ORIGINS") {
            app_settings.cors.allowed_origins = origins_str
                .split(',')
                .map(|s| s.trim().to_string())
                .filter(|s| !s.is_empty())
                .collect();
        }

        Ok(app_settings)
    }
}
