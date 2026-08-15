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
        let sql = include_str!(
            "../../migrations/20260801000003_add_rework_cost_to_dye_batch_rework/up.sql"
        );
        manager.get_connection().execute_unprepared(sql).await?;
        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        let sql = include_str!(
            "../../migrations/20260801000003_add_rework_cost_to_dye_batch_rework/down.sql"
        );
        manager.get_connection().execute_unprepared(sql).await?;
        Ok(())
    }
}
