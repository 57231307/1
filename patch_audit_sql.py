import os
import re

filepath = 'backend/migrations/20260323000001_initial_schema/up.sql'

with open(filepath, 'r', encoding='utf-8') as f:
    content = f.read()

audit_table = """
CREATE TABLE IF NOT EXISTS "audit_logs" (
    "id" SERIAL PRIMARY KEY,
    "table_name" VARCHAR(100) NOT NULL,
    "record_id" INTEGER NOT NULL,
    "action" VARCHAR(20) NOT NULL,
    "old_data" JSONB,
    "new_data" JSONB,
    "changed_fields" JSONB,
    "user_id" INTEGER,
    "created_at" TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP
);
"""

if '"audit_logs"' not in content:
    content += "\n" + audit_table
    with open(filepath, 'w', encoding='utf-8') as f:
        f.write(content)
