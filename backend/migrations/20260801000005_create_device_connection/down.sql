-- V15 P2 B05-P2-7：PDA / 工控终端连接资源管理表（回滚）
-- 删除顺序：先删索引，再删表（IF EXISTS 保证幂等）
DROP INDEX IF EXISTS "idx_device_connection_last_heartbeat";
DROP INDEX IF EXISTS "idx_device_connection_workshop";
DROP INDEX IF EXISTS "idx_device_connection_status";
DROP INDEX IF EXISTS "uq_device_connection_device_id";
DROP TABLE IF EXISTS "device_connection";
