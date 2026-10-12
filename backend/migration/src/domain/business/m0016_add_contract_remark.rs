//! 合同表头备注列迁移
//!
//! 向 sales_contracts / purchase_contracts 两张合同主表补 remark 备注列（TEXT，可空）。
//! 合同创建/编辑表单采集的备注需要持久化并在详情/列表回读；历史存量行该列为 NULL，
//! 语义即"未填写备注"，无需回填。

use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        let sql = r#"ALTER TABLE "sales_contracts" ADD COLUMN IF NOT EXISTS "remark" TEXT;
COMMENT ON COLUMN "sales_contracts"."remark" IS '合同备注（表头级，可空）';
ALTER TABLE "purchase_contracts" ADD COLUMN IF NOT EXISTS "remark" TEXT;
COMMENT ON COLUMN "purchase_contracts"."remark" IS '合同备注（表头级，可空）';"#;
        manager.get_connection().execute_unprepared(sql).await?;
        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        let sql = r#"ALTER TABLE "sales_contracts" DROP COLUMN IF EXISTS "remark";
ALTER TABLE "purchase_contracts" DROP COLUMN IF EXISTS "remark";"#;
        manager.get_connection().execute_unprepared(sql).await?;
        Ok(())
    }
}
