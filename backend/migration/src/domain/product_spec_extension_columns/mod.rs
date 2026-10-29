//! 产品主档规格扩展列迁移域。
//!
//! 本域只落一支结构迁移，登记为新的顶层域而非并入 business 域。business 域在复用库上
//! 台账已记账、整体跳过，把新列并进它的 up 链尾会使加列在存量库永远不会执行
//! （本仓有迁移嵌已应用域内不重跑的先例，见 purchase_greige_read_grant 域头口径）。
//! 链尾新域名在复用库上属 pending，必然执行。
//!
//! - m0106_add_product_spec_extension_columns 为 products 补 width_needle_included_cm、
//!   weight_tolerance、spandex_percent、usage 四列，为 product_colors 补 color_fastness_grade
//!   一列，并把 products.width 的列注释钉死为有效幅宽（净幅，单位 cm，不含针边），
//!   使米与公斤的换算与每公斤出米数继续只有该列一个幅宽入参源。
//!
//! 依赖关系，products 与 product_colors 两张目标表由 business 域 m0008 建表，早于本域，
//! 故本域排在 lib.rs 迁移链尾时两表必然存在。新列一律可空且无默认，NULL 语义为未录入。
//! 词表取值域的收口不在本域，由后续 product_vocab_check 域的先点名后收口两支承担。
//!
//! down 按本域新增的五列逐一删除并把 products.width 的列注释带回未加注释形态，
//! 不回退任何既有业务数据，可独立回退。

use sea_orm_migration::prelude::*;

mod m0106_add_product_spec_extension_columns;

pub struct Migration;

impl MigrationName for Migration {
    fn name(&self) -> &'static str {
        "m_product_spec_extension_columns"
    }
}

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        m0106_add_product_spec_extension_columns::Migration
            .up(manager)
            .await
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        m0106_add_product_spec_extension_columns::Migration
            .down(manager)
            .await
    }
}
