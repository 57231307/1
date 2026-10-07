use config::{Config, ConfigError, File};
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use std::sync::OnceLock;
use tracing::{error, warn};

/// 进程级 `UpdateConfig` 单例：`AppSettings::new` 校验通过后写入，
/// `SystemUpdateService`（无状态构造，`new()` 不带参数）与 SSRF 域名白名单由此读取
/// 下载镜像清单 / 校验开关，避免把第二套配置常量散落到下载链路。
/// 未加载配置时 `global_update_config()` 回退 [`UpdateConfig::default`]（官方优先 + 强校验 +
/// 启用内置公共默认加速镜像），该默认值本身即安全兜底：校验锚仍仅官方域，默认镜像只搬字节、
/// 失败优雅跳官方，绝不 brick 启动。
static GLOBAL_UPDATE_CONFIG: OnceLock<UpdateConfig> = OnceLock::new();

/// 读取进程级更新配置；未初始化时返回安全默认值（官方优先 + 强校验 + 内置默认镜像仅作字节候选）。
pub fn global_update_config() -> UpdateConfig {
    GLOBAL_UPDATE_CONFIG.get().cloned().unwrap_or_default()
}

/// 将校验通过的 `UpdateConfig` 写入进程级单例（重复调用仅首次生效，幂等）。
fn set_global_update_config(cfg: UpdateConfig) {
    // OnceLock 进程生命周期内仅设置一次；测试/多实例场景下重复 set 静默忽略旧值即可
    let _ = GLOBAL_UPDATE_CONFIG.set(cfg);
}

/// 进程级环保税适用税额单例：`AppSettings::new` 解析完成后写入，
/// 环保税计税链路（`handlers/environmental_tax_handler.rs` → `EnvironmentalTaxService`）
/// 由此读取部署配置的地方适用税额（元/污染当量），避免在业务代码里出现第二套手写税额常量。
///
/// **无硬编码默认值**：未加载配置或未配置该项时返回 `None`（部署态，启动不 panic），
/// 计税端点必须据此显式失败并记 warn，禁止按任何默认税额继续算。
static GLOBAL_ENV_TAX_RATE: OnceLock<Option<Decimal>> = OnceLock::new();

/// 读取进程级环保税适用税额（元/污染当量）；`None` = 未配置（调用方必须显式拒绝计税）。
pub fn global_env_tax_rate_per_equivalent() -> Option<Decimal> {
    GLOBAL_ENV_TAX_RATE.get().copied().flatten()
}

/// 将解析后的适用税额写入进程级单例（重复调用仅首次生效，幂等；仿 `set_global_update_config`）。
fn set_global_env_tax_rate(rate: Option<Decimal>) {
    let _ = GLOBAL_ENV_TAX_RATE.set(rate);
}

