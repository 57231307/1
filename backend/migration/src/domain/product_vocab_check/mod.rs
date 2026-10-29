//! 产品主档规格扩展列词表 CHECK 域。
//!
//! 两支同域迁移，注册次序固定，先点名后收口。
//! - m0107_report_product_spec_vocab_dirty_values 严格只读，逐列逐值把收口前的现状以
//!   RAISE NOTICE 摊进迁移日志，绝不 RAISE EXCEPTION，可任意重跑，down 显式 no-op。
//!   另把本批不收口的四列自由文本分布一并摊出，为后续是否收口留现场。
//! - m0108_apply_product_spec_vocab_checks 才做存量守卫与 CHECK 施加与收尾自证，
//!   非空越界残留即拒滚，存量没干净就不许进 CHECK，其判据依据正是 m0107 留在日志里的
//!   逐值分账。
//!
//! 覆盖四列，色牢度等级取自身词表，数值三列取各自边界口径，不做跨列并集。色牢度权威词表
//! 在 backend 侧 models::status::general::color_fastness。两支各携带同一份清单，
//! 由纸面锁钉彼此逐字相等。
//!
//! 台账名取域名，注册在 lib.rs 迁移链尾，晚于 product_spec_extension_columns 域（该域加这批列），
//! 也晚于 business 域（该域 m0008 建 products 与 product_colors 两表）。

use sea_orm_migration::prelude::*;

mod m0107_report_product_spec_vocab_dirty_values;
mod m0108_apply_product_spec_vocab_checks;

pub struct Migration;

impl MigrationName for Migration {
    fn name(&self) -> &'static str {
        "m_product_vocab_check"
    }
}

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        // 先只读点名，后收口。点名支的现场日志是收口支守卫拒绝时可对照的判据。
        m0107_report_product_spec_vocab_dirty_values::Migration
            .up(manager)
            .await?;
        m0108_apply_product_spec_vocab_checks::Migration
            .up(manager)
            .await?;
        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        // 逆序回滚，先撤收口支施加的 CHECK，再回滚只读点名支（其 up 对库内状态零改变，
        // down 是显式 no-op）。
        m0108_apply_product_spec_vocab_checks::Migration
            .down(manager)
            .await?;
        m0107_report_product_spec_vocab_dirty_values::Migration
            .down(manager)
            .await?;
        Ok(())
    }
}
