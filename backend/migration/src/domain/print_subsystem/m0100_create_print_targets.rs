//! print_targets 表建表迁移
//!
//! ## 六要素
//! - **功能**:创建打印目标登记表 `print_targets`,存储打印机与打印机池两类打印目标的
//!   符号编码、显示名、连接方式参数与缺省份数等主数据。
//! - **调用方**:SeaORM migrator 链,经 `domain/print_subsystem` 域注册;`up` 内后于
//!   `print_templates` / `print_tasks` 执行,本表与二者无外键依赖,顺序仅为域内追加。
//! - **入参**:无(纯 DDL 迁移)。
//! - **传给谁**:无应用层直调;表由打印子系统的派发路径按实体映射读取,目标以
//!   `target_code` 符号名被模板与任务引用。
//! - **存什么**:目标编码与显示名、种类(cups/ipp/network/system_command)、按种类选用的
//!   连接字段(队列名/主机/端口/端点 URL/命令模板)、是否池及池成员编码数组、
//!   缺省份数、启用状态、创建与更新时间。
//! - **存哪里**:PostgreSQL `public.print_targets`。
//!
//! ## 形态说明
//! - `target_code` 为业务唯一符号编码(模板与任务通过它引用目标),建 UNIQUE;不要求任何
//!   真实设备地址字面量出现在本迁移或列默认值中。
//! - 连接字段按种类可空:`chk_print_targets_kind_required` 以单条布尔逻辑具名 CHECK 约束
//!   非池行必须携带其种类所需的连接字段(cups⇒queue_name、network⇒host、ipp⇒endpoint_url、
//!   system_command⇒command_template);池行(is_pool=true)豁免该要求,路由由其成员行承载。
//! - `chk_print_targets_pool_members` 约束池行的 `pool_members` 为非空 JSON 数组
//!   (先以 `jsonb_typeof` 判形再取长度,避免非数组值触发的函数报错)。
//! - `pool_members` 恒给值(缺省空数组),落 `NOT NULL` + DEFAULT,与实体侧非 Option Json
//!   字段一致。
//! - `CREATE TABLE IF NOT EXISTS` + 具名约束:幂等可重放;不在本迁移内写入任何种子行,
//!   目标数据由管理界面录入。

use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        let sql = r#"
CREATE TABLE IF NOT EXISTS print_targets (
    id SERIAL PRIMARY KEY,
    target_code VARCHAR(100) NOT NULL,
    display_name VARCHAR(200) NOT NULL,
    kind VARCHAR(20) NOT NULL,
    queue_name VARCHAR(100),
    host VARCHAR(100),
    port INTEGER,
    endpoint_url VARCHAR(300),
    command_template TEXT,
    is_pool BOOLEAN NOT NULL DEFAULT false,
    pool_members JSONB NOT NULL DEFAULT '[]'::jsonb,
    default_copies SMALLINT NOT NULL DEFAULT 1,
    enabled BOOLEAN NOT NULL DEFAULT true,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    CONSTRAINT uq_print_targets_code UNIQUE (target_code),
    CONSTRAINT chk_print_targets_kind CHECK (kind IN ('cups','ipp','network','system_command')),
    CONSTRAINT chk_print_targets_default_copies CHECK (default_copies BETWEEN 1 AND 99),
    CONSTRAINT chk_print_targets_kind_required CHECK (
        is_pool = true
        OR (kind = 'cups' AND queue_name IS NOT NULL)
        OR (kind = 'network' AND host IS NOT NULL)
        OR (kind = 'ipp' AND endpoint_url IS NOT NULL)
        OR (kind = 'system_command' AND command_template IS NOT NULL)
    ),
    CONSTRAINT chk_print_targets_pool_members CHECK (
        is_pool = false
        OR (jsonb_typeof(pool_members) = 'array' AND jsonb_array_length(pool_members) > 0)
    )
);
COMMENT ON TABLE "print_targets" IS '打印目标登记:打印机与打印机池的符号编码、连接方式参数与缺省打印配置';
COMMENT ON COLUMN "print_targets"."target_code" IS '目标业务唯一符号编码,模板与任务经它引用目标';
COMMENT ON COLUMN "print_targets"."kind" IS '打印连接种类,与 print_tasks.backend_kind 取值集同源,决定该行必需的连接字段';
COMMENT ON COLUMN "print_targets"."queue_name" IS 'cups 种类所用的打印队列名';
COMMENT ON COLUMN "print_targets"."host" IS 'network 种类所用的主机地址';
COMMENT ON COLUMN "print_targets"."port" IS 'network 种类所用的端口(如 9100)';
COMMENT ON COLUMN "print_targets"."endpoint_url" IS 'ipp 种类所用的 IPP 端点 URL';
COMMENT ON COLUMN "print_targets"."command_template" IS 'system_command 种类所用的占位符命令模板';
COMMENT ON COLUMN "print_targets"."is_pool" IS '是否打印机池:池行不直连设备,由成员编码数组路由到各单台目标';
COMMENT ON COLUMN "print_targets"."pool_members" IS '池成员 target_code 字符串数组;is_pool=true 时必为非空数组';
COMMENT ON COLUMN "print_targets"."default_copies" IS '发起打印时的缺省份数';
CREATE INDEX IF NOT EXISTS idx_print_targets_kind ON print_targets (kind);
CREATE INDEX IF NOT EXISTS idx_print_targets_enabled ON print_targets (enabled);
"#;
        manager.get_connection().execute_unprepared(sql).await?;
        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        let sql = r#"
DROP INDEX IF EXISTS idx_print_targets_enabled;
DROP INDEX IF EXISTS idx_print_targets_kind;
DROP TABLE IF EXISTS print_targets;
"#;
        manager.get_connection().execute_unprepared(sql).await?;
        Ok(())
    }
}
