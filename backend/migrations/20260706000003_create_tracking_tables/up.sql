-- v11 批次 143 P1-2：用户行为追踪分析模块
-- 创建 page_views / user_behaviors 表，支持页面访问统计、热门页面、漏斗分析、用户路径分析

-- ============================================
-- 1. 页面访问记录表
-- ============================================
CREATE TABLE IF NOT EXISTS "page_views" (
    "id" BIGSERIAL PRIMARY KEY,
    "session_id" VARCHAR(100),
    "user_id" INTEGER,
    "path" VARCHAR(500) NOT NULL,
    "referrer" VARCHAR(500),
    "user_agent" VARCHAR(500),
    "ip_address" VARCHAR(45),
    "viewed_at" TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP
);

CREATE INDEX IF NOT EXISTS "idx_page_views_path" ON "page_views" ("path");
CREATE INDEX IF NOT EXISTS "idx_page_views_viewed_at" ON "page_views" ("viewed_at");
CREATE INDEX IF NOT EXISTS "idx_page_views_session" ON "page_views" ("session_id");
CREATE INDEX IF NOT EXISTS "idx_page_views_user" ON "page_views" ("user_id");

COMMENT ON TABLE "page_views" IS '页面访问记录表';
COMMENT ON COLUMN "page_views"."session_id" IS '会话 ID（匿名用户标识）';
COMMENT ON COLUMN "page_views"."user_id" IS '用户 ID（登录用户）';
COMMENT ON COLUMN "page_views"."path" IS '页面路径';
COMMENT ON COLUMN "page_views"."referrer" IS '来源页面';
COMMENT ON COLUMN "page_views"."viewed_at" IS '访问时间';

-- ============================================
-- 2. 用户行为记录表
-- ============================================
CREATE TABLE IF NOT EXISTS "user_behaviors" (
    "id" BIGSERIAL PRIMARY KEY,
    "session_id" VARCHAR(100),
    "user_id" INTEGER,
    "event_type" VARCHAR(50) NOT NULL,
    "event_target" VARCHAR(200),
    "event_data" JSONB,
    "path" VARCHAR(500),
    "ip_address" VARCHAR(45),
    "occurred_at" TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP
);

CREATE INDEX IF NOT EXISTS "idx_user_behaviors_event_type" ON "user_behaviors" ("event_type");
CREATE INDEX IF NOT EXISTS "idx_user_behaviors_occurred_at" ON "user_behaviors" ("occurred_at");
CREATE INDEX IF NOT EXISTS "idx_user_behaviors_session" ON "user_behaviors" ("session_id");
CREATE INDEX IF NOT EXISTS "idx_user_behaviors_user" ON "user_behaviors" ("user_id");

COMMENT ON TABLE "user_behaviors" IS '用户行为记录表';
COMMENT ON COLUMN "user_behaviors"."event_type" IS '事件类型（click/scroll/submit 等）';
COMMENT ON COLUMN "user_behaviors"."event_target" IS '事件目标（元素 ID/类名等）';
COMMENT ON COLUMN "user_behaviors"."event_data" IS '事件附加数据（JSON）';
