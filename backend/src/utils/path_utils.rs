// V15 P0-S21 修复：扩展模块前缀白名单至 60+ 类，清理脏数据。
//
// 原实现仅 29 项，且含 15+ 脏数据（位置错误/单复数错误/非模块前缀），
// 导致生产域、采购域等关键模块的权限粒度降级为模块级。
// 现按路由实际挂载情况清理并补齐，覆盖 18 个业务域的全部模块前缀。
//
// 模块前缀定义：位于 URL segment3 位置（/api/v1/erp/{segment3}/{segment4}），
// 且 segment4 才是真实资源类型的路径段。extract_resource_info 据此判断
// resource_type 取 segment3 还是 segment4。

/// 判断是否为模块前缀（segment3 位置，其下 segment4 才是真实资源类型）
pub fn is_module_prefix(part: &str) -> bool {
    is_system_module_prefix(part) || is_business_module_prefix(part)
}

/// 系统/基础设施类模块前缀（认证、IAM、集成、流程、AI、管理域）
fn is_system_module_prefix(part: &str) -> bool {
    matches!(
        part,
        // ===== 认证与系统域 =====
        "auth"
            | "ws"
            | "init"
            | "system-update"
            | "audit-logs"
            | "user"
            | "data-import"
            // ===== IAM 与组织域 =====
            | "user-notification-settings"
            // ===== 集成与网关域 =====
            | "webhooks"
            | "api-gateway"
            // ===== 流程域 =====
            | "bpm"
            // ===== AI 域 =====
            | "ai"
            // ===== 管理域 =====
            | "admin"
    )
}

/// 业务类模块前缀（销售、采购、库存、生产、财务、CRM、质量、分析域）
fn is_business_module_prefix(part: &str) -> bool {
    matches!(
        part,
        // ===== 销售域 =====
        "sales"
            | "quotations"
            | "custom-orders"
            | "color-cards"
            | "color-prices"
            | "trading"
            // ===== 采购域（V15 修正：purchases → purchase，与实际路由一致）=====
            | "purchase"
            // ===== 库存仓储域 =====
            | "inventory"
            | "scanner"
            // ===== 生产域（V15 新增：原缺失导致 30+ 资源共用 production 权限码）=====
            | "production"
            | "production-orders"
            // material-shortage / scheduling 已由 V15 提升到根级（/material-shortage/*、/scheduling/*），
            // 其第四段是同一资源下的动作/查询维度而非子资源；留在模块前缀表会让资源名漂到 seg4
            // （alerts/list/summary/tasks），命中不了注册表权威名。改作直接资源，见 is_misc_direct_resource。
            // ===== 财务域 =====
            | "finance"
            | "ap"
            | "ar"
            // assist-accounting / bad-debts / collection-tasks / finance-alerts 是根级挂载的财务族
            // 直接资源（与上方 material-shortage/scheduling 同型）：其第四段是同一资源下的查询维度
            // 或动作（balance/records/writeoffs/auto-generate/trigger-scan…），不是子资源；留在模块前缀
            // 表会让资源名漂到 seg4、派生注册表外死键。改作直接资源，见 is_core_direct_resource。
            // ===== CRM 域 =====
            | "crm"
            // ===== 质量与追溯域 =====
            | "business-trace"
            // V15 P1-14.4-D/14.12-B：补齐质量子模块前缀
            | "quality-8d-reports"
            // ===== 分析与报表域 =====
            | "reports"
            | "bi"
            | "advanced"
            | "search"
            // V15 P1-14.4-D/14.12-B：补齐色卡与 OA 子模块前缀
            | "bulk-color-approvals"
            | "oa-announcements"
    )
}

/// 判断路径段是否为记录 ID（全为 ASCII 数字）。
///
/// 调用方：`middleware/permission.rs::extract_resource_info` 选资源名候选段时。
/// 为什么需要：`custom-orders`、`quotations`、`color-cards` 这类段既是模块前缀、本身又是资源，
/// 其 `/x/{id}` 面在运行时把数字 ID 交到"取 seg4 当资源名"的分支上，派生出 `123:read` 这种
/// `PERMISSION_RESOURCES` 里不存在、任何角色都授予不了的键（非超管打开详情面必 403）。
/// 命中本判据时资源名回落到模块前缀自身，ID 从同一段取回。
pub fn is_id_like_segment(part: &str) -> bool {
    !part.is_empty() && part.bytes().all(|b| b.is_ascii_digit())
}

