#![allow(dead_code)]
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
    pub redis: RedisConfig,
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

#[derive(Debug, Clone, Deserialize)]
pub struct RedisConfig {
    pub url: String,
    pub max_connections: usize,
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
                panic!("配置解析失败：{}", e);
            }
        };

        app_settings.load_sensitive_from_env();

        if !Self::validate_jwt_secret(&app_settings.auth.jwt_secret) {
            panic!("致命错误：JWT 密钥强度不足或使用默认密钥！生产环境必须提供至少 32 字节的安全随机密钥，且不能包含常见弱模式。");
        }

        let env = app_settings.env.to_lowercase();
        if env == "production" && app_settings.auth.cookie_secret.is_none() {
            panic!("生产环境必须配置独立的 auth.cookie_secret，不能降级使用 jwt_secret");
        }

        if let Ok(origins_str) = std::env::var("CORS__ALLOWED_ORIGINS") {
            app_settings.cors.allowed_origins = origins_str
                .split(',')
                .map(|s| s.trim().to_string())
                .filter(|s| !s.is_empty())
                .collect();
        }

        if app_settings.cors.allowed_origins.is_empty() {
            tracing::warn!(
                "安全警告: 未配置 CORS 允许来源，默认仅允许 localhost。请在生产环境配置正确的 CORS__ALLOWED_ORIGINS 或 cors.allowed_origins"
            );
            app_settings.cors.allowed_origins = vec![
                "http://localhost:3000".to_string(),
                "http://127.0.0.1:3000".to_string(),
            ];
        }

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

    fn load_sensitive_from_env(&mut self) {
        if let Ok(password) = std::env::var("DATABASE_PASSWORD") {
            self.database.password = password;
        }

        if let Ok(jwt_secret) = std::env::var("JWT_SECRET") {
            self.auth.jwt_secret = jwt_secret;
        }

        if let Ok(cookie_secret) = std::env::var("COOKIE_SECRET") {
            self.auth.cookie_secret = Some(cookie_secret);
        }

        if let Ok(prev_secret) = std::env::var("PREVIOUS_JWT_SECRET") {
            self.auth.previous_jwt_secret = Some(prev_secret);
        }

        if let Ok(audit_secret) = std::env::var("AUDIT_SECRET_KEY") {
            if audit_secret.len() < 32 {
                panic!("AUDIT_SECRET_KEY 必须至少 32 字节");
            }
        }
    }

    fn validate_jwt_secret(secret: &str) -> bool {
        if secret.len() < 32 {
            return false;
        }

        let weak_patterns = [
            "change-in-production",
            "change-this",
            "local-dev",
            "your_secure",
            "default",
            "test",
            "example",
        ];

        let secret_lower = secret.to_lowercase();
        for pattern in &weak_patterns {
            if secret_lower.contains(pattern) {
                return false;
            }
        }

        let unique_chars: std::collections::HashSet<char> = secret.chars().collect();
        let entropy_ratio = unique_chars.len() as f64 / secret.len() as f64;
        entropy_ratio > 0.3
    }
}
