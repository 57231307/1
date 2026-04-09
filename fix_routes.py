import re

file_path = "frontend/src/app/mod.rs"
with open(file_path, "r", encoding="utf-8") as f:
    content = f.read()

broken_routes = [
    "FabricOrders", "SalesContracts", "FinanceInvoices", "FinancePayments",
    "PurchasePrices", "SalesPrices", "SalesAnalysis", "SupplierEvaluations",
    "AccountSubjects", "Vouchers", "QualityInspections", "Batches",
    "FundManagement", "FixedAssets", "CustomerCredits", "DualUnitConverter",
    "FiveDimensions", "BusinessTrace", "ApPaymentRequests", "ApPayments",
    "ApReconciliations", "ApReports", "AssistAccounting", "PurchaseContracts",
    "CostCollections", "PurchaseInspections", "CrmLeads", "CrmOpportunities",
    "BudgetManagement"
]

# We need to remove these from enum Route and from fn switch
for route in broken_routes:
    # Remove from enum Route
    content = re.sub(r'#\[at\("[^"]+"\)]\s+' + route + r',?\s*', '', content)
    # Remove from match switch
    content = re.sub(r'Route::' + route + r'\s*=>\s*protected_route\(\|\| html! \{ <[^>]+> \}\),?\s*', '', content)

with open(file_path, "w", encoding="utf-8") as f:
    f.write(content)

print("Removed broken routes")
