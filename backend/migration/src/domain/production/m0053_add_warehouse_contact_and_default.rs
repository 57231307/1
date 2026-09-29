//! warehouses 表补 contact_person / is_default 列（前后端契约对齐）
//!
//! 背景：frontend/scripts/check-contract.mjs ALLOWLIST 挂账 Warehouse.contact_person / is_default，
//! 前端「联系人」列与表单、「默认仓库」开关均读取/提交这两个字段，后端 Model 无列导致
//! 响应恒空、提交被 serde 忽略。本迁移扩展 schema 接入（沿用 m0051 warehouse_type 先例）。
//!
//! 蓝绿部署规范：is_default NOT NULL DEFAULT FALSE（有 DEFAULT 满足 25.4-J 检查）；
//! contact_person NULLABLE。

use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        let sql = r#"
ALTER TABLE warehouses ADD COLUMN IF NOT EXISTS contact_person VARCHAR(100);
ALTER TABLE warehouses ADD COLUMN IF NOT EXISTS is_default BOOLEAN NOT NULL DEFAULT FALSE;
COMMENT ON COLUMN "warehouses"."contact_person" IS '仓库联系人（前端「联系人」列与表单）';
COMMENT ON COLUMN "warehouses"."is_default" IS '默认仓库标志（全局唯一，应用层保证互斥）';
"#;
        manager.get_connection().execute_unprepared(sql).await?;
        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        let sql = r#"
ALTER TABLE warehouses DROP COLUMN IF EXISTS is_default;
ALTER TABLE warehouses DROP COLUMN IF EXISTS contact_person;
"#;
        manager.get_connection().execute_unprepared(sql).await?;
        Ok(())
    }
}
