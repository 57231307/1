//! 缸号（dye_batch）完工实际产出三列（任务 #168）
//!
//! 依据：
//! - `.monkeycode/docs/research/fabric-industry-research.md:149-158`——缸号全生命周期
//!   须承载"最终落布重量"；印染以缸号为追溯主键，完工申报必登记实际产量。
//! - 单位成本/单位能耗都以产量为分母（cost_collections.output_quantity_kg/_meters、
//!   energy_allocation_record.output_quantity），此前 dye_batch 只有 planned_quantity，
//!   完工端点零请求体，实际产出无处登记、投入产出无法对账。
//!
//! 列语义（与 cost_collection 既有分母列命名对齐）：
//! - `actual_output_kg`：完工登记的实际落布重量（kg），回填 cost_collection.output_quantity_kg；
//! - `actual_output_m`：完工登记的实际落布长度（米），回填 cost_collection.output_quantity_meters；
//! - `greige_input_kg`：坯布投料量（kg），投入侧对账锚点（产出/投料比即失重率口径）。
//!
//! 三列均可空 NULL：历史行与新建成（未完工）行均为空，完工强制必填由端点校验保证
//! （models/dye_batch.rs 侧为 Option<Decimal>）。列注释风格对齐同表 remarks 列
//! （domain/v15/mod.rs:4534 `COMMENT ON COLUMN "dye_batch"."remarks" IS '...'`）。
//!
//! 迁移幂等：ADD COLUMN IF NOT EXISTS，不修改任何历史迁移。

use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        let sql = r#"
ALTER TABLE "dye_batch" ADD COLUMN IF NOT EXISTS "actual_output_kg" DECIMAL(12,2);
ALTER TABLE "dye_batch" ADD COLUMN IF NOT EXISTS "actual_output_m" DECIMAL(12,2);
ALTER TABLE "dye_batch" ADD COLUMN IF NOT EXISTS "greige_input_kg" DECIMAL(12,2);
COMMENT ON COLUMN "dye_batch"."actual_output_kg" IS '完工登记的实际落布重量（kg，完工时强制必填；成本/能耗分母的唯一来源）';
COMMENT ON COLUMN "dye_batch"."actual_output_m" IS '完工登记的实际落布长度（米，完工时强制必填；按米口径的成本/能耗分母）';
COMMENT ON COLUMN "dye_batch"."greige_input_kg" IS '完工登记的坯布投料量（kg，完工时强制必填；投入产出对账的投入侧锚点）';
"#;
        if !sql.trim().is_empty() {
            manager.get_connection().execute_unprepared(sql).await?;
        }
        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        let sql = r#"
ALTER TABLE "dye_batch" DROP COLUMN IF EXISTS "actual_output_kg";
ALTER TABLE "dye_batch" DROP COLUMN IF EXISTS "actual_output_m";
ALTER TABLE "dye_batch" DROP COLUMN IF EXISTS "greige_input_kg";
"#;
        if !sql.trim().is_empty() {
            manager.get_connection().execute_unprepared(sql).await?;
        }
        Ok(())
    }
}
