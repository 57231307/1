import os
import re
from glob import glob

base_dir = "frontend/src/pages/"

# List of files to delete
files_to_delete = [
    "account_subject.rs",
    "ap_payment.rs",
    "ap_payment_request.rs",
    "ap_reconciliation.rs",
    "ap_report.rs",
    "assist_accounting.rs",
    "batch.rs",
    "budget_management.rs",
    "business_trace.rs",
    "cost_collection.rs",
    "crm_lead.rs",
    "crm_opportunity.rs",
    "customer_credit.rs",
    "dual_unit_converter.rs",
    "fabric_order.rs",
    "finance_invoice.rs",
    "finance_payment.rs",
    "financial_analysis.rs",
    "five_dimension.rs",
    "fixed_asset.rs",
    "fund_management.rs",
    "inventory_adjustment.rs", # actually keep if needed, but let's see. The spec says 盘点,调拨,异常处理(报损报溢), we have inventory_count, inventory_transfer, maybe we should keep inventory_adjustment. Yes, keep it.
    "product_category.rs", # might keep
    "purchase_contract.rs",
    "purchase_inspection.rs",
    "purchase_price.rs",
    "quality_inspection.rs",
    "sales_analysis.rs",
    "sales_contract.rs",
    "sales_price.rs",
    "supplier_evaluation.rs",
    "voucher.rs",
    "warehouse_list.rs" # wait, we need warehouse management
]

# Keep:
# dashboard.rs, login.rs, init.rs, mod.rs, 
# operation_log.rs, user_list.rs, role_list.rs,
# customer.rs, supplier.rs, product_list.rs, warehouse_list.rs, department_list.rs
# purchase_order.rs, purchase_receipt.rs, purchase_return.rs, 
# sales_order.rs, sales_delivery.rs, sales_return.rs, customer_statement.rs,
# inventory_stock.rs, inventory_transfer.rs, inventory_count.rs, inventory_reservation.rs, inventory_adjustment.rs
# ar_invoice.rs, ar_receipt.rs, ar_verification.rs
# ap_invoice.rs, ap_verification.rs

to_remove = [
    "account_subject.rs", "ap_payment.rs", "ap_payment_request.rs", "ap_reconciliation.rs", "ap_report.rs",
    "assist_accounting.rs", "batch.rs", "budget_management.rs", "business_trace.rs", "cost_collection.rs",
    "crm_lead.rs", "crm_opportunity.rs", "customer_credit.rs", "dual_unit_converter.rs", "fabric_order.rs",
    "finance_invoice.rs", "finance_payment.rs", "financial_analysis.rs", "five_dimension.rs", "fixed_asset.rs",
    "fund_management.rs", "purchase_contract.rs", "purchase_inspection.rs", "purchase_price.rs",
    "quality_inspection.rs", "sales_analysis.rs", "sales_contract.rs", "sales_price.rs", "supplier_evaluation.rs",
    "voucher.rs"
]

for f in to_remove:
    path = os.path.join(base_dir, f)
    if os.path.exists(path):
        os.remove(path)
        print(f"Deleted {path}")

# Now we need to update frontend/src/pages/mod.rs
with open("frontend/src/pages/mod.rs", "r", encoding="utf-8") as f:
    mod_lines = f.readlines()

new_mod_lines = []
for line in mod_lines:
    keep = True
    for f in to_remove:
        mod_name = f.replace('.rs', '')
        if f"pub mod {mod_name};" in line or f"pub use {mod_name}::" in line:
            keep = False
            break
    if keep:
        new_mod_lines.append(line)

with open("frontend/src/pages/mod.rs", "w", encoding="utf-8") as f:
    f.writelines(new_mod_lines)
print("Updated frontend/src/pages/mod.rs")

# Update frontend/src/app/mod.rs
with open("frontend/src/app/mod.rs", "r", encoding="utf-8") as f:
    app_content = f.read()

# We will remove the routes from the enum and the match switch.
# This requires AST or regex.
# Let's just run cargo check after and see the errors, or manually prune.
