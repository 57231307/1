use sea_orm_migration::prelude::*;

// V15 P2 B05-P2-7：PDA / 工控终端连接资源管理表
//
// 记录车间设备（PDA / 工控终端 / 扫码枪）与服务端的连接资源状态，
// 支持注册 / 心跳 / 下线 / 超时清理的生命周期闭环。
// 唯一约束：device_id 一台设备一条记录（重复注册走应用层 upsert 路径）。
//
// 关联文件：
//   - migrations/20260801000005_create_device_connection/up.sql
//   - migrations/20260801000005_create_device_connection/down.sql
//   - models/device_connection.rs
//   - services/device_connection_service.rs
//   - handlers/device_connection_handler.rs
//   - routes/device_connection.rs
//   - bootstrap/service_bootstrap.rs（start_device_connection_cleanup_task 心跳超时清理）

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        let sql =
            r#"-- V15 P2 B05-P2-7：PDA / 工控终端连接资源管理表
-- 记录车间设备（PDA / 工控终端 / 扫码枪）与服务端的连接资源状态，
-- 支持注册 / 心跳 / 下线 / 超时清理的生命周期闭环。
-- 状态机：online（在线）→ offline（主动下线）/ timeout（心跳超时）
-- 唯一约束：device_id 一台设备一条记录，重复注册走应用层 upsert 路径
CREATE TABLE IF NOT EXISTS "device_connection" (
    "id"                BIGSERIAL PRIMARY KEY,
    "device_id"         VARCHAR(128) NOT NULL,
    "device_name"       VARCHAR(128),
    "device_type"       VARCHAR(32) NOT NULL DEFAULT 'other',
    "user_id"           INTEGER,
    "username"          VARCHAR(128),
    "workshop"          VARCHAR(64),
    "ip_address"        VARCHAR(64),
    "session_token"     VARCHAR(255),
    "status"            VARCHAR(16) NOT NULL DEFAULT 'online',
    "last_heartbeat_at" TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    "connected_at"      TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    "disconnected_at"   TIMESTAMPTZ,
    "metadata"          JSONB,
    "created_at"        TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    "updated_at"        TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

-- 唯一索引：device_id 一台设备一条记录（upsert 依据）
CREATE UNIQUE INDEX IF NOT EXISTS "uq_device_connection_device_id"
    ON "device_connection" ("device_id");

-- 索引：按状态查询在线设备列表（高频查询路径）
CREATE INDEX IF NOT EXISTS "idx_device_connection_status"
    ON "device_connection" ("status");

-- 索引：按车间汇总在线设备数（看板高频查询）
CREATE INDEX IF NOT EXISTS "idx_device_connection_workshop"
    ON "device_connection" ("workshop");

-- 索引：定时任务按 last_heartbeat_at 扫描超时设备
CREATE INDEX IF NOT EXISTS "idx_device_connection_last_heartbeat"
    ON "device_connection" ("last_heartbeat_at");

COMMENT ON TABLE "device_connection" IS
    'V15 P2 B05-P2-7：PDA/工控终端连接资源管理表，注册/心跳/下线/超时清理全生命周期';
COMMENT ON COLUMN "device_connection.status" IS
    '连接状态：online（在线）/ offline（主动下线）/ timeout（心跳超时）';
COMMENT ON COLUMN "device_connection.device_type" IS
    '设备类型：pda / industrial_terminal / scanner / other';"#;
        manager.get_connection().execute_unprepared(sql).await?;
        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        let sql =
            r#"-- V15 P2 B05-P2-7：PDA / 工控终端连接资源管理表（回滚）
-- 删除顺序：先删索引，再删表（IF EXISTS 保证幂等）
DROP INDEX IF EXISTS "idx_device_connection_last_heartbeat";
DROP INDEX IF EXISTS "idx_device_connection_workshop";
DROP INDEX IF EXISTS "idx_device_connection_status";
DROP INDEX IF EXISTS "uq_device_connection_device_id";
DROP TABLE IF EXISTS "device_connection";"#;
        manager.get_connection().execute_unprepared(sql).await?;
        Ok(())
    }
}