#[derive(Debug, Clone, Deserialize)]
pub struct AppSettings {
    pub server: ServerConfig,
    pub database: DatabaseConfig,
    /// 认证配置（`#[serde(default)]` 允许 auth 段缺失，由环境变量填充）。
    /// 缺失时走 `AuthConfig::default()`，由 `load_sensitive_from_env()` 从 JWT_SECRET 等填充。
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
    /// 慢查询采集配置（默认 enabled=true，每 5 分钟采集 pg_stat_statements）。
    /// 扩展未安装时静默 warn 失败不阻断启动；CI/单机环境可在配置中关闭。
    #[serde(default)]
    pub slow_query: SlowQuerySettings,
    /// 面料行业配置（6 个核心配置项，支持环境变量覆盖）
    /// 缺失时走 [`FabricIndustryConfig::default()`]，关键配置（如 dyehouse_vat_count）由 main.rs fail-fast 校验。
    #[serde(default)]
    pub fabric_industry: FabricIndustryConfig,
    /// 系统更新下载配置（多镜像加速 + 官方 SHA-256 强校验，防镜像投毒）。
    /// `#[serde(default)]`：缺失 update 段时走 [`UpdateConfig::default`]（官方优先 + 强校验 +
    /// 启用内置公共默认加速镜像），老配置无需补充即可解析。运维显式镜像清单来自 `config.update.mirrors`
    /// / 环境变量 `UPDATE__MIRRORS`；内置默认加速镜像（`DEFAULT_RELEASE_MIRRORS`）由
    /// `use_default_mirrors`（默认 true）开关，二者均只搬 tar 字节、不作校验值信任锚。
    #[serde(default)]
    pub update: UpdateConfig,
    /// 环保税适用税额（元/污染当量；env 覆盖键 `ENV_TAX_RATE_PER_EQUIVALENT`）——
    /// **地方可变值**：《环境保护税法》附表规定法定幅度为每污染当量 **1.2–12 元**，
    /// 具体适用税额由省级人民政府在本行政区域内确定（运维取值须落在该法定幅度内）。
    /// 与之分源的法定不可调值（污染当量值）见 `crate::constants::environmental_tax`。
    /// `Option` + `#[serde(default)]`：未配置 = `None`，属部署态而非代码缺陷，
    /// **启动不 panic、计税端点显式失败**（禁止任何硬编码默认税额继续算）。
    #[serde(default)]
    pub env_tax_rate_per_equivalent: Option<Decimal>,
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

/// 认证配置（派生 `Default` 支持 `#[serde(default)]`）。
/// 默认 `jwt_secret=""` 会被 `validate_secret()` 拒绝，确保未配置时 fail-fast。
#[derive(Debug, Clone, Default, Deserialize)]
pub struct AuthConfig {
    /// JWT 密钥（`#[serde(default)]` 允许 auth 段缺失，由 `JWT_SECRET` 环境变量填充）。
    /// 空字符串会被 `validate_secret()` 拒绝，确保安全。
    #[serde(default)]
    pub jwt_secret: String,
    pub previous_jwt_secret: Option<String>,
    pub cookie_secret: Option<String>,
    /// 独立 Webhook HMAC 密钥，与 JWT_SECRET 分离（防泄露后 webhook 被伪造）。
    /// None 时 fallback 到 `webhook.inherit_jwt_secret`（默认 false），启动时 fail-fast 要求显式配置。
    #[serde(default)]
    pub webhook_secret: Option<String>,
    /// 是否允许 webhook 复用 JWT_SECRET（仅用于迁移期，默认 false）
    #[serde(default)]
    #[allow(dead_code, reason = "预留配置字段")]
    pub webhook_inherit_jwt_secret: bool,
}

#[derive(Debug, Clone, Deserialize)]
pub struct LogConfig {
    pub level: String,
    pub dir: String,
    /// 日志文件保留天数（默认 90 天），过期文件由 LogCleanupService 自动清理
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

/// 慢查询采集配置（默认 enabled=true，每 5 分钟采集 pg_stat_statements）。
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

/// 面料行业配置（6 个核心配置项，支持环境变量覆盖）。
///
/// 业务背景：面料行业 6 个核心配置项支撑业务可配置性：
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

/// 更新下载镜像优先策略。
/// 无论选择哪种顺序，官方域（github.com / objects.githubusercontent.com）永远列入候选并作为最终兜底。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum MirrorOrder {
    /// 镜像优先：先试配置镜像加速，官方源作为兜底（墙内/慢链路部署常用）。
    MirrorFirst,
    /// 官方优先（默认）：先官方源，镜像仅在官方失败时兜底，最大限度降低投毒面。
    OfficialFirst,
}

impl Default for MirrorOrder {
    fn default() -> Self {
        MirrorOrder::OfficialFirst
    }
}

/// 内置默认 GitHub Release 下载加速镜像 base URL 白名单（官方优先策略的默认档）。
///
/// 定位与安全边界（务必区分于运维显式配置的 `config.update.mirrors`）：
/// - 这些是社区广泛使用的第三方公共加速代理，仅用于**搬运大 tar 字节**；
///   校验基准（SHA-256）永远只从官方域取（见 `is_official_digest_host` /
///   `validate_official_digest_url`），**本清单绝不进入 `OFFICIAL_DOWNLOAD_HOSTS`**，
///   故默认镜像 host 恒被 `is_official_digest_host` 判 false（不可能从它取校验值）。
/// - 公共镜像可用性会漂移/离线，因此**绝不参与 `validate_update_mirrors` 的启动 fail-fast**
///   （该函数只校验运维显式配置的 `config.mirrors`）。默认镜像只在下载期作为候选按序尝试、
///   失败即优雅跳到下一候选/官方（`try_download_candidate` 逐个 try、失败 continue 语义），
///   信任锚仍是官方 digest → fail-closed，**不会因默认镜像解析不到而 brick 启动**。
/// - 运维可用环境变量 `UPDATE__MIRRORS`（叠加/覆盖显式清单）或
///   `UPDATE__USE_DEFAULT_MIRRORS=false`（彻底关闭内置默认）调整；也可 `use_default_mirrors=false`。
///
/// 宁少勿滥：仅收录公开、https、社区长期在用的 ghproxy 系加速域。
/// 注意：`ghproxy.net` 故意**不**纳入内置默认——既有集成测试
/// `tests/services_system_update_integrity_test.rs::validate_download_url_rejects_insecure_and_disallowed_hosts`
/// 以其作为"未配置镜像 host 必须被下载白名单拒绝"的负向探针（该文件不在本次改动范围）；
/// 若需纳入须与该测试一并评审更新。可用 `UPDATE__MIRRORS` 显式叠加、`use_default_mirrors=false` 关闭。
pub(crate) const DEFAULT_RELEASE_MIRRORS: &[&str] =
    &["https://gh-proxy.com", "https://mirror.ghproxy.com"];

/// 系统更新下载配置（多镜像加速 + 官方强校验，防镜像投毒）。
///
/// 信任模型（安全红线，违反即引入 RCE）：
/// - `mirrors` **只用于搬运大 tar 字节**；校验基准永远只从官方域取（见 `github.rs` / facade），
///   绝不从镜像拼接 checksum URL。
/// - `verify_digest=true` 时若两官方源（CI 上传的 `.sha256` 资产 / GitHub API `assets[].digest`）
///   都拿不到 SHA-256 校验值 → **fail-closed 拒绝 apply**（见 `UpdateError::ChecksumUnavailable`）。
/// - 运维镜像清单只从本配置（config.yaml / 环境变量 `UPDATE__MIRRORS`）读取，不在业务代码里散落；
///   另有内置公共默认加速镜像 [`DEFAULT_RELEASE_MIRRORS`]（`use_default_mirrors=true` 时并入候选，
///   仍只搬字节、不作校验锚、不参与启动 fail-fast，可用 `use_default_mirrors=false` 关闭）。
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct UpdateConfig {
    /// GitHub Release 加速镜像 base URL 列表（如 `https://ghfast.top`）。
    /// 空列表（默认）= 仅运维未显式配置镜像；此时是否并入内置默认加速镜像由
    /// [`UpdateConfig::use_default_mirrors`] 决定。下载时镜像改写形如 `{mirror}/{githubAssetUrl}`。
    pub mirrors: Vec<String>,
    /// 是否启用内置公共默认加速镜像 [`DEFAULT_RELEASE_MIRRORS`]（默认 true，官方优先策略的默认档）。
    /// 置 true 时默认镜像仅并入**下载候选**与**字节下载允许域**，绝不进入校验值信任锚
    /// （`OFFICIAL_DOWNLOAD_HOSTS` / `is_official_digest_host`），也绝不参与启动 fail-fast
    /// （`validate_update_mirrors` 只校验运维显式 `mirrors`）；置 false 则只用运维 `mirrors`。
    pub use_default_mirrors: bool,
    /// 镜像优先 / 官方优先（默认 [`MirrorOrder::OfficialFirst`]）。
    pub mirror_order: MirrorOrder,
    /// 是否强制 SHA-256 完整校验（默认 true）。
    /// 生产环境必须保持 true；置 false 仅在校验源不可达的隔离调试场景，且会显式告警不静默。
    pub verify_digest: bool,
    /// 下载连接超时（秒）。
    pub connect_timeout_secs: u64,
    /// 下载读取（单次 chunk）超时（秒）。
    pub read_timeout_secs: u64,
}

impl Default for UpdateConfig {
    fn default() -> Self {
        Self {
            mirrors: Vec::new(),
            use_default_mirrors: true,
            mirror_order: MirrorOrder::OfficialFirst,
            verify_digest: true,
            connect_timeout_secs: 15,
            read_timeout_secs: 180,
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
        Self::load_update_from_env(&mut app_settings);
        // 镜像清单 fail-fast 校验（非法 https/内网/IP 字面量 → 拒绝启动），
        // 校验通过后再写入进程级单例供下载链路读取，避免带病镜像进入运行期。
        Self::validate_update_mirrors(&app_settings)?;
        set_global_update_config(app_settings.update.clone());
        // 环保税适用税额（地方可变值）写入进程级单例供计税链路读取；
        // None（未配置）同样是合法部署态——计税端点会显式失败，不在此处报错、不 panic。
        set_global_env_tax_rate(app_settings.env_tax_rate_per_equivalent);
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
                ConfigError::Message(format!("无法加载配置（配置文件 + 环境变量均失败）：{}", e))
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

    /// 将 config.yaml 的 env 字段同步到 APP_ENV 环境变量。
    fn sync_env_variable(app_settings: &AppSettings) {
        // 优先级：APP_ENV 环境变量 > config.yaml env 字段（环境变量已设置时不覆盖）
        if std::env::var("APP_ENV").is_err() {
            if !app_settings.env.is_empty() {
                // SAFETY: 在启动时同步环境变量，单线程操作
                unsafe { std::env::set_var("APP_ENV", &app_settings.env) };
                tracing::info!(
                    env = %app_settings.env,
                    "从 config.yaml 同步 env 字段到 APP_ENV 环境变量（APP_ENV 原未设置）"
                );
            }
        } else {
            tracing::debug!("APP_ENV 环境变量已设置，config.yaml env 字段被覆盖");
        }
    }

    /// 校验 JWT/COOKIE/WEBHOOK 密钥强度（COOKIE/WEBHOOK 仅在其存在时校验）。
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

    /// 从环境变量覆盖 `update` 段（仿 `load_cors_from_env` 逗号分隔范式）。
    /// - `UPDATE__MIRRORS`：逗号分隔的镜像 base URL 列表（覆盖 config.yaml mirrors）。
    /// - `UPDATE__USE_DEFAULT_MIRRORS`：`true/1/yes/on`→启用内置默认镜像 /
    ///   `false/0/no/off`→关闭；其它值保持默认（true）。
    /// - `UPDATE__MIRROR_ORDER`：`mirror`→MirrorFirst / `official`→OfficialFirst。
    /// - `UPDATE__VERIFY_DIGEST`：`0/false/no`→关闭强校验（默认 true，仅显式关闭）。
    /// - `UPDATE__CONNECT_TIMEOUT_SECS` / `UPDATE__READ_TIMEOUT_SECS`：超时时钟。
    /// 环境变量优先级高于 config.yaml（与全仓 config crate + env 覆盖约定一致）。
    fn load_update_from_env(app_settings: &mut AppSettings) {
        if let Ok(mirrors_str) = std::env::var("UPDATE__MIRRORS") {
            app_settings.update.mirrors = mirrors_str
                .split(',')
                .map(|s| s.trim().to_string())
                .filter(|s| !s.is_empty())
                .collect();
        }

        if let Ok(v) = std::env::var("UPDATE__USE_DEFAULT_MIRRORS") {
            let normalized = v.trim().to_lowercase();
            match normalized.as_str() {
                "1" | "true" | "yes" | "on" => app_settings.update.use_default_mirrors = true,
                "0" | "false" | "no" | "off" => app_settings.update.use_default_mirrors = false,
                other => {
                    warn!(
                        "UPDATE__USE_DEFAULT_MIRRORS 取值 '{}' 无法识别，保持默认(启用内置公共加速镜像；仍官方优先、校验值只信官方)",
                        other
                    );
                }
            }
        }

        if let Ok(order_str) = std::env::var("UPDATE__MIRROR_ORDER") {
            let normalized = order_str.trim().to_lowercase();
            app_settings.update.mirror_order = match normalized.as_str() {
                "mirror" | "mirror_first" | "mirrorfirst" => MirrorOrder::MirrorFirst,
                "official" | "official_first" | "officialfirst" | "" => MirrorOrder::OfficialFirst,
                other => {
                    warn!(
                        "UPDATE__MIRROR_ORDER 取值 '{}' 无法识别，回退官方优先(OfficialFirst)以保证镜像仅作兜底",
                        other
                    );
                    MirrorOrder::OfficialFirst
                }
            };
        }

        if let Ok(v) = std::env::var("UPDATE__VERIFY_DIGEST") {
            let normalized = v.trim().to_lowercase();
            // 默认 true：仅显式 0/false/no/off 才关闭；其它非空值保持开启（安全默认，不因笔误弱化校验）
            app_settings.update.verify_digest =
                !matches!(normalized.as_str(), "0" | "false" | "no" | "off");
        }

        if let Ok(v) = std::env::var("UPDATE__CONNECT_TIMEOUT_SECS") {
            if let Ok(parsed) = v.trim().parse::<u64>() {
                app_settings.update.connect_timeout_secs = parsed;
            }
        }

        if let Ok(v) = std::env::var("UPDATE__READ_TIMEOUT_SECS") {
            if let Ok(parsed) = v.trim().parse::<u64>() {
                app_settings.update.read_timeout_secs = parsed;
            }
        }
    }

    /// 启动 fail-fast 校验 `update.mirrors`（配置错误绝不容忍到运行期被利用）。
    /// 每个镜像必须满足：
    /// 1. 合法**绝对 URL** 且 scheme=https；
    /// 2. host 非 IP 字面量（镜像须为域名，禁裸 IP 绕过 DNS/SSRF 判定）；
    /// 3. host 不在 blocked-hostname（localhost/.local/.internal/...）黑名单；
    /// 4. 通过 `ssrf_guard::validate_url_and_resolve`（解析后挡内网/loopback/云元数据）。
    /// 任一不满足即返回 `ConfigError` 拒绝启动。空列表（默认仅官方）直接通过。
    fn validate_update_mirrors(app_settings: &AppSettings) -> Result<(), ConfigError> {
        for raw in &app_settings.update.mirrors {
            let mirror = raw.trim();
            if mirror.is_empty() {
                // 空白项已在 load_update_from_env 过滤，此处再兜底跳过，避免误拒启动
                continue;
            }

            let parsed = url::Url::parse(mirror).map_err(|e| {
                ConfigError::Message(format!(
                    "致命错误：update.mirrors 中镜像 '{}' 不是合法绝对 URL: {}",
                    mirror, e
                ))
            })?;

            if parsed.scheme() != "https" {
                return Err(ConfigError::Message(format!(
                    "致命错误：update.mirrors 镜像 '{}' 必须使用 https（当前 scheme={}），防止降级为明文下载被中间人投毒",
                    mirror,
                    parsed.scheme()
                )));
            }

            let host = parsed.host_str().ok_or_else(|| {
                ConfigError::Message(format!(
                    "致命错误：update.mirrors 镜像 '{}' 缺少主机名",
                    mirror
                ))
            })?;

            if host.parse::<std::net::IpAddr>().is_ok() {
                return Err(ConfigError::Message(format!(
                    "致命错误：update.mirrors 镜像 '{}' 的 host 为 IP 字面量，镜像必须是可解析的域名（禁裸 IP 绕过 DNS/SSRF 判定）",
                    mirror
                )));
            }

            // 复用 SSRF 判定：blocked-hostname + 解析后内网/loopback/云元数据拦截
            crate::utils::ssrf_guard::validate_url_and_resolve(mirror).map_err(|e| {
                ConfigError::Message(format!(
                    "致命错误：update.mirrors 镜像 '{}' 未通过 SSRF/内网校验: {}",
                    mirror, e
                ))
            })?;

            tracing::info!(
                "update.mirrors 镜像 '{}' 通过启动校验（https + 非 IP + SSRF 安全解析）",
                mirror
            );
        }

        Ok(())
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

        // WEBHOOK_SECRET 支持环境变量填充：仅靠 config.yaml 时，部署场景下
        // webhook_secret 缺失会静默走 inherit_jwt_secret 兜底。
        if let Ok(webhook_secret) = std::env::var("WEBHOOK_SECRET") {
            self.auth.webhook_secret = Some(webhook_secret);
        }

        // AUDIT_SECRET_KEY 走 validate_secret 强度校验（而非仅长度 < 32 判断），
        // 与 JWT_SECRET / COOKIE_SECRET 共享同一套规则，可拦截 placeholder/change-me
        // 等弱模式密钥。
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

        // BINGXI_PORT 环境变量覆盖 config.yaml 的 server.port
        // 用途：双实例蓝绿部署时，bingxi-backend@blue.service 用 8082，
        // bingxi-backend@green.service 用 8083，通过环境变量区分实例端口。
        if let Ok(v) = std::env::var("BINGXI_PORT") {
            if !v.trim().is_empty() {
                self.server.port = v;
            }
        }

        // 面料行业配置从环境变量覆盖（6 个核心配置项）
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

        // 环保税适用税额（元/污染当量，地方可变值）从环境变量覆盖
        // （优先级：ENV_TAX_RATE_PER_EQUIVALENT 环境变量 > config.yaml 顶层字段 > 未配置=None）。
        // 未配置保持 None：计税端点显式失败（**不取任何默认税额**）；
        // 配置了但解析失败记显式 WARN 后同样保持 None——两种形态都会在计税时给出真实错误，
        // 绝不静默按某个数值继续算。法定幅度 1.2–12 元/污染当量，由省级确定（运维取值依据）。
        if let Ok(v) = std::env::var("ENV_TAX_RATE_PER_EQUIVALENT") {
            match v.trim().parse::<Decimal>() {
                Ok(parsed) => self.env_tax_rate_per_equivalent = Some(parsed),
                Err(e) => warn!(
                    "ENV_TAX_RATE_PER_EQUIVALENT 取值 '{}' 无法解析为税额（{}）：保持为未配置，环保税计税端点将显式报错（不采用默认值）",
                    v, e
                ),
            }
        }

        Ok(())
    }

    fn validate_secret(secret: &str) -> bool {
        if secret.len() < 32 {
            return false;
        }

        // 弱密钥黑名单：覆盖常见占位符、文档示例、默认密钥模式——
        // 含 your_/your_jwt_secret/your_cookie_secret 等占位符前缀，
        // 以及 placeholder/change-me/at-least-32 等文档示例片段，
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

// =====================================================
// 内置默认加速镜像配置不变量单测（静态、不触网）
// =====================================================
#[cfg(test)]
mod default_mirrors_tests {
    use super::*;

    /// 默认档：`use_default_mirrors=true`、官方优先、强校验（安全默认）。
    #[test]
    fn update_config_default_enables_builtin_mirrors_official_first() {
        let cfg = UpdateConfig::default();
        assert!(
            cfg.use_default_mirrors,
            "默认档应启用内置公共加速镜像（官方优先兜底，非运维留空全自配）"
        );
        assert!(
            matches!(cfg.mirror_order, MirrorOrder::OfficialFirst),
            "默认官方优先"
        );
        assert!(cfg.verify_digest, "默认强制校验");
        assert!(
            cfg.mirrors.is_empty(),
            "默认运维显式清单为空（内置默认另由 use_default_mirrors 控制）"
        );
    }

    /// 内置默认镜像必须是 https、非空、host 为域名（非 IP 字面量）——不触网，仅静态形态校验。
    /// 这些 host **绝不应**是官方校验锚域（校验值只信官方，见 github/system_update_service 测试）。
    #[test]
    fn default_release_mirrors_are_https_hostnames_not_official() {
        assert!(
            !DEFAULT_RELEASE_MIRRORS.is_empty(),
            "内置默认镜像白名单不应为空"
        );
        for raw in DEFAULT_RELEASE_MIRRORS {
            let parsed = url::Url::parse(raw)
                .unwrap_or_else(|e| panic!("默认镜像 '{}' 不是合法 URL: {}", raw, e));
            assert_eq!(parsed.scheme(), "https", "默认镜像必须 https: {}", raw);
            let host = parsed
                .host_str()
                .unwrap_or_else(|| panic!("默认镜像 '{}' 缺少 host", raw));
            assert!(
                host.parse::<std::net::IpAddr>().is_err(),
                "默认镜像 host 必须为域名而非 IP 字面量: {}",
                host
            );
        }
    }
}
