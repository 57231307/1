//! `pii_reveal_audit`：PII 按需揭示的留痕表（唯一写入方
//! `services/crm/pii_reveal.rs::record_reveal`，由
//! `handlers/crm_customer_handler.rs::reveal_customer_pii` 在每次成功揭示后调用）
//!
//! 语义（列设计逐条对应判据）：
//! - `record_type` + `record_id`：被揭示的记录指向（本波唯一真实载体为客户域，
//!   取值 `customer`；`record_id` 宽度与 `customers.id`（INTEGER）逐字一致，
//!   依据 system 域 m0001 建表与 business 域 m0070 owner 归一后的实测列型）；
//! - `revealed_fields`：被揭示字段 token 集合（`text[]`，取值由应用层白名单
//!   `services/crm/pii_reveal.rs::PII_REVEAL_WHITELIST` 产生）。数组列直接建
//!   `text[]` 而非 TEXT 标量，避免落入"模型 Vec 字段 vs 生效 DDL 标量"的列型
//!   漂移族（本域 m0071 归一的 12 列即该族前科）；
//! - `operator_user_id`：操作人（INTEGER，与 `users.id` 逐字一致），由服务端会话
//!   `AuthContext.user_id` 注入，请求体不得承载身份；
//! - `reason`：本次查看用途（必填由应用层校验，本列 NOT NULL 兜住落库面）；
//! - `revealed_at`：揭示时间戳。
//! 本表**不存任何 PII 原文列**——原文只在揭示响应里瞬时出现，留痕只记
//! "谁、何时、以何用途、揭示了哪条记录的哪些字段"。
//!
//! 为什么 `record_id`/`operator_user_id` 不建外键：审计事实必须比被审计对象
//! 活得久——客户行删除与用户停用删除都不能被 RESTRICT 阻断，也不能被
//! CASCADE 连带抹掉留痕；指向关系由应用层写入口保证（operator 恒为会话用户
//! id，record 恒为通过行级门后的真实客户 id）。
//!
//! 审计行不可被业务端点删除/修改：全仓除本波新增的 `record_reveal` 插入点外，
//! 没有任何 handler/service 以本表为删除或更新目标（契约锁
//! `backend/tests/contract_wave11_pii_reveal_test.rs` 的列形状钉同时防止
//! 后续批次把本表接进业务写口）。
//!
//! 幂等与回滚：up 全程 `IF NOT EXISTS`（建表/建索引/注释），重跑等价，并以
//! RAISE NOTICE 把"本次是新建还是已存在"可见化（禁止静默），建表后关键列
//! 自检缺失即 RAISE EXCEPTION；不回填、不给任何业务表加列。down 对称：
//! `DROP TABLE IF EXISTS`（索引与约束随表消失），仅回收本表，不触碰其它对象。

use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        let sql = r#"
-- 建表前先记录在场状态，供建表后的 NOTICE 区分「本次新建」与「已存在，重跑等价」
DO $$
BEGIN
    IF EXISTS (SELECT 1 FROM information_schema.tables
                WHERE table_schema = current_schema() AND table_name = 'pii_reveal_audit')
    THEN
        RAISE NOTICE 'm0085：pii_reveal_audit 已存在，本次重跑等价（IF NOT EXISTS 不改变任何行）';
    ELSE
        RAISE NOTICE 'm0085：pii_reveal_audit 不存在，本次新建';
    END IF;
END
$$;

CREATE TABLE IF NOT EXISTS "pii_reveal_audit" (
    "id" BIGSERIAL PRIMARY KEY,
    "record_type" VARCHAR(50) NOT NULL,
    "record_id" INTEGER NOT NULL,
    "revealed_fields" TEXT[] NOT NULL,
    "operator_user_id" INTEGER NOT NULL,
    "reason" TEXT NOT NULL,
    "revealed_at" TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP,
    CONSTRAINT "chk_pii_reveal_audit_revealed_fields_nonempty" CHECK (cardinality("revealed_fields") > 0)
);

-- 按记录回读留痕（谁看过这条客户）与按操作人回读（某人看过哪些）两条查询面
CREATE INDEX IF NOT EXISTS "idx_pii_reveal_audit_record"
    ON "pii_reveal_audit" ("record_type", "record_id");
CREATE INDEX IF NOT EXISTS "idx_pii_reveal_audit_operator_time"
    ON "pii_reveal_audit" ("operator_user_id", "revealed_at");

COMMENT ON TABLE "pii_reveal_audit" IS 'PII 按需揭示留痕表：只记指向/字段集合/操作人/用途/时间，不存原文';
COMMENT ON COLUMN "pii_reveal_audit"."record_type" IS '记录类型（本波取值 customer，指向 customers.id）';
COMMENT ON COLUMN "pii_reveal_audit"."record_id" IS '被揭示记录 ID（与 customers.id 同宽 INTEGER，无外键：审计须比业务行存活久）';
COMMENT ON COLUMN "pii_reveal_audit"."revealed_fields" IS '被揭示字段 token 集合（应用层白名单值域，text[]）';
COMMENT ON COLUMN "pii_reveal_audit"."operator_user_id" IS '操作人用户 ID（服务端会话注入，无外键：审计须比用户行存活久）';
COMMENT ON COLUMN "pii_reveal_audit"."reason" IS '本次查看用途（必填）';
COMMENT ON COLUMN "pii_reveal_audit"."revealed_at" IS '揭示时间戳';

-- 建表后关键列自检：缺列即中止迁移链，禁止"表在但形状不对"静默过关
DO $$
BEGIN
    IF NOT (EXISTS (SELECT 1 FROM information_schema.columns
                     WHERE table_schema = current_schema()
                       AND table_name = 'pii_reveal_audit' AND column_name = 'revealed_fields')
          AND EXISTS (SELECT 1 FROM information_schema.columns
                     WHERE table_schema = current_schema()
                       AND table_name = 'pii_reveal_audit' AND column_name = 'operator_user_id')
          AND EXISTS (SELECT 1 FROM information_schema.columns
                     WHERE table_schema = current_schema()
                       AND table_name = 'pii_reveal_audit' AND column_name = 'reason'))
    THEN
        RAISE EXCEPTION 'm0085：pii_reveal_audit 建表后关键列（revealed_fields/operator_user_id/reason）缺失，禁止静默放行';
    END IF;
END
$$;
"#;
        manager.get_connection().execute_unprepared(sql).await?;
        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .get_connection()
            .execute_unprepared("DROP TABLE IF EXISTS \"pii_reveal_audit\";")
            .await?;
        Ok(())
    }
}
