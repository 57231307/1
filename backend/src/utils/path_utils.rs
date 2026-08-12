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

/// 系统/基础设施类模块前缀（认证、IAM、通知、集成、流程、AI、管理域）
fn is_system_module_prefix(part: &str) -> bool {
    matches!(
        part,
        // ===== 认证与系统域 =====
        "auth"
            | "ws"
            | "init"
            | "system-update"
            | "dashboard"
            | "audit-logs"
            | "slow-queries"
            | "user"
            | "data-import"
            // ===== IAM 与组织域 =====
            | "data-permissions"
            | "user-notification-settings"
            // ===== 通知域 =====
            | "notifications"
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
            | "material-shortage"
            | "scheduling"
            // ===== 财务域 =====
            | "finance"
            | "ap"
            | "ar"
            | "assist-accounting"
            // V15 P1-14.4-D/14.12-B：补齐财务子模块前缀
            | "bad-debts"
            | "collection-tasks"
            | "finance-alerts"
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

/// 判断是否为已知资源段（模块前缀 + 直接资源，用于权限中间件拒绝未知路由）
pub fn is_known_resource_segment(part: &str) -> bool {
    // 先检查是否为模块前缀
    if is_module_prefix(part) {
        return true;
    }

    is_direct_resource(part)
}

/// V15 P1-14.4-C：模块前缀资源消歧映射表（同资源段跨模块时对齐权限定义；sales 域 orders 保留原名其余加 sales- 前缀，purchase 域全部加 purchase- 前缀）
pub fn resolve_module_prefixed_resource(module_prefix: &str, resource: &str) -> String {
    match (module_prefix, resource) {
        // ===== 采购域：权限定义使用 purchase- 前缀 =====
        ("purchase", "orders") => "purchase-orders".to_string(),
        ("purchase", "returns") => "purchase-returns".to_string(),
        ("purchase", "receipts") => "purchase-receipts".to_string(),
        ("purchase", "contracts") => "purchase-contracts".to_string(),
        ("purchase", "prices") => "purchase-prices".to_string(),
        // ===== 销售域：orders 保留原名，其余加 sales- 前缀 =====
        ("sales", "returns") => "sales-returns".to_string(),
        ("sales", "contracts") => "sales-contracts".to_string(),
        ("sales", "prices") => "sales-prices".to_string(),
        // ===== 其他情况：保留 resource 原名 =====
        _ => resource.to_string(),
    }
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
            | "fixed-assets"
            | "budgets"
            | "financial-analysis"
            | "fund-management"
            | "currencies"
            | "exchange-rates"
            | "ar-reconciliations"
            | "ar-reconciliations-enhanced"
            | "ar-reconciliation-alias"
            // ===== 生产直接资源 =====
            | "quality-standards"
            | "print-templates"
            | "suppliers"
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
            // ===== 审计与日志直接资源 =====
            | "logs"
            | "health"
            | "system-config"
    )
}
