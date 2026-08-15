use sea_orm_migration::prelude::*;

// Batch 483 P0-B12：售后与质量集成（关联 quality_issue_id，触发 8D 流程）
//
// 业务背景：
//   V15 审计报告 batch-19 §23.3 缺陷 4 — 售后与质量集成缺失
//
//   面料行业质量异常的售后处理是核心业务场景。当前实现：
//   - 客户投诉色差问题，售后工单无法关联到生产过程的质量异常记录（quality_issue）
//   - 无法追溯质量异常是否已通过 8D 流程闭环
//   - 售后处理与质量改进脱钩，相同质量问题可能反复发生
//
//   after_sales 表无 quality_issue_id 字段，与 quality_issue 表完全隔离。
//   quality_8d_report 表（Batch 480 P0-F20 已建）通过 quality_issue_id 一对一关联
//   quality_issue，但无任何售后场景的触发入口。
//
// 修复方案：
//   1. after_sales 表新增 quality_issue_id 字段关联 quality_issue 表
//   2. 售后服务新增 trigger_8d_investigation 方法：
//      自动创建 quality_issue（若未关联）+ 启动 8D 流程
//   3. 售后状态机新增 quality_investigating 中间态（opened → quality_investigating → processing）
//
// 设计依据：V15 审计报告 P0-B12（batch-19 §23.3 缺陷 4）
// 依赖：Batch 480 P0-F20（8D 流程已实现，quality_8d_report 表 + QualityEightDService）
// 关联文件：
//   - models/after_sales.rs（新增 quality_issue_id 字段 + QualityIssue Relation）
//   - services/custom_order_aftersales_service.rs（trigger_8d_investigation 方法）

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .get_connection()
            .execute_unprepared(
                r#"
                -- ============================================================
                -- P0-B12：after_sales 表新增 quality_issue_id 关联字段
                -- ============================================================

                -- 关联质量异常 ID（指向 quality_issues.id）
                -- 业务语义：当售后工单 issue_type='complaint'（客诉）时，
                -- 自动或手工关联到生产过程的质量异常记录。
                -- 关联后可启动 8D 流程（quality_8d_report）进行根因分析。
                ALTER TABLE "after_sales"
                    ADD COLUMN IF NOT EXISTS "quality_issue_id" BIGINT;

                -- 索引：按质量异常反查售后工单
                CREATE INDEX IF NOT EXISTS "idx_after_sales_quality_issue_id"
                    ON "after_sales"("quality_issue_id");

                COMMENT ON COLUMN "after_sales"."quality_issue_id" IS '关联质量异常 ID（P0-B12：售后与质量集成锚点，指向 quality_issues.id；客诉类工单关联后可启动 8D 流程进行根因分析）';
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
                -- 回滚 after_sales 新增字段
                DROP INDEX IF EXISTS "idx_after_sales_quality_issue_id";
                ALTER TABLE "after_sales"
                    DROP COLUMN IF EXISTS "quality_issue_id";
                "#,
            )
            .await?;
        Ok(())
    }
}
