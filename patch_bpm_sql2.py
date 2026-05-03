import os

filepath = 'backend/migrations/20260323000001_initial_schema/up.sql'

with open(filepath, 'r', encoding='utf-8') as f:
    content = f.read()

bpm_tables = """
CREATE TABLE IF NOT EXISTS "bpm_process_definition" (
    "id" SERIAL PRIMARY KEY,
    "name" VARCHAR(100) NOT NULL,
    "code" VARCHAR(50) NOT NULL UNIQUE,
    "description" TEXT,
    "category" VARCHAR(50),
    "version" VARCHAR(20),
    "config" JSONB,
    "status" VARCHAR(20) NOT NULL DEFAULT 'DRAFT',
    "is_deleted" BOOLEAN NOT NULL DEFAULT FALSE,
    "created_at" TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP,
    "updated_at" TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP
);

CREATE TABLE IF NOT EXISTS "bpm_process_instance" (
    "id" SERIAL PRIMARY KEY,
    "process_definition_id" INTEGER NOT NULL REFERENCES "bpm_process_definition"("id"),
    "instance_no" VARCHAR(50) NOT NULL UNIQUE,
    "business_type" VARCHAR(50),
    "business_id" INTEGER,
    "business_no" VARCHAR(50),
    "applicant_id" INTEGER NOT NULL,
    "status" VARCHAR(20) NOT NULL DEFAULT 'PROCESSING',
    "variables" JSONB,
    "start_time" TIMESTAMPTZ,
    "end_time" TIMESTAMPTZ,
    "is_deleted" BOOLEAN NOT NULL DEFAULT FALSE,
    "created_at" TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP,
    "updated_at" TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP
);

CREATE TABLE IF NOT EXISTS "bpm_task" (
    "id" SERIAL PRIMARY KEY,
    "process_instance_id" INTEGER NOT NULL REFERENCES "bpm_process_instance"("id"),
    "task_no" VARCHAR(50) NOT NULL UNIQUE,
    "node_id" VARCHAR(50) NOT NULL,
    "node_name" VARCHAR(100) NOT NULL,
    "name" VARCHAR(100) NOT NULL,
    "task_type" VARCHAR(50) NOT NULL,
    "assignee_id" INTEGER,
    "status" VARCHAR(20) NOT NULL DEFAULT 'PENDING',
    "comment" TEXT,
    "business_type" VARCHAR(50),
    "business_id" INTEGER,
    "is_deleted" BOOLEAN NOT NULL DEFAULT FALSE,
    "created_at" TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP,
    "updated_at" TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP,
    "completed_at" TIMESTAMPTZ
);
"""

if '"bpm_process_definition"' not in content:
    content += "\n" + bpm_tables
    with open(filepath, 'w', encoding='utf-8') as f:
        f.write(content)
