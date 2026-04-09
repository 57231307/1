use config::{Config, Environment, File};
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
    pub host: String,
    pub port: String,
}

impl Default for ServerConfig {
    fn default() -> Self {
        Self {
            host: "0.0.0.0".to_string(),
            port: "8080".to_string(),
        }
    }
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

impl Default for DatabaseConfig {
    fn default() -> Self {
        Self {
            connection_string: "".to_string(),
            host: "localhost".to_string(),
            port: 5432,
            name: "bingxi".to_string(),
            username: "postgres".to_string(),
            password: "".to_string(),
            max_connections: 10,
            ssl_mode: "prefer".to_string(),
            ssl_ca: None,
        }
    }
}

#[derive(Debug, Clone, Deserialize)]
pub struct AuthConfig {
    pub jwt_secret: String,
    pub token_expiry_hours: i64,
}

impl Default for AuthConfig {
    fn default() -> Self {
        Self {
            jwt_secret: "your-secret-key-change-in-production".to_string(),
            token_expiry_hours: 24,
        }
    }
}

#[derive(Debug, Clone, Deserialize)]
pub struct GrpcConfig {
    pub host: String,
    pub port: u16,
}

impl Default for GrpcConfig {
    fn default() -> Self {
        Self {
            host: "0.0.0.0".to_string(),
            port: 50051,
        }
    }
}

#[derive(Debug, Clone, Deserialize)]
pub struct LogConfig {
    pub level: String,
    pub dir: String,
}

impl Default for LogConfig {
    fn default() -> Self {
        Self {
            level: "info".to_string(),
            dir: "./logs".to_string(),
        }
    }
}

#[derive(Debug, Clone, Deserialize)]
pub struct CorsConfig {
    pub allowed_origins: Vec<String>,
}

impl Default for CorsConfig {
    fn default() -> Self {
        Self {
            allowed_origins: vec!["http://localhost:3000".to_string()],
        }
    }
}

fn main() {
    let settings = Config::builder()
        .add_source(File::with_name("config").required(false))
        .add_source(Environment::default().separator("__"))
        .build()
        .unwrap();

    match settings.try_deserialize::<AppSettings>() {
        Ok(s) => println!("Success: {:?}", s),
        Err(e) => println!("Error: {}", e),
    }
}