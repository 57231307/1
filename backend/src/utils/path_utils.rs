/// 判断是否为模块前缀（如 sales, purchases, finance 等）
pub fn is_module_prefix(part: &str) -> bool {
    matches!(
        part,
        "sales"
            | "purchases"
            | "finance"
            | "inventory"
            | "gl"
            | "ap"
            | "ar"
            | "bpm"
            | "crm"
            | "ai"
            | "reports"
            | "tenants"
            | "webhooks"
            | "api-keys"
            | "supplier-evaluation"
            | "customer-credits"
            | "financial-analysis"
            | "fund-management"
            | "quality-inspection"
            | "cost-collections"
            | "sales-analysis"
            | "sales-prices"
            | "purchase-prices"
            | "sales-returns"
            | "ar-reconciliations"
            | "exchange-rates"
    )
}
