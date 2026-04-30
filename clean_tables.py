import re

with open('backend/database/migration/001_consolidated_schema.sql', 'r', encoding='utf-8') as f:
    content = f.read()

# Fix CREATE TABLE account_subjects, vouchers, etc. that lack IF NOT EXISTS
content = content.replace('CREATE TABLE account_subjects (', 'CREATE TABLE IF NOT EXISTS account_subjects (')
content = content.replace('CREATE TABLE vouchers (', 'CREATE TABLE IF NOT EXISTS vouchers (')
content = content.replace('CREATE TABLE voucher_items (', 'CREATE TABLE IF NOT EXISTS voucher_items (')
content = content.replace('CREATE TABLE inventory_counts (', 'CREATE TABLE IF NOT EXISTS inventory_counts (')
content = content.replace('CREATE TABLE inventory_count_items (', 'CREATE TABLE IF NOT EXISTS inventory_count_items (')
content = content.replace('CREATE TABLE inventory_transfers (', 'CREATE TABLE IF NOT EXISTS inventory_transfers (')
content = content.replace('CREATE TABLE inventory_transfer_items (', 'CREATE TABLE IF NOT EXISTS inventory_transfer_items (')
content = content.replace('CREATE TABLE business_trace_chain (', 'CREATE TABLE IF NOT EXISTS business_trace_chain (')
content = content.replace('CREATE TABLE business_trace_snapshot (', 'CREATE TABLE IF NOT EXISTS business_trace_snapshot (')

# Fix missing column reference in index
content = content.replace('CREATE INDEX IF NOT EXISTS idx_poi_material_id ON purchase_order_item(material_id);', 'CREATE INDEX IF NOT EXISTS idx_poi_product_id ON purchase_order_item(product_id);')
content = content.replace('CREATE INDEX IF NOT EXISTS idx_pri_material_id ON purchase_receipt_item(material_id);', 'CREATE INDEX IF NOT EXISTS idx_pri_product_id ON purchase_receipt_item(product_id);')

with open('backend/database/migration/001_consolidated_schema.sql', 'w', encoding='utf-8') as f:
    f.write(content)

print("Fixed CREATE TABLE and Indexes")
