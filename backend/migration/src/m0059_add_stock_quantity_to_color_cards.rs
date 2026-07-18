use sea_orm_migration::prelude::*;

// Batch 477 P0-F10 修复：color_cards 表新增 stock_quantity 字段
//
// 色卡库存联动方案选择：
//   方案 A（采用）：color_cards.stock_quantity INT NOT NULL DEFAULT 0
//     - 简单直接，色卡作为「实物资产」管理（一张卡就是一件），与 fabric inventory_stock 解耦
//     - issue 时扣减 stock_quantity -= issue_qty
//     - return_card 时还原 stock_quantity += issue_qty
//     - mark_lost/mark_damaged 时不还原（卡片消耗）
//     - cancel_issue 时还原 stock_quantity += issue_qty
//   方案 B（未采用）：关联 inventory_stock 表
//     - 需新增 (product_id, color_no, dye_lot_no) 关联，复杂度高
//     - 色卡本身不是面料，进入 fabric inventory 体系语义错位
//
// 设计依据：V15 审计报告 类九 P0-F10
// 关联文件：models/color_card.rs / services/color_card_issue_service.rs

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .get_connection()
            .execute_unprepared(
                r#"
                -- color_cards 表新增 stock_quantity 字段（色卡库存数量，单张发放场景默认 1）
                ALTER TABLE "color_cards" ADD COLUMN IF NOT EXISTS "stock_quantity" INT NOT NULL DEFAULT 0;

                COMMENT ON COLUMN "color_cards"."stock_quantity" IS '色卡库存数量（可用于发放，发放时扣减、归还时还原、遗失/损坏时不还原）';

                -- 初始化存量色卡的 stock_quantity = total_colors（兼容旧数据，避免发放即报库存不足）
                -- total_colors 字段语义为「色卡包含的颜色数」，作为初始库存量参考
                UPDATE "color_cards" SET "stock_quantity" = GREATEST("total_colors", 1) WHERE "stock_quantity" = 0;
                "#,
            )
            .await?;

        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .get_connection()
            .execute_unprepared(
                r#"
                ALTER TABLE "color_cards" DROP COLUMN IF EXISTS "stock_quantity";
                "#,
            )
            .await?;
        Ok(())
    }
}
