-- V15 P2 B05-P2-6：染缸设备占用/释放记录表
-- 记录染缸设备被缸号占用与释放的全生命周期，支持设备资源调度与产能可视化。
-- 唯一约束：同一 vat_id 同时只能有一条 status='occupied' 的记录（部分唯一索引）。
CREATE TABLE IF NOT EXISTS "dye_vat_occupation" (
    "id"           BIGSERIAL PRIMARY KEY,
    "vat_id"       INTEGER NOT NULL,
    "batch_id"     INTEGER NOT NULL,
    "batch_no"     VARCHAR(64),
    "occupied_at"  TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    "released_at"  TIMESTAMPTZ,
    "status"       VARCHAR(16) NOT NULL DEFAULT 'occupied',
    "created_at"   TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    "updated_at"   TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

-- 索引：按 vat_id 查询当前占用状态
CREATE INDEX IF NOT EXISTS "idx_dye_vat_occupation_vat_id"
    ON "dye_vat_occupation" ("vat_id");

-- 索引：按 batch_id 查询缸号占用的染缸
CREATE INDEX IF NOT EXISTS "idx_dye_vat_occupation_batch_id"
    ON "dye_vat_occupation" ("batch_id");

-- 部分唯一索引：同一 vat_id 同时只能有一条 status='occupied' 的记录（防重复占用）
CREATE UNIQUE INDEX IF NOT EXISTS "uq_dye_vat_occupation_vat_occupied"
    ON "dye_vat_occupation" ("vat_id")
    WHERE "status" = 'occupied';

COMMENT ON TABLE "dye_vat_occupation" IS
    'V15 P2 B05-P2-6：染缸占用记录表，缸号进入 dyeing 占用 / 离开 dyeing 释放';
COMMENT ON COLUMN "dye_vat_occupation.status" IS
    '占用状态：occupied（已占用）/ released（已释放）';
