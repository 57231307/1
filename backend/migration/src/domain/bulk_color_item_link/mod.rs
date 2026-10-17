//! 批色记录挂销售订单明细行：为 `bulk_color_approval` 补可空列 `sales_order_item_id`（外键指向
//! `sales_order_items.id`，`ON DELETE SET NULL`）与配套检索索引。
//!
//! ## 用途
//! 批色单此前只挂订单级 `sales_order_id`，无法由具体明细行发起、也无法按行做发货门禁。补该列后：
//! - 建单可从销售订单明细行发起，反范式四维列（`product_id`/`color_no`）以该行为准回填；
//! - 发货门禁按 `(product_id, color_no)` 归一匹配到行，仅对本次发货涉及的行判审批态，
//!   未挂行（NULL）的历史记录退化回订单级整单阻断。
//!
//! ## 语义与幂等
//! - 列可空。存量行 NULL 的语义 = “未关联明细行”，发货门禁据此走订单级整单阻断旧行为，
//!   不因新列为空而放宽或阻断既有单据。
//! - 新增可空列不回填、不改写任何存量值，故无需备份表即可精确回退；
//!   `ADD COLUMN IF NOT EXISTS` / `CREATE INDEX IF NOT EXISTS` / `DROP ... IF EXISTS` 幂等可重放。
//!
//! ## 注册位置
//! 作为链尾新顶层域而非并入既有域：并入已应用的域在复用库上会因台账已记账而被整体跳过，
//! 导致列永不补上。链尾新域名在复用库上属 pending，必然执行。依赖 `sales_order_items` 建表
//! （business/v15 域）与 `bulk_color_approval` 建表（v15 域），二者均早于本域完成。

use sea_orm_migration::prelude::*;

pub struct Migration;

impl MigrationName for Migration {
    fn name(&self) -> &'static str {
        "m_bulk_color_item_link"
    }
}

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        let conn = manager.get_connection();
        conn.execute_unprepared(
            r#"ALTER TABLE "bulk_color_approval" ADD COLUMN IF NOT EXISTS "sales_order_item_id" INTEGER REFERENCES "sales_order_items"("id") ON DELETE SET NULL;"#,
        )
        .await?;
        conn.execute_unprepared(
            r#"CREATE INDEX IF NOT EXISTS "idx_bca_sales_order_item_id" ON "bulk_color_approval"("sales_order_item_id");"#,
        )
        .await?;
        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        let conn = manager.get_connection();
        conn.execute_unprepared(r#"DROP INDEX IF EXISTS "idx_bca_sales_order_item_id";"#)
            .await?;
        conn.execute_unprepared(
            r#"ALTER TABLE "bulk_color_approval" DROP COLUMN IF EXISTS "sales_order_item_id";"#,
        )
        .await?;
        Ok(())
    }
}
