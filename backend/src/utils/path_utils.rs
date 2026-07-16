// V15 P0-S21 修复：扩展模块前缀白名单至 60+ 类，清理脏数据。
//
// 原实现仅 29 项，且含 15+ 脏数据（位置错误/单复数错误/非模块前缀），
// 导致生产域、采购域等关键模块的权限粒度降级为模块级。
// 现按路由实际挂载情况清理并补齐，覆盖 18 个业务域的全部模块前缀。
//
// 模块前缀定义：位于 URL segment3 位置（/api/v1/erp/{segment3}/{segment4}），
// 且 segment4 才是真实资源类型的路径段。extract_resource_info 据此判断
// resource_type 取 segment3 还是 segment4。

/// 判断是否为模块前缀（如 sales, purchase, finance, production 等）
///
/// 模块前缀位于 URL segment3 位置，其下 segment4 才是真实资源类型。
/// 例如 `/api/v1/erp/sales/orders` 中 `sales` 是模块前缀，`orders` 是资源类型。
pub fn is_module_prefix(part: &str) -> bool {
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
            // ===== 销售域 =====
            | "sales"
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
            // ===== CRM 域 =====
            | "crm"
            // ===== 质量与追溯域 =====
            | "business-trace"
            // ===== 分析与报表域 =====
            | "reports"
            | "bi"
            | "advanced"
            | "search"
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

/// 判断是否为已知资源段（segment3 位置的所有合法值）
///
/// V15 P0-S21 新增：用于权限中间件拒绝未知路由。
/// 包含所有合法的 segment3 值（模块前缀 + 直接资源），总数 60+。
/// 若请求路径的 segment3 不在此列表中，权限中间件将直接拒绝。
pub fn is_known_resource_segment(part: &str) -> bool {
    // 先检查是否为模块前缀
    if is_module_prefix(part) {
        return true;
    }

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
            // ===== 分析与高级功能直接资源 =====
            | "convert"
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
            // ===== 审计与日志直接资源 =====
            | "logs"
            | "health"
            | "system-config"
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    // ===== is_module_prefix 测试 =====

    #[test]
    fn test_is_module_prefix_销售域() {
        assert!(is_module_prefix("sales"));
        assert!(is_module_prefix("quotations"));
        assert!(is_module_prefix("custom-orders"));
        assert!(is_module_prefix("color-cards"));
        assert!(is_module_prefix("color-prices"));
        assert!(is_module_prefix("trading"));
    }

    #[test]
    fn test_is_module_prefix_采购域修正拼写() {
        // V15 修正：purchases → purchase
        assert!(is_module_prefix("purchase"));
        assert!(!is_module_prefix("purchases"));
    }

    #[test]
    fn test_is_module_prefix_生产域新增() {
        // V15 新增：production 原缺失
        assert!(is_module_prefix("production"));
        assert!(is_module_prefix("material-shortage"));
        assert!(is_module_prefix("scheduling"));
    }

    #[test]
    fn test_is_module_prefix_财务域() {
        assert!(is_module_prefix("finance"));
        assert!(is_module_prefix("ap"));
        assert!(is_module_prefix("ar"));
        assert!(is_module_prefix("assist-accounting"));
    }

    #[test]
    fn test_is_module_prefix_认证与系统域() {
        assert!(is_module_prefix("auth"));
        assert!(is_module_prefix("ws"));
        assert!(is_module_prefix("init"));
        assert!(is_module_prefix("system-update"));
        assert!(is_module_prefix("dashboard"));
        assert!(is_module_prefix("audit-logs"));
        assert!(is_module_prefix("slow-queries"));
        assert!(is_module_prefix("user"));
        assert!(is_module_prefix("data-import"));
    }

    #[test]
    fn test_is_module_prefix_分析与报表域() {
        assert!(is_module_prefix("reports"));
        assert!(is_module_prefix("bi"));
        assert!(is_module_prefix("advanced"));
        assert!(is_module_prefix("search"));
    }

    #[test]
    fn test_is_module_prefix_已清理脏数据() {
        // V15 清理：以下脏数据应已移除
        assert!(!is_module_prefix("purchases")); // 拼写错误
        assert!(!is_module_prefix("api-keys")); // 路径不存在
        assert!(!is_module_prefix("gl")); // 不是路径段
        assert!(!is_module_prefix("supplier-evaluation")); // 位置错误
        assert!(!is_module_prefix("customer-credits")); // 位置错误
        assert!(!is_module_prefix("quality-inspection")); // 位置错误
        assert!(!is_module_prefix("cost-collections")); // 位置错误
        assert!(!is_module_prefix("sales-analysis")); // 位置错误
        assert!(!is_module_prefix("sales-prices")); // 位置错误
        assert!(!is_module_prefix("purchase-prices")); // 位置错误
        assert!(!is_module_prefix("sales-returns")); // 位置错误
        assert!(!is_module_prefix("financial-analysis")); // 非模块前缀
        assert!(!is_module_prefix("fund-management")); // 非模块前缀
        assert!(!is_module_prefix("ar-reconciliations")); // 非模块前缀
        assert!(!is_module_prefix("exchange-rates")); // 非模块前缀
    }

    #[test]
    fn test_is_module_prefix_未知段返回false() {
        assert!(!is_module_prefix("unknown-module"));
        assert!(!is_module_prefix(""));
        assert!(!is_module_prefix("fake"));
    }

    // ===== is_known_resource_segment 测试 =====

    #[test]
    fn test_is_known_resource_segment_包含所有模块前缀() {
        assert!(is_known_resource_segment("sales"));
        assert!(is_known_resource_segment("purchase"));
        assert!(is_known_resource_segment("production"));
        assert!(is_known_resource_segment("finance"));
    }

    #[test]
    fn test_is_known_resource_segment_包含直接资源() {
        assert!(is_known_resource_segment("users"));
        assert!(is_known_resource_segment("roles"));
        assert!(is_known_resource_segment("departments"));
        assert!(is_known_resource_segment("products"));
        assert!(is_known_resource_segment("vouchers"));
        assert!(is_known_resource_segment("suppliers"));
        assert!(is_known_resource_segment("system-config"));
        assert!(is_known_resource_segment("health"));
    }

    #[test]
    fn test_is_known_resource_segment_未知段返回false() {
        assert!(!is_known_resource_segment("unknown-resource"));
        assert!(!is_known_resource_segment(""));
        assert!(!is_known_resource_segment("hack"));
    }
}
