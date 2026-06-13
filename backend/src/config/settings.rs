#![allow(dead_code)]
// TODO(tech-debt): 业务接入或重评估后逐项移除；rustc 1.94+ 编译时由编译器报告具体死代码位置。
// TODO(tech-debt): 业务接入后逐项移除此标注；rustc 1.94+ 编译时由编译器报告具体死代码位置。
use config::{Config, ConfigError, File};
use serde::{Deserialize, Serialize};
use tracing::{error, warn};

#[derive(Debug, Clone, Deserialize)]
pub struct AppSettings {
    pub server: ServerConfig,
    pub database: DatabaseConfig,
    pub auth: AuthConfig,
    pub log: LogConfig,
    /// CORS 配置：使用字段级 `#[serde(default)]`，当 `cors` 段或任意
    /// 子字段缺失时，自动回退到 [`CorsConfig::default()`]，避免因
    /// 配置遗漏导致服务启动 panic。
    #[serde(default)]
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
    /// 最小连接数，连接池会始终保持的活跃连接数量
    pub min_connections: Option<u32>,
    /// 获取连接超时时间（毫秒），默认 10000ms
    pub acquire_timeout_ms: Option<u64>,
    /// 连接空闲超时时间（毫秒），默认 300000ms（5分钟）
    pub idle_timeout_ms: Option<u64>,
    /// 连接最大生命周期（毫秒），默认 1800000ms（30分钟）
    pub max_lifetime_ms: Option<u64>,
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
pub struct LogConfig {
    pub level: String,
    pub dir: String,
}

/// CORS 跨域配置
///
/// 启用 `#[serde(default)]` 后，配置文件中：
/// - `cors` 段整体缺失 → 走 [`CorsConfig::default()`]
/// - 任意子字段缺失 → 走 [`CorsConfig::default()`]
///
/// 保证部署时 CORS 配置的容错性（参见 #27048461019 修复）。
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct CorsConfig {
    /// 允许的来源列表（默认仅本地开发）
    pub allowed_origins: Vec<String>,
    /// 是否允许携带凭证
    pub allow_credentials: bool,
    /// 允许的 HTTP 方法
    pub allowed_methods: Vec<String>,
    /// 允许的请求头
    pub allowed_headers: Vec<String>,
    /// 预检请求缓存秒数
    pub max_age_secs: u64,
}

impl Default for CorsConfig {
    fn default() -> Self {
        Self {
            allowed_origins: vec![
                "http://localhost:3000".to_string(),
                "http://localhost:5173".to_string(),
            ],
            allow_credentials: true,
            allowed_methods: vec![
                "GET".to_string(),
                "POST".to_string(),
                "PUT".to_string(),
                "DELETE".to_string(),
                "OPTIONS".to_string(),
            ],
            allowed_headers: vec![
                "Content-Type".to_string(),
                "Authorization".to_string(),
                "X-Requested-With".to_string(),
            ],
            max_age_secs: 3600,
        }
    }
}

impl CorsConfig {
    /// 从环境变量加载（使用 CORS_ALLOWED_ORIGINS 单下划线环境变量，
    /// 用于在无法读取 config 文件时直接构造 CORS 配置的兜底场景）
    pub fn from_env() -> Self {
        let mut config = Self::default();
        if let Ok(origins) = std::env::var("CORS_ALLOWED_ORIGINS") {
            config.allowed_origins = origins
                .split(',')
                .map(|s| s.trim().to_string())
                .filter(|s| !s.is_empty())
                .collect();
        }
        config
    }
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
                warn!("无法加载配置文件: {}", e);
                warn!("将尝试从环境变量加载配置");
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

        let mut app_settings: AppSettings = match settings.try_deserialize::<AppSettings>() {
            Ok(s) => s,
            Err(e) => {
                error!("═══════════════════════════════════════════════════════════════");
                error!("配置解析失败：{}", e);
                error!("═══════════════════════════════════════════════════════════════");
                error!("可能原因:");
                error!("  1. config.yaml 中存在未知字段名（拼写错误）");
                error!("  2. 某个字段类型不匹配（如 port 应该是数字/字符串）");
                error!("  3. 缺少必填字段（除 cors 外其他段都是必填）");
                error!("═══════════════════════════════════════════════════════════════");
                error!("  注意: cors 段已启用 serde(default)，缺失字段会走默认值。");
                error!("═══════════════════════════════════════════════════════════════");
                return Err(e);
            }
        };

        app_settings.load_sensitive_from_env();

        if !Self::validate_secret(&app_settings.auth.jwt_secret) {
            panic!("致命错误：JWT_SECRET 密钥强度不足或使用默认密钥！生产环境必须提供至少 32 字节的安全随机密钥，且不能包含常见弱模式。");
        }

        if let Some(cookie_secret) = &app_settings.auth.cookie_secret {
            if !Self::validate_secret(cookie_secret) {
                panic!("致命错误：COOKIE_SECRET 密钥强度不足或使用默认密钥！生产环境必须提供至少 32 字节的安全随机密钥，且不能包含常见弱模式。");
            }
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

        if let Ok(db_url) = std::env::var("DATABASE_URL") {
            self.database.connection_string = db_url;
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

    fn validate_secret(secret: &str) -> bool {
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
