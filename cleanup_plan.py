import re

file_path = "frontend/src/app/mod.rs"
with open(file_path, "r", encoding="utf-8") as f:
    content = f.read()

# Modules to keep for Fabric Wholesale ERP
keep_modules = [
    # Auth & System
    "Init", "Login", "Dashboard", "Users", "Roles", "OperationLogs",
    
    # Base Info
    "Products", "ProductCategories", "Warehouses", "Departments", "Customers", "Suppliers",
    
    # Inventory
    "Inventory", "Transfers", "Counts", "InventoryReservations", "InventoryAdjustments",
    
    # Sales
    "Sales", "SalesDeliveries", "SalesReturns", "CustomerStatement",
    
    # Purchase
    "PurchaseOrders", "PurchaseReceipts", "PurchaseReturns", "PurchasePrices",
    
    # Finance
    "ArInvoices", "ApInvoices", "ArVerifications", "ApVerifications"
]

# All existing routes that are NOT in keep_modules should be removed
# For example: FabricOrders, SalesContracts, FinanceInvoices, FinancePayments, PurchasePrices, SalesPrices, SalesReturns (keep this), SalesAnalysis
# CostCollection, FinancialAnalysis, AssistAccounting, FixedAsset, FundManagement, ApPaymentRequest, ApPayment, ApReconciliation, ApReport
# BusinessTrace, Voucher, AccountSubject, CrmLead, CrmOpportunity, CustomerCredit, SupplierEvaluation, QualityInspection, PurchaseInspection, PurchaseContract

print("Done planning cleanup.")
