#!/usr/bin/env python3
import os
import re

handlers_dir = "/workspace/backend/src/handlers"

files_to_fix = [
    "purchase_receipt_handler.rs",
    "purchase_return_handler.rs",
    "ap_invoice_handler.rs",
    "ap_payment_handler.rs",
    "ap_payment_request_handler.rs",
    "ap_reconciliation_handler.rs",
    "ap_report_handler.rs",
    "ap_verification_handler.rs",
    "budget_management_handler.rs",
    "customer_credit_handler.rs",
    "fixed_asset_handler.rs",
    "fund_management_handler.rs",
    "purchase_contract_handler.rs",
    "quality_standard_handler.rs",
    "sales_contract_handler.rs",
    "financial_analysis_handler.rs",
    "purchase_price_handler.rs",
    "quality_inspection_handler.rs",
    "sales_analysis_handler.rs",
    "sales_price_handler.rs",
    "supplier_evaluation_handler.rs",
    "dye_batch_handler.rs",
    "dye_recipe_handler.rs",
    "greige_fabric_handler.rs",
]

for filename in files_to_fix:
    filepath = os.path.join(handlers_dir, filename)
    if not os.path.exists(filepath):
        print(f"文件不存在: {filepath}")
        continue
    
    with open(filepath, 'r', encoding='utf-8') as f:
        content = f.read()
    
    original_content = content
    
    if "use crate::utils::app_state::AppState;" not in content:
        content = re.sub(
            r'(use crate::.*?;)',
            r'\1\nuse crate::utils::app_state::AppState;',
            content,
            count=1
        )
    
    content = re.sub(r'::new\(db\)', '::new(state.db.clone())', content)
    
    content = re.sub(r'&\*db', '&*state.db', content)
    
    if content != original_content:
        with open(filepath, 'w', encoding='utf-8') as f:
            f.write(content)
        print(f"已修复: {filename}")
    else:
        print(f"无需修改: {filename}")

print("\n批量修复完成!")
