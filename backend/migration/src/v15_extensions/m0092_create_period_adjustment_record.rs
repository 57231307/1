use sea_orm_migration::prelude::*;

// V15 P2 B05-P2-10：期末调整记录表（暂估 / 摊销 / 预提）
//
// 支持期末权责发生制调整：暂估入库、待摊费用摊销、预提费用。
// 状态机：draft(草稿) → confirmed(已确认，生成凭证) → reversed(已冲销，红字凭证) / cancelled(已取消)
// 结账时由 accounting_period_service.close_period 调用 PeriodAdjustmentService 批量确认 draft 记录。
//
// 关联文件：
//   - migrations/20260801000006_create_period_adjustment_record/up.sql
//   - migrations/20260801000006_create_period_adjustment_record/down.sql
//   - models/period_adjustment_record.rs
//   - services/period_adjustment_service.rs
//   - handlers/period_adjustment_handler.rs
//   - routes/period_adjustment.rs
//   - services/accounting_period_service.rs（close_period 注入 confirm_pending 调用）

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        let sql =
            include_str!("../../migrations/20260801000006_create_period_adjustment_record/up.sql");
        manager.get_connection().execute_unprepared(sql).await?;
        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        let sql = include_str!(
            "../../migrations/20260801000006_create_period_adjustment_record/down.sql"
        );
        manager.get_connection().execute_unprepared(sql).await?;
        Ok(())
    }
}
