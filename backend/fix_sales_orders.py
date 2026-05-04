import re

with open('database/migration/001_consolidated_schema.sql', 'r', encoding='utf-8') as f:
    content = f.read()

content = content.replace(
    "completed_at TIMESTAMPTZ,\n    created_at TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP,",
    "completed_at TIMESTAMPTZ,\n    is_deleted BOOLEAN NOT NULL DEFAULT false,\n    created_at TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP,"
)

with open('database/migration/001_consolidated_schema.sql', 'w', encoding='utf-8') as f:
    f.write(content)
