//! 匹号领域二期：生产/入库/外发/销售/出库/对账单据条目携带匹号/批号字段
//! （设计见 docs/piece-number-domain-design.md；用户规则：染色匹号贯穿
//!   入库/外发/销售/出库/对账，生产匹号用于外发发料环节）

use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        let sql = r#"
-- 入库单条目：染色匹号（入库使用染色匹号，净布为生产匹号）
ALTER TABLE purchase_receipt_item ADD COLUMN IF NOT EXISTS piece_no VARCHAR(100);
-- 销售订单条目：染色匹号
ALTER TABLE sales_order_items ADD COLUMN IF NOT EXISTS piece_no VARCHAR(100);
-- 销售出库条目：染色匹号
ALTER TABLE sales_delivery_item ADD COLUMN IF NOT EXISTS piece_no VARCHAR(100);
-- 委外订单条目（发料）：生产匹号（外发染色引用生产匹）
ALTER TABLE outsourcing_order_item ADD COLUMN IF NOT EXISTS piece_no VARCHAR(100);
-- 应收对账条目：匹号 + 批号/色号/缸号（对账需要体现）
ALTER TABLE ar_reconciliation_items ADD COLUMN IF NOT EXISTS piece_no VARCHAR(100);
ALTER TABLE ar_reconciliation_items ADD COLUMN IF NOT EXISTS batch_no VARCHAR(255);
ALTER TABLE ar_reconciliation_items ADD COLUMN IF NOT EXISTS color_no VARCHAR(255);
ALTER TABLE ar_reconciliation_items ADD COLUMN IF NOT EXISTS dye_lot_no VARCHAR(255);
-- 应付对账条目：同上（采购对账体现）
ALTER TABLE ap_reconciliation ADD COLUMN IF NOT EXISTS piece_no VARCHAR(100);
ALTER TABLE ap_reconciliation ADD COLUMN IF NOT EXISTS batch_no VARCHAR(255);
ALTER TABLE ap_reconciliation ADD COLUMN IF NOT EXISTS dye_lot_no VARCHAR(255);
"#;
        if !sql.trim().is_empty() {
            manager.get_connection().execute_unprepared(sql).await?;
        }
        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        let sql = r#"
ALTER TABLE purchase_receipt_item DROP COLUMN IF EXISTS piece_no;
ALTER TABLE sales_order_items DROP COLUMN IF EXISTS piece_no;
ALTER TABLE sales_delivery_item DROP COLUMN IF EXISTS piece_no;
ALTER TABLE outsourcing_order_item DROP COLUMN IF EXISTS piece_no;
ALTER TABLE ar_reconciliation_items DROP COLUMN IF EXISTS piece_no;
ALTER TABLE ar_reconciliation_items DROP COLUMN IF EXISTS batch_no;
ALTER TABLE ar_reconciliation_items DROP COLUMN IF EXISTS color_no;
ALTER TABLE ar_reconciliation_items DROP COLUMN IF EXISTS dye_lot_no;
ALTER TABLE ap_reconciliation DROP COLUMN IF EXISTS piece_no;
ALTER TABLE ap_reconciliation DROP COLUMN IF EXISTS batch_no;
ALTER TABLE ap_reconciliation DROP COLUMN IF EXISTS dye_lot_no;
"#;
        if !sql.trim().is_empty() {
            manager.get_connection().execute_unprepared(sql).await?;
        }
        Ok(())
    }
}
