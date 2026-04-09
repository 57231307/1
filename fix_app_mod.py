import re

file_path = "frontend/src/app/mod.rs"
with open(file_path, "r", encoding="utf-8") as f:
    content = f.read()

# We need to remove the broken imports. Let's just rewrite the import block.
import_pattern = r"use crate::pages::\{(.*?)\};"

def replace_imports(match):
    imports = match.group(1).replace('\n', ' ').split(',')
    valid_imports = []
    
    broken_imports = [
        "crm_lead::CrmLeadPage", "crm_opportunity::CrmOpportunityPage", "AccountSubjectPage",
        "ApPaymentPage", "ApPaymentRequestPage", "ApReconciliationPage", "ApReportPage",
        "AssistAccountingPage", "BatchPage", "BudgetManagementPage", "BusinessTracePage",
        "CostCollectionPage", "CustomerCreditPage", "DualUnitConverterPage", "FabricOrderPage",
        "FinanceInvoicePage", "FinancePaymentPage", "FinancialAnalysisPage", "FiveDimensionPage",
        "FixedAssetPage", "FundManagementPage", "PurchaseContractPage", "PurchaseInspectionPage",
        "PurchasePricePage", "QualityInspectionPage", "SalesAnalysisPage", "SalesContractPage",
        "SalesPricePage", "SupplierEvaluationPage", "VoucherPage"
    ]
    
    for imp in imports:
        imp = imp.strip()
        if not imp:
            continue
        if imp not in broken_imports:
            valid_imports.append(imp)
            
    return "use crate::pages::{\n    " + ",\n    ".join(valid_imports) + "\n};"

content = re.sub(import_pattern, replace_imports, content, flags=re.DOTALL)

with open(file_path, "w", encoding="utf-8") as f:
    f.write(content)

print("Updated app/mod.rs imports")
