-- 主备隔离模块 migration 回滚

DROP TABLE IF EXISTS failover_config;
DROP TABLE IF EXISTS failover_event;
DROP TABLE IF EXISTS failover_status;
