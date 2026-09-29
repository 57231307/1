//! 销售订单行交货数量容差行级列
//!
//! 为 `sales_order_items` 新增可空 `quantity_tolerance_pct DECIMAL(5,2)`：
//! - NULL = 未行级指定，由服务端按「品类默认（面料 5% / 计件 0%）> 全局默认（5%）」解析；
//! - 非空 = 行级覆盖（含「约」订单写入 10.00）。
//!
//! 与 m0058 对 purchase_order_item / sales_contract_items 的同名列语义完全对称，
//! 使销售发货门控 (`validate_shipment_within_tolerance`) 支持行级覆盖。
//!
//! 本列非外键、宽度固定 DECIMAL(5,2)（最大 999.99，覆盖 0-100% 及裕量），
//! 与 m0044 `fix_fk_types` 白名单（仅处理被引用表 FK 宽度）无关。

use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        let sql = r#"
ALTER TABLE "sales_order_items" ADD COLUMN IF NOT EXISTS "quantity_tolerance_pct" DECIMAL(5,2);
"#;
        if !sql.trim().is_empty() {
            manager.get_connection().execute_unprepared(sql).await?;
        }
        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        let sql = r#"
ALTER TABLE "sales_order_items" DROP COLUMN IF EXISTS "quantity_tolerance_pct";
"#;
        if !sql.trim().is_empty() {
            manager.get_connection().execute_unprepared(sql).await?;
        }
        Ok(())
    }
}
