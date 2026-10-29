//! 导出审批令牌生命周期改造域
//!
//! 本域一条迁移（m0105），给 export_approval_request 加列加索引加 CHECK 约束，
//! 并登记 BPM 流程定义。注册位置：全链尾——新顶层条目，复用库上属 pending 必然执行。

mod m0105_extend_export_approval_for_bpm_and_token;

use sea_orm_migration::prelude::*;

pub struct Migration;

impl MigrationName for Migration {
    fn name(&self) -> &'static str {
        "m_export_approval_lifecycle"
    }
}

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        m0105_extend_export_approval_for_bpm_and_token::Migration
            .up(manager)
            .await
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        m0105_extend_export_approval_for_bpm_and_token::Migration
            .down(manager)
            .await
    }
}
