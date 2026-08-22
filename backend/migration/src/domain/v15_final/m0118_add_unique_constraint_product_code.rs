use sea_orm_migration::prelude::*;

/// 16.16 修复：为 products.code 补充 UNIQUE 约束
///
/// code 字段在实体注释中标注为"唯一"，但建表迁移未加 UNIQUE 约束，
/// 存在并发或绕过应用层校验时写入重复 code 的风险，此处补齐 DB 层约束。
#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .get_connection()
            .execute_unprepared(
                r#"
                ALTER TABLE products
                    ADD CONSTRAINT IF NOT EXISTS uk_products_code UNIQUE (code);
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
                ALTER TABLE products
                    DROP CONSTRAINT IF EXISTS uk_products_code;
                "#,
            )
            .await?;
        Ok(())
    }
}
