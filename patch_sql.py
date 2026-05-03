import re

filepath = 'backend/migrations/20260323000001_initial_schema/up.sql'

with open(filepath, 'r', encoding='utf-8') as f:
    content = f.read()

# Find all CREATE TABLE blocks
# We want to insert `"is_deleted" BOOLEAN NOT NULL DEFAULT FALSE,` before the last `);` or before `created_at`?
# Actually, just before the closing parenthesis.

def inject_is_deleted(match):
    table_content = match.group(0)
    if 'is_deleted' in table_content.lower():
        return table_content
    
    # Insert before the last `);`
    # Some tables have PRIMARY KEY definitions at the end, that's fine.
    
    # Let's insert it before `created_at` or at the end.
    if '"created_at"' in table_content:
        table_content = re.sub(r'(\n\s*)("created_at")', r'\1"is_deleted" BOOLEAN NOT NULL DEFAULT FALSE,\1\2', table_content)
    else:
        # Just insert before the last newline
        lines = table_content.split('\n')
        for i in range(len(lines)-1, -1, -1):
            if lines[i].strip() == ');':
                lines.insert(i, '    "is_deleted" BOOLEAN NOT NULL DEFAULT FALSE,')
                break
        table_content = '\n'.join(lines)
        
    return table_content

# Regex to find CREATE TABLE blocks
new_content = re.sub(r'CREATE TABLE [^{]+\{[^}]+\}', inject_is_deleted, content)
# Wait, SQL uses `(` not `{`
new_content = re.sub(r'CREATE TABLE [^(]+\([^;]+\);', inject_is_deleted, content)

with open(filepath, 'w', encoding='utf-8') as f:
    f.write(new_content)

print("Injected is_deleted into up.sql")