/// 判断是否为已知资源段（模块前缀 + 直接资源，用于权限中间件拒绝未知路由）
pub fn is_known_resource_segment(part: &str) -> bool {
    // 先检查是否为模块前缀
    if is_module_prefix(part) {
        return true;
    }

    is_direct_resource(part)
}

/// 真·双层模块前缀组合（seg3 与 seg4 **都**在 [`is_module_prefix`] 表内，且 seg4 确实是
/// seg3 挂载下的子模块，资源名落在 seg5）。
///
/// 为什么需要这张表：`extract_resource_info` 的"两段都是模块前缀就跳段取 seg5"判据
/// 只看词面，不看挂载关系。同一个词在别的挂载点下是子模块、在本挂载点下却是**资源名**，
/// 跳段就会把查询维度/动作段当成权限资源名，派生出 `by-time:read`、`kpi:read` 这类
/// 注册表里根本没有、任何角色都无法被授予的伪键（fail-closed ⇒ 除通配 admin 外全员 403，
/// 且本域已登记的授权键成为死码）。
///
/// 表内容 = 全量挂载逐条实证（`frontend/scripts/route-snapshot.txt` 中 seg3/seg4 同时
/// 命中模块前缀的全部组合）：未列出的组合一律**不跳段**，资源名取 seg4。
/// 新增子模块挂载时必须同时在此登记，否则该挂载的资源键会漂到 seg5 的维度名上。
pub fn is_nested_module_prefix(module_prefix: &str, sub_prefix: &str) -> bool {
    matches!(
        (module_prefix, sub_prefix),
        // advanced 域下挂 ai / reports 两个子模块（资源名在 seg5：
        // anomaly-detection、recipe-optimization、templates、execute…）
        ("advanced", "ai")
            | ("advanced", "reports")
            // 应付/应收各自的报表子模块（aging/daily/monthly/statistics）
            | ("ap", "reports")
            | ("ar", "reports")
            // 色卡域下的报表子模块（customer-ledger/issue-detail/…）
            | ("color-cards", "reports")
            // 财务域下的报表子模块（balance-sheet/cash-flow/…）
            | ("finance", "reports")
            // 生产域下的生产工单子模块（f059841b 消歧的那一族，资源名在 seg5="orders"）
            | ("production", "production-orders")
            // purchase 挂载下的 purchase 子模块（inspections/returns）
            | ("purchase", "purchase")
    )
}

