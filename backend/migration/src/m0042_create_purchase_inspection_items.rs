//! 采购质检明细表迁移（批次 131 v9 复审 P0）
//!
//! 创建时间: 2026-07-05
//! 关联修复: v9 复审 P0 — purchase_inspection_handler 4 个明细 CRUD 端点全部占位
//!   - list_inspection_items 返回硬编码空列表
//!   - create_inspection_item / update_inspection_item / delete_inspection_item 仅记日志不落库
//!
//! 创建 purchase_inspection_items 表存储质检明细（inspection_id/product_id/item_name/
//! qualified_quantity/unqualified_quantity/remark），替代占位实现。

use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        let sql = include_str!("../../migrations/20260705000004_create_purchase_inspection_items/up.sql");
        if !sql.trim().is_empty() {
            manager.get_connection().execute_unprepared(sql).await?;
        }
        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        let sql = include_str!("../../migrations/20260705000004_create_purchase_inspection_items/down.sql");
        if !sql.trim().is_empty() {
            manager.get_connection().execute_unprepared(sql).await?;
        }
        Ok(())
    }
}
