//! CRM 状态词表 DB CHECK 约束迁移
//!
//! 将 `models/status::crm_lead` / `models/status::crm_opportunity` 的权威词表钉入数据库，
//! 阻止任何绕过服务层校验的写入产生脏值（与入参校验同一字典，避免两处维护）：
//! - `crm_lead.lead_status`          VARCHAR(20) NULL：new/contacted/qualified/assigned/converted/pool/lost
//!   （取值逐项等于 `models/status::crm_lead::ALL`）
//! - `crm_opportunity.opportunity_stage`  VARCHAR(50) NULL：QUALIFICATION/NEEDS_ANALYSIS/PROPOSAL/NEGOTIATION/CLOSED_WON/CLOSED_LOST
//!   （逐项等于 `models/status::crm_opportunity::ALL_STAGES`）
//! - `crm_opportunity.opportunity_status` VARCHAR(20) NULL：OPEN/CLOSED_WON/CLOSED_LOST
//!   （逐项等于 `models/status::crm_opportunity::ALL_STATUSES`；无 draft/cancelled/pool，均为幽灵值）
//!
//! 列可空性与宽度保持不变（词表最长值 qualified(9)/CLOSED_LOST(10)/NEEDS_ANALYSIS(14)
//! 均在原 VARCHAR(20)/VARCHAR(50) 宽度内），CHECK 采用 `col IS NULL OR col IN (...)` 保留 NULL。
//!
//! 加约束前先回填 order_crud 赢单回写码 bug（services/so/order_crud.rs 原写小写
//! `closed_won`/`won`，CRM 统计与门控按 `CLOSED_WON` 比对而漏计）产生的已知存量脏码，
//! 使历史赢单商机重新计入；该回写属本词表规范码的数据侧修复，不做任何猜测性归一。
//!
//! 幂等：先 DROP CONSTRAINT IF EXISTS 再 ADD，可安全重跑；CHECK 不涉及 FK 类型，
//! 不受 m0044 fix_fk_types（BIGINT→INT 改写）影响。建表位于 business 域 m0013，
//! 本迁移排在全部域迁移之后，满足依赖顺序。

use sea_orm_migration::prelude::*;

pub struct Migration;

impl MigrationName for Migration {
    fn name(&self) -> &'static str {
        "m_crm_vocab_check"
    }
}

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        let sql = r#"
-- 1) 回填赢单回写码历史脏值（order_crud 写小写 closed_won/won，权威码为 CLOSED_WON）
UPDATE "crm_opportunity" SET "opportunity_status" = 'CLOSED_WON' WHERE "opportunity_status" = 'won';
UPDATE "crm_opportunity" SET "opportunity_stage" = 'CLOSED_WON' WHERE "opportunity_stage" = 'closed_won';

-- 2) crm_lead.lead_status（词表来源 models/status::crm_lead::ALL，小写）
ALTER TABLE "crm_lead" DROP CONSTRAINT IF EXISTS "chk_crm_lead_lead_status";
ALTER TABLE "crm_lead" ADD CONSTRAINT "chk_crm_lead_lead_status" CHECK (
  "lead_status" IS NULL OR "lead_status" IN ('new','contacted','qualified','assigned','converted','pool','lost')
);

-- 3) crm_opportunity.opportunity_stage（词表来源 models/status::crm_opportunity::ALL_STAGES，大写）
ALTER TABLE "crm_opportunity" DROP CONSTRAINT IF EXISTS "chk_crm_opportunity_stage";
ALTER TABLE "crm_opportunity" ADD CONSTRAINT "chk_crm_opportunity_stage" CHECK (
  "opportunity_stage" IS NULL OR "opportunity_stage" IN ('QUALIFICATION','NEEDS_ANALYSIS','PROPOSAL','NEGOTIATION','CLOSED_WON','CLOSED_LOST')
);

-- 4) crm_opportunity.opportunity_status（词表来源 models/status::crm_opportunity::ALL_STATUSES，大写）
ALTER TABLE "crm_opportunity" DROP CONSTRAINT IF EXISTS "chk_crm_opportunity_status";
ALTER TABLE "crm_opportunity" ADD CONSTRAINT "chk_crm_opportunity_status" CHECK (
  "opportunity_status" IS NULL OR "opportunity_status" IN ('OPEN','CLOSED_WON','CLOSED_LOST')
);
"#;
        manager.get_connection().execute_unprepared(sql).await?;
        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        // 回滚仅移除约束；回填为 CLOSED_WON 的规范码不回退（小写脏码属错误数据，无恢复价值）
        let sql = r#"
ALTER TABLE "crm_lead" DROP CONSTRAINT IF EXISTS "chk_crm_lead_lead_status";
ALTER TABLE "crm_opportunity" DROP CONSTRAINT IF EXISTS "chk_crm_opportunity_stage";
ALTER TABLE "crm_opportunity" DROP CONSTRAINT IF EXISTS "chk_crm_opportunity_status";
"#;
        manager.get_connection().execute_unprepared(sql).await?;
        Ok(())
    }
}
