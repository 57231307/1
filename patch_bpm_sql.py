import os

filepath = 'backend/migrations/20260323000001_initial_schema/up.sql'

with open(filepath, 'r', encoding='utf-8') as f:
    content = f.read()

bpm_tables = """
-- BPM: Approval Templates
CREATE TABLE IF NOT EXISTS "approval_templates" (
    "id" SERIAL PRIMARY KEY,
    "name" VARCHAR(100) NOT NULL,
    "resource_type" VARCHAR(50) NOT NULL UNIQUE, -- e.g., 'sales_order', 'purchase_order'
    "description" TEXT,
    "is_active" BOOLEAN NOT NULL DEFAULT TRUE,
    "is_deleted" BOOLEAN NOT NULL DEFAULT FALSE,
    "created_at" TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP,
    "updated_at" TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP
);

-- BPM: Approval Nodes (Steps in a template)
CREATE TABLE IF NOT EXISTS "approval_nodes" (
    "id" SERIAL PRIMARY KEY,
    "template_id" INTEGER NOT NULL REFERENCES "approval_templates"("id"),
    "step_order" INTEGER NOT NULL,
    "approver_role_id" INTEGER, -- if approval is by role
    "approver_user_id" INTEGER, -- if approval is by specific user
    "condition_expr" JSONB,     -- e.g., {"amount_greater_than": 10000}
    "is_deleted" BOOLEAN NOT NULL DEFAULT FALSE,
    "created_at" TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP,
    "updated_at" TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP
);

-- BPM: Approval Instances (Active approval flows for specific documents)
CREATE TABLE IF NOT EXISTS "approval_instances" (
    "id" SERIAL PRIMARY KEY,
    "template_id" INTEGER NOT NULL REFERENCES "approval_templates"("id"),
    "resource_id" INTEGER NOT NULL, -- ID of the actual document
    "status" VARCHAR(20) NOT NULL DEFAULT 'PENDING', -- PENDING, APPROVED, REJECTED, CANCELLED
    "current_step_order" INTEGER NOT NULL DEFAULT 1,
    "applicant_id" INTEGER NOT NULL REFERENCES "users"("id"),
    "is_deleted" BOOLEAN NOT NULL DEFAULT FALSE,
    "created_at" TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP,
    "updated_at" TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP
);

-- BPM: Approval Logs (Audit trail for approvals)
CREATE TABLE IF NOT EXISTS "approval_logs" (
    "id" SERIAL PRIMARY KEY,
    "instance_id" INTEGER NOT NULL REFERENCES "approval_instances"("id"),
    "node_id" INTEGER REFERENCES "approval_nodes"("id"),
    "approver_id" INTEGER NOT NULL REFERENCES "users"("id"),
    "action" VARCHAR(20) NOT NULL, -- APPROVE, REJECT, COMMENT
    "comments" TEXT,
    "is_deleted" BOOLEAN NOT NULL DEFAULT FALSE,
    "created_at" TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP
);
"""

if '"approval_templates"' not in content:
    content += "\n" + bpm_tables
    with open(filepath, 'w', encoding='utf-8') as f:
        f.write(content)
    print("Injected BPM tables into up.sql")
else:
    print("BPM tables already exist in up.sql")
