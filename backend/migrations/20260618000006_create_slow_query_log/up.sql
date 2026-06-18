-- 创建慢查询日志表（P13 批 1 B-慢查询审计）
-- 创建时间: 2026-06-18
-- 关联计划: 2026-06-18-p13-batch1-comprehensive-plan.md §2.2
--
-- 数据源：pg_stat_statements 视图（pg_stat_statements 扩展提供）
-- 采集方式：后台任务 slow_query_collector 每 5 分钟查询 pg_stat_statements，
--           过滤 mean_exec_time > 100ms 的查询，写入本表
-- 用途：运维审计 / SQL 性能优化 / 慢查询排行
--
-- 字段说明：
-- - id: 主键自增
-- - query_text: SQL 文本（来自 pg_stat_statements.query 列）
-- - execution_time_ms: 平均执行时间（毫秒，来自 pg_stat_statements.mean_exec_time）
-- - calls: 调用次数（来自 pg_stat_statements.calls，用于计算总耗时 = calls * mean_exec_time）
-- - rows_examined: 平均扫描行数（来自 pg_stat_statements.rows）
-- - database_name: 数据库名（当前固定为配置中的数据库名；保留字段以备将来多库部署）
-- - tenant_id: 租户 ID（慢查询是系统级数据，但允许按租户分类以便扩展；NULL = 系统级）
-- - captured_at: 采集时间
--
-- 索引策略：
-- - idx_slow_query_captured: 按时间范围查询（list 接口）
-- - idx_slow_query_exec_time: 按执行时间倒序（stats TOP 10 聚合）
-- - idx_slow_query_tenant: 按租户过滤（多租户隔离扩展）

CREATE TABLE IF NOT EXISTS "slow_query_log" (
    "id" BIGSERIAL PRIMARY KEY,
    "query_text" TEXT NOT NULL,
    "execution_time_ms" DOUBLE PRECISION NOT NULL,
    "calls" BIGINT NOT NULL DEFAULT 0,
    "rows_examined" BIGINT NOT NULL DEFAULT 0,
    "database_name" VARCHAR(128),
    "tenant_id" INTEGER,
    "captured_at" TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

-- 时间范围查询索引
CREATE INDEX IF NOT EXISTS "idx_slow_query_captured" ON "slow_query_log"("captured_at" DESC);

-- 按执行时间倒序索引（TOP 10 聚合）
CREATE INDEX IF NOT EXISTS "idx_slow_query_exec_time" ON "slow_query_log"("execution_time_ms" DESC);

-- 租户过滤索引（多租户隔离）
CREATE INDEX IF NOT EXISTS "idx_slow_query_tenant" ON "slow_query_log"("tenant_id");

COMMENT ON TABLE "slow_query_log" IS '慢查询日志（来源：pg_stat_statements 视图 + 后台定时采集）';
COMMENT ON COLUMN "slow_query_log"."query_text" IS 'SQL 文本';
COMMENT ON COLUMN "slow_query_log"."execution_time_ms" IS '平均执行时间（毫秒）';
COMMENT ON COLUMN "slow_query_log"."calls" IS '调用次数（来自 pg_stat_statements.calls）';
COMMENT ON COLUMN "slow_query_log"."rows_examined" IS '平均扫描行数（来自 pg_stat_statements.rows）';
COMMENT ON COLUMN "slow_query_log"."database_name" IS '数据库名（系统级元数据）';
COMMENT ON COLUMN "slow_query_log"."tenant_id" IS '租户 ID（NULL = 系统级）';
COMMENT ON COLUMN "slow_query_log"."captured_at" IS '采集时间';
