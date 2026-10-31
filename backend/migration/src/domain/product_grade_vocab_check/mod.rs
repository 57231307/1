//! 产品等级列词表 CHECK 域。
//!
//! 两支同域迁移，注册次序固定，先点名后收口。
//! - m0122_report_product_grade_vocab_dirty_values 严格只读，把 products.product_grade
//!   收口前的全表行数、越界行逐值分布、已录入值分布与当前列宽以 RAISE NOTICE 摊进
//!   迁移日志，绝不 RAISE EXCEPTION，可任意重跑，down 显式 no-op。
//! - m0123_apply_product_grade_vocab_check 才做存量守卫与 CHECK 施加与收尾自证，
//!   非空越界残留即拒滚，存量没干净就不许进 CHECK，其判据依据正是 m0122 留在日志里的
//!   逐值分账。
//!
//! 词表三档 优等品 一等品 合格品，权威单源在 backend 侧
//! models::status::general::product_grade::ALL_GRADES；两支各携带同一份清单，
//! 由纸面锁钉彼此逐字相等。NULL 语义为未录入即合法，本域不加 NOT NULL、不加 DEFAULT。
//! 本列与库存等级（一等品 二等品 等外品）是两套互不相干的词表，禁止跨域并集。
//!
//! 台账名取域名，注册在 lib.rs 迁移链尾，晚于 system 域与 v15 域（两域各有一条对该列的
//! 历史加列语句，宽度漂移的成因与摊出口径见 m0122 文件头）。

use sea_orm_migration::prelude::*;

mod m0122_report_product_grade_vocab_dirty_values;
mod m0123_apply_product_grade_vocab_check;

pub struct Migration;

impl MigrationName for Migration {
    fn name(&self) -> &'static str {
        "m_product_grade_vocab_check"
    }
}

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        // 先只读点名，后收口。点名支的现场日志是收口支守卫拒绝时可对照的判据。
        m0122_report_product_grade_vocab_dirty_values::Migration
            .up(manager)
            .await?;
        m0123_apply_product_grade_vocab_check::Migration
            .up(manager)
            .await?;
        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        // 逆序回滚，先撤收口支施加的 CHECK，再回滚只读点名支（其 up 对库内状态零改变，
        // down 是显式 no-op）。
        m0123_apply_product_grade_vocab_check::Migration
            .down(manager)
            .await?;
        m0122_report_product_grade_vocab_dirty_values::Migration
            .down(manager)
            .await?;
        Ok(())
    }
}