/// V15 P1-14.4-C：模块前缀资源消歧映射表（同资源段跨模块时对齐权限定义；sales 域 orders 保留原名其余加 sales- 前缀，purchase 域全部加 purchase- 前缀）
/// 「URL 段 → 权限资源名」唯一映射表：(模块前缀 seg3, 该前缀下取到的段, PERMISSION_RESOURCES 权威名)。
/// 未列出的组合保留 URL 段原名，与旧 match 的默认分支逐字节同语义；本表只做对齐，不新增任何"角色 × 资源 × 动作"授权。
/// 挂载统一波次改名时，改动只落在「路由注册 + 本表」两处，不得再往查表函数里加分支特例。
const RESOURCE_NAME_MAP: &[(&str, &str, &str)] = &[
    ("bi", "sales", "bi-analysis"),
    ("purchase", "orders", "purchase-orders"),
    ("purchase", "returns", "purchase-returns"),
    ("purchase", "receipts", "purchase-receipts"),
    ("purchase", "contracts", "purchase-contracts"),
    ("purchase", "prices", "purchase-prices"),
    ("sales", "returns", "sales-returns"),
    ("sales", "contracts", "sales-contracts"),
    ("sales", "prices", "sales-prices"),
    ("crm", "leads", "crm-leads"),
    ("crm", "opportunities", "crm-opportunities"),
    ("crm", "rfm", "customers"),
    ("production", "orders", "production-orders"),
    ("production", "quality-inspection", "quality-inspections"),
    ("system-update", "update", "system-update-update"),
    ("system-update", "check", "system-update-check"),
    ("system-update", "rollback", "system-update-rollback"),
    ("system-update", "upload", "system-update-upload"),
    ("system-update", "version", "system-update-version"),
    (
        "system-update",
        "update-status",
        "system-update-update-status",
    ),
    (
        "system-update",
        "local-update",
        "system-update-local-update",
    ),
    ("system-update", "local-check", "system-update-local-check"),
    (
        "system-update",
        "local-releases",
        "system-update-local-releases",
    ),
    (
        "system-update",
        "current-version",
        "system-update-current-version",
    ),
    ("system-update", "tasks", "system-update-tasks"),
    ("system-update", "backups", "system-update-backups"),
    ("system-update", "versions", "system-update-versions"),
    // ===== 财务域资源随域改名（乙波 finance）=====
    // ap/ar/finance 三个模块前缀下同名 URL 段（invoices/payments/verifications…）必须各归各域，
    // 否则一名两键、跨域越权或派生注册表外死键。报表子模块（ap/ar/finance 的 reports）资源名在
    // seg5，走 is_nested_module_prefix 跳段后仍由本表按 (前缀, seg5段) 消歧，跳段判据不动。
    ("ap", "invoices", "ap-invoices"),
    ("ap", "payments", "ap-payments"),
    ("ap", "verifications", "ap-verifications"),
    ("ap", "payment-requests", "ap-payment-requests"),
    ("ap", "reconciliations", "ap-reconciliations"),
    ("ap", "aging", "ap-aging"),
    ("ap", "daily", "ap-daily"),
    ("ap", "monthly", "ap-monthly"),
    ("ap", "statistics", "ap-statistics"),
    ("ar", "invoices", "ar-invoices"),
    ("ar", "payments", "ar-payments"),
    ("ar", "verifications", "ar-verifications"),
    ("ar", "collections", "ar-collections"),
    ("ar", "aging", "ar-aging"),
    ("ar", "daily", "ar-daily"),
    ("ar", "monthly", "ar-monthly"),
    ("ar", "statistics", "ar-statistics"),
    ("finance", "invoices", "finance-invoices"),
    ("finance", "payments", "finance-payments"),
    ("finance", "audit", "finance-audit"),
    ("finance", "balance-sheet", "finance-balance-sheet"),
    ("finance", "cash-flow", "finance-cash-flow"),
    ("finance", "general-ledger", "finance-general-ledger"),
    ("finance", "income-statement", "finance-income-statement"),
    ("finance", "subsidiary-ledger", "finance-subsidiary-ledger"),
    ("finance", "trial-balance", "finance-trial-balance"),
    ("finance", "drill-down", "finance-drill-down"),
    ("bpm", "tasks", "bpm-tasks"),
    ("bpm", "approval", "bpm-approvals"),
    ("bpm", "instances", "bpm-instances"),
    ("bpm", "process", "bpm-process"),
    ("bpm", "definitions", "bpm-definitions"),
    ("bpm", "versions", "bpm-versions"),
    ("bpm", "templates", "bpm-templates"),
    ("bpm", "monitor", "bpm-monitor"),
    ("bpm", "business-relation", "bpm-business-relation"),
    ("bpm", "visualization", "bpm-visualization"),
];

/// 按映射表把模块前缀下的 URL 段对齐到注册表权威资源名；未命中保留原名。
/// 调用方：`middleware/permission.rs::extract_resource_info`（运行时键派生）与权限目录/种子侧同源校验。
pub fn resolve_module_prefixed_resource(module_prefix: &str, resource: &str) -> String {
    for (mp, seg, canonical) in RESOURCE_NAME_MAP {
        if mp == &module_prefix && seg == &resource {
            return (*canonical).to_string();
        }
    }
    resource.to_string()
}

/// 判断是否为直接资源（非模块前缀的 segment3 合法值）
fn is_direct_resource(part: &str) -> bool {
    is_core_direct_resource(part) || is_misc_direct_resource(part)
}

