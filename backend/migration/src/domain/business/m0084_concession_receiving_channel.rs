//! 采购收货「让步接收 / 复检改判」通道：真实列 + 检验状态 DB CHECK 同批落地
//!
//! ## 六要素
//! - **功能**：① `purchase_receipt` 新增让步/改判真实列（让步理由/操作人/时间、
//!   改判理由/操作人/时间/累计次数）；② 对 `inspection_status` 施加 DB CHECK
//!   `chk_purchase_receipt_inspection_status`，取值集 = 权威词表
//!   `models/status/purchase_inventory.rs::purchase_receipt_inspection::ALL`
//!   （PENDING/PASSED/REJECTED/CONCESSION_ACCEPTED 四值，词表与 CHECK 同批）。
//! - **调用方**：SeaORM migrator 链，注册在 `domain/business` 域 up 链尾、down 链首
//!   （目标表 purchase_receipt 由本域 m0009 建，早于链尾；值域归一由 production 域
//!   m0057 完成，域执行顺序 business→production，存量四值集合无需再归一）。
//! - **入参**：无（纯列/约束构建，不读取应用之外的数据）。
//! - **传给谁**：写入方为 `services/purchase_receipt_ops/state.rs` 的
//!   `concede_receipt`（写 concession_reason/concession_by/concession_at，状态列写
//!   CONCESSION_ACCEPTED）与 `rejudge_receipt`（写 rejudge_reason/rejudge_by/rejudge_at、
//!   rejudge_count 累加，状态列改判为 PASSED/REJECTED）；读取方为收货列表/详情
//!   （`purchase_receipt_ops::query` 的 PurchaseReceiptDto 同源列）与前端让步理由回显。
//!   旁路写入由本 CHECK 在数据库层拦截。
//! - **存什么**：让步/改判的专用真实列——理由**严禁**借用 notes/remarks 等既有列
//!   （语义挪用是本仓 S-2 缺陷同族）；历史逐次痕迹走既有 `audit_log`
//!   （`AuditLogService::update_with_audit` 写改判前后全行快照与操作人）。
//! - **存哪里**：PostgreSQL `public.purchase_receipt` 表列与 pg_constraint 约束。
//!
//! 幂等：列 ADD COLUMN IF NOT EXISTS、约束先 DROP IF EXISTS 再施加，可安全重跑；
//! down 对称回退（删约束 + 删本迁移新增列，不触碰既有列与数据行）。
//! 存量守卫（fail-visible）：加 CHECK 前统计词表外存量（样例值点名，非 0 即中止），
//! 不在迁移里 UPDATE 造值洗数据。
//! 契约锁：`backend/tests/contract_wave11_concession_receiving_flow_test.rs` 断言
//! 本文件 ALLOWED_SQL 解析集 == 应用层词表 ALL（逐元素相等，任一侧单独增删判红）。

use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

