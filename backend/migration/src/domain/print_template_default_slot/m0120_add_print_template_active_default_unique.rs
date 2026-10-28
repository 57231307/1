//! print_templates 默认位库侧唯一门与列注释更正（单支顶层域内迁移）
//!
//! ## 六要素
//! - 功能：给"同一单据类型至多一行启用且默认"的打印模板不变量加数据库侧兜底，
//!   并更正 is_default 列注释，使注释如实描述互斥域是单据类型 doc_type 而非 module。
//! - 调用方：SeaORM migrator 链，经 domain/print_template_default_slot 域注册为
//!   lib.rs 与 domain/mod.rs 链尾的顶层台账条目。print_templates 表由 print_subsystem
//!   域的 m0098 建表，print_subsystem 早于本域执行，表必已存在。
//! - 入参：无（纯约束与注释迁移，不读取应用之外的数据，不写任何业务行）。
//! - 传给谁：出件解析侧 print_task_handler 的 load_active_default_template 与该链路
//!   同口径互斥（解析按 doc_type 加 status 启用加 is_default 真三条件过滤、按 id 升序
//!   取首行）；写入侧 print_handler 的建模板、改模板、设默认三处竞位清理同样按这三条
//!   件降位，本索引是其数据库侧兜底，堵住先查后插竞态落出第二行的窗口。
//! - 存什么：pg_class 中的部分唯一索引 uq_print_template_active_default 与
//!   pg_description 中的列注释新文本；不改写任何数据行，不改列类型与既有约束。
//! - 存哪里：PostgreSQL public.print_templates 表的索引与 is_default 列注释。
//!
//! ## 谓词同源（逐字符对齐，禁止单边放宽）
//! 索引键与谓词三条件的分工如下，与解析侧过滤一一对应：
//! - doc_type 为索引键，对应解析的 DocType 等值过滤（渲染链路查找键）；
//! - is_default 真，对应解析的 IsDefault.eq(true)；
//! - status 为词表启用 token，对应解析的 Status.eq(print_template_status::ACTIVE)，
//!   词表 ACTIVE 的字面值 active 与谓词中的 status 取值逐字符一致（见
//!   models/status/print 的 print_template_status 定义与建表 CHECK 取值集）。
//! 索引谓词文本 WHERE is_default AND status='active' 在本文件恰好出现三处
//! （存量探测统计段、存量探测样例段、建索引段），三处同源一体改动，
//! 任何一处被简化（如只剩 WHERE is_default）都会打红本文件配套契约锁。
//!
//! ## 存量违例处置（fail-visible 中止，禁止洗数据）
//! 建索引前先做只读探测：按谓词三条件聚合，任一 doc_type 出现不止一行启用默认即
//! RAISE EXCEPTION 点名中止（消息含违例单据类型个数、最多 10 组样例的单据类型、
//! 行数与最小 id），本迁移不加索引、不 UPDATE、不 DELETE、不改名造值圆场，照
//! m0068 探测段、m0082 探测段与 status_vocab_check 拒绝段的同族范式。人工处置口径
//! 与出件解析同源，解析按 id 升序取首行，故保留每个 doc_type id 最小的一行，其余行
//! 由业务侧显式降位或停用，处置完成后重跑本迁移。
//!
//! ## 幂等与回退
//! up 为先只读探测、再 CREATE UNIQUE INDEX IF NOT EXISTS、后 COMMENT 更新，重跑等价；
//! 探测命中违例即中止且幂等性不受影响（有索引在场时探测必然零命中，重跑安全）。
//! down 对称：DROP INDEX IF EXISTS 并还原 m0098 的列注释原文，不触碰任何数据行，
//! 回滚后回到"仅应用层竞位清理"的现状。

use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        // 三段一条语句流：探测中止在前，建索引在中，列注释更正在后。
        // 探测段与建索引段的谓词文本逐字符一致，样例段带最小 id 供人工按口径处置。
        let sql = r#"
DO $$
DECLARE
    dup_type_count INTEGER;
    dup_samples TEXT;
BEGIN
    SELECT COUNT(*) INTO dup_type_count
      FROM (
        SELECT "doc_type"
          FROM "print_templates"
         WHERE is_default AND status='active'
         GROUP BY "doc_type"
        HAVING COUNT(*) > 1
      ) d;
    IF dup_type_count > 0 THEN
        SELECT string_agg(sample, ', ' ORDER BY sample) INTO dup_samples
          FROM (
            SELECT "doc_type" || '（启用默认 ' || COUNT(*)::text || ' 行，最小 id ' || MIN("id")::text || '）' AS sample
              FROM "print_templates"
             WHERE is_default AND status='active'
             GROUP BY "doc_type"
            HAVING COUNT(*) > 1
            LIMIT 10
          ) s;
        RAISE EXCEPTION '表 print_templates 存在 % 个单据类型有不止一行启用且默认的模板（样例 %），本迁移拒绝在带存量违例的表上建部分唯一索引 uq_print_template_active_default，且不做 UPDATE 或 DELETE 洗数据。',
            dup_type_count, dup_samples
            USING HINT = '人工处置口径与出件解析同源（按 id 升序取首行），保留每个单据类型 id 最小的一行，其余行由业务侧显式降位（is_default 置 false）或停用（status 置 inactive），处置完成后重跑本迁移。';
    END IF;
END
$$;
CREATE UNIQUE INDEX IF NOT EXISTS uq_print_template_active_default ON print_templates(doc_type) WHERE is_default AND status='active';
COMMENT ON COLUMN "print_templates"."is_default" IS '是否缺省模板。互斥域是单据类型 doc_type（同一 doc_type 下启用且默认的行至多一行，由部分唯一索引 uq_print_template_active_default 在库侧兜底）。停用行的默认位仅留存，不参与按单据类型的自动解析。自动解析按 id 升序取首行。';
"#;
        manager.get_connection().execute_unprepared(sql).await?;
        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        // 对称回滚：只撤索引并还原建表迁移的列注释原文，不触碰任何数据行。
        let sql = r#"
DROP INDEX IF EXISTS uq_print_template_active_default;
COMMENT ON COLUMN "print_templates"."is_default" IS '同模块内是否标记为缺省模板';
"#;
        manager.get_connection().execute_unprepared(sql).await?;
        Ok(())
    }
}