/// 核心业务直接资源（IAM、产品目录、财务、生产域）
fn is_core_direct_resource(part: &str) -> bool {
    matches!(
        part,
        // ===== IAM 直接资源 =====
        "users"
            | "roles"
            | "departments"
            | "permissions"
            | "field-permissions"
            | "permission-audits"
            // 数据权限配置面：根级直接资源（与 print-tasks、company-profile 同判据）。其下第四段是
            // 记录 ID、按角色查询的维度段或资源类型/范围类型两个下拉取数面，均非子资源；留在模块
            // 前缀表会让资源名漂到第四段，派生出 resource-types、scope-types 这类注册表外、任何
            // 角色都授予不了的伪键，按角色面还会被错认成角色域的 roles 键（持 roles:read 的岗位
            // 越能进本面、授 data-permissions:read 的岗位反而进不来）。登记为直接资源后列表、详情、
            // 按角色、两个下拉与写删各面统一派生 data-permissions，与注册表逐字符一致。
            | "data-permissions"
            // ===== 产品目录直接资源 =====
            | "products"
            | "categories"
            | "product-categories"
            | "warehouses"
            | "boms"
            | "chemicals"
            | "chemical-categories"
            | "chemical-lots"
            | "chemical-requisitions"
        // ===== 财务直接资源 =====
        | "subjects"
            | "vouchers"
            | "accounting-periods"
            | "fixed-assets"
            | "budgets"
            | "financial-analysis"
            | "fund-management"
            | "currencies"
            | "exchange-rates"
            | "ar-reconciliations"
            | "ar-reconciliations-enhanced"
            | "period-report-snapshots"
            | "aging-alert-rules"
            | "aging-grades"
            | "asset-categories"
            | "industry-benchmarks"
            // 财务族根级直接资源（第四段是维度/动作，非子资源；从模块前缀表迁入，与 slow-queries/
            // material-shortage 同型）：URL 段即权威资源名，登记后派生键与 PERMISSION_RESOURCES 同词。
            | "bad-debts"
            | "collection-tasks"
            | "finance-alerts"
            | "assist-accounting"
            // ===== 生产直接资源 =====
            | "quality-standards"
            | "print-templates"
            // 打印任务是 print-templates 同族的根级直接资源：URL 段即权威资源名（无模块前缀、
            // 末段是记录 ID 而非子资源）。不在白名单则 permission 中间件 validate_route_whitelist
            // 对全员（含 admin）fail-closed 403；登记后派生键即 print-tasks，与 PERMISSION_RESOURCES
            // 及授权种子同词表。print-targets（管理打印机/打印机池）为同族根级直接资源，一并登记，
            // 派生键即 print-targets。
            | "print-tasks"
            | "print-targets"
            | "suppliers"
            | "supplier-qualifications"
    )
}

