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
            | "slow-queries"
            | "user"
            | "data-import"
            // ===== IAM 与组织域 =====
            | "data-permissions"
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
pub fn resolve_module_prefixed_resource(module_prefix: &str, resource: &str) -> String {
    match (module_prefix, resource) {
        // ===== BI 分析域：`/erp/bi/sales/*` 的资源段消歧 =====
        // seg4="sales" 同时是销售域的模块前缀词（`is_module_prefix` 命中），历史上被
        // 双层前缀规则跳段取了 seg5（by-time/by-customer/trend/kpi/pivot…），而这些是
        // **查询维度/动作段**、不是资源；派生出的 `by-time:read` 等键在注册表
        // （`init_service.rs` 资源清单）与角色种子中都不存在 ⇒ BI 销售分析对持
        // `bi-analysis:read` 授权的角色也恒 403（已登记的键成死码）。
        // 消歧到注册表权威名 bi-analysis（与同域 `sales-analysis` 的登记方式同构），
        // 且必须与 `is_nested_module_prefix` 的"bi/sales 不是子模块"判定同时成立才生效。
        // 方向为纯收窄：BI 端点只认 BI 自己的键，不因 URL 里出现 "sales" 一词
        // 就让任何持有销售域键的角色顺带读到 BI 面（与 production-orders 那一刀同理，
        // 不新增任何"角色 × 资源 × 动作"授权）。
        ("bi", "sales") => "bi-analysis".to_string(),
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
        // ===== CRM 域：`/erp/crm/leads*` 的资源段消歧 =====
        // "leads" 未登记在 `init_service::PERMISSION_RESOURCES`，注册表权威名是
        // `crm-leads`。不消歧的后果双向都错：① 矩阵里已登记的 ("crm-leads", *) 授权行
        // （crm_manager/crm_rep/customer_service 等）对本域列表/详情/写操作是**死码**，
        // 持权岗位访问自己的线索反而 403；② 运行时要匹配上只能靠库里存在未登记的
        // ("leads", *) 行，等于把权限键名定在注册表之外。
        // 消歧到注册表权威名 crm-leads，与 purchase-/sales- 前缀族同构，纯对齐不新增授权。
        ("crm", "leads") => "crm-leads".to_string(),
        // ===== 生产域：`/erp/production/production-orders/orders*` 的资源段消歧 =====
        // 该路由是双层模块前缀（seg3=production、seg4=production-orders 均在
        // is_module_prefix 表内），extract_resource_info（middleware/permission.rs:274-279）
        // 取 seg5="orders" 走默认分支 ⇒ 派生成销售订单的 `orders:*` 码。后果双向都错：
        // ① 持有 ("orders","read") 的销售角色（种子 permission.rs:285 等）能通过
        //    中间件读到**生产工单**列表与详情（跨域越权面，行级 scope 只是部分缓解）；
        // ② 生产侧自己的 ("production-orders","*")（:484/:526，注册表
        //    init_service.rs 亦登记 production-orders）对本域列表/详情是**死码**，
        //    生产岗访问自己的生产工单反而 403。
        // 消歧到注册表权威名 production-orders，同时关掉越权面并复活死授权；
        // 纯收窄，不新增任何"角色×资源×动作"。
        ("production", "orders") => "production-orders".to_string(),
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
            | "accounting-periods"
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
        // ===== 仪表板与站内信：整体是一个注册资源 =====
        // /dashboard/{overview,sales-stats,layout,...} 与 /notifications/{unread-count,read-all,:id,...}
        // 的第四段是同一资源下的动作或记录 ID，不是独立权限资源；
        // 权限注册表与角色种子里登记的资源名就是 dashboard / notifications。
        // 若误判为模块前缀，推导出的资源名会变成 sales-stats/unread-count/<记录ID>，
        // 任何角色种子都不含这些名字，非 admin 用户必然 403（fail-closed 静默失效）。
        | "dashboard"
        | "notifications"
        // ===== 审计与日志直接资源 =====
        | "logs"
        | "health"
        | "system-config"
        // ===== 单据号查重（前端自动生成单据号后确认唯一性）=====
        | "document-no"
        // V16 P0：审批流与 AI 模型直接资源（traversal 全量对照补齐，admin 被拒修复）
        | "export-approvals"
        | "role-change-approvals"
        | "ai-models"
        // V16 P0：外贸/职业健康/物流域直接资源（37-print 矩阵 403 修复）
        | "export-inspections"
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
