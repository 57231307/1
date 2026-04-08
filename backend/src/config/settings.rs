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
            .add_source(config::Environment::default().separator("__"))
            .build()?;

        // 尝试反序列化，如果失败则使用默认值
        let mut app_settings = match settings.try_deserialize::<AppSettings>() {
            Ok(settings) => settings,
            Err(e) => {
                println!("Failed to deserialize settings, falling back to defaults. Error: {}", e);
                // 使用默认值
                AppSettings {
                    server: ServerConfig {
                        host: "0.0.0.0".to_string(),
                        port: "8080".to_string(),
                    },
                    database: DatabaseConfig {
                        connection_string: "".to_string(),
                        host: "localhost".to_string(),
                        port: 5432,
                        name: "bingxi".to_string(),
                        username: "postgres".to_string(),
                        password: "".to_string(),
                        max_connections: 10,
                        ssl_mode: "prefer".to_string(),
                        ssl_ca: None,
                    },
                    auth: AuthConfig {
                        jwt_secret: "your-secret-key-change-in-production".to_string(),
                        token_expiry_hours: 24,
                    },
                    grpc: GrpcConfig {
                        host: "0.0.0.0".to_string(),
                        port: 50051,
                    },
                    log: LogConfig {
                        level: "info".to_string(),
                        dir: "./logs".to_string(),
                    },
                    cors: CorsConfig {
                        allowed_origins: vec!["http://localhost:3000".to_string()],
                    },
                    env: "development".to_string(),
                }
            }
        };

        // 处理 CORS 允许来源的环境变量（逗号分隔列表）
        if let Ok(origins_str) = std::env::var("CORS__ALLOWED_ORIGINS") {
            app_settings.cors.allowed_origins = origins_str
                .split(',')
                .map(|s| s.trim().to_string())
                .filter(|s| !s.is_empty())
                .collect();
        }

        // 确保数据库连接字符串存在
        if app_settings.database.connection_string.is_empty() {
            app_settings.database.connection_string = format!(
                "postgres://{}:{}@{}:{}/{}",
                app_settings.database.username,
                app_settings.database.password,
                app_settings.database.host,
                app_settings.database.port,
                app_settings.database.name
            );
        }

        Ok(app_settings)
    }
}
