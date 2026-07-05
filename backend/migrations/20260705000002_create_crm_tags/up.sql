-- 批次 122 v8 复审 P1 修复：CRM 标签字典表
-- 原 crm_customer_handler list_tags 返回硬编码 5 个标签，create_tag/delete_tag 为空操作假实现。
-- 现新增 crm_tag 表存储标签字典（id/name/color/category/created_at），handler 真实接入。
-- 保留 crm_lead.tags TEXT[] 数组字段向后兼容（add_tags handler 仍覆盖式更新该数组）。

CREATE TABLE IF NOT EXISTS "crm_tag" (
    "id" SERIAL PRIMARY KEY,
    "name" VARCHAR(30) NOT NULL UNIQUE,
    "color" VARCHAR(20) NOT NULL DEFAULT '#1890ff',
    "category" VARCHAR(50),
    "created_by" INTEGER,
    "created_at" TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP,
    "updated_at" TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP
);

COMMENT ON TABLE "crm_tag" IS 'CRM 标签字典表（批次 122 v8 复审 P1 修复：替代 list_tags 硬编码 + create_tag/delete_tag 假实现）';
COMMENT ON COLUMN "crm_tag"."name" IS '标签名称（唯一，长度 1-30）';
COMMENT ON COLUMN "crm_tag"."color" IS '标签颜色（HEX 格式，默认 #1890ff）';
COMMENT ON COLUMN "crm_tag"."category" IS '标签分类（可选，如 customer/lead/supplier）';
COMMENT ON COLUMN "crm_tag"."created_by" IS '创建者用户 ID';

-- 创建按分类查询的索引
CREATE INDEX IF NOT EXISTS "idx_crm_tag_category" ON "crm_tag" ("category");

-- 初始化预定义标签（与原硬编码 list_tags 保持一致，保证向后兼容）
INSERT INTO "crm_tag" ("name", "color", "category") VALUES
    ('VIP', '#f50', 'customer'),
    ('重点客户', '#2db7f5', 'customer'),
    ('潜在客户', '#87d068', 'lead'),
    ('新客户', '#108ee9', 'lead'),
    ('流失客户', '#f50', 'customer')
ON CONFLICT ("name") DO NOTHING;
