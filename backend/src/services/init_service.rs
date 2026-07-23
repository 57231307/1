//! 系统初始化服务（facade）
//!
//! 本文件为 facade 入口，仅保留 `InitService` struct + `new` 构造函数 + 公共类型/常量 + 单元测试。
//! 业务实现已按职责拆分到 `init_service_ops/` 子模块（与 `init_service` 同为 `crate::services` 下兄弟模块）：
//! - `init_service_ops::setup`：数据库连接 + 初始化入口 + 迁移（10 方法）
//! - `init_service_ops::role`：默认角色创建（10 方法）
//! - `init_service_ops::permission`：角色权限矩阵 + 互斥规则（2 方法）
//! - `init_service_ops::dept_user`：默认部门 + 管理员用户 + 重置密码（3 方法）
//!
//! 外部调用路径不变：`crate::services::init_service::InitService` 等保持稳定。
//! `db` 字段使用 `pub(crate)` 可见性，init_service_ops 子模块的 impl 块可直接访问。

use crate::utils::error::AppError;
use sea_orm::DatabaseConnection;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::Mutex;

// V15 P0-S20 修复：权限资源注册表与操作权限码定义
//
// 原实现仅 11 类权限资源，实际 60+ 类业务模块。
// 现补齐至 60+ 类权限资源，每个资源配 11 个操作权限码，
// 覆盖面料行业 ERP 全业务场景。

/// 11 个操作权限码（V15 P0-S20 定义）
///
/// 每个权限资源均可配置这 11 个操作权限码，
/// "*" 为通配符，表示拥有该资源的全部操作权限。
pub const PERMISSION_ACTIONS: &[&str] = &[
    "read",   // 读取/查询
    "create", // 新建/创建
    "update", // 修改/更新
    "delete", // 删除
    "print",  // 打印
    "export", // 导出
    "import", // 导入
    "audit",  // 审核
    "approve", // 审批通过
    "reject",  // 审批驳回
    "*",       // 通配符（全部操作）
];

/// 60+ 类权限资源注册表（V15 P0-S20 补齐）
///
/// 覆盖面料行业 ERP 全业务场景：
/// IAM/产品目录/库存/销售/采购/生产/质量/财务/CRM/物流/人力/安全/IT/系统/分析/通知/集成
pub const PERMISSION_RESOURCES: &[&str] = &[
    // ===== IAM 与组织域 =====
    "users", "roles", "departments", "permissions", "field-permissions",
    // ===== 产品目录域 =====
    "products", "categories", "warehouses", "boms",
    "chemicals", "chemical-categories", "chemical-lots", "chemical-requisitions",
    // ===== 库存仓储域 =====
    "inventory", "stock", "piece-split", "transfers", "adjustments",
    "reservations", "counts", "batches", "stock-alerts",
    // ===== 销售域 =====
    "orders", "fabric-orders", "sales-contracts", "sales-prices", "sales-returns",
    "quotations", "custom-orders", "color-cards", "color-prices",
    // ===== 采购域 =====
    "purchase-orders", "purchase-receipts", "purchase-returns", "purchase-contracts",
    "purchase-prices", "suppliers", "supplier-evaluations",
    // ===== 生产域（面料行业深化）=====
    "production-orders", "dye-batches", "dye-recipes", "dye-batch-lifecycle-logs",
    "dye-batch-state-rules", "dye-batch-reworks", "dye-batch-operations",
    "greige-fabrics", "lab-dip", "production-recipes", "process-routes", "flow-cards",
    "outsourcing-orders", "outsourcing-receipts", "outsourcing-vouchers",
    "business-modes", "business-mode-links",
    "mrp", "mrp-history", "capacity", "scheduling", "material-shortage",
    // ===== 质量域 =====
    "quality-inspections", "quality-issues", "quality-standards",
    "fabric-inspections", "fabric-defects",
    // ===== 财务域 =====
    "vouchers", "subjects", "fixed-assets", "budgets", "cost-collections",
    "ar", "ap", "gl", "financial-analysis", "fund-management", "fund-transfers",
    "currencies", "exchange-rates", "ar-reconciliations", "accounting-periods",
    "wages", "wage-rates", "wage-records",
    // ===== 能耗域 =====
    "energy-meters", "energy-consumptions", "energy-rules", "energy-allocations",
    // ===== CRM 域 =====
    "crm-leads", "crm-opportunities", "crm-customers", "customers", "customer-credits",
    "five-dimension",
    // ===== 物流与贸易域 =====
    "logistics", "ship-orders", "incoterms",
    // ===== 人力资源域 =====
    "employees",
    // ===== 安全环保域 =====
    "safety-records", "environmental-records", "equipment", "maintenance-records",
    // ===== 分析与报表域 =====
    "reports", "bi-analysis", "dashboard", "sales-analysis",
    // ===== 通知与 OA 域 =====
    "notifications", "email-templates", "email-records", "oa-announcements",
    "business-trace",
    // ===== 系统域 =====
    "system-config", "audit-logs", "slow-queries", "print-templates",
    "data-import", "permissions-audit",
    // ===== AI 智能域（V15 P0-S26 新增）=====
    // 对应 routes/analytics.rs ai() + advanced() AI 端点 + routes/system.rs ai_extend 端点
    "ai-forecast", "ai-inventory-opt", "ai-anomaly", "ai-recommendation",
    "ai-recipe-opt", "ai-quality-pred", "ai-process-opt", "ai-summary",
];