/// 检验状态词表的 SQL 字面量清单（与 models/status/purchase_inventory.rs 的
/// purchase_receipt_inspection::ALL 同源逐字符一致；守卫、CHECK、列注释共用这一份，
/// 禁止另写一套）。
const ALLOWED_SQL: &str = "'PENDING','PASSED','REJECTED','CONCESSION_ACCEPTED'";

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        let conn = manager.get_connection();

        // 1) 让步/改判真实列（全部可空或带默认，存量行不受影响；列注释固化写入方）。
        conn.execute_unprepared(
            r#"
ALTER TABLE "purchase_receipt" ADD COLUMN IF NOT EXISTS "concession_reason" TEXT;
ALTER TABLE "purchase_receipt" ADD COLUMN IF NOT EXISTS "concession_by" INTEGER;
ALTER TABLE "purchase_receipt" ADD COLUMN IF NOT EXISTS "concession_at" TIMESTAMP;
ALTER TABLE "purchase_receipt" ADD COLUMN IF NOT EXISTS "rejudge_reason" TEXT;
ALTER TABLE "purchase_receipt" ADD COLUMN IF NOT EXISTS "rejudge_by" INTEGER;
ALTER TABLE "purchase_receipt" ADD COLUMN IF NOT EXISTS "rejudge_at" TIMESTAMP;
ALTER TABLE "purchase_receipt" ADD COLUMN IF NOT EXISTS "rejudge_count" INTEGER NOT NULL DEFAULT 0;
COMMENT ON COLUMN "purchase_receipt"."concession_reason" IS '最近一次让步接收理由（必填业务值，专用列，禁挪用 notes）；写入方 concede_receipt';
COMMENT ON COLUMN "purchase_receipt"."concession_by" IS '最近一次让步接收操作人（服务端会话派生，请求体不承载身份）；写入方 concede_receipt';
COMMENT ON COLUMN "purchase_receipt"."concession_at" IS '最近一次让步接收时间；写入方 concede_receipt';
COMMENT ON COLUMN "purchase_receipt"."rejudge_reason" IS '最近一次复检改判理由（必填业务值）；写入方 rejudge_receipt';
COMMENT ON COLUMN "purchase_receipt"."rejudge_by" IS '最近一次复检改判操作人（服务端会话派生）；写入方 rejudge_receipt';
COMMENT ON COLUMN "purchase_receipt"."rejudge_at" IS '最近一次复检改判时间；写入方 rejudge_receipt';
COMMENT ON COLUMN "purchase_receipt"."rejudge_count" IS '复检改判累计次数；写入方 rejudge_receipt 累加';
        "#,
        )
        .await?;

        // 2) 幂等前置：删旧约束（重跑/半态库不报错）。
        conn.execute_unprepared(
            r#"ALTER TABLE "purchase_receipt" DROP CONSTRAINT IF EXISTS "chk_purchase_receipt_inspection_status";"#,
        )
        .await?;

        // 3) 存量守卫（fail-visible）：inspection_status 列 NOT NULL，NULL 也计入词表外；
        //    非 0 即点名样例值并中止，不静默加约束后把脏行留在库里。
        let guard = format!(
            r#"
DO $$
DECLARE
    bad_rows INTEGER;
    samples  TEXT;
BEGIN
    SELECT COUNT(*) INTO bad_rows FROM "purchase_receipt"
      WHERE "inspection_status" IS NULL OR "inspection_status" NOT IN ({allowed});
    IF bad_rows = 0 THEN
        RAISE NOTICE 'm0084：purchase_receipt.inspection_status 无词表外存量行，可安全施加 CHECK chk_purchase_receipt_inspection_status。';
    ELSE
        SELECT string_agg(sample, ', ') INTO samples
          FROM (SELECT DISTINCT COALESCE("inspection_status", '<NULL>') AS sample
                  FROM "purchase_receipt"
                 WHERE "inspection_status" IS NULL OR "inspection_status" NOT IN ({allowed})
                 LIMIT 10) s;
        RAISE EXCEPTION 'm0084：purchase_receipt.inspection_status 存在 % 行词表外取值（样例值：%），拒绝施加 CHECK chk_purchase_receipt_inspection_status。', bad_rows, samples
            USING HINT = '本迁移拒绝 UPDATE 造值：请按权威词表 models/status/purchase_inventory.rs::purchase_receipt_inspection（PENDING/PASSED/REJECTED/CONCESSION_ACCEPTED）逐行核实更正存量，更正后重跑迁移；若脏值来自脚本旁路写入，先修写入方，不要在迁移里圆场。';
    END IF;
END;
$$;
"#,
            allowed = ALLOWED_SQL,
        );
        conn.execute_unprepared(&guard).await?;

        // 4) 施加 CHECK 并以列注释固化权威词表指向（列 NOT NULL，不设 NULL 放行；
        //    CONCESSION_ACCEPTED 长 19 字符，VARCHAR(20) 容纳，列宽不改）。
        let apply = format!(
            r#"
ALTER TABLE "purchase_receipt" ADD CONSTRAINT "chk_purchase_receipt_inspection_status" CHECK ("inspection_status" IN ({allowed}));
COMMENT ON COLUMN "purchase_receipt"."inspection_status" IS '采购收货检验状态：权威词表 models/status/purchase_inventory.rs::purchase_receipt_inspection，取值集与 CHECK chk_purchase_receipt_inspection_status 逐项相等（契约锁 contract_wave11_concession_receiving_flow_test）';
"#,
            allowed = ALLOWED_SQL,
        );
        conn.execute_unprepared(&apply).await?;

        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        let conn = manager.get_connection();
        conn.execute_unprepared(
            r#"ALTER TABLE "purchase_receipt" DROP CONSTRAINT IF EXISTS "chk_purchase_receipt_inspection_status";"#,
        )
        .await?;
        conn.execute_unprepared(
            r#"
ALTER TABLE "purchase_receipt" DROP COLUMN IF EXISTS "rejudge_count";
ALTER TABLE "purchase_receipt" DROP COLUMN IF EXISTS "rejudge_at";
ALTER TABLE "purchase_receipt" DROP COLUMN IF EXISTS "rejudge_by";
ALTER TABLE "purchase_receipt" DROP COLUMN IF EXISTS "rejudge_reason";
ALTER TABLE "purchase_receipt" DROP COLUMN IF EXISTS "concession_at";
ALTER TABLE "purchase_receipt" DROP COLUMN IF EXISTS "concession_by";
ALTER TABLE "purchase_receipt" DROP COLUMN IF EXISTS "concession_reason";
        "#,
        )
        .await?;

        Ok(())
    }
}
