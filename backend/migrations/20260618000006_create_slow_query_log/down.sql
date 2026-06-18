-- 删除慢查询日志表（P13 批 1 B-慢查询审计，down 迁移）
-- 创建时间: 2026-06-18
-- 关联计划: 2026-06-18-p13-batch1-comprehensive-plan.md §2.2
--
-- 回滚会丢失所有已采集的慢查询数据，请谨慎使用。
DROP TABLE IF EXISTS "slow_query_log";
