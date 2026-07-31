use sea_orm_migration::prelude::*;

// V15 P2 B05-P2-6：染缸设备占用/释放记录表
//
// 记录染缸设备被缸号占用与释放的全生命周期，支持设备资源调度与产能可视化。
// 唯一约束：同一 vat_id 同时只能有一条 status='occupied' 的记录（部分唯一索引）。
//
// 关联文件：
//   - migrations/20260801000004_create_dye_vat_occupation/up.sql
//   - migrations/20260801000004_create_dye_vat_occupation/down.sql
//   - models/dye_vat_occupation.rs
//   - services/dye_vat_occupation_service.rs
//   - services/event_bus_ops/listener.rs（handle_dye_batch_status_changed 触发 occupy/release）

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        let sql = include_str!("../../migrations/20260801000004_create_dye_vat_occupation/up.sql");
        manager.get_connection().execute_unprepared(sql).await?;
        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        let sql =
            include_str!("../../migrations/20260801000004_create_dye_vat_occupation/down.sql");
        manager.get_connection().execute_unprepared(sql).await?;
        Ok(())
    }
}
