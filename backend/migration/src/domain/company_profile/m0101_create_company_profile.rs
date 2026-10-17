//! company_profile 表建表迁移
//!
//! ## 六要素
//! - **功能**:创建公司档案表 `company_profile`。全库唯一的公司抬头主数据(名称、简称、
//!   敏感工商/税务/银行信息、联系方式、logo、备注),单例形态:整表仅一行,主键 `id` 恒 1,
//!   并播种该行(空名占位,由 `PUT /company-profile/full` 补全)。
//! - **调用方**:SeaORM migrator 链,经 `domain/company_profile` 域注册,置于全链尾
//!   (`print_subsystem` 之后)的新顶层域;表由 `models/company_profile` 实体映射,
//!   读写走 `services/company_profile_service`,打印 letterhead 走 `services/print_service`。
//! - **入参**:无(纯 DDL + 播种迁移)。
//! - **传给谁**:无其他表外键引用本表;`logo` 列(base64 裸串 PNG)与 `name` 列被打印
//!   渲染路径读取合成页眉,`GET /company-profile` 公开投影只出 name/short_name/logo 三列。
//! - **存什么**:公司名称(必填 NOT NULL)、简称;敏感列:统一社会信用代码、法定代表人、
//!   税务登记号、开户行、银行账号、纳税人类型;联络列:电话/传真/邮箱/网址/地址;
//!   工商列:注册资本(DECIMAL)、成立日期(DATE);`logo`(TEXT,base64 编码 PNG 字节,
//!   无 data: 前缀);`description`(TEXT);审计列:`updated_at`(TIMESTAMPTZ,默认 now())
//!   与 `updated_by`(INTEGER,默认 0=系统播种行尚未有人编辑)。
//! - **存哪里**:PostgreSQL `public.company_profile`。
//!
//! ## 形态说明
//! - 列可空性与 `models/company_profile.rs` 实体逐字段一致:`name`/`updated_at`/`updated_by`
//!   为 NOT NULL 非 Option,其余业务列一律可空 Option;不给业务列设默认值,避免"缺键即默认"
//!   掩盖未提交事实。
//! - `ck_company_profile_singleton CHECK (id = 1)`:以具名约束固化单例不变式,应用层
//!   (`company_profile_service::COMPANY_PROFILE_ID`)按同一值取行;约束被违反时由 DB 拒绝,
//!   不靠应用自觉。
//! - 播种 `INSERT ... ON CONFLICT (id) DO NOTHING`:幂等可重放,复用库上重复执行不报错、
//!   不覆盖已编辑的档案行。
//! - `down` 仅 DROP 本表:无外键引用方,回退不波及其他域。

use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        let sql = r#"
CREATE TABLE IF NOT EXISTS company_profile (
    id INTEGER PRIMARY KEY,
    name VARCHAR(200) NOT NULL,
    short_name VARCHAR(100),
    credit_code VARCHAR(50),
    legal_representative VARCHAR(100),
    tax_registration_number VARCHAR(50),
    bank_name VARCHAR(200),
    bank_account VARCHAR(100),
    taxpayer_type VARCHAR(50),
    phone VARCHAR(50),
    fax VARCHAR(50),
    email VARCHAR(100),
    website VARCHAR(200),
    address TEXT,
    registered_capital DECIMAL(18,2),
    establishment_date DATE,
    logo TEXT,
    description TEXT,
    updated_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_by INTEGER NOT NULL DEFAULT 0,
    CONSTRAINT ck_company_profile_singleton CHECK (id = 1)
);
INSERT INTO company_profile (id, name) VALUES (1, '') ON CONFLICT (id) DO NOTHING;
COMMENT ON TABLE "company_profile" IS '公司档案单例:抬头主数据(名称/敏感工商信息/联系方式/logo),整表仅 id=1 一行';
COMMENT ON COLUMN "company_profile"."id" IS '单例主键,恒为 1(ck_company_profile_singleton 固化);应用层按该常量取行';
COMMENT ON COLUMN "company_profile"."name" IS '公司名称,打印页眉与公开投影的唯一来源,NOT NULL(空名占位行由首次 PUT 补全)';
COMMENT ON COLUMN "company_profile"."credit_code" IS '统一社会信用代码(敏感列,仅全量读写面可见)';
COMMENT ON COLUMN "company_profile"."legal_representative" IS '法定代表人(敏感列,仅全量读写面可见)';
COMMENT ON COLUMN "company_profile"."tax_registration_number" IS '税务登记号(敏感列,仅全量读写面可见)';
COMMENT ON COLUMN "company_profile"."bank_name" IS '开户银行(敏感列,仅全量读写面可见)';
COMMENT ON COLUMN "company_profile"."bank_account" IS '银行账号(敏感列,仅全量读写面可见)';
COMMENT ON COLUMN "company_profile"."taxpayer_type" IS '纳税人类型(敏感列,仅全量读写面可见)';
COMMENT ON COLUMN "company_profile"."registered_capital" IS '注册资本(DECIMAL(18,2),可空)';
COMMENT ON COLUMN "company_profile"."establishment_date" IS '成立日期(DATE,可空)';
COMMENT ON COLUMN "company_profile"."logo" IS '公司 logo:base64 编码 PNG 字节的裸串(无 data: 前缀),写入侧校验 PNG 魔数与 512KB 上限';
COMMENT ON COLUMN "company_profile"."updated_by" IS '最后编辑人 user_id;0 表示播种后从未被编辑';
"#;
        manager.get_connection().execute_unprepared(sql).await?;
        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        let sql = r#"
DROP TABLE IF EXISTS company_profile;
"#;
        manager.get_connection().execute_unprepared(sql).await?;
        Ok(())
    }
}
