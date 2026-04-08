import re

with open('/workspace/backend/database/migration/001_consolidated_schema.sql', 'r') as f:
    content = f.read()

pattern_insert = re.compile(r'INSERT\s+INTO\s+[^;]+;', re.IGNORECASE)
for match in pattern_insert.finditer(content):
    stmt = match.group(0)
    if not (re.search(r'ON\s+CONFLICT', stmt, re.IGNORECASE) or 'SELECT' in stmt.upper()):
        print("WOULD MODIFY:")
        print(stmt[:50] + " ... " + stmt[-20:])
        print("-------------")
