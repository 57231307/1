//! 产品匹/卷换算元数据列
//!
//! 为 `products` 新增可空 `meters_per_piece`、`meters_per_roll DECIMAL(12,4)`：
//! - 作为「匹↔米」「卷↔米」换算视图的单一真源入参（配合 utils/dual_unit_converter 的
//!   固定换算率与克重×幅宽算法）；
//! - NULL = 历史产品未配置，不影响既有数据；报价/订单单位跟随产品交易单位（中文 token）；
//! - 本组列不参与库存落库——库存真相仍为 `inventory_stocks` / `sales_order_items` 的
//!   米/公斤双列，码/匹/卷仅按这些元数据换算为「约」视图。
//!
//! 迁移幂等：ADD COLUMN IF NOT EXISTS，可空默认 NULL。

use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        let sql = r#"
ALTER TABLE "products" ADD COLUMN IF NOT EXISTS "meters_per_piece" DECIMAL(12,4);
ALTER TABLE "products" ADD COLUMN IF NOT EXISTS "meters_per_roll" DECIMAL(12,4);
"#;
        if !sql.trim().is_empty() {
            manager.get_connection().execute_unprepared(sql).await?;
        }
        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        let sql = r#"
ALTER TABLE "products" DROP COLUMN IF EXISTS "meters_per_piece";
ALTER TABLE "products" DROP COLUMN IF EXISTS "meters_per_roll";
"#;
        if !sql.trim().is_empty() {
            manager.get_connection().execute_unprepared(sql).await?;
        }
        Ok(())
    }
}
