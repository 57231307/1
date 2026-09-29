//! 定制订单状态 CHECK 约束补齐打样/报价阶段
//!
//! 状态机 `utils/process_state_machine.rs::CustomOrderStatus::as_str()` 在 V15 P0-B11 起
//! 会写入 `lab_dip`、`quotation` 两个新阶段（draft→lab_dip→quotation→yarn_purchasing→…），
//! 而建表迁移 `m0044` 的 `chk_custom_order_status` 仅允许原 8 值、不含这两个 token。
//! `POST /custom-orders/{id}/advance` 推进到 lab_dip/quotation 时写 status 即违反该约束，
//! 数据库回 `violates check constraint "chk_custom_order_status"`，对上游表现为 DATABASE_ERROR(500)。
//!
//! 本迁移重建该 CHECK 约束，允许集与状态机 `as_str()` 的全部产出逐字符对齐
//! （全小写、下划线分隔），使门控值/比较点与写入值同源。

use sea_orm_migration::prelude::*;

/// 状态机 `as_str()` 产出的全部合法 status 值（10 个，逐字符对齐，不含任何派生别名）。
const ALLOWED_STATUS_VALUES: &str = "'draft', 'lab_dip', 'quotation', 'yarn_purchasing', \
     'dyeing', 'finishing', 'delivery', 'after_sales', 'completed', 'cancelled'";

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        // 先删旧约束（IF EXISTS 兼容已升级与全新库），再按状态机产出重建含打样/报价的允许集。
        let sql = format!(
            r#"ALTER TABLE "custom_orders" DROP CONSTRAINT IF EXISTS "chk_custom_order_status";
ALTER TABLE "custom_orders" ADD CONSTRAINT "chk_custom_order_status" CHECK ("status" IN ({ALLOWED_STATUS_VALUES}));
COMMENT ON COLUMN "custom_orders"."status" IS '订单状态：draft(草稿) / lab_dip(打样中) / quotation(报价中) / yarn_purchasing(纱线采购) / dyeing(染整) / finishing(后整理) / delivery(交付) / after_sales(售后) / completed(已完成) / cancelled(已取消)';"#
        );
        manager.get_connection().execute_unprepared(&sql).await?;
        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        // 回滚恢复建表迁移 m0044 的原始 8 值约束（不含 lab_dip/quotation）。
        let sql = r#"ALTER TABLE "custom_orders" DROP CONSTRAINT IF EXISTS "chk_custom_order_status";
ALTER TABLE "custom_orders" ADD CONSTRAINT "chk_custom_order_status" CHECK ("status" IN (
    'draft', 'yarn_purchasing', 'dyeing', 'finishing',
    'delivery', 'after_sales', 'completed', 'cancelled'
));"#;
        manager.get_connection().execute_unprepared(sql).await?;
        Ok(())
    }
}
