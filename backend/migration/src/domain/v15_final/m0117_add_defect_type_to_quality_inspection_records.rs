use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        // 结构化缺陷类型，替代 remark 关键词归因
        manager
            .get_connection()
            .execute_unprepared(
                r#"
                ALTER TABLE quality_inspection_records
                    ADD COLUMN IF NOT EXISTS defect_type VARCHAR(50);

                COMMENT ON COLUMN quality_inspection_records.defect_type IS '结构化缺陷类型：color_diff(色差)/color_fastness(色牢度)/spec(规格不符)/damage(破损)/other';
                "#,
            )
            .await?;
        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .get_connection()
            .execute_unprepared(
                r#"
                ALTER TABLE quality_inspection_records
                    DROP COLUMN IF EXISTS defect_type;
                "#,
            )
            .await?;
        Ok(())
    }
}
