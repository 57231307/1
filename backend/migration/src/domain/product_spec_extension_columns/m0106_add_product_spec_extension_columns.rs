//! 产品主档规格扩展列迁移（公差、带针幅宽、氨纶含量、用途、色牢度等级）
//!
//! 背景：面料成交规格是八维一套档（品种组织与单双面、纱支根数与原料、克重目标值与公差、
//! 幅宽有效值与公差且分有效幅宽与带针幅宽、弹性要求行业口径为氨纶含量、颜色色号与色牢度等级、
//! 后整理工艺、用途）。products 既有列覆盖了组织、纱支、密度、克重目标值、有效幅宽、后整理与
//! 色号（product_colors 表），缺公差、带针幅宽、氨纶含量、用途与色牢度等级五处，本迁移补齐。
//!
//! 语义定案：
//! - products.width 列语义钉死为「有效幅宽（净幅，cm），不含针边」，米↔公斤换算与出米数
//!   公式（utils/dual_unit_converter）继续以该列为唯一幅宽入参源，不改算式；
//!   带针幅是另一概念，另存 width_needle_included_cm，两列都允许只有其一（历史行 NULL 即未录入）。
//! - 克重目标值仍存 gram_weight，本批新增的是公差（±g/m²）伴生列。
//! - 单双面归入 structure（组织结构）词表维度，不单开列；词表 CHECK 由后续迁移收口，
//!   本迁移只加列与注释，自由文本阶段不加 DB 约束（存量值分布未点名，先约束即盲门）。
//! - 全部新列可空、无 DEFAULT：NULL 语义为未录入；给数值列硬造 0 默认会把未录入显示成 0，
//!   与「不以 0 兜底掩盖缺值」口径冲突。
//! - 校验口径：取值域（非负、上限、公差≥0）在应用层入参校验拒绝并点名（services/product_service
//!   取值域判据函数），DB 约束由词表与数值 CHECK 后续批收口。
//!
//! down 对称删列，不触碰任何既有业务数据；新列删除即回到未加列形态。
//! 幂等前置 IF NOT EXISTS / IF EXISTS，重跑安全。
//!
//! 域与注册位置，本迁移注册在 product_spec_extension_columns 独立顶层域的 lib.rs 链尾，
//! 晚于 business 域（该域 m0008 建 products 与 product_colors 两表），也晚于全部建表域。
//! 独立成域而非并入 business 域的理由是该域在复用库上台账已记账会整体跳过，
//! 并入即等于加列在存量库永不执行，本域在复用库上属 pending 必然执行。

use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        let sql = r#"ALTER TABLE "products" ADD COLUMN IF NOT EXISTS "width_needle_included_cm" DECIMAL(10,2);
ALTER TABLE "products" ADD COLUMN IF NOT EXISTS "weight_tolerance" DECIMAL(10,2);
ALTER TABLE "products" ADD COLUMN IF NOT EXISTS "spandex_percent" DECIMAL(5,2);
ALTER TABLE "products" ADD COLUMN IF NOT EXISTS "usage" VARCHAR(200);
COMMENT ON COLUMN "products"."width" IS '有效幅宽（净幅，单位 cm，不含针边）；米↔公斤换算与每公斤出米数公式的唯一幅宽入参源；带针幅存 width_needle_included_cm，两列独立可空';
COMMENT ON COLUMN "products"."width_needle_included_cm" IS '带针幅宽（含针边，单位 cm，可空）；开幅/预缩交付参考，不参与克重换算';
COMMENT ON COLUMN "products"."weight_tolerance" IS '克重公差（±g/m²，非负，可空）；目标值存 gram_weight，本列记允差幅度';
COMMENT ON COLUMN "products"."spandex_percent" IS '氨纶含量（%，0~100，可空）；弹性要求的行业口径，与 fabric_composition 成分文本并存，本列供过滤与换算';
COMMENT ON COLUMN "products"."usage" IS '用途（自由文本，可空，如 服装面料/家纺/产业用）；词表 CHECK 由后续批收口';
ALTER TABLE "product_colors" ADD COLUMN IF NOT EXISTS "color_fastness_grade" VARCHAR(20);
COMMENT ON COLUMN "product_colors"."color_fastness_grade" IS '色牢度等级（可空；GB 等级形态 1/1-2/2/2-3/3/3-4/4/4-5/5，词表 CHECK 由后续批收口）';"#;
        manager.get_connection().execute_unprepared(sql).await?;
        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        let sql = r#"ALTER TABLE "products" DROP COLUMN IF EXISTS "width_needle_included_cm";
ALTER TABLE "products" DROP COLUMN IF EXISTS "weight_tolerance";
ALTER TABLE "products" DROP COLUMN IF EXISTS "spandex_percent";
ALTER TABLE "products" DROP COLUMN IF EXISTS "usage";
ALTER TABLE "product_colors" DROP COLUMN IF EXISTS "color_fastness_grade";
COMMENT ON COLUMN "products"."width" IS NULL;"#;
        manager.get_connection().execute_unprepared(sql).await?;
        Ok(())
    }
}
