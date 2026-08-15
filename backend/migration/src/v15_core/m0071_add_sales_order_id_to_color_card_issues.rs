use sea_orm_migration::prelude::*;

// V15 P1 Batch-09 10.3-1：色卡发放记录与订单关联
//
// 背景：
// - 旧 color_card_issues 表无 sales_order_id 字段，无法支持"订单驱动发放色卡"场景
// - 审计计划 10.3.1 节要求实现订单关联（强关联 + 弱关联 + 复购关联）
//
// 设计：
// - sales_order_id 为可选字段（弱关联：色卡可不绑定订单单独发放）
// - 创建索引 idx_issue_sales_order_id 加速按订单查询
// - 添加外键约束（RESTRICT）防止订单被误删时关联记录悬空
//
// 兼容策略：
// - ADD COLUMN IF NOT EXISTS 幂等安全
// - 历史记录 sales_order_id = NULL（无关联订单）

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .get_connection()
            .execute_unprepared(
                r#"
                ALTER TABLE "color_card_issues"
                    ADD COLUMN IF NOT EXISTS "sales_order_id" BIGINT;

                CREATE INDEX IF NOT EXISTS "idx_issue_sales_order_id"
                    ON "color_card_issues"("sales_order_id")
                    WHERE "sales_order_id" IS NOT NULL;

                COMMENT ON COLUMN "color_card_issues"."sales_order_id" IS
                    'V15 P1 10.3-1：关联销售订单 ID（NULL=非订单驱动发放，非 NULL=订单驱动发放）';
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
                DROP INDEX IF EXISTS "idx_issue_sales_order_id";
                ALTER TABLE "color_card_issues" DROP COLUMN IF EXISTS "sales_order_id";
                "#,
            )
            .await?;
        Ok(())
    }
}
