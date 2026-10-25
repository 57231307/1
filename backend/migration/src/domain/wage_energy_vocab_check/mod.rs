//! 工资与能耗状态列词表 CHECK 域
//!
//! 两支同域迁移，注册次序固定，先点名后收口。
//! - m0102_report_wage_energy_vocab_dirty_values 严格只读，逐列逐值把收口前的现状以
//!   RAISE NOTICE 摊进迁移日志，绝不 RAISE EXCEPTION，可任意重跑，down 显式 no-op。
//! - m0103_apply_wage_energy_vocab_checks 才做存量守卫与 CHECK 施加与收尾自证，
//!   词表外残留即拒滚，回填没干净就不许进 CHECK 的判据依据正是 m0102 留在日志里的逐值分账。
//!
//! 覆盖十二列，取值集合逐列取自身词表不做跨列并集，权威词表在 backend 侧 models::status::
//! wage_energy_chemical_business。台账名取域名，注册在 lib.rs 迁移链尾，晚于建表域。

use sea_orm_migration::prelude::*;

mod m0102_report_wage_energy_vocab_dirty_values;
mod m0103_apply_wage_energy_vocab_checks;

pub struct Migration;

impl MigrationName for Migration {
    fn name(&self) -> &'static str {
        "m_wage_energy_vocab_check"
    }
}

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        // 先只读点名，后收口。点名支的现场日志是收口支守卫拒绝时可对照的判据。
        m0102_report_wage_energy_vocab_dirty_values::Migration
            .up(manager)
            .await?;
        m0103_apply_wage_energy_vocab_checks::Migration
            .up(manager)
            .await?;
        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        // 逆序回滚，先撤收口支施加的 CHECK，再回滚只读点名支（其 up 对库内状态零改变，
        // down 是显式 no-op）。
        m0103_apply_wage_energy_vocab_checks::Migration
            .down(manager)
            .await?;
        m0102_report_wage_energy_vocab_dirty_values::Migration
            .down(manager)
            .await?;
        Ok(())
    }
}
