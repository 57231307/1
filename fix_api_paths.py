#!/usr/bin/env python3
import os
import re

services_dir = '/workspace/frontend/src/services'

files_to_fix = [
    'account_subject_service.rs',
    'ap_invoice_service.rs',
    'ap_payment_request_service.rs',
    'ap_payment_service.rs',
    'ap_reconciliation_service.rs',
    'ap_report_service.rs',
    'ap_verification_service.rs',
    'ar_invoice_service.rs',
    'assist_accounting_service.rs',
    'batch_service.rs',
    'budget_management_service.rs',
    'business_trace_service.rs',
    'cost_collection_service.rs',
    'customer_credit_service.rs',
    'dashboard_service.rs',
    'department_service.rs',
    'dual_unit_converter_service.rs',
    'dye_batch_service.rs',
    'dye_recipe_service.rs',
    'fabric_order_service.rs',
    'finance_invoice_service.rs',
    'finance_payment_service.rs',
    'financial_analysis_service.rs',
    'five_dimension_service.rs',
    'fixed_asset_service.rs',
    'fund_management_service.rs',
    'greige_fabric_service.rs',
    'init_service.rs',
    'inventory_adjustment_service.rs',
    'inventory_count_service.rs',
    'inventory_transfer_service.rs',
    'purchase_contract_service.rs',
    'purchase_inspection_service.rs',
    'purchase_order_service.rs',
    'purchase_price_service.rs',
    'purchase_receipt_service.rs',
    'purchase_return_service.rs',
    'quality_inspection_service.rs',
    'sales_analysis_service.rs',
    'sales_contract_service.rs',
    'sales_price_service.rs',
    'supplier_evaluation_service.rs',
    'supplier_service.rs',
    'voucher_service.rs',
]

for filename in files_to_fix:
    filepath = os.path.join(services_dir, filename)
    if not os.path.exists(filepath):
        print(f'Skipping {filename} - file not found')
        continue
    
    with open(filepath, 'r', encoding='utf-8') as f:
        content = f.read()
    
    # Replace /api/v1/erp/ with /
    original_content = content
    content = re.sub(r'"/api/v1/erp/', r'"/', content)
    
    if content != original_content:
        with open(filepath, 'w', encoding='utf-8') as f:
            f.write(content)
        print(f'Fixed {filename}')
    else:
        print(f'No changes needed for {filename}')

print('Done!')
