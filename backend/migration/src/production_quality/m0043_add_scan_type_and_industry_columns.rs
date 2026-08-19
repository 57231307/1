//! inventory_piece.scan_type + crm_lead.industry 列迁移（v11 批次 153 P2-A）

use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        let sql = r#"-- v11 批次 153 P2-A：为 inventory_piece 表添加 scan_type 列，支持扫码历史按类型筛选
ALTER TABLE inventory_piece ADD COLUMN IF NOT EXISTS scan_type VARCHAR(50);

-- v11 批次 153 P2-A：为 crm_lead 表添加 industry 列，支持客户池按行业筛选
ALTER TABLE crm_lead ADD COLUMN IF NOT EXISTS industry VARCHAR(100);"#;
        if !sql.trim().is_empty() {
            manager.get_connection().execute_unprepared(sql).await?;
        }
        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        let sql = r#"-- v11 批次 153 P2-A：回滚 inventory_piece.scan_type 列
ALTER TABLE inventory_piece DROP COLUMN IF EXISTS scan_type;

-- v11 批次 153 P2-A：回滚 crm_lead.industry 列
ALTER TABLE crm_lead DROP COLUMN IF EXISTS industry;"#;
        if !sql.trim().is_empty() {
            manager.get_connection().execute_unprepared(sql).await?;
        }
        Ok(())
    }
}
