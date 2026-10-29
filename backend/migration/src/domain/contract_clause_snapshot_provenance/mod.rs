//! 合同条款快照来源指针独立域
//!
//! 本域把 m0119 的 `contract_clauses.template_clause_id` 补列从合同模板建表域里搬出来，
//! 单独作为顶层迁移台账挂在链尾。为什么不能塞在 `contract_template_schema` 里 该域已在
//! 复用测试库上入账，SeaORM 台账按域名记账，域内任何新增子迁移在复用库上永远不再重放，
//! 补列即缺席；只有另立新域让域名本身处于 pending，才能被 migrate run 拾起。
//! - m0119_add_contract_clause_snapshot_provenance 给快照表追加一列可空无默认的
//!   `template_clause_id`，记录固化当时引用的模板条款项行主键，供详情腿单义回查必填标记。
//!
//! 台账名取域名，注册在 lib.rs 迁移链尾，紧跟 `contract_template_schema` 之后。
//! 全部形态为可空加列，不 UPDATE、不 DELETE、不回填，历史快照行留空即“无溯源指针”，
//! 读侧对空指针走 (模板, 条款码) 唯一行回落，多义如实给空值，绝不伪造。

use sea_orm_migration::prelude::*;

mod m0119_add_contract_clause_snapshot_provenance;

pub struct Migration;

impl MigrationName for Migration {
    fn name(&self) -> &'static str {
        "m_contract_clause_snapshot_provenance"
    }
}

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        m0119_add_contract_clause_snapshot_provenance::Migration
            .up(manager)
            .await?;
        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        m0119_add_contract_clause_snapshot_provenance::Migration
            .down(manager)
            .await?;
        Ok(())
    }
}
