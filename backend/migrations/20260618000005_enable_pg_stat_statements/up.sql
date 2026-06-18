-- 启用 pg_stat_statements 扩展（P13 批 1 B-慢查询审计）
-- 创建时间: 2026-06-18
-- 关联计划: 2026-06-18-p13-batch1-comprehensive-plan.md §2.2
--
-- pg_stat_statements 是 PostgreSQL 官方扩展，用于跟踪所有执行的 SQL 语句的统计信息
-- （调用次数 / 平均执行时间 / 行数 / 共享块命中率等）。
-- 本项目用作慢查询审计的数据源——后台采集任务每 5 分钟查询该视图，
-- 过滤出 mean_exec_time > 100ms 的查询，写入 slow_query_log 表。
--
-- 注意：
-- 1. 该扩展需要在 postgresql.conf 的 shared_preload_libraries 中预装：
--    shared_preload_libraries = 'pg_stat_statements'
-- 2. 容器/CI 环境中如未预装共享库，本 up.sql 仍可执行成功（CREATE EXTENSION IF NOT EXISTS
--    会跳过已存在或不可用的扩展）；后台采集任务会自动降级（见 slow_query_collector.rs）。
-- 3. 降级方案：在 down.sql 中记录原因，应用层增加 SQL 慢查询日志兜底。

CREATE EXTENSION IF NOT EXISTS pg_stat_statements;
