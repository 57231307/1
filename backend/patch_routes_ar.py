import re

with open('backend/src/routes/mod.rs', 'r') as f:
    content = f.read()

old_ar_routes = '''    let ar_routes = Router::new()
        .route("/invoices", get(ar_invoice_handler::list_invoices))
        .route("/invoices", post(ar_invoice_handler::create_invoice));'''

new_ar_routes = '''    let ar_routes = Router::new()
        .route("/invoices", get(ar_invoice_handler::list_invoices))
        .route("/invoices", post(ar_invoice_handler::create_invoice))
        .route("/invoices/:id", get(ar_invoice_handler::get_invoice))
        .route("/invoices/:id", put(ar_invoice_handler::update_invoice))
        .route("/invoices/:id", delete(ar_invoice_handler::delete_invoice))
        .route("/invoices/:id/approve", post(ar_invoice_handler::approve_invoice))
        .route("/invoices/:id/cancel", post(ar_invoice_handler::cancel_invoice));'''

content = content.replace(old_ar_routes, new_ar_routes)

with open('backend/src/routes/mod.rs', 'w') as f:
    f.write(content)
