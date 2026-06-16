-- 色卡仓储管理模块 migration - color_card_items 表
-- 创建时间: 2026-06-17
-- 关联 spec: docs/superpowers/specs/2026-06-16-color-card-design.md §3.3

-- 色卡明细表：每个色号的色彩空间坐标、配方关联、价格关联
CREATE TABLE IF NOT EXISTS "color_card_items" (
    "id" BIGSERIAL PRIMARY KEY,
    "color_card_id" BIGINT NOT NULL REFERENCES "color_cards"("id") ON DELETE CASCADE,
    "color_code" VARCHAR(50) NOT NULL,
    "color_name" VARCHAR(200) NOT NULL,
    "rgb_r" INT NOT NULL CHECK ("rgb_r" BETWEEN 0 AND 255),
    "rgb_g" INT NOT NULL CHECK ("rgb_g" BETWEEN 0 AND 255),
    "rgb_b" INT NOT NULL CHECK ("rgb_b" BETWEEN 0 AND 255),
    "cmyk_c" DECIMAL(5,2) CHECK ("cmyk_c" IS NULL OR ("cmyk_c" BETWEEN 0 AND 100)),
    "cmyk_m" DECIMAL(5,2) CHECK ("cmyk_m" IS NULL OR ("cmyk_m" BETWEEN 0 AND 100)),
    "cmyk_y" DECIMAL(5,2) CHECK ("cmyk_y" IS NULL OR ("cmyk_y" BETWEEN 0 AND 100)),
    "cmyk_k" DECIMAL(5,2) CHECK ("cmyk_k" IS NULL OR ("cmyk_k" BETWEEN 0 AND 100)),
    "lab_l" DECIMAL(6,2) CHECK ("lab_l" IS NULL OR ("lab_l" BETWEEN 0 AND 100)),
    "lab_a" DECIMAL(6,2) CHECK ("lab_a" IS NULL OR ("lab_a" BETWEEN -128 AND 127)),
    "lab_b" DECIMAL(6,2) CHECK ("lab_b" IS NULL OR ("lab_b" BETWEEN -128 AND 127)),
    "pantone_code" VARCHAR(50),
    "cncs_code" VARCHAR(50),
    "custom_code" VARCHAR(50),
    "hex_value" VARCHAR(7) NOT NULL,
    "dye_recipe_id" BIGINT REFERENCES "dye_recipes"("id"),
    "product_color_price_id" BIGINT REFERENCES "product_color_prices"("id"),
    "swatch_image_url" TEXT,
    "sequence" INT NOT NULL DEFAULT 0,
    "tenant_id" BIGINT NOT NULL,
    "created_at" TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    "updated_at" TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    CONSTRAINT "uq_color_card_items_card_code" UNIQUE ("color_card_id", "color_code")
);

-- 索引
CREATE INDEX IF NOT EXISTS "idx_color_items_card" ON "color_card_items"("color_card_id");
CREATE INDEX IF NOT EXISTS "idx_color_items_code" ON "color_card_items"("color_code");
CREATE INDEX IF NOT EXISTS "idx_color_items_pantone" ON "color_card_items"("pantone_code") WHERE "pantone_code" IS NOT NULL;
CREATE INDEX IF NOT EXISTS "idx_color_items_cncs" ON "color_card_items"("cncs_code") WHERE "cncs_code" IS NOT NULL;
CREATE INDEX IF NOT EXISTS "idx_color_items_tenant" ON "color_card_items"("tenant_id");
CREATE INDEX IF NOT EXISTS "idx_color_items_dye_recipe" ON "color_card_items"("dye_recipe_id");
CREATE INDEX IF NOT EXISTS "idx_color_items_price" ON "color_card_items"("product_color_price_id");

COMMENT ON TABLE "color_card_items" IS '色卡明细表 - 纺织行业色号详细参数与关联业务';
COMMENT ON COLUMN "color_card_items"."hex_value" IS 'HEX 颜色值 #RRGGBB';
COMMENT ON COLUMN "color_card_items"."lab_l" IS 'CIELab 颜色空间 L (亮度 0-100)';
COMMENT ON COLUMN "color_card_items"."lab_a" IS 'CIELab 颜色空间 a (红绿 -128~127)';
COMMENT ON COLUMN "color_card_items"."lab_b" IS 'CIELab 颜色空间 b (黄蓝 -128~127)';
COMMENT ON COLUMN "color_card_items"."sequence" IS '色卡中色号的显示顺序';
