use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        // 添加 custom_field_1 到 custom_field_5 字段到 crm_leads 表
        manager
            .alter_table(
                Table::alter()
                    .table(CrmLeads::Table)
                    .add_column(
                        ColumnDef::new(CrmLeads::CustomField1)
                            .string_len(255)
                            .null(),
                    )
                    .add_column(
                        ColumnDef::new(CrmLeads::CustomField2)
                            .string_len(255)
                            .null(),
                    )
                    .add_column(
                        ColumnDef::new(CrmLeads::CustomField3)
                            .string_len(255)
                            .null(),
                    )
                    .add_column(
                        ColumnDef::new(CrmLeads::CustomField4)
                            .text()
                            .null(),
                    )
                    .add_column(
                        ColumnDef::new(CrmLeads::CustomField5)
                            .text()
                            .null(),
                    )
                    .to_owned(),
            )
            .await?;

        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        // 删除字段
        manager
            .alter_table(
                Table::alter()
                    .table(CrmLeads::Table)
                    .drop_column(CrmLeads::CustomField1)
                    .drop_column(CrmLeads::CustomField2)
                    .drop_column(CrmLeads::CustomField3)
                    .drop_column(CrmLeads::CustomField4)
                    .drop_column(CrmLeads::CustomField5)
                    .to_owned(),
            )
            .await?;

        Ok(())
    }
}

#[derive(DeriveIden)]
enum CrmLeads {
    Table,
    CustomField1,
    CustomField2,
    CustomField3,
    CustomField4,
    CustomField5,
}
