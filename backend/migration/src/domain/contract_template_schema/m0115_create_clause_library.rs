//! 条款库主数据建表迁移
//!
//! 功能 创建 `clause_library`,一枚条款码对应一枚可复用条款正文,供多枚模板引用同一枚文本,
//! 改一处多处生效,而生效只影响新签合同。
//! 调用方 SeaORM migrator 链,经 `domain/contract_template_schema` 域注册为顶层链尾条目,
//! 域内先于模板条款项执行,供 `contract_template_clauses.library_clause_id` 外键引用。
//! 入参 无,纯 DDL 迁移。
//! 传给谁 建表内不写任何种子行,条款正文由模板内容域按实体映射写入。
//! 存什么 条款码、标题、正文、适用合同类别、生命周期状态、备注与创建人。
//! 存哪里 PostgreSQL `public.clause_library`。
//!
//! 结构取舍
//! - `clause_code` 全表唯一,同码并存多行会让“改一处多处生效”退化成改哪一处都无据可依，
//!   因此不建 (clause_code, version) 复合形态。改版历史由合同条款快照存的固化原文承担。
//! - `applies_contract_type` 可空。空值表示通用条款,非空值表示只服务某一类合同的专章条款,
//!   取值集与模板类别词表同源。
//! - `status` 与模板共用同一生命周期词表,库里未启用的条款不得被模板引用为启用条款,
//!   该引用一致性属服务层职责,本表只钉取值集。
//! - 条款码 CHECK 是闭合集,取值集与域内词表常量单点同源。
//! - `title` 与 `body` 非空。正文允许为空串的条款行是假可供性,库里不给它落地的通道。

use super::{CLAUSE_CODE_ALLOWED_SQL, CONTRACT_TYPE_ALLOWED_SQL, TEMPLATE_STATUS_ALLOWED_SQL};
use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        let sql = format!(
            r#"
CREATE TABLE IF NOT EXISTS clause_library (
    id SERIAL PRIMARY KEY,
    clause_code VARCHAR(64) NOT NULL,
    title VARCHAR(200) NOT NULL,
    body TEXT NOT NULL,
    applies_contract_type VARCHAR(32),
    status VARCHAR(16) NOT NULL DEFAULT 'draft',
    remark TEXT,
    created_by INTEGER NOT NULL DEFAULT 0,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    CONSTRAINT uq_clause_library_code UNIQUE (clause_code),
    CONSTRAINT chk_clause_library_code CHECK (clause_code IN ({clause_code})),
    CONSTRAINT chk_clause_library_applies_type CHECK (applies_contract_type IS NULL OR applies_contract_type IN ({contract_type})),
    CONSTRAINT chk_clause_library_status CHECK (status IN ({status})),
    CONSTRAINT chk_clause_library_body CHECK (body <> '')
);
COMMENT ON TABLE "clause_library" IS '条款库主数据，一枚条款码一枚可复用正文，多模板引用同一行';
COMMENT ON COLUMN "clause_library"."clause_code" IS '条款码，全表唯一，取值集与必备条款骨架加专章条款的闭合集同源';
COMMENT ON COLUMN "clause_library"."applies_contract_type" IS '专章条款适用的合同类别，空值表示通用条款';
COMMENT ON COLUMN "clause_library"."body" IS '条款正文，含占位符，由模板内容域写入，本迁移不播种';
COMMENT ON COLUMN "clause_library"."status" IS '条款生命周期，与模板同一套词表，draft 起步经法务复核置 active';
CREATE INDEX IF NOT EXISTS idx_clause_library_applies_type
    ON clause_library (applies_contract_type);
"#,
            clause_code = CLAUSE_CODE_ALLOWED_SQL,
            contract_type = CONTRACT_TYPE_ALLOWED_SQL,
            status = TEMPLATE_STATUS_ALLOWED_SQL,
        );
        manager.get_connection().execute_unprepared(&sql).await?;
        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        let sql = r#"
DROP INDEX IF EXISTS idx_clause_library_applies_type;
DROP TABLE IF EXISTS clause_library;
"#;
        manager.get_connection().execute_unprepared(sql).await?;
        Ok(())
    }
}
