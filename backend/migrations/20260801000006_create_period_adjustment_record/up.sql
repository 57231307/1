-- V15 P2 B05-P2-10：期末调整记录表（暂估 / 摊销 / 预提）
-- 依据企业会计准则权责发生制，期末对已发生尚未入账业务做调整分录。
-- 状态机：draft(草稿) → confirmed(已确认，生成凭证) → reversed(已冲销，红字凭证) / cancelled(已取消)
-- 蓝绿部署兼容：所有字段均 NULLABLE 或带 DEFAULT，新增约束仅在事务内执行
CREATE TABLE IF NOT EXISTS "period_adjustment_record" (
    "id"                    BIGSERIAL PRIMARY KEY,
    "adjustment_no"         VARCHAR(64) NOT NULL,
    "adjustment_type"       VARCHAR(32) NOT NULL,
    "period"                VARCHAR(16) NOT NULL,
    "description"           VARCHAR(255) NOT NULL DEFAULT '',
    "debit_subject_code"    VARCHAR(32) NOT NULL,
    "debit_subject_name"    VARCHAR(128) NOT NULL,
    "credit_subject_code"   VARCHAR(32) NOT NULL,
    "credit_subject_name"   VARCHAR(128) NOT NULL,
    "amount"                DECIMAL(14, 2) NOT NULL DEFAULT 0,
    "source_type"           VARCHAR(64),
    "source_bill_id"        INTEGER,
    "source_bill_no"        VARCHAR(64),
    "voucher_id"            INTEGER,
    "reverse_voucher_id"    INTEGER,
    "status"                VARCHAR(16) NOT NULL DEFAULT 'draft',
    "confirmed_by"          INTEGER,
    "confirmed_at"          TIMESTAMPTZ,
    "reversed_by"           INTEGER,
    "reversed_at"           TIMESTAMPTZ,
    "remarks"               VARCHAR(500),
    "is_deleted"            BOOLEAN NOT NULL DEFAULT FALSE,
    "created_by"            INTEGER,
    "created_at"            TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    "updated_at"            TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

-- 唯一索引：调整单号全局唯一
CREATE UNIQUE INDEX IF NOT EXISTS "uq_period_adjustment_record_no"
    ON "period_adjustment_record" ("adjustment_no");

-- 索引：按期间查询期末调整（结账时批量确认）
CREATE INDEX IF NOT EXISTS "idx_period_adjustment_record_period"
    ON "period_adjustment_record" ("period");

-- 索引：按状态过滤待确认/待冲销记录
CREATE INDEX IF NOT EXISTS "idx_period_adjustment_record_status"
    ON "period_adjustment_record" ("status");

-- 索引：按类型分类统计（暂估/摊销/预提）
CREATE INDEX IF NOT EXISTS "idx_period_adjustment_record_type"
    ON "period_adjustment_record" ("adjustment_type");

COMMENT ON TABLE "period_adjustment_record" IS
    'V15 P2 B05-P2-10：期末调整记录表，支持暂估/摊销/预提三类调整，确认生成凭证，暂估类可红字冲销';
COMMENT ON COLUMN "period_adjustment_record.adjustment_type" IS
    '调整类型：estimate(暂估) / amortization(摊销) / provision(预提)';
COMMENT ON COLUMN "period_adjustment_record.status" IS
    '状态：draft(草稿) / confirmed(已确认) / reversed(已冲销) / cancelled(已取消)';
