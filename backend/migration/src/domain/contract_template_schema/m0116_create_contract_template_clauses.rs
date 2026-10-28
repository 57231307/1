//! 模板条款项建表迁移
//!
//! 功能 创建 `contract_template_clauses`,存放一枚模板下的条款项,含条款库引用、模板内正文、
//! 顺序、必填标记、条件出条款标记与逐条法务复核留痕。
//! 调用方 SeaORM migrator 链,经 `domain/contract_template_schema` 域注册为顶层链尾条目,
//! 域内在模板主表与条款库之后执行,是本域两张被引用表的引用方。
//! 入参 无,纯 DDL 迁移。
//! 传给谁 条款项行由后端模板读写腿写入,签约时由合同创建腿把当时代生效的正文固化进合同条款快照。
//! 存什么 见功能段。
//! 存哪里 PostgreSQL `public.contract_template_clauses`。
//!
//! 结构取舍
//! - 正文来源二选一,由表级 CHECK `chk_ctc_text_source` 钉住。`library_clause_id` 与 `body`
//!   恰好一个非空,两者都空则条款无正文,两者都给则同一条款有两个真相源、必然漂移。
//!   库文与内文都不允许是空串。
//! - 同模板同条款码同适用条件的位置唯一,由部分表达式索引钉住,谓词把空适用条件折叠成空串,
//!   与只读探测的分组口径逐字符同形。同一模板内同一条款码可并存不同适用条件的变体,
//!   但不得并存同条件的两行。
//! - 法务复核状态非 pending 时必须有复核人,把“法务复核后才能置 active”落到逐条粒度。
//! - 外键 `template_id` 用默认 RESTRICT,不级联删。已签约合同引用过的模板被删会让快照行断链。
//! - 条款码、适用条件、复核状态三列的 CHECK 取值集与域内词表常量单点同源。

use super::{APPLIES_WHEN_ALLOWED_SQL, CLAUSE_CODE_ALLOWED_SQL, LEGAL_REVIEW_STATUS_ALLOWED_SQL};
use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        let create_sql = format!(
            r#"
CREATE TABLE IF NOT EXISTS contract_template_clauses (
    id SERIAL PRIMARY KEY,
    template_id INTEGER NOT NULL,
    clause_code VARCHAR(64) NOT NULL,
    title VARCHAR(200) NOT NULL,
    body TEXT,
    library_clause_id INTEGER,
    required BOOLEAN NOT NULL DEFAULT false,
    sort SMALLINT NOT NULL,
    applies_when VARCHAR(32),
    legal_review_status VARCHAR(16) NOT NULL DEFAULT 'pending',
    legal_reviewed_by INTEGER,
    legal_reviewed_at TIMESTAMPTZ,
    legal_review_remark TEXT,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    CONSTRAINT fk_ctc_template FOREIGN KEY (template_id) REFERENCES contract_templates (id),
    CONSTRAINT fk_ctc_library FOREIGN KEY (library_clause_id) REFERENCES clause_library (id),
    CONSTRAINT chk_ctc_clause_code CHECK (clause_code IN ({clause_code})),
    CONSTRAINT chk_ctc_applies_when CHECK (applies_when IS NULL OR applies_when IN ({applies_when})),
    CONSTRAINT chk_ctc_legal_review_status CHECK (legal_review_status IN ({review_status})),
    CONSTRAINT chk_ctc_text_source CHECK (
        (library_clause_id IS NOT NULL)::integer + (body IS NOT NULL)::integer = 1
        AND (body IS NULL OR body <> '')
    ),
    CONSTRAINT chk_ctc_sort CHECK (sort BETWEEN 1 AND 999),
    CONSTRAINT chk_ctc_review_pair CHECK (legal_review_status = 'pending' OR legal_reviewed_by IS NOT NULL)
);
COMMENT ON TABLE "contract_template_clauses" IS '模板下的条款项，条款库引用与模板内正文二选一，带顺序与必填标记与逐条法务复核';
COMMENT ON COLUMN "contract_template_clauses"."library_clause_id" IS '引用的条款库行，与 body 恰好一个非空';
COMMENT ON COLUMN "contract_template_clauses"."body" IS '模板内自带正文，用于不复用条款库的定制条款，与 library_clause_id 恰好一个非空';
COMMENT ON COLUMN "contract_template_clauses"."required" IS '必填标记，置真的条款在合同录入面不可删';
COMMENT ON COLUMN "contract_template_clauses"."sort" IS '条款顺序，签约固化时随正文一起写进合同条款快照';
COMMENT ON COLUMN "contract_template_clauses"."applies_when" IS '条件出条款标记，空值表示通用条款，非空值只在命中该业务形态时出条款';
COMMENT ON COLUMN "contract_template_clauses"."legal_review_status" IS '逐条法务复核状态，非 pending 时必须有复核人留痕';
"#,
            clause_code = CLAUSE_CODE_ALLOWED_SQL,
            applies_when = APPLIES_WHEN_ALLOWED_SQL,
            review_status = LEGAL_REVIEW_STATUS_ALLOWED_SQL,
        );
        manager
            .get_connection()
            .execute_unprepared(&create_sql)
            .await?;

        // 位置唯一索引前的只读探测。分组口径与索引表达式的 COALESCE 完全一致,
        // 折叠空适用条件后才比分组,命中重复即点名中止,不静默保留其中一行。
        let probe_sql = r#"
