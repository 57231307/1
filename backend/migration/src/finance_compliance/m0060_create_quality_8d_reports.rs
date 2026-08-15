use sea_orm_migration::prelude::*;

// Batch 480 P0-F20：8D 质量管理流程建表
//
// 业务背景：审计报告 v15/batch-18 类二十一 维度 4 缺陷 4.1 (P0)
//   - quality_issues 表仅有 issue_type/severity/description/discovered_at/resolved_at/resolution/status
//   - 仅 open/resolved/closed 三态，无 8D 各阶段独立字段
//   - Grep 8D|5Why|fishbone|根因|纠正预防 在 backend/src 中零结果
//   - 审计计划 21.4 要求：质量问题必须走 8D 流程
//     D1 团队→D2 描述→D3 临时措施→D4 根因→D5 永久措施→D6 验证→D7 预防→D8 闭环
//
// 修复方案（审计建议 + batch-19 §23.3 缺陷 4 修复建议）：
//   - 新建 quality_8d_reports 表（与 quality_issues 一对一关联）
//   - 11 态状态机：not_started / d0_plan / d1_team / d2_problem / d3_interim / d4_root_cause /
//                  d5_permanent / d6_verify / d7_prevent / d8_recognize / closed
//   - 每阶段独立字段记录团队/描述/临时措施/根因方法（5why/fishbone）/永久措施/验证/预防/表彰
//   - D4 根因分析方法字段（缺陷 4.2 P1 修复）：5why / fishbone / other
//   - D5 永久措施责任人 + 完成日期字段（缺陷 4.3 P1 修复）：跟踪 + 超期告警
//
// 关联文件：models/quality_8d_report.rs / services/quality_8d_service.rs /
//          handlers/quality_8d_handler.rs / routes/quality_8d.rs

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .get_connection()
            .execute_unprepared(
                r#"
                -- 8D 质量管理流程报告表（V15 P0-F20 创建）
                -- 与 quality_issues 一对一关联（一个质量异常最多启动一个 8D 报告）
                -- 11 态状态机：not_started → d0_plan → d1_team → d2_problem → d3_interim
                --              → d4_root_cause → d5_permanent → d6_verify → d7_prevent
                --              → d8_recognize → closed
                CREATE TABLE IF NOT EXISTS "quality_8d_reports" (
                    "id" BIGSERIAL PRIMARY KEY,
                    -- 关联质量异常（一对一）
                    "quality_issue_id" BIGINT NOT NULL REFERENCES "quality_issues"("id") ON DELETE CASCADE,
                    -- 11 态状态机
                    "status" VARCHAR(20) NOT NULL DEFAULT 'not_started',
                    -- D0 准备阶段（计划与发起）
                    "d0_date" TIMESTAMPTZ,
                    "d0_prepared_by" INTEGER REFERENCES "users"("id") ON DELETE SET NULL,
                    "d0_plan" TEXT,
                    -- D1 团队组建
                    "d1_date" TIMESTAMPTZ,
                    "d1_team_members" TEXT,
                    -- D2 问题描述
                    "d2_date" TIMESTAMPTZ,
                    "d2_problem_description" TEXT,
                    -- D3 临时措施（围堵）
                    "d3_date" TIMESTAMPTZ,
                    "d3_interim_action" TEXT,
                    -- D4 根本原因分析（缺陷 4.2：5Why/鱼骨图）
                    "d4_date" TIMESTAMPTZ,
                    "d4_root_cause_method" VARCHAR(20),
                    "d4_root_cause_detail" TEXT,
                    "d4_root_cause_summary" TEXT,
                    -- D5 永久纠正措施（缺陷 4.3：责任人 + 完成日期跟踪）
                    "d5_date" TIMESTAMPTZ,
                    "d5_permanent_action" TEXT,
                    "d5_action_owner" VARCHAR(100),
                    "d5_due_date" DATE,
                    "d5_completed_at" TIMESTAMPTZ,
                    -- D6 实施验证
                    "d6_date" TIMESTAMPTZ,
                    "d6_verification_result" TEXT,
                    -- D7 预防措施（标准化）
                    "d7_date" TIMESTAMPTZ,
                    "d7_prevention_action" TEXT,
                    -- D8 团队表彰与闭环
                    "d8_date" TIMESTAMPTZ,
                    "d8_closure_summary" TEXT,
                    -- 关闭信息
                    "closed_at" TIMESTAMPTZ,
                    "closed_by" INTEGER REFERENCES "users"("id") ON DELETE SET NULL,
                    -- 元数据
                    "created_at" TIMESTAMPTZ NOT NULL DEFAULT NOW(),
                    "updated_at" TIMESTAMPTZ NOT NULL DEFAULT NOW(),
                    -- 约束
                    CONSTRAINT "chk_q8d_status" CHECK (
                        "status" IN (
                            'not_started', 'd0_plan', 'd1_team', 'd2_problem', 'd3_interim',
                            'd4_root_cause', 'd5_permanent', 'd6_verify', 'd7_prevent',
                            'd8_recognize', 'closed'
                        )
                    ),
                    CONSTRAINT "chk_q8d_root_cause_method" CHECK (
                        "d4_root_cause_method" IS NULL OR
                        "d4_root_cause_method" IN ('5why', 'fishbone', 'other')
                    )
                );

                -- 索引（3 个，覆盖高频查询场景）
                CREATE INDEX IF NOT EXISTS "idx_q8d_quality_issue_id" ON "quality_8d_reports"("quality_issue_id");
                CREATE INDEX IF NOT EXISTS "idx_q8d_status" ON "quality_8d_reports"("status");
                -- 一个 quality_issue 最多一个 8D 报告（一对一）
                CREATE UNIQUE INDEX IF NOT EXISTS "uq_q8d_quality_issue_id" ON "quality_8d_reports"("quality_issue_id");

                COMMENT ON TABLE "quality_8d_reports" IS '8D 质量管理流程报告表 - D0~D8 八步流程 + 11 态状态机';
                COMMENT ON COLUMN "quality_8d_reports"."d4_root_cause_method" IS '根因分析方法：5why（五问法）/ fishbone（鱼骨图）/ other（其他）';
                COMMENT ON COLUMN "quality_8d_reports"."d5_action_owner" IS 'D5 永久措施责任人姓名或工号';
                COMMENT ON COLUMN "quality_8d_reports"."d5_due_date" IS 'D5 永久措施计划完成日期（超期由定时任务扫描告警）';
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
                DROP TABLE IF EXISTS "quality_8d_reports";
                "#,
            )
            .await?;
        Ok(())
    }
}
