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
        let sql = include_str!("../../migrations/20260801000005_create_device_connection/up.sql");
        manager.get_connection().execute_unprepared(sql).await?;
        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        let sql = include_str!("../../migrations/20260801000005_create_device_connection/down.sql");
        manager.get_connection().execute_unprepared(sql).await?;
        Ok(())
    }
}
