import re

with open('/workspace/backend/database/migration/001_consolidated_schema.sql', 'r') as f:
    content = f.read()

# Pattern for ALTER TABLE ... ADD CONSTRAINT ...
pattern = re.compile(r'ALTER\s+TABLE\s+([a-zA-Z0-9_]+)\s+ADD\s+CONSTRAINT\s+([a-zA-Z0-9_]+)(.*?);', re.IGNORECASE)

def replacer(match):
    table = match.group(1)
    constraint = match.group(2)
    rest = match.group(3)
    
    # We should add a DROP CONSTRAINT IF EXISTS statement right before the ADD CONSTRAINT statement
    drop_stmt = f'ALTER TABLE {table} DROP CONSTRAINT IF EXISTS {constraint};\n'
    add_stmt = f'ALTER TABLE {table} ADD CONSTRAINT {constraint}{rest};'
    return drop_stmt + add_stmt

new_content = pattern.sub(replacer, content)

with open('/workspace/backend/database/migration/001_consolidated_schema.sql', 'w') as f:
    f.write(new_content)

print("Constraints made idempotent.")
