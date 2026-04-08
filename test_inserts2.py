import re
with open('/workspace/backend/database/migration/001_consolidated_schema.sql', 'r') as f:
    content = f.read()

pattern_insert = re.compile(r'INSERT\s+INTO\s+[^;]+;', re.IGNORECASE)
for match in pattern_insert.finditer(content):
    stmt = match.group(0)
    print("MATCHED:")
    print(stmt[:50] + " ... " + stmt[-20:])
    print("HAS SELECT:", 'SELECT' in stmt.upper())
    print("HAS CONFLICT:", bool(re.search(r'ON\s+CONFLICT', stmt, re.IGNORECASE)))
    print("-------------")
