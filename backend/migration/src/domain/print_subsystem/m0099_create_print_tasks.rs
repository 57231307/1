//! print_tasks 表建表迁移
//!
//! ## 六要素
//! - **功能**：创建打印任务台账表 `print_tasks`,记录每次打印请求的发起人、来源单据、目标
//!   设备/池、打印参数快照、后端类型与状态流转结果。
//! - **调用方**：SeaORM migrator 链,经 `domain/print_subsystem` 域注册为顶层链尾条目;
//!   `up` 内后于 `print_templates` 执行,`template_id` 外键引用 `print_templates(id)`。
//! - **入参**：无(纯 DDL 迁移)。
//! - **传给谁**：无应用层直调;表由打印子系统的任务读写路径按实体映射访问。
//! - **存什么**：任务编码、发起人及其名称、单据类型与单据 ID、所选模板、目标种类与名称、
//!   份数/纸张/方向/四边边距/条码等参数快照、来源模块、附加参数、状态、后端类型、外部作业号、
//!   错误信息、派发与完成时间。
//! - **存哪里**：PostgreSQL `public.print_tasks`。
//!
//! ## 形态说明
//! - `doc_id` 为多态单据引用(可指向不同类型单据的主键),按多态引用列加宽先例直接建为 BIGINT,
//!   与实体侧 i64 绑定宽度一致,避免落入 INTEGER 列的运行期二进制格式错。
//! - `payload_params` 恒给值(缺省空对象),落 `NOT NULL` + DEFAULT,与实体侧非 Option Json 字段一致。
//! - `chk_print_tasks_failed_has_error`:状态为 failed 时 `error_message` 必非空——失败原因必须
//!   可见,不允许静默失败;其余状态不受该约束限制。
//! - 目标地址(打印机 IP / 队列名)不落本表字面量,`target_name` 存符号名,真实地址由运行期配置解析。
//! - `CREATE TABLE IF NOT EXISTS` + 具名约束:全新库直接建表,复用库上本域作为链尾新顶层条目
//!   必然执行,幂等可重放;不在本迁移内写入任何种子行。

use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        let sql = r#"
CREATE TABLE IF NOT EXISTS print_tasks (
    id SERIAL PRIMARY KEY,
    job_code VARCHAR(50) NOT NULL,
    requested_by INTEGER NOT NULL,
    requested_by_name VARCHAR(100) NOT NULL,
    doc_type VARCHAR(100) NOT NULL,
    doc_id BIGINT NOT NULL,
    template_id INTEGER REFERENCES print_templates(id) ON DELETE RESTRICT,
    target_kind VARCHAR(10) NOT NULL,
    target_name VARCHAR(100) NOT NULL,
    copies INTEGER NOT NULL DEFAULT 1,
    paper_size VARCHAR(10) NOT NULL DEFAULT 'A4',
    orientation VARCHAR(10) NOT NULL DEFAULT 'portrait',
    margin_top_mm SMALLINT NOT NULL DEFAULT 15,
    margin_bottom_mm SMALLINT NOT NULL DEFAULT 15,
    margin_left_mm SMALLINT NOT NULL DEFAULT 15,
    margin_right_mm SMALLINT NOT NULL DEFAULT 15,
    barcode_enabled BOOLEAN NOT NULL DEFAULT true,
    source_module VARCHAR(30) NOT NULL,
    payload_params JSONB NOT NULL DEFAULT '{}'::jsonb,
    status VARCHAR(20) NOT NULL DEFAULT 'pending',
    backend_kind VARCHAR(20) NOT NULL,
    external_job_id VARCHAR(100),
    error_message TEXT,
    dispatched_at TIMESTAMPTZ,
    completed_at TIMESTAMPTZ,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    CONSTRAINT uq_print_tasks_job_code UNIQUE (job_code),
    CONSTRAINT chk_print_tasks_target_kind CHECK (target_kind IN ('printer','pool')),
    CONSTRAINT chk_print_tasks_orientation CHECK (orientation IN ('portrait','landscape')),
    CONSTRAINT chk_print_tasks_copies CHECK (copies BETWEEN 1 AND 99),
    CONSTRAINT chk_print_tasks_margins CHECK (
        margin_top_mm BETWEEN 0 AND 50
        AND margin_bottom_mm BETWEEN 0 AND 50
        AND margin_left_mm BETWEEN 0 AND 50
        AND margin_right_mm BETWEEN 0 AND 50
    ),
    CONSTRAINT chk_print_tasks_status CHECK (status IN ('pending','queued','dispatched','succeeded','failed')),
    CONSTRAINT chk_print_tasks_backend_kind CHECK (backend_kind IN ('cups','ipp','network','system_command')),
    CONSTRAINT chk_print_tasks_failed_has_error CHECK (status <> 'failed' OR error_message IS NOT NULL)
);
COMMENT ON TABLE "print_tasks" IS '打印任务台账:每次打印请求的发起、参数快照与状态流转结果';
COMMENT ON COLUMN "print_tasks"."doc_id" IS '多态单据引用主键(按单据类型指向不同表),列宽为 BIGINT';
COMMENT ON COLUMN "print_tasks"."target_kind" IS '打印目标种类:单台打印机或打印机池';
COMMENT ON COLUMN "print_tasks"."target_name" IS '打印目标符号名,真实设备地址由运行期配置解析';
COMMENT ON COLUMN "print_tasks"."payload_params" IS '随任务携带的附加打印参数(JSON 对象)';
COMMENT ON COLUMN "print_tasks"."backend_kind" IS '打印派发所用的后端类型';
COMMENT ON COLUMN "print_tasks"."external_job_id" IS '后端返回的外部作业标识,用于回查与取消';
COMMENT ON COLUMN "print_tasks"."error_message" IS '失败原因;状态为 failed 时必非空';
CREATE INDEX IF NOT EXISTS idx_print_tasks_status ON print_tasks (status);
CREATE INDEX IF NOT EXISTS idx_print_tasks_requested_by ON print_tasks (requested_by);
CREATE INDEX IF NOT EXISTS idx_print_tasks_doc_type_doc_id ON print_tasks (doc_type, doc_id);
"#;
        manager.get_connection().execute_unprepared(sql).await?;
        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        let sql = r#"
DROP INDEX IF EXISTS idx_print_tasks_doc_type_doc_id;
DROP INDEX IF EXISTS idx_print_tasks_requested_by;
DROP INDEX IF EXISTS idx_print_tasks_status;
DROP TABLE IF EXISTS print_tasks;
"#;
        manager.get_connection().execute_unprepared(sql).await?;
        Ok(())
    }
}
