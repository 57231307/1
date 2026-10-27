//! 产品每卷米数列下线迁移域。
//!
//! 本域只落一支结构迁移，登记为新的顶层域而非并入任何已应用域：并入复用库上台账已记账的域
//! 会被整体跳过，DROP 在存量库永不执行。链尾新域在复用库属 pending，必然执行。
//!
//! - m0109_drop_product_meters_per_roll 按裁定口径下线 products.meters_per_roll。
//!   该列在产品表单里从来只是每匹米数的照抄值（一卷即一匹的现网口径），不携带独立信息；
//!   按卷换算腿已改由每匹米数单源供给（utils/dual_unit_converter），后端入参与出参
//!   与前端类型均已摘除该键，代码先改、库后收。
//!
//! 顺序依赖。代码面（models、handlers、services、quotation_convert 卷制腿）先停止读写该列，
//! 本迁移再 DROP；迁移绝不 UPDATE 业务数据，若探测到两列信息不等的行即点名判红中止，
//! 交人工核对把信息并入每匹米数后重跑，不存在静默 DROP 掉有信息量数据的路径。
//!
//! down 只恢复列形态（加回可空列），不恢复数据（数据从未被本迁移改写，也无权威来源可回填）。

use sea_orm_migration::prelude::*;

mod m0109_drop_product_meters_per_roll;

pub struct Migration;

impl MigrationName for Migration {
    fn name(&self) -> &'static str {
        "m_product_roll_column_drop"
    }
}

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        m0109_drop_product_meters_per_roll::Migration
            .up(manager)
            .await
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        m0109_drop_product_meters_per_roll::Migration
            .down(manager)
            .await
    }
}
