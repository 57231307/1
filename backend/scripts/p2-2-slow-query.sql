-- P2-2 慢查询扫描
-- 用法：psql -h 39.99.34.194 -U bingxi -d bingxi_erp -f scripts/p2-2-slow-query.sql
-- 输出：3 个查询结果，用于 Wave 4 P2-2 基线报告
-- 说明：所有查询仅 SELECT，不修改数据

-- 1. 高 seq_scan 表（缺索引）
\echo ''
\echo '## 1. 高 seq_scan 表（缺索引，seq_scan 占比 > 50%）'
\echo ''

SELECT
  schemaname,
  relname,
  seq_scan,
  idx_scan,
  ROUND(100.0 * seq_scan / NULLIF(seq_scan + idx_scan, 0), 2) AS seq_pct
FROM pg_stat_user_tables
WHERE seq_scan > 0
  AND (seq_scan + idx_scan) > 100
ORDER BY seq_pct DESC
LIMIT 20;

-- 2. 未使用索引
\echo ''
\echo '## 2. 未使用索引（idx_scan = 0）'
\echo ''

SELECT
  schemaname,
  relname,
  indexrelname,
  pg_size_pretty(pg_relation_size(indexrelid)) AS size
FROM pg_stat_user_indexes
WHERE idx_scan = 0
ORDER BY pg_relation_size(indexrelid) DESC
LIMIT 20;

-- 3. 慢查询（pg_stat_statements，扩展可能未启用）
\echo ''
\echo '## 3. Top 20 慢 SQL（需要 pg_stat_statements 扩展）'
\echo ''

DO $$
BEGIN
  IF EXISTS (SELECT 1 FROM pg_extension WHERE extname = 'pg_stat_statements') THEN
    PERFORM 1;
  ELSE
    RAISE NOTICE 'pg_stat_statements 扩展未启用，跳过此查询';
    RAISE NOTICE '启用方式：shared_preload_libraries = pg_stat_statements';
  END IF;
END $$;

-- 如果启用，执行：
SELECT calls, mean_exec_time, query
FROM pg_stat_statements
ORDER BY mean_exec_time DESC
LIMIT 20;
