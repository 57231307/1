use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        let sql = r#"-- 创建染色批次和配方表
CREATE TABLE IF NOT EXISTS "dye_batch" (
    "id" SERIAL PRIMARY KEY,
    "batch_no" VARCHAR(50) NOT NULL UNIQUE,
    "color_code" VARCHAR(50) NOT NULL,
    "color_name" VARCHAR(100) NOT NULL,
    "fabric_type" VARCHAR(100),
    "weight_kg" DECIMAL(10,2),
    "status" VARCHAR(20) NOT NULL DEFAULT 'pending',
    "production_date" TIMESTAMPTZ,
    "completion_date" TIMESTAMPTZ,
    "quality_grade" VARCHAR(20),
    "remarks" TEXT,
    "created_by" INTEGER,
    "created_at" TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    "updated_at" TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE TABLE IF NOT EXISTS "dye_recipe" (
    "id" SERIAL PRIMARY KEY,
    "recipe_name" VARCHAR(100) NOT NULL,
    "color_code" VARCHAR(50) NOT NULL,
    "ingredients" JSONB,
    "instructions" TEXT,
    "created_by" INTEGER,
    "created_at" TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    "updated_at" TIMESTAMPTZ NOT NULL DEFAULT NOW()
);"#;
        if !sql.trim().is_empty() {
            manager.get_connection().execute_unprepared(sql).await?;
        }
        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        let sql = r#"-- 删除 dye_batch 和 dye_recipe 表
DROP TABLE IF EXISTS "dye_batch" CASCADE;
DROP TABLE IF EXISTS "dye_recipe" CASCADE;"#;
        if !sql.trim().is_empty() {
            manager.get_connection().execute_unprepared(sql).await?;
        }
        Ok(())
    }
}
