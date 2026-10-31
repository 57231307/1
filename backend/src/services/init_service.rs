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

/// 60+ 类权限资源注册表（V15 P0-S20 补齐）（覆盖面料行业 ERP 全业务场景：IAM/产品目录/库存/销售/采购/生产/质量/财务/CRM/物流/人力/安全/IT/系统/分析/通知/集成）
pub const PERMISSION_RESOURCES: &[&str] = &[
    // ===== IAM 与组织域 =====
    "users",
    "roles",
    "departments",
    "permissions",
    "field-permissions",
    // 数据权限配置段收编登记。七个端点（列表、详情、按角色查询、资源类型与范围类型两个下拉、
    // 设置与删除）的派生资源名均为 data-permissions；未登记则目录勾不到、种子授不了，授权行
    // 运行期永不命中。本波只补登记，不在角色矩阵新增授予行，也不写存量库补授权。理由：该族
    // 写面与列表面在处理器内已由 require_admin_role 收口为管理员专用，管理员持通配可达；是否
    // 给非 admin 岗位发放该键属扩权裁定，尚未签署，不在本修自签。
    "data-permissions",
    // ===== 产品目录域 =====
    "products",
    "categories",
    // 产品分类路由别名段的运行时权限键（`/product-categories*` ⇒ `product-categories:*`，
    // 见 routes/mod.rs:368-391 与 middleware/permission.rs 的资源段取自路径段）。
    // 与 "categories" 是两条不同资源码，`categories:*` 不覆盖它 ⇒ 采购岗读分类树
    // 必须授此键（CI 矩阵 purchaser 真缺口，裁定 R-6）。
    "product-categories",
    "warehouses",
    "boms",
    "chemicals",
    "chemical-categories",
    "chemical-lots",
    "chemical-requisitions",
    // ===== 库存仓储域 =====
    "inventory",
    "stock",
    // 匹号领域查询与成品布入库标签（GET /inventory/pieces、/inventory/pieces/{id}/print）。
    // 必须在此注册：`middleware/permission.rs::extract_resource_info` 对模块前缀 inventory
    // 取 segment4 作资源段（utils/path_utils.rs:102-117 默认分支），运行时权限键是
    // `pieces:read` / `pieces:print`，与 `inventory:*` 不同源、不会被其覆盖。
    "pieces",
    "piece-split",
    "transfers",
    "adjustments",
    "reservations",
    "counts",
    "batches",
    "stock-alerts",
    // 逐匹扫码入库（`POST /scanner/scan-receipt`，routes/analytics.rs 的 scanner 路由族）。
    // `scanner` 是模块前缀（utils/path_utils.rs::is_business_module_prefix），资源段取其下
    // 第四段 `scan-receipt`，该段在 `RESOURCE_NAME_MAP` 里无消歧条目故保留 URL 原名，运行时
    // 权限键是 `scan-receipt:create`（POST 按方法派生，末段不在动作关键字表内）。未登记即
    // 派生死键：`matches_permission` 是资源段精确匹配，`inventory:*` 与 `pieces:*` 都不覆盖
    // 它，除通配超管外任何角色都授予不了、访问一律 403。授予面见
    // `init_service_ops/permission.rs` 的仓储执行岗分组，存量库同口径补授见迁移
    // `scan_receipt_grant` 域。
    "scan-receipt",
    // ===== 销售域 =====
    "orders",
    "fabric-orders",
    "sales-contracts",
    "sales-prices",
    "sales-returns",
    "quotations",
    "custom-orders",
    "color-cards",
    "color-prices",
    // ===== 采购域 =====
    "purchase-orders",
    "purchase-receipts",
    "purchase-returns",
    "purchase-contracts",
    "purchase-prices",
    "suppliers",
    "supplier-evaluations",
    "supplier-blacklists",
    // 供应商 SKU 对照表与其父级目录（路由挂在 purchase 模块下：
    // `routes/purchase.rs:461/488/507` 的 `/sku-mappings`、`/supplier-products`、
    // `/supplier-product-colors`）。运行时权限键取 segment4（同 `purchase-orders` 的推导口径），
    // 所以这三条是与 `suppliers` 彼此独立的资源码；矩阵 `init_service_ops/permission.rs`
    // 已在授予它们，注册表缺登记就会让闸门①（未登记资源码 = 授予后永不命中）判红。
    "sku-mappings",
    "supplier-products",
    "supplier-product-colors",
    // ===== 生产域（面料行业深化）=====
    "production-orders",
    "dye-batches",
    "dye-recipes",
    "dye-batch-lifecycle-logs",
    "dye-batch-state-rules",
    "dye-batch-reworks",
    "dye-batch-operations",
    "greige-fabrics",
    "lab-dip",
    "production-recipes",
    "process-routes",
    "flow-cards",
    "outsourcing-orders",
    "outsourcing-receipts",
    "outsourcing-vouchers",
    "business-modes",
    "business-mode-links",
    "mrp",
    "mrp-history",
    "capacity",
    "scheduling",
    "material-shortage",
    // ===== 质量域 =====
    "quality-inspections",
    "quality-issues",
    "quality-standards",
    "fabric-inspections",
    "fabric-defects",
    // ===== 财务域 =====
    "vouchers",
    "subjects",
    "fixed-assets",
    "budgets",
    "cost-collections",
    "financial-analysis",
    "fund-management",
    "currencies",
    "exchange-rates",
    "ar-reconciliations",
    // 应收对账增强面：根级直接资源，第四段是同一资源下的动作/维度；此前 URL 段可派生却从未进本表
    // （命中不了注册表即对非超管 fail-closed 403），补登记与 path_utils 的白名单条目同词。
    "ar-reconciliations-enhanced",
    "accounting-periods",
    "wages",
    "wage-rates",
    "wage-records",
    // 应付/应收/财务三域发票·付款·核销等资源随域改名（乙波 finance）：与
    // utils/path_utils.rs::RESOURCE_NAME_MAP 的消歧目标逐字符对齐，缺任一枚该面授权即成死码。
    "ap-invoices",
    "ap-payments",
    "ap-verifications",
    "ap-payment-requests",
    "ap-reconciliations",
    "ap-aging",
    "ap-daily",
    "ap-monthly",
    "ap-statistics",
    "ar-invoices",
    "ar-payments",
    "ar-verifications",
    "ar-collections",
    "ar-aging",
    "ar-daily",
    "ar-monthly",
    "ar-statistics",
    "finance-invoices",
    "finance-payments",
    "finance-audit",
    "finance-balance-sheet",
    "finance-cash-flow",
    "finance-general-ledger",
    "finance-income-statement",
    "finance-subsidiary-ledger",
    "finance-trial-balance",
    "finance-drill-down",
    // 财务族根级直接资源（第四段是维度/动作，非子资源；从模块前缀降级，与 slow-queries 同型）。
    "bad-debts",
    "collection-tasks",
    "finance-alerts",
    "assist-accounting",
    // 路由早已注册、资源段却从未进本表的收编（仅补登记，不改名、不扩权）。
    "environmental-tax",
    "export-refunds",
    "period-adjustments",
    // ===== 能耗域 =====
    "energy-meters",
    "energy-consumptions",
    "energy-rules",
    "energy-allocations",
    // ===== CRM 域 =====
    "crm-leads",
    "crm-opportunities",
    "crm-customers",
    "customers",
    "customer-credits",
    "five-dimension",
    // 客户转移审批流（/crm/transfer-approvals 族）：URL 第四段 transfer-approvals 即派生
    // 资源名（crm 为模块前缀、该段不在消歧映射表内，保留原名）。建单、详情、取消、经理审批、
    // 总监审批五面共用本资源名，动作键分别为 create/read/cancel/managerapprove/
    // directorapprove。后两者由 PATH_ACTION_SEGMENT_MAP 把 URL 末段（manager-approve/
    // director-approve）显式映射为合规动作码派生（动作码正形约束为纯小写字母或 `*`，
    // URL 保持连字符形态），与建单 create 键分离（SoD 职责分锁，
    // 见 init_service_ops/permission.rs 销售域分组）。
    "transfer-approvals",
    // ===== 物流与贸易域 =====
    "logistics",
    "ship-orders",
    "incoterms",
    // 出口商检单：运行时权限键 `export-inspections:{read,create,update,print}` 由 URL 段
    // 推导（seg3=export-inspections 为直接资源），必须在此登记否则矩阵授权行运行期永不命中。
    "export-inspections",
    // 纺织品安全合规域：运行时权限键 `textile-submissions:{read,create,update,delete}`、
    // `textile-reports:{read,create,update}`、`textile-test-items:{read,create,update,delete}`、
    // `textile-limit-rules:{read,create,update,delete}`、`textile-export-requirements:{read,create,update,delete}`
    // 均由 seg3 直接资源推导，判据同上。
    "textile-submissions",
    "textile-reports",
    "textile-test-items",
    "textile-limit-rules",
    "textile-export-requirements",
    // ===== 人力资源域 =====
    "employees",
    // ===== 安全环保域 =====
    "safety-records",
    "environmental-records",
    "equipment",
    "maintenance-records",
    // ===== 分析与报表域 =====
    "reports",
    // 以下五枚是"路由早已注册、资源段却从未进本表"的收编：路径均挂在 `/api/v1/erp/<段>`
    // 根级（segment3 即资源码），缺登记时中间件按"未知的资源路径"对**任何**角色拒（含管理员）；
    // 补登记只解决"路径认识"，能否放行仍由下面各岗位的授权行决定。
    // 期末报表快照：GET 列表/详情/verify 派生 read，POST 生成派生 create。
    "period-report-snapshots",
    // 账龄预警规则与账龄档位：财务侧配置主数据，GET=read、POST=create、PUT=update、DELETE=delete。
    "aging-alert-rules",
    "aging-grades",
    // 资产类别（固定资产卡片表单取数）与行业基准（财务分析对标取数）。
    "asset-categories",
    "industry-benchmarks",
    // 供应商资质到期扫描 `POST /supplier-qualifications/scan-expiry-warnings`：末段不在
    // 动作关键字表内，故按方法派生为 create（不是 scan）；读取资质走同资源 read。
    "supplier-qualifications",
    "bi-analysis",
    "dashboard",
    "sales-analysis",
    // ===== 通知与 OA 域 =====
    "notifications",
    "email-templates",
    "email-records",
    "oa-announcements",
    "business-trace",
    // ===== 系统域 =====
    "system-config",
    "audit-logs",
    "slow-queries",
    "print-templates",
    // 合同模板体系读写腿（模板主数据与条款项，routes/contract_template.rs 根级挂载）：
    // URL 段即权威资源名，与 path_utils 直接资源表同词登记；未登记则派生死键、全员 403。
    // 动作面只消费 read/create/update/delete/approve 五个已派生动作，不扩动作词表也不新增授予。
    "contract-templates",
    // 打印任务提交/查询、打印目标管理：与 print-templates 同族的根级直接资源，未登记则路由派生键
    // 命中不了任何授权行、对非超管恒 fail-closed 403。
    "print-tasks",
    "print-targets",
    // 公司档案：`GET/PUT /company-profile/full` 的 seg3 派生资源键 company-profile
    // （GET→read、PUT→update）；公开投影 `GET /company-profile` 走 AUTH_ONLY 精确豁免、
    // 不经业务权限码，但全量面必须在本注册表登记，否则 fail-closed 403。
    "company-profile",
    "data-import",
    "permission-audits",
    // ===== 系统更新域（高危敏感面，资源键统一带 system-update- 前缀与其余业务隔离）=====
    // 与 utils/path_utils.rs 的 system-update 消歧映射逐字符对齐：运行时权限键由 URL 段派生，
    // 这里的资源名必须与派生结果完全一致，否则系统更新授权行运行期永不命中（死码）。
    // 读取面九枚、写入面按派生规则覆盖 create 与 cancel 两类动作，全部在此登记。
    "system-update-update",
    "system-update-check",
    "system-update-rollback",
    "system-update-upload",
    "system-update-version",
    "system-update-update-status",
    "system-update-local-update",
    "system-update-local-check",
    "system-update-local-releases",
    "system-update-current-version",
    "system-update-tasks",
    "system-update-backups",
    "system-update-versions",
    // ===== 审批流域 =====
    "export-approvals",
    "role-change-approvals",
    // BPM 工作流面：`bpm` 是模块前缀，URL 第四段派生出的裸名 tasks/approval/templates 等与
    // 数据导入域的同名段冲突，故一律按 RESOURCE_NAME_MAP 消歧成 bpm-* 后再登记。
    // bpm-approvals 面只有 POST /bpm/approval/execute 一个端点，execute 不是路径动作关键字，
    // 其运行期键因此是 bpm-approvals:create（审批中心的执行入口），不存在 approve 形态的派生键。
    "bpm-tasks",
    "bpm-approvals",
    "bpm-instances",
    "bpm-process",
    "bpm-definitions",
    "bpm-versions",
    "bpm-templates",
    "bpm-monitor",
    "bpm-business-relation",
    "bpm-visualization",
    // ===== AI 智能域（V15 P0-S26 新增）=====
    // 对应 routes/analytics.rs ai() + advanced() AI 端点 + routes/system.rs ai_extend 端点
    "ai-forecast",
    "ai-inventory-opt",
    "ai-anomaly",
    "ai-recommendation",
    "ai-recipe-opt",
    "ai-quality-pred",
    "ai-process-opt",
    "ai-summary", // 大货批色审批：URL 段 bulk-color-approvals 即派生资源名（含 DELETE 清理通道），
    // 此前未登记 ⇒ 目录勾不到、种子授不了，非超管对该面只有超管通配可达。
    "bulk-color-approvals",
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
        use percent_encoding::{NON_ALPHANUMERIC, utf8_percent_encode};
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

/// 系统初始化服务（struct 定义保留在 facade，impl 块按职责分散到 `init_service_ops/` 子模块。）
pub struct InitService {
    /// 数据库连接句柄（`pub(crate)` 可见性：init_service_ops 兄弟模块的 impl 块需直接访问此字段。）
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
            InitError::ValidationError(e) => {
                AppError::validation_displayable(format!("参数校验失败: {}", e))
            }
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
