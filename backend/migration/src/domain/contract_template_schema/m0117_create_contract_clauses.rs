//! 合同条款快照建表迁移
//!
//! 功能 创建 `contract_clauses`,签约时把当时代生效的条款正文固化到合同实例,一条款一行,
//! 存固化当时的标题、正文原文、来源模板、模板版本号与条款顺序。
//! 调用方 SeaORM migrator 链,经 `domain/contract_template_schema` 域注册为顶层链尾条目,
//! 域内在模板主表之后执行,引用 `contract_templates.id`。
//! 入参 无,纯 DDL 迁移。
//! 传给谁 合同创建腿在签约动作内按模板条款渲染并写入本表,合同详情腿与打印腿只读本表取约定内容。
//! 存什么 见功能段。
//! 存哪里 PostgreSQL `public.contract_clauses`。
//!
//! 为什么必须有本表
//! 合同实例只存 template_id 是不够的。模板改版会顺着引用把已签合同的约定内容一起改掉,
//! 法务与审计就无法回答“这份合同当时签的是哪一版哪一条”,而这是合同条款区唯一的可采信口径。
//! 所以固化时的正文原文与模板版本号必须落在本表行上,模板后续怎么改都不回溯改写已固化行。
//!
//! 结构取舍
//! - 正文列名 `body_rendered`,存渲染完占位符后的成品文本,NOT NULL。固化动作若还没跑,
//!   合同在本表就没有行,读侧据此判“模板前签约,条款未固化”,不给空串冒充已固化。
//!   两条 CHECK 把这层含义钉死。`chk_cc_body_rendered` 拒空串;`chk_cc_body_fully_rendered`
//!   拒残留 `{{` 起始占位符的文本,即签约腿漏填占位符时数据库当场判红,
//!   不允许把半成品当约定内容存进快照。占位符的白名单可解析性属渲染腿与判定器职责。
//! - `contract_kind` 与 `contract_id` 组合指向两侧合同主表之一。两族合同主表各自的 id 序列
//!   独立自增,单靠 `contract_id` 无法定位是哪一侧,故本表按多态引用建模,不给 `contract_id`
//!   建外键。指向由 `chk_contract_clauses_kind` 限定在 sales 与 purchase 两值内,
//!   引用完整性由合同删除腿与查询腿在服务层保证,建表迁移无法用外键代判。
//! - 与模板条款项之间**不建外键**。建了就等于让快照跟着模板行走,正是本表要消除的追溯改写。
//!   来源可追溯性由 `template_id` 与 `template_version` 两列自证。
//! - 同一合同的同一条款码只能固化一行,重复签约写入必须走更新而非插入第二行。
//!   建索引前按同口径只读探测既有重复,命中即点名中止。

use super::{CLAUSE_CODE_ALLOWED_SQL, CONTRACT_KIND_ALLOWED_SQL};
use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        let create_sql = format!(
            r#"
CREATE TABLE IF NOT EXISTS contract_clauses (
    id SERIAL PRIMARY KEY,
    contract_kind VARCHAR(16) NOT NULL,
    contract_id INTEGER NOT NULL,
    clause_code VARCHAR(64) NOT NULL,
    title VARCHAR(200) NOT NULL,
    body_rendered TEXT NOT NULL,
    template_id INTEGER NOT NULL,
    template_version INTEGER NOT NULL,
    sort SMALLINT NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    CONSTRAINT fk_cc_template FOREIGN KEY (template_id) REFERENCES contract_templates (id),
    CONSTRAINT chk_cc_contract_kind CHECK (contract_kind IN ({contract_kind})),
    CONSTRAINT chk_cc_clause_code CHECK (clause_code IN ({clause_code})),
    CONSTRAINT chk_cc_body_rendered CHECK (body_rendered <> ''),
    CONSTRAINT chk_cc_sort CHECK (sort BETWEEN 1 AND 999),
    CONSTRAINT chk_cc_template_version CHECK (template_version >= 1)
);
COMMENT ON TABLE "contract_clauses" IS '合同条款快照，签约时固化的条款正文原文与模板版本号，模板改版不回溯改写本表已固化行';
COMMENT ON COLUMN "contract_clauses"."contract_kind" IS '合同侧别，sales 指向 sales_contracts，purchase 指向 purchase_contracts';
COMMENT ON COLUMN "contract_clauses"."contract_id" IS '所属合同主键，与 contract_kind 组合定位，两族 id 各自独立故不建外键';
COMMENT ON COLUMN "contract_clauses"."body_rendered" IS '固化当时的条款正文成品文本，占位符已填，本列是约定内容的唯一真相源';
COMMENT ON COLUMN "contract_clauses"."template_version" IS '固化当时的模板版本号，与 template_id 一起回答当时签的是哪一版';
COMMENT ON COLUMN "contract_clauses"."sort" IS '固化当时的条款顺序，模板后续调序不回溯改写本列';
CREATE INDEX IF NOT EXISTS idx_contract_clauses_contract
    ON contract_clauses (contract_kind, contract_id);
CREATE INDEX IF NOT EXISTS idx_contract_clauses_template
    ON contract_clauses (template_id);
"#,
            contract_kind = CONTRACT_KIND_ALLOWED_SQL,
            clause_code = CLAUSE_CODE_ALLOWED_SQL,
        );
        manager
            .get_connection()
            .execute_unprepared(&create_sql)
            .await?;

        // 每合同每条款一行唯一索引前的只读探测。分组口径与索引列集合一致,
        // 命中重复即点名中止,不在迁移里删行或改条款码。
        let probe_sql = r#"
DO $$
DECLARE
    dup_group_count INTEGER;
    dup_samples     TEXT;
BEGIN
    SELECT COUNT(*) INTO dup_group_count
      FROM (
        SELECT contract_kind, contract_id, clause_code
          FROM contract_clauses
         GROUP BY contract_kind, contract_id, clause_code
        HAVING COUNT(*) > 1
      ) d;
    IF dup_group_count > 0 THEN
        SELECT string_agg(group_key, ', ' ORDER BY group_key) INTO dup_samples
          FROM (
            SELECT contract_kind || '/' || contract_id || '/' || clause_code AS group_key
              FROM contract_clauses
             GROUP BY contract_kind, contract_id, clause_code
            HAVING COUNT(*) > 1
            LIMIT 10
          ) s;
        RAISE EXCEPTION 'contract_clauses 存在 % 组同合同同条款码的并行快照行（样例 侧别/合同主键/条款码 %）。同一合同的同一条款只能固化一行，本迁移拒绝在带存量重复的表上建唯一索引，且不做静默保留其一。请人工核实哪一行是当时的签约文本，其余删除后重跑迁移。',
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
CREATE UNIQUE INDEX IF NOT EXISTS uq_contract_clauses_contract_code
    ON contract_clauses (contract_kind, contract_id, clause_code);
ALTER TABLE "contract_clauses" DROP CONSTRAINT IF EXISTS "chk_cc_body_fully_rendered";
ALTER TABLE "contract_clauses" ADD CONSTRAINT "chk_cc_body_fully_rendered"
    CHECK ("body_rendered" NOT LIKE '%{{%');
"#;
        manager
            .get_connection()
            .execute_unprepared(index_sql)
            .await?;
        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        let sql = r#"
DROP INDEX IF EXISTS uq_contract_clauses_contract_code;
DROP INDEX IF EXISTS idx_contract_clauses_template;
DROP INDEX IF EXISTS idx_contract_clauses_contract;
DROP TABLE IF EXISTS contract_clauses;
"#;
        manager.get_connection().execute_unprepared(sql).await?;
        Ok(())
    }
}
