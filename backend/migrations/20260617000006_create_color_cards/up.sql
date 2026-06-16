-- 色卡仓储管理模块 migration - color_cards 表
-- 创建时间: 2026-06-17
-- 关联 spec: docs/superpowers/specs/2026-06-16-color-card-design.md §3.2

-- 色卡主表：色卡基本信息和生命周期状态
CREATE TABLE IF NOT EXISTS "color_cards" (
    "id" BIGSERIAL PRIMARY KEY,
    "card_no" VARCHAR(50) UNIQUE NOT NULL,
    "card_name" VARCHAR(200) NOT NULL,
    "card_type" VARCHAR(50) NOT NULL DEFAULT 'CUSTOM',
    "season" VARCHAR(20),
    "brand" VARCHAR(100),
    "total_colors" INT NOT NULL DEFAULT 0,
    "status" VARCHAR(20) NOT NULL DEFAULT 'active',
    "description" TEXT,
    "cover_image_url" TEXT,
    "tenant_id" BIGINT NOT NULL,
    "created_at" TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    "updated_at" TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    CONSTRAINT "chk_color_card_type" CHECK ("card_type" IN ('PANTONE', 'CNCS', 'CUSTOM')),
    CONSTRAINT "chk_color_card_status" CHECK ("status" IN ('active', 'archived', 'lost'))
);

-- 索引
CREATE INDEX IF NOT EXISTS "idx_color_cards_tenant" ON "color_cards"("tenant_id");
CREATE INDEX IF NOT EXISTS "idx_color_cards_status" ON "color_cards"("status");
CREATE INDEX IF NOT EXISTS "idx_color_cards_type_season" ON "color_cards"("card_type", "season");

COMMENT ON TABLE "color_cards" IS '色卡主表 - 纺织行业色卡生命周期与借出跟踪';
COMMENT ON COLUMN "color_cards"."card_no" IS '色卡编号，如 PANTONE-TPX-2024-SS';
COMMENT ON COLUMN "color_cards"."card_type" IS '色卡类型：PANTONE / CNCS / CUSTOM';
COMMENT ON COLUMN "color_cards"."season" IS '季节标签：2024SS / 2024AW / 经典';
COMMENT ON COLUMN "color_cards"."status" IS '状态：active(在用) / archived(归档) / lost(遗失)';
