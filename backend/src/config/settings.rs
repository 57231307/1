use config::{Config, ConfigError, File};
use serde::Deserialize;

#[derive(Debug, Clone, Deserialize, Default)]
pub struct AppSettings {
    #[serde(default)]
    pub server: ServerConfig,
    #[serde(default)]
    pub database: DatabaseConfig,
    #[serde(default)]
    pub auth: AuthConfig,
    #[serde(default)]
    pub grpc: GrpcConfig,
    #[serde(default)]
    pub log: LogConfig,
    #[serde(default)]
    pub cors: CorsConfig,
    #[serde(default = "default_env")]
    pub env: String,
}

fn default_env() -> String {
    "development".to_string()
}

#[derive(Debug, Clone, Deserialize)]
pub struct ServerConfig {
    #[serde(default = "default_host")]
    pub host: String,
    #[serde(default = "default_port")]
    pub port: String,
}

impl Default for ServerConfig {
    fn default() -> Self {
        Self {
            host: default_host(),
            port: default_port(),
        }
    }
}

fn default_host() -> String { "0.0.0.0".to_string() }
fn default_port() -> String { "8080".to_string() }

#[derive(Debug, Clone, Deserialize)]
pub struct DatabaseConfig {
    #[serde(default)]
    pub connection_string: String,
    #[allow(dead_code)]
    #[serde(default = "default_db_host")]
    pub host: String,
    #[allow(dead_code)]
    #[serde(default = "default_db_port")]
    pub port: u16,
    #[allow(dead_code)]
    #[serde(default = "default_db_name")]
    pub name: String,
    #[allow(dead_code)]
    #[serde(default = "default_db_user")]
    pub username: String,
    #[allow(dead_code)]
    #[serde(default)]
    pub password: String,
    #[allow(dead_code)]
    #[serde(default = "default_max_conn")]
    pub max_connections: u32,
    #[allow(dead_code)]
    #[serde(default = "default_ssl_mode")]
    pub ssl_mode: String,
    #[allow(dead_code)]
    #[serde(default)]
    pub ssl_ca: Option<String>,
}

impl Default for DatabaseConfig {
    fn default() -> Self {
        Self {
            connection_string: "".to_string(),
            host: default_db_host(),
            port: default_db_port(),
            name: default_db_name(),
            username: default_db_user(),
            password: "".to_string(),
            max_connections: default_max_conn(),
            ssl_mode: default_ssl_mode(),
            ssl_ca: None,
        }
    }
}

fn default_db_host() -> String { "localhost".to_string() }
fn default_db_port() -> u16 { 5432 }
fn default_db_name() -> String { "bingxi".to_string() }
fn default_db_user() -> String { "postgres".to_string() }
fn default_max_conn() -> u32 { 10 }
fn default_ssl_mode() -> String { "prefer".to_string() }

#[derive(Debug, Clone, Deserialize)]
pub struct AuthConfig {
    #[serde(default = "default_jwt")]
    pub jwt_secret: String,
    #[allow(dead_code)]
    #[serde(default = "default_expiry")]
    pub token_expiry_hours: i64,
}

impl Default for AuthConfig {
    fn default() -> Self {
        Self {
            jwt_secret: default_jwt(),
            token_expiry_hours: default_expiry(),
        }
    }
}

fn default_jwt() -> String { "your-super-secret-jwt-key-change-this-in-production".to_string() }
fn default_expiry() -> i64 { 24 }

#[derive(Debug, Clone, Deserialize)]
pub struct GrpcConfig {
    #[serde(default = "default_host")]
    pub host: String,
    #[serde(default = "default_grpc_port")]
    pub port: u16,
}

impl Default for GrpcConfig {
    fn default() -> Self {
        Self {
            host: default_host(),
            port: default_grpc_port(),
        }
    }
}

fn default_grpc_port() -> u16 { 50051 }

#[derive(Debug, Clone, Deserialize)]
pub struct LogConfig {
    #[serde(default = "default_log_level")]
    pub level: String,
    #[serde(default = "default_log_dir")]
    pub dir: String,
}

impl Default for LogConfig {
    fn default() -> Self {
        Self {
            level: default_log_level(),
            dir: default_log_dir(),
        }
    }
}

fn default_log_level() -> String { "info".to_string() }
fn default_log_dir() -> String { "/var/log/bingxi-erp".to_string() }

#[derive(Debug, Clone, Deserialize)]
pub struct CorsConfig {
    #[serde(default = "default_cors")]
    pub allowed_origins: Vec<String>,
}

impl Default for CorsConfig {
    fn default() -> Self {
        Self {
            allowed_origins: default_cors(),
        }
    }
}

fn default_cors() -> Vec<String> { vec!["http://localhost:3000".to_string()] }

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