/// 初始化任务状态（L-24 修复：补充终态与恢复路径文档）
/// 状态机：Running → Completed | Failed（终态）；Failed 后需重新调用 initialize 创建新 task_id 恢复。
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum InitTaskStatus {
    /// 正在运行（迁移 + 默认数据创建中，panic 会被 catch_unwind 隔离并转为 Failed）
    Running,
    /// 已完成（迁移 + 默认数据创建均成功，终态）
    Completed,
    /// 失败（迁移错误/创建错误/panic，终态；需重新调用 initialize 创建新任务恢复）
    Failed,
}

/// 全局初始化任务状态存储（内存存储，生产环境应改用 Redis）
static INIT_TASKS: std::sync::OnceLock<Arc<Mutex<HashMap<String, InitTaskStatus>>>> =
    std::sync::OnceLock::new();

/// 获取全局初始化任务状态存储
pub fn get_init_tasks() -> &'static Arc<Mutex<HashMap<String, InitTaskStatus>>> {
    INIT_TASKS.get_or_init(|| Arc::new(Mutex::new(HashMap::new())))
}

#[derive(Debug, Clone, serde::Deserialize, serde::Serialize)]
pub struct DatabaseConfig {
    pub host: String,
    pub port: String,
    pub name: String,
    pub username: String,
    pub password: String,
    /// SSL 模式：prefer（默认）/ require / disable 等
    /// v5 审计批次 21：读取配置值，缺省回退 prefer（比 disable 更安全）
    #[serde(default)]
    pub ssl_mode: Option<String>,
}

impl DatabaseConfig {
    pub fn to_connection_string(&self) -> String {
        // Use percent_encoding for url-encoding user/password/name. The host segment
        // of a postgres connection string lives in the URL "authority" position,
        // and its character set is already ASCII-safe (alphanumeric, '.', '-', ':',
        // '[', ']' for IPv6, '%' for already-encoded chars). Encoding '.' or any
        // alphabetic character in the host would break DNS / IP resolution, so we
        // pass the host through verbatim.
        use percent_encoding::{utf8_percent_encode, NON_ALPHANUMERIC};
        let encoded_username = utf8_percent_encode(&self.username, NON_ALPHANUMERIC).to_string();
        let encoded_password = utf8_percent_encode(&self.password, NON_ALPHANUMERIC).to_string();
        let encoded_name = utf8_percent_encode(&self.name, NON_ALPHANUMERIC).to_string();

        // SSL 模式来源：self.ssl_mode（来自 config.yaml 或前端请求），缺省时使用 prefer
        // v5 审计批次 21：原硬编码 "disable"，现改为读取配置值，默认 prefer
        // prefer 比 disable 更安全：先尝试 SSL 连接，失败再回退明文
        let ssl_mode = self.ssl_mode.as_deref().unwrap_or("prefer");

        format!(
            "postgres://{}:{}@{}:{}/{}?sslmode={}",
            encoded_username, encoded_password, self.host, self.port, encoded_name, ssl_mode
        )
    }
}

/// 系统初始化服务
///
/// struct 定义保留在 facade，impl 块按职责分散到 `init_service_ops/` 子模块。
pub struct InitService {
    /// 数据库连接句柄
    ///
    /// `pub(crate)` 可见性：init_service_ops 兄弟模块的 impl 块需直接访问此字段。
    pub(crate) db: Arc<DatabaseConnection>,
}

