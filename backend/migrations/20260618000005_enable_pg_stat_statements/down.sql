-- 停用 pg_stat_statements 扩展（P13 批 1 B-慢查询审计，down 迁移）
-- 创建时间: 2026-06-18
-- 关联计划: 2026-06-18-p13-batch1-comprehensive-plan.md §2.2
--
-- 注意：执行该 down.sql 会移除 pg_stat_statements 扩展。
-- 如果 postgresql.conf 中未在 shared_preload_libraries 中预装该扩展，
-- 该扩展将无法重新启用，需在生产环境提前备份。
-- 当前判断：迁移到下一个版本时通常不会执行 down.sql，此 down 仅作完整声明。

-- IF EXISTS 防止扩展不存在时阻塞
DROP EXTENSION IF EXISTS pg_stat_statements;
