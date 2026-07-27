use config::{Config, ConfigError, File};
use serde::{Deserialize, Serialize};
use tracing::{error, warn};

#[derive(Debug, Clone, Deserialize)]
pub struct AppSettings {
    pub server: ServerConfig,
    pub database: DatabaseConfig,
    /// 认证配置（`#[serde(default)]` 允许 auth 段缺失，由环境变量填充）。
    /// 批次 261 修复：缺失时走 `AuthConfig::default()`，由 `load_sensitive_from_env()` 从 JWT_SECRET 等填充。
    #[serde(default)]
    pub auth: AuthConfig,
    pub log: LogConfig,
    /// CORS 配置：字段级 `#[serde(default)]`，缺失时回退到 [`CorsConfig::default()`]。
    /// 避免因配置遗漏导致服务启动 panic。
    #[serde(default)]
    pub cors: CorsConfig,
    /// 事件总线配置（Kafka 可选后端，缺失时走 [`KafkaSettings::default()`]）。
    /// `#[serde(default)]` 保证老配置无需补充 kafka 段即可解析（默认 Broadcast 模式）。
    #[serde(default)]
    pub kafka: KafkaSettings,
    /// 慢查询采集配置（部署-3 修复，默认 enabled=true，每 5 分钟采集 pg_stat_statements）。
    /// 扩展未安装时静默 warn 失败不阻断启动；CI/单机环境可在配置中关闭。
    #[serde(default)]
    pub slow_query: SlowQuerySettings,
    /// 面料行业配置（V15 Batch05-P1-2：6 个核心配置项，支持环境变量覆盖）
    /// 缺失时走 [`FabricIndustryConfig::default()`]，关键配置（如 dyehouse_vat_count）由 main.rs fail-fast 校验。
    #[serde(default)]
    pub fabric_industry: FabricIndustryConfig,
    pub env: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct ServerConfig {
    pub host: String,
    pub port: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct DatabaseConfig {
    /// 数据库连接字符串，可留空；留空时由 host/port/name/username/password 自动拼接
    #[serde(default)]
    pub connection_string: String,
    pub host: String,
    pub port: u16,
    pub name: String,
    pub username: String,
    pub password: String,
    pub max_connections: u32,
}

/// 认证配置（批次 261：派生 `Default` 支持 `#[serde(default)]`）。
/// 默认 `jwt_secret=""` 会被 `validate_secret()` 拒绝，确保未配置时 fail-fast。
#[derive(Debug, Clone, Default, Deserialize)]
pub struct AuthConfig {
    /// JWT 密钥（`#[serde(default)]` 允许 auth 段缺失，由 `JWT_SECRET` 环境变量填充）。
    /// 空字符串会被 `validate_secret()` 拒绝，确保安全。
    #[serde(default)]
    pub jwt_secret: String,
    pub previous_jwt_secret: Option<String>,
    pub cookie_secret: Option<String>,
    /// M-2 修复：独立 Webhook HMAC 密钥，与 JWT_SECRET 分离（防泄露后 webhook 被伪造）。
    /// None 时 fallback 到 `webhook.inherit_jwt_secret`（默认 false），启动时 fail-fast 要求显式配置。
    #[serde(default)]
    pub webhook_secret: Option<String>,
    /// M-2 修复：是否允许 webhook 复用 JWT_SECRET（仅用于迁移期，默认 false）
    #[serde(default)]
    pub webhook_inherit_jwt_secret: bool,
}

#[derive(Debug, Clone, Deserialize)]
pub struct LogConfig {
    pub level: String,
    pub dir: String,
    /// V15 P1 20.8-B：日志文件保留天数（默认 90 天），过期文件由 LogCleanupService 自动清理
    #[serde(default = "default_log_retention_days")]
    pub retention_days: i32,
}

fn default_log_retention_days() -> i32 {
    90
}

/// 事件总线顶层配置（Kafka 可选后端，默认走进程内 Broadcast）。
/// `enabled=true` 时走 rskafka，连接失败自动降级回 Broadcast 并输出中文错误。
#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(default)]
pub struct KafkaSettings {
    /// 是否启用 Kafka 后端（生产可启用，CI 必须保持 false）
    pub enabled: bool,
    /// Kafka broker 列表（逗号分隔）
    pub brokers: String,
    /// 业务事件 topic
    pub topic: String,
    /// 消费者组 ID
    pub consumer_group: String,
    /// 客户端 ID
    pub client_id: String,
    /// topic 分区数（首次自动创建时使用）
    pub partitions: i32,
    /// 复制因子（首次自动创建时使用）
    pub replication_factor: i16,
    /// Kafka 连接超时（毫秒）
    pub connect_timeout_ms: u64,
    /// 启动时是否自动创建 topic
    pub auto_create_topic: bool,
}

impl Default for KafkaSettings {
    fn default() -> Self {
        Self {
            enabled: false,
            brokers: "localhost:9092".to_string(),
            topic: "erp_business_events".to_string(),
            consumer_group: "erp_event_consumer".to_string(),
            client_id: "bingxi-erp".to_string(),
            partitions: 12,
            replication_factor: 1,
            connect_timeout_ms: 5000,
            auto_create_topic: true,
        }
    }
}

/// 慢查询采集配置（部署-3 修复，默认 enabled=true，每 5 分钟采集 pg_stat_statements）。
/// 扩展未安装时 warn 失败不阻断；interval/threshold/limit 可调。
#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(default)]
pub struct SlowQuerySettings {
    /// 是否启用慢查询后台采集任务
    pub enabled: bool,
    /// 采集间隔（秒），默认 300（5 分钟）
    pub interval_secs: u64,
    /// 慢查询阈值（毫秒），超过此值的 SQL 才会被记录
    pub threshold_ms: f64,
    /// 单次采集最大行数，防止极端情况下表爆炸
    pub limit_rows: i64,
}

impl Default for SlowQuerySettings {
    fn default() -> Self {
        Self {
            enabled: true,
            interval_secs: 300,
            threshold_ms: 100.0,
            limit_rows: 100,
        }
    }
}

/// CORS 跨域配置（`#[serde(default)]`，段或字段缺失均走 [`CorsConfig::default()`]）。
/// 保证部署容错性（参见 #27048461019 修复）。
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

/// 面料行业配置（V15 Batch05-P1-2：6 个核心配置项，支持环境变量覆盖）。
///
/// 依据：V15 审计报告 类五 P1（batch-05 维度 5.5：配置依赖闭环 缺陷项 1）
/// 业务背景：面料行业 6 个核心配置完全缺失，影响业务可配置性：
///   1. DYEHOUSE_VAT_COUNT（染缸设备数）：配置染厂染缸总数，影响排缸算法和产能统计
///   2. PROCESS_UNIT_PRICE_BASE（工序单价基准）：工序单价基准，避免硬编码
///   3. ENERGY_ALLOCATION_RULE（能耗分摊规则）：能耗分摊规则（duration 按工时/quantity 按产量）
///   4. QUALITY_GRADE_THRESHOLD_A/B/C（A/B/C 分级阈值）：质量分级阈值，避免硬编码 95%/80%
///   5. DYEBATCH_STATUS_TIMEOUT（缸号状态机超时）：缸号状态超时阈值（秒），影响异常告警
#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(default)]
pub struct FabricIndustryConfig {
    /// 染缸设备数（DYEHOUSE_VAT_COUNT）：染厂染缸总数，影响排缸算法和产能统计
    pub dyehouse_vat_count: i32,
    /// 工序单价基准（PROCESS_UNIT_PRICE_BASE）：默认工序单价（元/件），可按客户/订单覆盖
    pub process_unit_price_base: f64,
    /// 能耗分摊规则（ENERGY_ALLOCATION_RULE）：duration(按工时) / quantity(按产量) / weight(按重量)
    pub energy_allocation_rule: String,
    /// A 级质量分级阈值（QUALITY_GRADE_THRESHOLD_A）：≥此值判 A 级（首级），默认 95.0
    pub quality_grade_threshold_a: f64,
    /// B 级质量分级阈值（QUALITY_GRADE_THRESHOLD_B）：≥此值判 B 级（让步接收），默认 80.0
    pub quality_grade_threshold_b: f64,
    /// C 级质量分级阈值（QUALITY_GRADE_THRESHOLD_C）：≥此值判 C 级（不合格），默认 60.0
    pub quality_grade_threshold_c: f64,
    /// 缸号状态机超时（DYEBATCH_STATUS_TIMEOUT）：缸号状态超时阈值（秒），超过触发告警
    pub dyebatch_status_timeout_secs: u64,
}

impl Default for FabricIndustryConfig {
    fn default() -> Self {
        Self {
            dyehouse_vat_count: 0,
            process_unit_price_base: 0.0,
            energy_allocation_rule: "duration".to_string(),
            quality_grade_threshold_a: 95.0,
            quality_grade_threshold_b: 80.0,
            quality_grade_threshold_c: 60.0,
            dyebatch_status_timeout_secs: 14400,
        }
    }
}

impl AppSettings {
    pub fn new() -> Result<Self, ConfigError> {
        let mut app_settings = Self::load_from_env()?;
        Self::sync_env_variable(&app_settings);
        Self::validate_secrets(&app_settings)?;
        Self::validate_production_config(&app_settings)?;
        Self::load_cors_from_env(&mut app_settings);
        Self::load_database_config(&mut app_settings);
        Ok(app_settings)
    }

    /// 加载配置：config.yaml/.env + 环境变量，反序列化为 AppSettings 并填充敏感字段。
    fn load_from_env() -> Result<AppSettings, ConfigError> {
        let config_builder = Config::builder()
            .add_source(File::with_name("config").required(false))
            .add_source(File::with_name(".env").required(false))
            .add_source(config::Environment::default().separator("__"));

        let settings = match config_builder.build() {
            Ok(c) => c,
            Err(e) => {
                warn!("无法加载配置文件: {}", e);
                warn!("将尝试从环境变量加载配置");
                Self::build_env_only_config()?
            }
        };

        let mut app_settings: AppSettings = match settings.try_deserialize::<AppSettings>() {
            Ok(s) => s,
            Err(e) => {
                Self::log_config_parse_error(&e);
                return Err(e);
            }
        };

        app_settings.load_sensitive_from_env()?;
        Ok(app_settings)
    }

    /// 配置文件加载失败时，回退到纯环境变量构建 Config。
    fn build_env_only_config() -> Result<Config, ConfigError> {
        Config::builder()
            .add_source(config::Environment::default().separator("__"))
            .build()
            .map_err(|e| {
                ConfigError::Message(format!(
                    "无法加载配置（配置文件 + 环境变量均失败）：{}",
                    e
                ))
            })
    }

    /// 输出配置解析失败的详细错误日志（可能原因 + cors 提示）。
    fn log_config_parse_error(e: &ConfigError) {
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
    }

    /// 将 config.yaml 的 env 字段同步到 APP_ENV 环境变量（批次 398 修复）。
    fn sync_env_variable(app_settings: &AppSettings) {
        // 优先级：APP_ENV 环境变量 > config.yaml env 字段（环境变量已设置时不覆盖）
        if std::env::var("APP_ENV").is_err() {
            if !app_settings.env.is_empty() {
                std::env::set_var("APP_ENV", &app_settings.env);
                tracing::info!(
                    env = %app_settings.env,
                    "从 config.yaml 同步 env 字段到 APP_ENV 环境变量（APP_ENV 原未设置）"
                );
            }
        } else {
            tracing::debug!("APP_ENV 环境变量已设置，config.yaml env 字段被覆盖");
        }
    }

    /// 校验 JWT/COOKIE/WEBHOOK 密钥强度（v5 审计批次 21：WEBHOOK 也纳入校验）。
    fn validate_secrets(app_settings: &AppSettings) -> Result<(), ConfigError> {
        if !Self::validate_secret(&app_settings.auth.jwt_secret) {
            return Err(ConfigError::Message(
                "致命错误：JWT_SECRET 密钥强度不足或使用默认密钥！生产环境必须提供至少 32 字节的安全随机密钥，且不能包含常见弱模式。".to_string(),
            ));
        }

        if let Some(cookie_secret) = &app_settings.auth.cookie_secret {
            if !Self::validate_secret(cookie_secret) {
                return Err(ConfigError::Message(
                    "致命错误：COOKIE_SECRET 密钥强度不足或使用默认密钥！生产环境必须提供至少 32 字节的安全随机密钥，且不能包含常见弱模式。".to_string(),
                ));
            }
        }

        if let Some(webhook_secret) = &app_settings.auth.webhook_secret {
            if !Self::validate_secret(webhook_secret) {
                return Err(ConfigError::Message(
                    "致命错误：WEBHOOK_SECRET 密钥强度不足或使用默认/弱模式密钥！生产环境必须提供至少 32 字节的安全随机密钥，且不能包含常见弱模式。".to_string(),
                ));
            }
        }

        Ok(())
    }

    /// 生产环境必须配置独立的 auth.cookie_secret。
    fn validate_production_config(app_settings: &AppSettings) -> Result<(), ConfigError> {
        let env = app_settings.env.to_lowercase();
        if env == "production" && app_settings.auth.cookie_secret.is_none() {
            return Err(ConfigError::Message(
                "生产环境必须配置独立的 auth.cookie_secret，不能降级使用 jwt_secret".to_string(),
            ));
        }
        Ok(())
    }

    /// 从 CORS__ALLOWED_ORIGINS 环境变量加载来源，为空时回退到 localhost 默认值。
    fn load_cors_from_env(app_settings: &mut AppSettings) {
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
    }

    /// connection_string 为空时按 host/port/name/username/password 自动拼接。
    fn load_database_config(app_settings: &mut AppSettings) {
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
    }

    fn load_sensitive_from_env(&mut self) -> Result<(), ConfigError> {
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

        // v5 审计批次 21：WEBHOOK_SECRET 从环境变量加载（原仅靠 config.yaml，
        // 部署场景下 webhook_secret 缺失会静默走 inherit_jwt_secret 兜底）
        if let Ok(webhook_secret) = std::env::var("WEBHOOK_SECRET") {
            self.auth.webhook_secret = Some(webhook_secret);
        }

        // v5 审计批次 21：AUDIT_SECRET_KEY 走 validate_secret 校验
        // 原仅做长度校验（< 32 字节），无法拦截 placeholder/change-me 等弱模式密钥。
        // 改用 validate_secret 后，与 JWT_SECRET / COOKIE_SECRET 共享同一套强度校验。
        if let Ok(audit_secret) = std::env::var("AUDIT_SECRET_KEY") {
            if !Self::validate_secret(&audit_secret) {
                return Err(ConfigError::Message(
                    "致命错误：AUDIT_SECRET_KEY 密钥强度不足或使用默认/弱模式密钥！生产环境必须提供至少 32 字节的安全随机密钥，且不能包含常见弱模式。".to_string(),
                ));
            }
        }

        // Kafka 事件总线后端：从环境变量覆盖
        if let Ok(v) = std::env::var("KAFKA_ENABLED") {
            self.kafka.enabled = matches!(v.to_lowercase().as_str(), "1" | "true" | "yes" | "on");
        }
        if let Ok(v) = std::env::var("KAFKA_BROKERS") {
            if !v.trim().is_empty() {
                self.kafka.brokers = v;
            }
        }
        if let Ok(v) = std::env::var("KAFKA_TOPIC") {
            if !v.trim().is_empty() {
                self.kafka.topic = v;
            }
        }
        if let Ok(v) = std::env::var("KAFKA_CONSUMER_GROUP") {
            if !v.trim().is_empty() {
                self.kafka.consumer_group = v;
            }
        }
        if let Ok(v) = std::env::var("KAFKA_CLIENT_ID") {
            if !v.trim().is_empty() {
                self.kafka.client_id = v;
            }
        }

        // P0-D15：BINGXI_PORT 环境变量覆盖 config.yaml 的 server.port
        // 用途：双实例蓝绿部署时，bingxi-backend@blue.service 用 8082，
        // bingxi-backend@green.service 用 8083，通过环境变量区分实例端口。
        if let Ok(v) = std::env::var("BINGXI_PORT") {
            if !v.trim().is_empty() {
                self.server.port = v;
            }
        }

        // V15 Batch05-P1-2：面料行业配置从环境变量覆盖（6 个核心配置项）
        // 优先级：环境变量 > config.yaml > 默认值
        if let Ok(v) = std::env::var("DYEHOUSE_VAT_COUNT") {
            if let Ok(parsed) = v.trim().parse::<i32>() {
                self.fabric_industry.dyehouse_vat_count = parsed;
            }
        }
        if let Ok(v) = std::env::var("PROCESS_UNIT_PRICE_BASE") {
            if let Ok(parsed) = v.trim().parse::<f64>() {
                self.fabric_industry.process_unit_price_base = parsed;
            }
        }
        if let Ok(v) = std::env::var("ENERGY_ALLOCATION_RULE") {
            let trimmed = v.trim().to_string();
            if !trimmed.is_empty() {
                self.fabric_industry.energy_allocation_rule = trimmed;
            }
        }
        if let Ok(v) = std::env::var("QUALITY_GRADE_THRESHOLD_A") {
            if let Ok(parsed) = v.trim().parse::<f64>() {
                self.fabric_industry.quality_grade_threshold_a = parsed;
            }
        }
        if let Ok(v) = std::env::var("QUALITY_GRADE_THRESHOLD_B") {
            if let Ok(parsed) = v.trim().parse::<f64>() {
                self.fabric_industry.quality_grade_threshold_b = parsed;
            }
        }
        if let Ok(v) = std::env::var("QUALITY_GRADE_THRESHOLD_C") {
            if let Ok(parsed) = v.trim().parse::<f64>() {
                self.fabric_industry.quality_grade_threshold_c = parsed;
            }
        }
        if let Ok(v) = std::env::var("DYEBATCH_STATUS_TIMEOUT") {
            if let Ok(parsed) = v.trim().parse::<u64>() {
                self.fabric_industry.dyebatch_status_timeout_secs = parsed;
            }
        }

        Ok(())
    }

    fn validate_secret(secret: &str) -> bool {
        if secret.len() < 32 {
            return false;
        }

        // 弱密钥黑名单：覆盖常见占位符、文档示例、默认密钥模式
        // v5 审计批次 21 扩展：新增 your_/your_jwt_secret/your_cookie_secret 等
        // 占位符前缀，以及 placeholder/change-me/at-least-32 等文档示例片段，
        // 防止 .env.example 或文档示例被原样复制到生产环境。
        // 该黑名单适用于 JWT_SECRET / COOKIE_SECRET / WEBHOOK_SECRET / AUDIT_SECRET_KEY。
        let weak_patterns = [
            "change-in-production",
            "change-this",
            "change-me",
            "local-dev",
            "your_secure",
            "your_jwt_secret",
            "your_cookie_secret",
            "your_webhook_secret",
            "your_audit_secret",
            "your_",
            "default",
            "test",
            "example",
            "placeholder",
            "at-least-32",
            "32-chars-long",
            "32-bytes-long",
            "secure-secret-in-production",
        ];

        let secret_lower = secret.to_lowercase();
        for pattern in &weak_patterns {
            if secret_lower.contains(pattern) {
                return false;
            }
        }

        // 熵比校验：唯一字符数 / 总长度
        // 阈值 0.15（部署修复）：原阈值 0.3 过高，导致 `openssl rand -hex 32` 生成的
        // 64 字符 hex 密钥（仅 16 种字符 0-9,a-f，熵比 = 16/64 = 0.25）被误拒。
        // 0.15 阈值仍能拦截全同字符密钥（1/32 = 0.03）和极度重复模式，
        // 同时放行 hex/base64 等标准编码的合法强密钥。
        let unique_chars: std::collections::HashSet<char> = secret.chars().collect();
        let entropy_ratio = unique_chars.len() as f64 / secret.len() as f64;
        entropy_ratio > 0.15
    }
}