DO $$
DECLARE
    dup_group_count INTEGER;
    dup_samples     TEXT;
BEGIN
    SELECT COUNT(*) INTO dup_group_count
      FROM (
        SELECT template_id, clause_code, COALESCE(applies_when, '') AS when_key
          FROM contract_template_clauses
         GROUP BY template_id, clause_code, COALESCE(applies_when, '')
        HAVING COUNT(*) > 1
      ) d;
    IF dup_group_count > 0 THEN
        SELECT string_agg(group_key, ', ' ORDER BY group_key) INTO dup_samples
          FROM (
            SELECT template_id || '/' || clause_code || '/' || COALESCE(applies_when, '未填') AS group_key
              FROM contract_template_clauses
             GROUP BY template_id, clause_code, COALESCE(applies_when, '')
            HAVING COUNT(*) > 1
            LIMIT 10
          ) s;
        RAISE EXCEPTION 'contract_template_clauses 存在 % 组同模板同条款码同适用条件的并行条款行（样例 模板/条款码/适用条件 %）。本迁移拒绝在带存量重复的表上建位置唯一索引，且不做静默保留其一或改序洗数据。请人工核实哪一行是该模板当时的条款行，其余删除或改码，处置后重跑迁移。',
            dup_group_count, dup_samples;
    END IF;
END
$$;
"#;
        manager
            .get_connection()
            .execute_unprepared(probe_sql)
            .await?;

        let index_sql = r#"
CREATE UNIQUE INDEX IF NOT EXISTS uq_ctc_template_clause_position
    ON contract_template_clauses (template_id, clause_code, COALESCE(applies_when, ''));
CREATE INDEX IF NOT EXISTS idx_ctc_template_sort
    ON contract_template_clauses (template_id, sort);
CREATE INDEX IF NOT EXISTS idx_ctc_library
    ON contract_template_clauses (library_clause_id);
"#;
        manager
            .get_connection()
            .execute_unprepared(index_sql)
            .await?;
        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        let sql = r#"
DROP INDEX IF EXISTS idx_ctc_library;
DROP INDEX IF EXISTS idx_ctc_template_sort;
DROP INDEX IF EXISTS uq_ctc_template_clause_position;
DROP TABLE IF EXISTS contract_template_clauses;
"#;
        manager.get_connection().execute_unprepared(sql).await?;
        Ok(())
    }
}
