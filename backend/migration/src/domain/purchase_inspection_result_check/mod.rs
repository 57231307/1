//! purchase_inspection.inspection_result 词表 CHECK 域（跨车道裁定项，两段式收口）
//!
//! 两支同域迁移，注册次序固定，先点名后收口（口径照 wage_energy_vocab_check 的
//! m0102 与 m0103 两段式，一步都不能少）。
//! - m0112a_report_purchase_inspection_result_dirty_values 严格只读，逐值把收口前现状
//!   以 RAISE NOTICE 摊进迁移日志，绝不 RAISE EXCEPTION，可任意重跑，down 显式 no-op。
//! - m0112b_apply_purchase_inspection_result_check 才做存量守卫与 CHECK 施加与收尾自证，
//!   词表外残留即拒滚，脏数据交业务逐行定性，不在迁移里 UPDATE 造值。
//!
//! 权威词表在 backend 侧 models::status::purchase_inventory::purchase_inspection_result
//! （pass、fail、partial 三值，NULL 为完成质检前合法初值）。全仓 grep 该表历史上零 CHECK，
//! 词表防护仅落应用层白名单，本域把它钉进数据库。台账名取域名 m_purchase_inspection_result_check，
//! 注册在 lib.rs 迁移链尾，晚于 business 建列。

use sea_orm_migration::prelude::*;

mod m0112a_report_purchase_inspection_result_dirty_values;
mod m0112b_apply_purchase_inspection_result_check;

pub struct Migration;

impl MigrationName for Migration {
    fn name(&self) -> &'static str {
        "m_purchase_inspection_result_check"
    }
}

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        // 先只读点名，后收口。点名支的现场日志是收口支守卫拒绝时可对照的判据。
        m0112a_report_purchase_inspection_result_dirty_values::Migration
            .up(manager)
            .await?;
        m0112b_apply_purchase_inspection_result_check::Migration
            .up(manager)
            .await?;
        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        // 逆序回滚：先撤收口支施加的 CHECK，再回滚只读点名支（其 up 对库内状态零改变，
        // down 是显式 no-op）。
        m0112b_apply_purchase_inspection_result_check::Migration
            .down(manager)
            .await?;
        m0112a_report_purchase_inspection_result_dirty_values::Migration
            .down(manager)
            .await?;
        Ok(())
    }
}