/// 辅助功能直接资源（分析、登录安全、邮件、AI、审计日志域）
fn is_misc_direct_resource(part: &str) -> bool {
    matches!(
        part,
        // ===== 分析与高级功能直接资源 =====
        "convert"
            | "validate"
            | "csv"
            | "excel"
            | "xlsx"
            | "stream"
            | "templates"
            | "report-templates"
            | "execute"
            | "export"
            | "aggregate"
            | "cache"
            | "page-view"
            | "popular-pages"
            | "behavior"
            | "funnel"
            | "user-path"
            // ===== 登录安全直接资源 =====
            | "login-logs"
            | "lock-status"
            | "unlock"
            | "login-statistics"
            | "stats"
            | "security-alerts"
            | "alerts"
            | "locked-accounts"
            // ===== 邮件直接资源 =====
            | "send"
            | "email-templates"
            | "email-records"
            | "email-statistics"
            // ===== AI 智能分析直接资源 =====
            | "forecast-sales"
            | "optimize-inventory"
            | "detect-anomalies"
            | "recommendations"
            // V15 P1 4.1+4.2：AI 端点资源类型白名单（含 advanced 域子资源）
            | "process-optimizations"
            | "quality-predictions"
            | "summary"
            | "by-color"
            | "by-product"
            | "recipe-optimization"
            | "quality-prediction"
            | "sales-forecast"
            | "inventory-optimization"
            | "anomaly-detection"
        // ===== 仪表板与站内信：整体是一个注册资源 =====
        // /dashboard/{overview,sales-stats,layout,...} 与 /notifications/{unread-count,read-all,:id,...}
        // 的第四段是同一资源下的动作或记录 ID，不是独立权限资源；
        // 权限注册表与角色种子里登记的资源名就是 dashboard / notifications。
        // 若误判为模块前缀，推导出的资源名会变成 sales-stats/unread-count/<记录ID>，
        // 任何角色种子都不含这些名字，非 admin 用户必然 403（fail-closed 静默失效）。
        | "dashboard"
        | "notifications"
        // ===== 缺料预警与排产：根级挂载，第四段是动作/维度不是子资源（与 dashboard 同型）=====
        | "material-shortage"
        | "scheduling"
        // ===== 审计与日志直接资源 =====
        | "logs"
        // 慢查询 `/slow-queries/stats` 的 stats 是同一资源下的查询维度而非子资源；
        // 留在模块前缀表会让资源名漂到 stats、派生成注册表外死键，改作直接资源后
        // 根级与 stats 面统一落到已登记的 slow-queries。
        | "slow-queries"
        | "health"
        | "system-config"
        // 公司档案根级直接资源：URL 段即权威资源名（无模块前缀；`/company-profile` 与
        // `/company-profile/full` 的 seg3 均派生 company-profile，full 是同一单例资源下的
        // 读面维度而非子资源）。不在白名单则 permission 中间件 validate_route_whitelist
        // 对全员（含 admin）fail-closed 403；登记后派生键与 PERMISSION_RESOURCES 及
        // system_admin 授权种子同词表。公开投影 GET /company-profile 走 AUTH_ONLY 精确
        // 豁免（不经 RBAC），全量读写在 /company-profile/full 过本资源权限码。
        | "company-profile"
        // ===== 单据号查重（前端自动生成单据号后确认唯一性）=====
        | "document-no"
        // V16 P0：审批流与 AI 模型直接资源（traversal 全量对照补齐，admin 被拒修复）
        | "export-approvals"
        | "role-change-approvals"
        | "ai-models"
        // V16 P0：外贸/职业健康/物流域直接资源（37-print 矩阵 403 修复）
        | "export-inspections"
        // 纺织品安全合规域直接资源（seg3 即权限资源名；未登记段会被白名单层
        // fail-closed 403 含 admin，登记后派生键与 PERMISSION_RESOURCES 及角色种子同词表）
        | "textile-submissions"
        | "textile-reports"
        | "textile-test-items"
        | "textile-limit-rules"
        | "textile-export-requirements"
        | "export-refunds"
        | "labor-contracts"
        | "logistics-tracking"
        | "occupational-health"
        | "pollution-monitoring"
        | "pollution-permits"
        | "social-insurance"
        // ===== 简化版修复批次新增域资源（缺白名单会被 fail-closed 403）=====
        | "outsourcing-orders"
        | "outsourcing-receipts"
        | "wage-rates"
        | "wage-records"
        | "wage-details"
        | "period-adjustments"
        | "contract-signatures"
        | "customer-shares"
        | "customer-team-members"
        | "environmental-tax"
        | "incoterms"
        | "device-connections"
        | "permission-delegations"
        | "role-relations"
        | "fabric-inspections"
        | "fabric-defects"
        | "chemical-categories"
        | "chemical-lots"
        // ===== 51 全端点扫描实证缺失的资源段（admin 全权仍被 fail-closed 拒绝）=====
        | "audit"
        | "business-modes"
        | "business-mode-links"
        // ===== 隐私同意域（与真实挂载对齐）=====
        // 实际挂载 = /api/v1/erp/privacy/{consents,opt-in-all,opt-out-all}（routes/analytics.rs
        // `.nest("/privacy", privacy())`），seg3=privacy。曾在此登记 "consents"（seg4 误登记为
        // seg3 的挂载漂移），导致端点对外不可达、admin 也被白名单层 403（未知的资源路径）。
        // 第四段是同一资源下的动作/查询面（consents/opt-in-all），与上方 dashboard/notifications
        // 判据同型，故按直接资源登记 privacy（权限资源名=privacy），不得改登记 seg4 伪资源名。
        | "privacy"
        | "customers"
        | "dye-batches"
        | "dye-batch-lifecycle-logs"
        | "dye-batch-operations"
        | "dye-batch-reworks"
        | "dye-batch-state-rules"
        | "dye-recipes"
        | "energy-allocations"
        | "energy-consumptions"
        | "energy-meters"
        | "energy-rules"
        | "flow-cards"
        | "lab-dip"
        | "logistics"
        | "me"
        | "mrp"
        | "mrp-history"
        | "orders"
        | "outsourcing-vouchers"
        | "payments"
        | "pool"
        | "process-routes"
        | "production-recipes"
        | "purchase-prices"
        | "receipts"
        | "sales-contracts"
        | "sales-prices"
        | "sales-returns"
        | "sales-users"
        | "stock"
        | "totp"
        | "unread-count"
        | "warnings"
    )
}
