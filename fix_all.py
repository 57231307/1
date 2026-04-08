import re

with open('/workspace/backend/database/migration/001_consolidated_schema.sql', 'r') as f:
    content = f.read()

# 1. Fix constraints
pattern_constraint = re.compile(r'ALTER\s+TABLE\s+([a-zA-Z0-9_]+)\s+ADD\s+CONSTRAINT\s+([a-zA-Z0-9_]+)(.*?);', re.IGNORECASE)
def replacer_constraint(match):
    table = match.group(1)
    constraint = match.group(2)
    rest = match.group(3)
    drop_stmt = f'ALTER TABLE {table} DROP CONSTRAINT IF EXISTS {constraint};\n'
    add_stmt = f'ALTER TABLE {table} ADD CONSTRAINT {constraint}{rest};'
    return drop_stmt + add_stmt

content = pattern_constraint.sub(replacer_constraint, content)

# 2. Fix inserts
pattern_insert = re.compile(r'INSERT\s+INTO\s+[^;]+;', re.IGNORECASE)
def replacer_insert(match):
    stmt = match.group(0)
    # Check if it already has ON CONFLICT
    if re.search(r'ON\s+CONFLICT', stmt, re.IGNORECASE) or 'SELECT' in stmt.upper():
        return stmt
    # Add ON CONFLICT DO NOTHING
    return stmt[:-1] + '\nON CONFLICT DO NOTHING;'

content = pattern_insert.sub(replacer_insert, content)

with open('/workspace/backend/database/migration/001_consolidated_schema.sql', 'w') as f:
    f.write(content)

print("Fixed both constraints and inserts safely.")
