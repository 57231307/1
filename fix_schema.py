import re

with open('backend/database/migration/001_consolidated_schema.sql', 'r', encoding='utf-8') as f:
    content = f.read()

# 1. Fix inventory_count_items
# Find all CREATE TABLE IF NOT EXISTS inventory_count_items
parts = content.split('CREATE TABLE IF NOT EXISTS inventory_count_items (')
if len(parts) == 4:
    # There are 3 definitions. The first is in parts[1], second in parts[2], third in parts[3]
    # Let's keep only parts[3] which is the latest one.
    
    # We need to remove the first two definitions entirely.
    # parts[1] ends with `);\n` and then some comments or indices.
    
    # Actually, it's safer to just do a targeted regex replace for the first two blocks
    pass

# targeted regex for old inventory_count_items
content = re.sub(
    r'CREATE TABLE IF NOT EXISTS inventory_count_items \([^;]+;\n',
    '',
    content,
    count=2
)

# 2. Fix duplicated account_subjects
content = re.sub(
    r'CREATE TABLE IF NOT EXISTS account_subjects \([^;]+;\n',
    '',
    content,
    count=1
)

# 3. Fix duplicated vouchers
content = re.sub(
    r'CREATE TABLE IF NOT EXISTS vouchers \([^;]+;\n',
    '',
    content,
    count=1
)

# 4. Fix duplicated voucher_items
content = re.sub(
    r'CREATE TABLE IF NOT EXISTS voucher_items \([^;]+;\n',
    '',
    content,
    count=1
)

# 5. Fix duplicated inventory_transfers
content = re.sub(
    r'CREATE TABLE IF NOT EXISTS inventory_transfers \([^;]+;\n',
    '',
    content,
    count=1
)

# 6. Fix duplicated inventory_transfer_items
content = re.sub(
    r'CREATE TABLE IF NOT EXISTS inventory_transfer_items \([^;]+;\n',
    '',
    content,
    count=1
)

# 7. Fix duplicated inventory_counts
content = re.sub(
    r'CREATE TABLE IF NOT EXISTS inventory_counts \([^;]+;\n',
    '',
    content,
    count=1
)

# 8. Fix duplicated business_trace_chain
content = re.sub(
    r'CREATE TABLE IF NOT EXISTS business_trace_chain \([^;]+;\n',
    '',
    content,
    count=1
)

# 9. Fix duplicated business_trace_snapshot
content = re.sub(
    r'CREATE TABLE IF NOT EXISTS business_trace_snapshot \([^;]+;\n',
    '',
    content,
    count=1
)

# 10. Remove duplicate indexes
content = re.sub(r'CREATE INDEX IF NOT EXISTS idx_inventory_count_items_count_id ON inventory_count_items\(count_id\);\n', '', content, count=1)
content = re.sub(r'CREATE INDEX IF NOT EXISTS idx_inventory_count_items_product_id ON inventory_count_items\(product_id\);\n', '', content, count=1)

with open('backend/database/migration/001_consolidated_schema.sql', 'w', encoding='utf-8') as f:
    f.write(content)

print("Cleaned up duplicated tables and indexes")
