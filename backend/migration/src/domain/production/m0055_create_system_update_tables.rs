//! system_update_tasks / system_update_backups 表创建（系统更新域持久化真实接入）
//!
//! 背景：handlers/system_update_handler.rs 原用 OnceLock<Mutex<Vec<T>>> 内存存储，
//! 服务重启后备份记录与更新任务全部丢失。本迁移落库两张表，handler 改为
//! SeaORM Entity CRUD，接入真实持久化。
//!
//! 蓝绿部署规范：created_at/updated_at 均带 NOT NULL DEFAULT now()（满足 25.4-J）。

use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        let sql = r#"
CREATE TABLE IF NOT EXISTS system_update_tasks (
    id SERIAL PRIMARY KEY,
    task_code VARCHAR(50) NOT NULL,
    task_type VARCHAR(20) NOT NULL,
    source_version VARCHAR(50) NOT NULL DEFAULT '',
    target_version VARCHAR(50) NOT NULL DEFAULT '',
    status VARCHAR(30) NOT NULL DEFAULT 'pending',
    progress INTEGER NOT NULL DEFAULT 0,
    error_message TEXT,
    created_by INTEGER NOT NULL DEFAULT 0,
    created_by_name VARCHAR(100) NOT NULL DEFAULT '',
    completed_at TIMESTAMPTZ,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT now()
);
COMMENT ON TABLE "system_update_tasks" IS '系统更新/下载/安装任务记录（原内存存储落库）';
CREATE INDEX IF NOT EXISTS idx_system_update_tasks_status ON system_update_tasks (status);

CREATE TABLE IF NOT EXISTS system_update_backups (
    id SERIAL PRIMARY KEY,
    backup_code VARCHAR(50) NOT NULL,
    backup_type VARCHAR(20) NOT NULL DEFAULT 'full',
    file_path VARCHAR(500) NOT NULL DEFAULT '',
    file_size BIGINT NOT NULL DEFAULT 0,
    description VARCHAR(500) NOT NULL DEFAULT '',
    status VARCHAR(30) NOT NULL DEFAULT 'creating',
    created_by INTEGER NOT NULL DEFAULT 0,
    created_by_name VARCHAR(100) NOT NULL DEFAULT '',
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT now()
);
COMMENT ON TABLE "system_update_backups" IS '系统备份记录（原内存存储落库）';
CREATE INDEX IF NOT EXISTS idx_system_update_backups_status ON system_update_backups (status);
"#;
        manager
            .get_connection()
            .execute_unprepared(sql)
            .await?;
        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        let sql = r#"
DROP TABLE IF EXISTS system_update_backups;
DROP TABLE IF EXISTS system_update_tasks;
"#;
        manager
            .get_connection()
            .execute_unprepared(sql)
            .await?;
        Ok(())
    }
}
