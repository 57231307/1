//! 销售报价单贸易条款表迁移
//!
//! 创建时间: 2026-06-16
//! 关联计划: 2026-06-16-sales-quotation-plan.md Task 1

use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        let sql =
            r#"-- 销售报价单贸易条款
-- 用于存储报价单中各类贸易条款（物流/付款/样品/检验）
-- 创建时间: 2026-06-16

CREATE TABLE IF NOT EXISTS "sales_quotation_terms" (
    "id" BIGSERIAL PRIMARY KEY,
    "quotation_id" BIGINT NOT NULL REFERENCES "sales_quotations"("id") ON DELETE CASCADE,
    "term_type" VARCHAR(50) NOT NULL,
    "term_key" VARCHAR(100) NOT NULL,
    "term_value" TEXT NOT NULL,
    "sequence" INT NOT NULL DEFAULT 0,

    CONSTRAINT "chk_term_type" CHECK ("term_type" IN ('logistics','payment','sample','inspection'))
);

CREATE INDEX IF NOT EXISTS "idx_quotation_terms_quotation" ON "sales_quotation_terms"("quotation_id");
CREATE INDEX IF NOT EXISTS "idx_quotation_terms_type" ON "sales_quotation_terms"("term_type");"#;
        if !sql.trim().is_empty() {
            manager.get_connection().execute_unprepared(sql).await?;
        }
        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        let sql = r#"-- 回滚 sales_quotation_terms 表
DROP INDEX IF EXISTS "idx_quotation_terms_type";
DROP INDEX IF EXISTS "idx_quotation_terms_quotation";
DROP TABLE IF EXISTS "sales_quotation_terms";"#;
        if !sql.trim().is_empty() {
            manager.get_connection().execute_unprepared(sql).await?;
        }
        Ok(())
    }
}
