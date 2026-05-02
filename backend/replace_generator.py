import os
import re

SERVICES_DIR = '/home/root0/桌面/121/1/backend/src/services'

def update_file(filename, entity_module, prefix, generate_method, column_name):
    filepath = os.path.join(SERVICES_DIR, filename)
    with open(filepath, 'r') as f:
        content = f.read()

    # Check if already using DocumentNumberGenerator
    if 'DocumentNumberGenerator::generate_no' in content:
        print(f"Skipping {filename} - already using DocumentNumberGenerator")
        return

    # Add import if missing
    if 'use crate::utils::number_generator::DocumentNumberGenerator;' not in content:
        # Find the first use crate::... and insert after
        match = re.search(r'^use crate::.*?;$', content, re.MULTILINE)
        if match:
            insert_pos = match.end()
            content = content[:insert_pos] + '\nuse crate::utils::number_generator::DocumentNumberGenerator;' + content[insert_pos:]

    # Replace the generate method
    # Pattern to match:
    # pub async fn generate_xxx_no(&self) -> Result<String, AppError> { ... }
    # or async fn generate_xxx_no(&self) -> Result<String, DbErr> { ... }
    
    # We will use regex to find the method signature and its body
    pattern = rf'(pub )?async fn {generate_method}\(&self\) -> Result<String, (sea_orm::)?(AppError|DbErr)> \{{.*?\n    \}}'
    
    # The replacement will handle mapping the error depending on the return type
    def replacer(m):
        is_pub = m.group(1) or ""
        error_type = m.group(3) # AppError or DbErr
        
        err_mapping = ""
        if error_type == "DbErr":
            err_mapping = "\n        .map_err(|e| sea_orm::DbErr::Custom(e.to_string()))"
            
        replacement = f"""{is_pub}async fn {generate_method}(&self) -> Result<String, {m.group(2) or ""}{error_type}> {{
        DocumentNumberGenerator::generate_no(
            &*self.db,
            "{prefix}",
            {entity_module}::Entity,
            {entity_module}::Column::{column_name},
        )
        .await{err_mapping}
    }}"""
        return replacement

    new_content = re.sub(pattern, replacer, content, flags=re.DOTALL)
    
    if new_content != content:
        with open(filepath, 'w') as f:
            f.write(new_content)
        print(f"Updated {filename}")
    else:
        print(f"No changes made to {filename}")

update_file('ap_payment_request_service.rs', 'ap_payment_request', 'PRQ', 'generate_request_no', 'RequestNo')
update_file('ap_reconciliation_service.rs', 'ap_reconciliation', 'REC', 'generate_reconciliation_no', 'ReconciliationNo')
update_file('ap_verification_service.rs', 'ap_verification', 'VER', 'generate_verification_no', 'VerificationNo')
update_file('ap_payment_service.rs', 'ap_payment', 'PAY', 'generate_payment_no', 'PaymentNo')
update_file('inventory_count_service.rs', 'inventory_count', 'IC', 'generate_count_no', 'CountNo')
update_file('ap_invoice_service.rs', 'ap_invoice', 'API', 'generate_invoice_no', 'InvoiceNo')
update_file('purchase_inspection_service.rs', 'purchase_inspection', 'PI', 'generate_inspection_no', 'InspectionNo')
update_file('inventory_transfer_service.rs', 'inventory_transfer', 'TRF', 'generate_transfer_no', 'TransferNo')

# Note: For ar_invoice_service and cost_collection_service, they might be synchronous or take arguments

update_file('purchase_order_service.rs', 'purchase_order', 'PO', 'generate_order_no', 'OrderNo')
update_file('purchase_order_service.rs', 'purchase_receipt', 'PR', 'generate_receipt_no', 'ReceiptNo') # Wait, maybe the column is different?
update_file('purchase_return_service.rs', 'purchase_return', 'RET', 'generate_return_no', 'ReturnNo')
update_file('sales_return_service.rs', 'sales_return', 'SRET', 'generate_return_no', 'ReturnNo')
update_file('sales_service.rs', 'sales_order', 'SO', 'generate_order_no', 'OrderNo')
update_file('inventory_adjustment_service.rs', 'inventory_adjustment', 'ADJ', 'generate_adjustment_no', 'AdjustmentNo')
update_file('purchase_receipt_service.rs', 'purchase_receipt', 'PR', 'generate_receipt_no', 'ReceiptNo')
