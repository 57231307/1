import os
import re

files = [
    ('backend/src/handlers/ap_invoice_handler.rs', 'ap'),
    ('backend/src/handlers/ar_invoice_handler.rs', 'ar'),
    ('backend/src/handlers/finance_invoice_handler.rs', 'finance')
]

for file, prefix in files:
    with open(file, 'r') as f:
        content = f.read()
    
    # Rename functions
    content = re.sub(r'pub async fn list_invoices', f'pub async fn list_{prefix}_invoices', content)
    content = re.sub(r'pub async fn create_invoice', f'pub async fn create_{prefix}_invoice', content)
    content = re.sub(r'pub async fn get_invoice', f'pub async fn get_{prefix}_invoice', content)
    content = re.sub(r'pub async fn update_invoice', f'pub async fn update_{prefix}_invoice', content)
    content = re.sub(r'pub async fn delete_invoice', f'pub async fn delete_{prefix}_invoice', content)
    content = re.sub(r'pub async fn approve_invoice', f'pub async fn approve_{prefix}_invoice', content)
    content = re.sub(r'pub async fn cancel_invoice', f'pub async fn cancel_{prefix}_invoice', content)
    
    with open(file, 'w') as f:
        f.write(content)

# Update routes
with open('backend/src/routes/mod.rs', 'r') as f:
    routes_content = f.read()

# Replace AP references
routes_content = re.sub(r'ap_invoice_handler::list_invoices', 'ap_invoice_handler::list_ap_invoices', routes_content)
routes_content = re.sub(r'ap_invoice_handler::create_invoice', 'ap_invoice_handler::create_ap_invoice', routes_content)
routes_content = re.sub(r'ap_invoice_handler::get_invoice', 'ap_invoice_handler::get_ap_invoice', routes_content)
routes_content = re.sub(r'ap_invoice_handler::update_invoice', 'ap_invoice_handler::update_ap_invoice', routes_content)
routes_content = re.sub(r'ap_invoice_handler::delete_invoice', 'ap_invoice_handler::delete_ap_invoice', routes_content)
routes_content = re.sub(r'ap_invoice_handler::approve_invoice', 'ap_invoice_handler::approve_ap_invoice', routes_content)
routes_content = re.sub(r'ap_invoice_handler::cancel_invoice', 'ap_invoice_handler::cancel_ap_invoice', routes_content)

# Replace AR references
routes_content = re.sub(r'ar_invoice_handler::list_invoices', 'ar_invoice_handler::list_ar_invoices', routes_content)
routes_content = re.sub(r'ar_invoice_handler::create_invoice', 'ar_invoice_handler::create_ar_invoice', routes_content)
routes_content = re.sub(r'ar_invoice_handler::get_invoice', 'ar_invoice_handler::get_ar_invoice', routes_content)
routes_content = re.sub(r'ar_invoice_handler::update_invoice', 'ar_invoice_handler::update_ar_invoice', routes_content)
routes_content = re.sub(r'ar_invoice_handler::delete_invoice', 'ar_invoice_handler::delete_ar_invoice', routes_content)
routes_content = re.sub(r'ar_invoice_handler::approve_invoice', 'ar_invoice_handler::approve_ar_invoice', routes_content)
routes_content = re.sub(r'ar_invoice_handler::cancel_invoice', 'ar_invoice_handler::cancel_ar_invoice', routes_content)

# Replace Finance references
routes_content = re.sub(r'finance_invoice_handler::list_invoices', 'finance_invoice_handler::list_finance_invoices', routes_content)
routes_content = re.sub(r'finance_invoice_handler::create_invoice', 'finance_invoice_handler::create_finance_invoice', routes_content)
routes_content = re.sub(r'finance_invoice_handler::get_invoice', 'finance_invoice_handler::get_finance_invoice', routes_content)
routes_content = re.sub(r'finance_invoice_handler::update_invoice', 'finance_invoice_handler::update_finance_invoice', routes_content)
routes_content = re.sub(r'finance_invoice_handler::delete_invoice', 'finance_invoice_handler::delete_finance_invoice', routes_content)
routes_content = re.sub(r'finance_invoice_handler::approve_invoice', 'finance_invoice_handler::approve_finance_invoice', routes_content)
routes_content = re.sub(r'finance_invoice_handler::cancel_invoice', 'finance_invoice_handler::cancel_finance_invoice', routes_content)

with open('backend/src/routes/mod.rs', 'w') as f:
    f.write(routes_content)
