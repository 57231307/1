//! 销售报价单明细表迁移
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
            r#"-- 销售报价单明细
-- 用于存储报价单中每个产品/色号的行项目
-- 创建时间: 2026-06-16

CREATE TABLE IF NOT EXISTS "sales_quotation_items" (
    "id" BIGSERIAL PRIMARY KEY,
    "quotation_id" BIGINT NOT NULL REFERENCES "sales_quotations"("id") ON DELETE CASCADE,

    "product_id" BIGINT NOT NULL REFERENCES "products"("id"),
    "color_id" BIGINT REFERENCES "product_colors"("id"),
    "color_code" VARCHAR(50),
    "pantone_code" VARCHAR(50),
    "cncs_code" VARCHAR(50),

    "specification" TEXT,
    "unit" VARCHAR(20) NOT NULL,

    "quantity" DECIMAL(18,2) NOT NULL,
    "unit_price" DECIMAL(18,6) NOT NULL,
    "unit_price_with_tax" DECIMAL(18,6) NOT NULL,
    "amount" DECIMAL(18,2) NOT NULL,
    "amount_with_tax" DECIMAL(18,2) NOT NULL,

    "tier_pricing" JSONB,
    "discount_rate" DECIMAL(5,2) DEFAULT 0,
    "discount_amount" DECIMAL(18,2) DEFAULT 0,

    "notes" TEXT,
    "sequence" INT NOT NULL DEFAULT 0
);

CREATE INDEX IF NOT EXISTS "idx_quotation_items_quotation" ON "sales_quotation_items"("quotation_id");
CREATE INDEX IF NOT EXISTS "idx_quotation_items_product" ON "sales_quotation_items"("product_id");
CREATE INDEX IF NOT EXISTS "idx_quotation_items_color" ON "sales_quotation_items"("color_id");"#;
        if !sql.trim().is_empty() {
            manager.get_connection().execute_unprepared(sql).await?;
        }
        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        let sql = r#"-- 回滚 sales_quotation_items 表
DROP INDEX IF EXISTS "idx_quotation_items_color";
DROP INDEX IF EXISTS "idx_quotation_items_product";
DROP INDEX IF EXISTS "idx_quotation_items_quotation";
DROP TABLE IF EXISTS "sales_quotation_items";"#;
        if !sql.trim().is_empty() {
            manager.get_connection().execute_unprepared(sql).await?;
        }
        Ok(())
    }
}
