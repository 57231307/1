import re

with open('/workspace/backend/database/migration/001_consolidated_schema.sql', 'r') as f:
    content = f.read()

# We need to find INSERT INTO ... VALUES (...) or INSERT INTO ... SELECT ...
# Actually, the safest way is to find the end of the INSERT statement and append ON CONFLICT DO NOTHING.
# A simple regex for INSERT statements:
# It starts with INSERT INTO, ends with a semicolon.
# We should only modify it if it doesn't already have ON CONFLICT DO NOTHING.

def replacer(match):
    stmt = match.group(0)
    if 'ON CONFLICT DO NOTHING' not in stmt.upper() and 'SELECT' not in stmt.upper():
        # Insert ON CONFLICT DO NOTHING before the semicolon
        return stmt[:-1] + '\nON CONFLICT DO NOTHING;'
    return stmt

pattern = re.compile(r'INSERT\s+INTO\s+[^;]+;', re.IGNORECASE)
new_content = pattern.sub(replacer, content)

with open('/workspace/backend/database/migration/001_consolidated_schema.sql', 'w') as f:
    f.write(new_content)

print("Inserts made idempotent.")
