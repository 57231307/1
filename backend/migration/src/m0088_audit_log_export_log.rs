use sea_orm_migration::prelude::*;

// V15 缺陷 10-4：审计日志导出二次审计表（防篡改）
//
// 独立于 audit_logs 表，记录每一次审计日志导出操作。
// 数据库触发器禁止 UPDATE / DELETE（仅允许 INSERT），
// 审计员无法篡改自身导出记录，满足 SOC2 / ISO27001 / 《数据安全法》第 32 条要求。
//
// 关联文件：
//   - migrations/20260801000002_audit_log_export_log/up.sql
//   - migrations/20260801000002_audit_log_export_log/down.sql
//   - models/audit_log_export_log.rs
//   - handlers/audit_log_handler.rs（export_audit_logs 写入防篡改表 + list_audit_log_export_logs 查询）

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        let sql = include_str!("../../migrations/20260801000002_audit_log_export_log/up.sql");
        manager
            .get_connection()
            .execute_unprepared(sql)
            .await?;
        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        let sql = include_str!("../../migrations/20260801000002_audit_log_export_log/down.sql");
        manager
            .get_connection()
            .execute_unprepared(sql)
            .await?;
        Ok(())
    }
}
