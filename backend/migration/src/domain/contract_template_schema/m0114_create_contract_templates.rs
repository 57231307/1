//! 合同模板主表建表迁移
//!
//! 功能 创建 `contract_templates`,存放合同模板的类别、我方立场、版本号、标题、生命周期状态、
//! 生效日与创建与法务复核留痕,是模板条款项与合同条款快照的被引用主表。
//! 调用方 SeaORM migrator 链,经 `domain/contract_template_schema` 域注册为顶层链尾条目,
//! 域内先于模板条款项与合同条款快照执行,供两表的外键引用。
//! 入参 无,纯 DDL 迁移。
//! 传给谁 建表内不写任何种子行,模板行由后端模板读写腿按实体映射访问,条款正文由模板内容域写入。
//! 存什么 见功能段。
//! 存哪里 PostgreSQL `public.contract_templates`。
//!
//! 结构取舍
//! - 业务主键是 (template_code, version) 组合唯一,同一枚编码可以并存多个版本行,
//!   只把 template_code 钉 UNIQUE 会让改版无处可落。
//! - 同一 (contract_type, party_side) 只允许一行为 active,由部分唯一索引
//!   `uq_contract_templates_active_type_side` 钉住,沿用本仓 ACTIVE 门既有范式。
//!   建索引前先按同口径谓词只读探测既有重复,命中即点名中止,不静默去重。
//! - 状态置 active 必须有 approved_by,由表级 CHECK 兜住,防止绕过服务层审批腿直写出无复核的启用模板。
//! - 类别、立场、状态三列的 CHECK 取值集与域内词表常量单点同源,不在本文件内联第二套清单。

use super::{CONTRACT_TYPE_ALLOWED_SQL, PARTY_SIDE_ALLOWED_SQL, TEMPLATE_STATUS_ALLOWED_SQL};
use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        let create_sql = format!(
            r#"
CREATE TABLE IF NOT EXISTS contract_templates (
    id SERIAL PRIMARY KEY,
    template_code VARCHAR(64) NOT NULL,
    contract_type VARCHAR(32) NOT NULL,
    party_side VARCHAR(16) NOT NULL,
    version INTEGER NOT NULL DEFAULT 1,
    title VARCHAR(200) NOT NULL,
    status VARCHAR(16) NOT NULL DEFAULT 'draft',
    effective_from DATE,
    created_by INTEGER NOT NULL DEFAULT 0,
    approved_by INTEGER,
    approved_at TIMESTAMPTZ,
    remark TEXT,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    CONSTRAINT uq_contract_templates_code_version UNIQUE (template_code, version),
    CONSTRAINT chk_contract_templates_contract_type CHECK (contract_type IN ({contract_type})),
    CONSTRAINT chk_contract_templates_party_side CHECK (party_side IN ({party_side})),
    CONSTRAINT chk_contract_templates_status CHECK (status IN ({status})),
    CONSTRAINT chk_contract_templates_version CHECK (version >= 1),
    CONSTRAINT chk_contract_templates_active_approved CHECK (status <> 'active' OR approved_by IS NOT NULL)
);
COMMENT ON TABLE "contract_templates" IS '合同模板主数据，承载模板版本与同一类别同一立场只允许一版启用的语义';
COMMENT ON COLUMN "contract_templates"."template_code" IS '模板业务编码，与 version 组合唯一，改版沿用同码递增版本';
COMMENT ON COLUMN "contract_templates"."contract_type" IS '合同类别，取值集与模板清单十类同源';
COMMENT ON COLUMN "contract_templates"."party_side" IS '我方立场，卖方或买方决定条款倾向，对等义务族取 both';
COMMENT ON COLUMN "contract_templates"."version" IS '模板版本号，签约固化时随条款正文一起写进合同条款快照';
COMMENT ON COLUMN "contract_templates"."status" IS '生命周期，draft 起步，法务复核后置 active，被新版取代后置 retired';
COMMENT ON COLUMN "contract_templates"."approved_by" IS '法务复核通过人，status 为 active 时不得为空';
COMMENT ON COLUMN "contract_templates"."effective_from" IS '模板生效日，空值表示尚未定档生效';
"#,
            contract_type = CONTRACT_TYPE_ALLOWED_SQL,
            party_side = PARTY_SIDE_ALLOWED_SQL,
            status = TEMPLATE_STATUS_ALLOWED_SQL,
        );
        manager
            .get_connection()
            .execute_unprepared(&create_sql)
            .await?;

        // 部分唯一索引前的只读探测。谓词与索引的 WHERE 完全同口径，命中重复即点名中止，
        // 不软删、不去重、不改名，交人工处置后重跑。
        let probe_sql = r#"
DO $$
DECLARE
    dup_group_count INTEGER;
    dup_samples     TEXT;
BEGIN
    SELECT COUNT(*) INTO dup_group_count
      FROM (
        SELECT contract_type, party_side
          FROM contract_templates
         WHERE status = 'active'
         GROUP BY contract_type, party_side
        HAVING COUNT(*) > 1
      ) d;
    IF dup_group_count > 0 THEN
        SELECT string_agg(group_key, ', ' ORDER BY group_key) INTO dup_samples
          FROM (
            SELECT contract_type || '/' || party_side AS group_key
              FROM contract_templates
             WHERE status = 'active'
             GROUP BY contract_type, party_side
            HAVING COUNT(*) > 1
            LIMIT 10
          ) s;
        RAISE EXCEPTION 'contract_templates 存在 % 组同类别同立场的并行启用模板（样例 %）。本迁移拒绝在带存量重复的表上建部分唯一索引，且不做静默去重或停用洗数据。请人工核实哪一版才是当时的启用版，其余置 retired 并在 remark 留痕，处置后重跑迁移。',
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
CREATE UNIQUE INDEX IF NOT EXISTS uq_contract_templates_active_type_side
    ON contract_templates (contract_type, party_side) WHERE status = 'active';
CREATE INDEX IF NOT EXISTS idx_contract_templates_type_side
    ON contract_templates (contract_type, party_side);
CREATE INDEX IF NOT EXISTS idx_contract_templates_status
    ON contract_templates (status);
"#;
        manager
            .get_connection()
            .execute_unprepared(index_sql)
            .await?;
        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        // 回滚只删本域建的索引与表本身，不触碰任何数据行的业务取值。
        let sql = r#"
DROP INDEX IF EXISTS idx_contract_templates_status;
DROP INDEX IF EXISTS idx_contract_templates_type_side;
DROP INDEX IF EXISTS uq_contract_templates_active_type_side;
DROP TABLE IF EXISTS contract_templates;
"#;
        manager.get_connection().execute_unprepared(sql).await?;
        Ok(())
    }
}