impl InitService {
    /// 创建初始化服务实例
    pub fn new(db: Arc<DatabaseConnection>) -> Self {
        Self { db }
    }
}

#[derive(Debug, thiserror::Error)]
pub enum InitError {
    #[error("系统已经初始化")]
    AlreadyInitialized,
    #[error("密码哈希错误：{0}")]
    HashError(String),
    #[error("数据库错误：{0}")]
    DatabaseError(String),
    #[error("用户不存在")]
    UserNotFound,
    #[error("配置错误：{0}")]
    ConfigError(String),
    /// 参数校验错误（P0 新增：用于密码强度等输入校验，HTTP 400）
    #[error("参数校验错误：{0}")]
    ValidationError(String),
}

impl From<InitError> for AppError {
    fn from(err: InitError) -> Self {
        match err {
            InitError::AlreadyInitialized => AppError::business("系统已经初始化"),
            InitError::HashError(e) => AppError::internal(format!("密码哈希错误: {}", e)),
            InitError::DatabaseError(e) => AppError::database(e),
            InitError::UserNotFound => AppError::not_found("用户不存在"),
            InitError::ConfigError(e) => AppError::bad_request(format!("配置错误: {}", e)),
            InitError::ValidationError(e) => AppError::validation(format!("参数校验失败: {}", e)),
        }
    }
}

#[derive(Debug, serde::Serialize)]
pub struct InitializationResult {
    pub success: bool,
    pub message: String,
    pub admin_username: String,
}

#[derive(Debug, serde::Deserialize)]
pub struct InitRequest {
    pub admin_username: String,
    pub admin_password: String,
}

#[derive(Debug, serde::Serialize)]
pub struct InitStatus {
    pub initialized: bool,
    pub message: String,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn to_connection_string_preserves_ip_host() {
        // 回退测试：确保 host 中合法的 IP 字符（数字、.）不会被错误编码
        // 批次 28 v7 P0-2 修复：原测试数据使用真实生产 IP，已改为 RFC 5737 文档示例段
        let cfg = DatabaseConfig {
            host: "192.0.2.100".to_string(),
            port: "5432".to_string(),
            name: "bingxi".to_string(),
            username: "bingxi".to_string(),
            password: "p@ss word".to_string(),
            // v5 审计批次 21：ssl_mode 缺省时回退到 prefer（原为 disable）
            ssl_mode: None,
        };
        let s = cfg.to_connection_string();
        // 关键断言：host 段不应被编码
        assert!(
            s.contains("@192.0.2.100:"),
            "host 不应被 percent-encoding，连接串 = {}",
            s
        );
        // 同时确保 username/password 仍然被正确编码
        assert!(
            s.starts_with("postgres://bingxi:p%40ss%20word@"),
            "s = {}",
            s
        );
        // v5 审计批次 21：ssl_mode 缺省时默认 prefer
        assert!(s.ends_with("/bingxi?sslmode=prefer"));
    }

    #[test]
    fn to_connection_string_preserves_dns_host() {
        // DNS 主机名也必须原样保留
        let cfg = DatabaseConfig {
            host: "db.example.com".to_string(),
            port: "5432".to_string(),
            name: "bingxi".to_string(),
            username: "u".to_string(),
            password: "p".to_string(),
            // v5 审计批次 21：ssl_mode 缺省时回退到 prefer
            ssl_mode: None,
        };
        let s = cfg.to_connection_string();
        assert!(s.contains("@db.example.com:5432/"), "s = {}", s);
    }

    #[test]
    fn to_connection_string_preserves_ipv6_host() {
        // IPv6 主机名应保留方括号（注意：这里我们只做 verbatim 透传；
        // 真正使用 IPv6 时应额外处理）
        let cfg = DatabaseConfig {
            host: "[::1]".to_string(),
            port: "5432".to_string(),
            name: "bingxi".to_string(),
            username: "u".to_string(),
            password: "p".to_string(),
            // v5 审计批次 21：ssl_mode 缺省时回退到 prefer
            ssl_mode: None,
        };
        let s = cfg.to_connection_string();
        assert!(s.contains("@[::1]:5432/"), "s = {}", s);
    }
}
