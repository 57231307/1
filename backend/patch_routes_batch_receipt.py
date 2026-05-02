import re

with open('backend/src/routes/mod.rs', 'r') as f:
    content = f.read()

# Fix batch_handler reference -> bulk_product_handler
content = content.replace('batch_handler::batch_create_products', 'bulk_product_handler::batch_create_products')
content = content.replace('batch_handler::batch_update_products', 'bulk_product_handler::batch_update_products')
content = content.replace('batch_handler::batch_delete_products', 'bulk_product_handler::batch_delete_products')

# Add missing purchase_receipt update and confirm routes
old_receipt_routes = '''    let purchase_receipt_routes = Router::new()
        .route("/", get(purchase_receipt_handler::list_receipts))
        .route("/", post(purchase_receipt_handler::create_receipt));'''

new_receipt_routes = '''    let purchase_receipt_routes = Router::new()
        .route("/", get(purchase_receipt_handler::list_receipts))
        .route("/", post(purchase_receipt_handler::create_receipt))
        .route("/:id", put(purchase_receipt_handler::update_receipt))
        .route("/:id/confirm", post(purchase_receipt_handler::confirm_receipt));'''

content = content.replace(old_receipt_routes, new_receipt_routes)

with open('backend/src/routes/mod.rs', 'w') as f:
    f.write(content)
