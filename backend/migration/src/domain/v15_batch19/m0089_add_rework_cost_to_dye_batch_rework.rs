use sea_orm_migration::prelude::*;

// V15 P2 B05-P2-2：dye_batch_rework 表新增 rework_cost 字段
//
// 配合 rework_type 枚举扩展（新增 re_dye/replenish_dye），记录每次回修的成本金额，
// 支持按回修类型分类统计（重染成本 vs 补染成本），用于成本核算与持续改进分析。
//
// 关联文件：
//   - migrations/20260801000003_add_rework_cost_to_dye_batch_rework/up.sql
//   - migrations/20260801000003_add_rework_cost_to_dye_batch_rework/down.sql
//   - models/dye_batch_rework.rs（rework_cost: Option<Decimal>）
//   - services/dye_batch_state_machine_validation.rs（validate_rework_type 白名单更新）

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        let sql = r#"-- V15 P2 B05-P2-2：dye_batch_rework 表新增 rework_cost 字段
-- 记录每次回修的成本金额，按 rework_type 分类统计（re_dye 重染 / replenish_dye 补染）
-- 字段可为空（历史数据无此字段，新数据由业务层按需写入）
ALTER TABLE dye_batch_rework
    ADD COLUMN IF NOT EXISTS rework_cost NUMERIC(14, 4);

COMMENT ON COLUMN dye_batch_rework.rework_cost IS '回修成本（V15 P2 B05-P2-2）：按 rework_type 分类统计，re_dye 整缸重染成本高 / replenish_dye 局部补染成本低';"#;
        manager.get_connection().execute_unprepared(sql).await?;
        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        let sql = r#"-- V15 P2 B05-P2-2 回滚：移除 rework_cost 字段
ALTER TABLE dye_batch_rework
    DROP COLUMN IF EXISTS rework_cost;"#;
        manager.get_connection().execute_unprepared(sql).await?;
        Ok(())
    }
}
