//! 纺织品安全合规域（GB 18401 / GB 31701 限值矩阵、送检单流程、第三方报告完整性门）
//!
//! 三支同域迁移，注册次序固定，先建表、再收口词表、后播种矩阵。
//! - m0109_create_textile_safety_tables 建八张表：检测项目主数据、限值判定表、送检单、
//!   送检项目清单、第三方报告、逐项目实测结果、童装加考核查、出口市场要求登记。
//! - m0110_apply_textile_safety_vocab_checks 对全部取值列施加词表 CHECK（先点名后施加，
//!   新库表恒空，守卫对存量库残留同样拒绝，不洗数据）。
//! - m0111_seed_textile_safety_matrix 播种 GB 18401 与 GB 31701 的项目、限值矩阵与出口
//!   市场要求种子；ON CONFLICT DO NOTHING 幂等可重放，不覆盖人工维护的判定表行。
//!
//! 台账名取域名 m_textile_safety，注册在 lib.rs 迁移链尾，晚于 system 建 products 表；
//! 既有七张检验表（quality_inspection_records 族）不在本域范围，本域仅以 product_id
//! 外键只读关联 products。

use sea_orm_migration::prelude::*;

mod m0109_create_textile_safety_tables;
mod m0110_apply_textile_safety_vocab_checks;
mod m0111_seed_textile_safety_matrix;

pub struct Migration;

impl MigrationName for Migration {
    fn name(&self) -> &'static str {
        "m_textile_safety"
    }
}

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        // 先建表、再收口词表、后播种矩阵。播种晚于 CHECK，种子行天然受约束校验，
        // 种子与词表漂移会在迁移执行时撞 23514 而不是静默入库。
        m0109_create_textile_safety_tables::Migration
            .up(manager)
            .await?;
        m0110_apply_textile_safety_vocab_checks::Migration
            .up(manager)
            .await?;
        m0111_seed_textile_safety_matrix::Migration
            .up(manager)
            .await?;
        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        // 逆序回滚：先撤种子、再撤 CHECK、最后撤表。撤表前种子已被回收，
        // FK RESTRICT 不会拦住被本域自身建出的行。
        m0111_seed_textile_safety_matrix::Migration
            .down(manager)
            .await?;
        m0110_apply_textile_safety_vocab_checks::Migration
            .down(manager)
            .await?;
        m0109_create_textile_safety_tables::Migration
            .down(manager)
            .await?;
        Ok(())
    }
}
