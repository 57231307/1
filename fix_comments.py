import re

with open('backend/database/migration/001_consolidated_schema.sql', 'r', encoding='utf-8') as f:
    content = f.read()

# Comment out COMMENT ON FUNCTION with ()
content = re.sub(r"^(COMMENT ON FUNCTION [a-zA-Z0-9_]+\(\) IS .*?;)$", r"-- \1", content, flags=re.MULTILINE)

with open('backend/database/migration/001_consolidated_schema.sql', 'w', encoding='utf-8') as f:
    f.write(content)

print("Fixed invalid comments")
