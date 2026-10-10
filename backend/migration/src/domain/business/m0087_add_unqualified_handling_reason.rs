//! 不合格品处置理由列迁移
//!
//! 向 `unqualified_products` 补 `handling_reason` 处置理由列（TEXT，可空）。
//! 该列记录"为什么这样处置"，与已有的 `unqualified_reason`（不合格原因）、
//! `handling_method`（处置方式）语义各异、互不替代：前者说明缺陷本身，后者说明
//! 处置动作，本列说明采取该处置的理由。处置理由为自由文本，由处置录入接口写入、
//! 详情/列表回读，非空/非空白校验在应用层完成，DB 侧不设 CHECK（自由文本无固定
//! 词表，加约束即自造取值域）。历史存量行该列为 NULL，语义即"未填写处置理由"，
//! 无需回填。down 对称删列，不触碰任何既有业务数据。

use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        let sql = r#"ALTER TABLE "unqualified_products" ADD COLUMN IF NOT EXISTS "handling_reason" TEXT;
COMMENT ON COLUMN "unqualified_products"."handling_reason" IS '不合格品处置理由（自由文本，可空，NULL=未填写）；应用层校验非空白，DB 不设 CHECK';"#;
        manager.get_connection().execute_unprepared(sql).await?;
        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        let sql = r#"ALTER TABLE "unqualified_products" DROP COLUMN IF EXISTS "handling_reason";"#;
        manager.get_connection().execute_unprepared(sql).await?;
        Ok(())
    }
}
