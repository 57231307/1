-- 客户联系人表迁移（批次 90b P2-12）
-- 创建时间: 2026-07-03
-- 关联修复: 前端 crm/detail.vue "新增联系人功能待实现" 占位符实现
--
-- 新建 customer_contacts 表，记录客户的多个联系人信息（含主联系人标识）。
-- 替代 crm_customer_handler.rs:list_contacts 中从 crm_lead 拼接 JSON 的伪实现。

CREATE TABLE IF NOT EXISTS "customer_contacts" (
    "id" SERIAL PRIMARY KEY,
    "customer_id" INTEGER NOT NULL REFERENCES "customers"("id") ON DELETE CASCADE,
    "name" VARCHAR(50) NOT NULL,
    "title" VARCHAR(100),
    "phone" VARCHAR(50) NOT NULL,
    "email" VARCHAR(100),
    "is_primary" BOOLEAN NOT NULL DEFAULT FALSE,
    "remarks" VARCHAR(500),
    "created_by" INTEGER,
    "created_at" TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP,
    "updated_at" TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP
);

-- 每个客户最多一个主联系人，部分唯一索引：is_primary=true 时 (customer_id, is_primary) 唯一
CREATE UNIQUE INDEX IF NOT EXISTS "uk_customer_contacts_primary"
    ON "customer_contacts"("customer_id", "is_primary")
    WHERE "is_primary" = TRUE;

CREATE INDEX IF NOT EXISTS "idx_customer_contacts_customer" ON "customer_contacts"("customer_id");

COMMENT ON TABLE "customer_contacts" IS '客户联系人表（批次 90b P2-12 占位符实现）';
COMMENT ON COLUMN "customer_contacts"."customer_id" IS '客户 ID（关联 customers.id）';
COMMENT ON COLUMN "customer_contacts"."name" IS '联系人姓名';
COMMENT ON COLUMN "customer_contacts"."title" IS '职务';
COMMENT ON COLUMN "customer_contacts"."phone" IS '联系电话';
COMMENT ON COLUMN "customer_contacts"."email" IS '联系邮箱';
COMMENT ON COLUMN "customer_contacts"."is_primary" IS '是否主要联系人（每个客户最多一个主联系人，由部分唯一索引约束）';
COMMENT ON COLUMN "customer_contacts"."remarks" IS '备注';
COMMENT ON COLUMN "customer_contacts"."created_by" IS '创建人（关联 users.id）';
