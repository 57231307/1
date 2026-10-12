//! 出库四维补强（用户 2026-10-02 纠正口径：出库对染色布强制四维 = 缸号/色号/批次/匹号）：
//! 调拨明细表补匹号列。
//!
//! 背景：m0052 为「生产/入库/外发/销售/出库/对账」单据条目批量补 piece_no 时遗漏了
//! inventory_transfer_items（调拨出库单据明细），导致调拨链路的匹号维度无落点——
//! 出库明细只能以请求参数校验匹号、发运时无法按单持久化的匹号消耗库存匹。
//! 本迁移照 m0052 先例补同一列（VARCHAR(100)，可空：白坯布合法无匹号）。
//! 目标表 inventory_transfer_items 由 system 域 m0001 建表，早于本域执行，直接注册本域可行。

use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        let sql = r#"
-- 调拨出库明细：染色匹号（出库四维追溯之第四维；白坯布为 NULL）
ALTER TABLE inventory_transfer_items ADD COLUMN IF NOT EXISTS piece_no VARCHAR(100);
"#;
        if !sql.trim().is_empty() {
            manager.get_connection().execute_unprepared(sql).await?;
        }
        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        let sql = r#"
ALTER TABLE inventory_transfer_items DROP COLUMN IF EXISTS piece_no;
"#;
        if !sql.trim().is_empty() {
            manager.get_connection().execute_unprepared(sql).await?;
        }
        Ok(())
    }
}
