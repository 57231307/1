//! print_templates 表建表迁移
//!
//! ## 六要素
//! - **功能**：创建打印模板主数据表 `print_templates`,存储单据类型到打印模板的映射、
//!   纸张/方向/份数/四边边距/条码等打印缺省参数,以及模板正文与变量。
//! - **调用方**：SeaORM migrator 链,经 `domain/print_subsystem` 域注册为顶层链尾条目;
//!   `up` 内先于 `print_tasks` 执行,供 `print_tasks.template_id` 外键引用。
//! - **入参**：无(纯 DDL 迁移)。
//! - **传给谁**：无应用层直调;表由打印子系统的模板管理读写路径按实体映射访问。
//! - **存什么**：模板编码、名称、所属模块、单据类型、纸张/方向、缺省份数与四边边距、
//!   是否缺省模板与状态,以及 HTML 正文 / CSS 样式 / 变量集合。
//! - **存哪里**：PostgreSQL `public.print_templates`。
//!
//! ## 形态说明
//! - `template_code` 为业务唯一编码,建 UNIQUE;`doc_type` 承载“单据类型→模板”的映射查找,
//!   按该列的检索路径建单列索引。
//! - `content` / `css_styles` / `variables` 三列在写入方恒给值(正文与样式缺省空串、变量缺省
//!   空对象),故落 `NOT NULL` + DEFAULT,与实体侧非 Option 字段一致。
//! - 方向、份数、四边边距、状态各以具名 CHECK 闭合取值,约束取值集与打印子系统状态词表同源。
//! - `CREATE TABLE IF NOT EXISTS` + 具名约束:全新库直接建表,复用库上本域作为链尾新顶层条目
//!   必然执行,幂等可重放;不在本迁移内写入任何种子行(缺省模板由应用启动阶段落库)。

use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        let sql = r#"
CREATE TABLE IF NOT EXISTS print_templates (
    id SERIAL PRIMARY KEY,
    template_code VARCHAR(100) NOT NULL,
    template_name VARCHAR(200) NOT NULL,
    description TEXT NOT NULL DEFAULT '',
    module VARCHAR(50) NOT NULL,
    template_type VARCHAR(50) NOT NULL DEFAULT 'order',
    doc_type VARCHAR(100) NOT NULL,
    paper_size VARCHAR(10) NOT NULL DEFAULT 'A4',
    orientation VARCHAR(10) NOT NULL DEFAULT 'portrait',
    copies_default SMALLINT NOT NULL DEFAULT 1,
    margin_top_mm SMALLINT NOT NULL DEFAULT 15,
    margin_bottom_mm SMALLINT NOT NULL DEFAULT 15,
    margin_left_mm SMALLINT NOT NULL DEFAULT 15,
    margin_right_mm SMALLINT NOT NULL DEFAULT 15,
    barcode_default BOOLEAN NOT NULL DEFAULT true,
    default_target_name VARCHAR(100),
    content TEXT NOT NULL DEFAULT '',
    css_styles TEXT NOT NULL DEFAULT '',
    variables JSONB NOT NULL DEFAULT '{}'::jsonb,
    is_default BOOLEAN NOT NULL DEFAULT false,
    status VARCHAR(20) NOT NULL DEFAULT 'active',
    created_by INTEGER NOT NULL DEFAULT 0,
    created_by_name VARCHAR(100) NOT NULL DEFAULT '',
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    CONSTRAINT uq_print_templates_code UNIQUE (template_code),
    CONSTRAINT chk_print_templates_orientation CHECK (orientation IN ('portrait','landscape')),
    CONSTRAINT chk_print_templates_copies_default CHECK (copies_default BETWEEN 1 AND 99),
    CONSTRAINT chk_print_templates_margins CHECK (
        margin_top_mm BETWEEN 0 AND 50
        AND margin_bottom_mm BETWEEN 0 AND 50
        AND margin_left_mm BETWEEN 0 AND 50
        AND margin_right_mm BETWEEN 0 AND 50
    ),
    CONSTRAINT chk_print_templates_status CHECK (status IN ('active','inactive'))
);
COMMENT ON TABLE "print_templates" IS '打印模板主数据:承载单据类型到模板的映射与打印缺省参数';
COMMENT ON COLUMN "print_templates"."template_type" IS '模板归类(order/report 等),序列化时映射为前端 type 键';
COMMENT ON COLUMN "print_templates"."doc_type" IS '关联单据类型标识,是“单据→模板”映射的查找键';
COMMENT ON COLUMN "print_templates"."created_by_name" IS '创建人用户名,用于模板列表直接展示';
COMMENT ON COLUMN "print_templates"."module" IS '模板归属的业务模块(如 sales/production/finance/logistics)';
COMMENT ON COLUMN "print_templates"."copies_default" IS '发起打印时的缺省份数';
COMMENT ON COLUMN "print_templates"."margin_top_mm" IS '缺省上边距(毫米)';
COMMENT ON COLUMN "print_templates"."margin_bottom_mm" IS '缺省下边距(毫米)';
COMMENT ON COLUMN "print_templates"."margin_left_mm" IS '缺省左边距(毫米)';
COMMENT ON COLUMN "print_templates"."margin_right_mm" IS '缺省右边距(毫米)';
COMMENT ON COLUMN "print_templates"."barcode_default" IS '是否缺省在打印任务中启用车间/单据条码';
COMMENT ON COLUMN "print_templates"."default_target_name" IS '缺省打印目标的符号名(实际目标地址由运行期配置解析)';
COMMENT ON COLUMN "print_templates"."is_default" IS '同模块内是否标记为缺省模板';
CREATE INDEX IF NOT EXISTS idx_print_templates_doc_type ON print_templates (doc_type);
"#;
        manager.get_connection().execute_unprepared(sql).await?;
        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        let sql = r#"
DROP INDEX IF EXISTS idx_print_templates_doc_type;
DROP TABLE IF EXISTS print_templates;
"#;
        manager.get_connection().execute_unprepared(sql).await?;
        Ok(())
    }
}
