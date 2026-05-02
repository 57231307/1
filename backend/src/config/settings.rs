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
    pub host: String,
    pub port: u16,
    pub name: String,
    pub username: String,
    pub password: String,
    pub max_connections: u32,
    pub ssl_mode: String,
    pub ssl_ca: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct AuthConfig {
    pub jwt_secret: String,
    pub previous_jwt_secret: Option<String>,
    pub cookie_secret: Option<String>,
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
        let config_builder = Config::builder()
            .add_source(File::with_name("config").required(false))
            .add_source(File::with_name(".env").required(false))
            .add_source(config::Environment::default().separator("__"));

        let settings = match config_builder.build() {
            Ok(c) => c,
            Err(e) => {
                eprintln!("警告: 无法加载配置文件: {}", e);
                eprintln!("将尝试从环境变量加载配置");
                // 尝试仅从环境变量构建
                match Config::builder()
                    .add_source(config::Environment::default().separator("__"))
                    .build()
                {
                    Ok(c) => c,
                    Err(_) => {
                        panic!("无法加载配置，系统启动失败");
                    }
                }
            }
        };

        let mut app_settings: AppSettings = match settings.try_deserialize() {
            Ok(s) => s,
            Err(e) => {
                panic!("配置解析失败: {}", e);
            }
        };

        // 强制检查 JWT 密钥强度
        if app_settings.auth.jwt_secret.len() < 32 || app_settings.auth.jwt_secret.contains("change-in-production") || app_settings.auth.jwt_secret.contains("change-this") {
            panic!("致命错误: JWT 密钥强度不足或使用默认密钥！生产环境必须提供至少 32 字节的安全随机密钥。");
        }

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
