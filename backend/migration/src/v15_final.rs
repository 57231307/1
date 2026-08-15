//! V15 最终完善
//!
//! 合并自: 11 个迁移文件

use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        // === m0106_batch_dye_lot_unique_constraint.rs ===
// 1. 删除 batch_no 单字段 UNIQUE 约束
        manager
            .drop_index(
                Index::drop()
                    .name("batch_dye_lot_batch_no_key")
                    .table(BatchDyeLot::Table)
                    .to_owned(),
            )
            .await?;

        // 2. 添加 (dye_lot_no, batch_no) 组合唯一约束
        manager
            .create_index(
                Index::create()
                    .name("idx_batch_dye_lot_dye_lot_no_batch_no")
                    .table(BatchDyeLot::Table)
                    .col(BatchDyeLot::DyeLotNo)
                    .col(BatchDyeLot::BatchNo)
                    .unique()
                    .to_owned(),
            )
            .await?;

        // 3. 为 dye_batch 表也添加 (dye_lot_no, batch_no) 组合索引（非唯一，辅助查询）
        manager
            .create_index(
                Index::create()
                    .name("idx_dye_batch_dye_lot_no_batch_no")
                    .table(DyeBatch::Table)
                    .col(DyeBatch::DyeLotNo)
                    .col(DyeBatch::BatchNo)
                    .to_owned(),
            )
            .await?;
        // === m0107_add_color_card_capability_fields.rs ===
// 添加 dyeing_capability 字段到 color_cards 表
        manager
            .alter_table(
                Table::alter()
                    .table(ColorCards::Table)
                    .add_column(
                        ColumnDef::new(ColorCards::DyeingCapability)
                            .string_len(50)
                            .null(),
                    )
                    .to_owned(),
            )
            .await?;

        // 添加 printing_capability 字段到 color_cards 表
        manager
            .alter_table(
                Table::alter()
                    .table(ColorCards::Table)
                    .add_column(
                        ColumnDef::new(ColorCards::PrintingCapability)
                            .string_len(50)
                            .null(),
                    )
                    .to_owned(),
            )
            .await?;

        // 添加 color_fastness_grade 字段到 color_cards 表
        manager
            .alter_table(
                Table::alter()
                    .table(ColorCards::Table)
                    .add_column(
                        ColumnDef::new(ColorCards::ColorFastnessGrade)
                            .string_len(20)
                            .null(),
                    )
                    .to_owned(),
            )
            .await?;
        // === m0108_create_customer_addresses.rs ===
// 创建 customer_addresses 表
        manager
            .create_table(
                Table::create()
                    .table(CustomerAddresses::Table)
                    .if_not_exists()
                    .col(
                        ColumnDef::new(CustomerAddresses::Id)
                            .integer()
                            .not_null()
                            .auto_increment()
                            .primary_key(),
                    )
                    .col(
                        ColumnDef::new(CustomerAddresses::CustomerId)
                            .integer()
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(CustomerAddresses::AddressType)
                            .string_len(50)
                            .not_null()
                            .default("shipping"),
                    )
                    .col(
                        ColumnDef::new(CustomerAddresses::ContactName)
                            .string_len(100)
                            .null(),
                    )
                    .col(
                        ColumnDef::new(CustomerAddresses::Phone)
                            .string_len(20)
                            .null(),
                    )
                    .col(
                        ColumnDef::new(CustomerAddresses::Province)
                            .string_len(50)
                            .null(),
                    )
                    .col(
                        ColumnDef::new(CustomerAddresses::City)
                            .string_len(50)
                            .null(),
                    )
                    .col(
                        ColumnDef::new(CustomerAddresses::District)
                            .string_len(50)
                            .null(),
                    )
                    .col(
                        ColumnDef::new(CustomerAddresses::Address)
                            .string_len(500)
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(CustomerAddresses::ZipCode)
                            .string_len(20)
                            .null(),
                    )
                    .col(
                        ColumnDef::new(CustomerAddresses::IsDefault)
                            .boolean()
                            .not_null()
                            .default(false),
                    )
                    .col(
                        ColumnDef::new(CustomerAddresses::CreatedAt)
                            .timestamp_with_time_zone()
                            .not_null()
                            .default(Expr::cust("NOW()")),
                    )
                    .col(
                        ColumnDef::new(CustomerAddresses::UpdatedAt)
                            .timestamp_with_time_zone()
                            .not_null()
                            .default(Expr::cust("NOW()")),
                    )
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk_customer_addresses_customer_id")
                            .from(CustomerAddresses::Table, CustomerAddresses::CustomerId)
                            .to(Customers::Table, Customers::Id)
                            .on_delete(ForeignKeyAction::Cascade),
                    )
                    .to_owned(),
            )
            .await?;

        // 创建索引
        manager
            .create_index(
                Index::create()
                    .name("idx_customer_addresses_customer_id")
                    .table(CustomerAddresses::Table)
                    .col(CustomerAddresses::CustomerId)
                    .to_owned(),
            )
            .await?;

        manager
            .create_index(
                Index::create()
                    .name("idx_customer_addresses_is_default")
                    .table(CustomerAddresses::Table)
                    .col(CustomerAddresses::IsDefault)
                    .to_owned(),
            )
            .await?;
        // === m0109_add_customer_special_process.rs ===
// 添加 special_process 字段到 customers 表
        manager
            .alter_table(
                Table::alter()
                    .table(Customers::Table)
                    .add_column(ColumnDef::new(Customers::SpecialProcess).text().null())
                    .to_owned(),
            )
            .await?;
        // === m0110_create_aging_grade_configs.rs ===
// 创建 aging_grade_configs 表
        manager
            .create_table(
                Table::create()
                    .table(AgingGradeConfigs::Table)
                    .if_not_exists()
                    .col(
                        ColumnDef::new(AgingGradeConfigs::Id)
                            .integer()
                            .not_null()
                            .auto_increment()
                            .primary_key(),
                    )
                    .col(
                        ColumnDef::new(AgingGradeConfigs::GradeName)
                            .string_len(50)
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(AgingGradeConfigs::MinDays)
                            .integer()
                            .not_null(),
                    )
                    .col(ColumnDef::new(AgingGradeConfigs::MaxDays).integer().null())
                    .col(
                        ColumnDef::new(AgingGradeConfigs::ProvisionRate)
                            .decimal_len(5, 4)
                            .not_null()
                            .default(0),
                    )
                    .col(ColumnDef::new(AgingGradeConfigs::Description).text().null())
                    .col(
                        ColumnDef::new(AgingGradeConfigs::IsActive)
                            .boolean()
                            .not_null()
                            .default(true),
                    )
                    .col(
                        ColumnDef::new(AgingGradeConfigs::CreatedAt)
                            .timestamp_with_time_zone()
                            .not_null()
                            .default(Expr::cust("NOW()")),
                    )
                    .col(
                        ColumnDef::new(AgingGradeConfigs::UpdatedAt)
                            .timestamp_with_time_zone()
                            .not_null()
                            .default(Expr::cust("NOW()")),
                    )
                    .to_owned(),
            )
            .await?;
        // === m0111_create_industry_benchmark_configs.rs ===
// 创建 industry_benchmark_configs 表
        manager
            .create_table(
                Table::create()
                    .table(IndustryBenchmarkConfigs::Table)
                    .if_not_exists()
                    .col(
                        ColumnDef::new(IndustryBenchmarkConfigs::Id)
                            .integer()
                            .not_null()
                            .auto_increment()
                            .primary_key(),
                    )
                    .col(
                        ColumnDef::new(IndustryBenchmarkConfigs::Industry)
                            .string_len(100)
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(IndustryBenchmarkConfigs::MetricName)
                            .string_len(100)
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(IndustryBenchmarkConfigs::BenchmarkValue)
                            .decimal_len(15, 4)
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(IndustryBenchmarkConfigs::Unit)
                            .string_len(50)
                            .null(),
                    )
                    .col(
                        ColumnDef::new(IndustryBenchmarkConfigs::Description)
                            .text()
                            .null(),
                    )
                    .col(
                        ColumnDef::new(IndustryBenchmarkConfigs::IsActive)
                            .boolean()
                            .not_null()
                            .default(true),
                    )
                    .col(
                        ColumnDef::new(IndustryBenchmarkConfigs::CreatedAt)
                            .timestamp_with_time_zone()
                            .not_null()
                            .default(Expr::cust("NOW()")),
                    )
                    .col(
                        ColumnDef::new(IndustryBenchmarkConfigs::UpdatedAt)
                            .timestamp_with_time_zone()
                            .not_null()
                            .default(Expr::cust("NOW()")),
                    )
                    .to_owned(),
            )
            .await?;

        // 创建索引
        manager
            .create_index(
                Index::create()
                    .name("idx_industry_benchmark_configs_industry")
                    .table(IndustryBenchmarkConfigs::Table)
                    .col(IndustryBenchmarkConfigs::Industry)
                    .to_owned(),
            )
            .await?;
        // === m0112_add_accounting_period_close_fields.rs ===
// 添加 closed_by 字段到 accounting_periods 表
        manager
            .alter_table(
                Table::alter()
                    .table(AccountingPeriods::Table)
                    .add_column(ColumnDef::new(AccountingPeriods::ClosedBy).integer().null())
                    .to_owned(),
            )
            .await?;

        // 添加 closed_at 字段到 accounting_periods 表
        manager
            .alter_table(
                Table::alter()
                    .table(AccountingPeriods::Table)
                    .add_column(
                        ColumnDef::new(AccountingPeriods::ClosedAt)
                            .timestamp_with_time_zone()
                            .null(),
                    )
                    .to_owned(),
            )
            .await?;

        // 添加 close_notes 字段到 accounting_periods 表
        manager
            .alter_table(
                Table::alter()
                    .table(AccountingPeriods::Table)
                    .add_column(ColumnDef::new(AccountingPeriods::CloseNotes).text().null())
                    .to_owned(),
            )
            .await?;
        // === m0113_add_fixed_asset_depreciation_start_date.rs ===
// 添加 depreciation_start_date 字段到 fixed_assets 表
        manager
            .alter_table(
                Table::alter()
                    .table(FixedAssets::Table)
                    .add_column(
                        ColumnDef::new(FixedAssets::DepreciationStartDate)
                            .date()
                            .null(),
                    )
                    .to_owned(),
            )
            .await?;
        // === m0114_add_customer_source_fields.rs ===
// 添加 source 字段到 customers 表
        manager
            .alter_table(
                Table::alter()
                    .table(Customers::Table)
                    .add_column(ColumnDef::new(Customers::Source).string_len(50).null())
                    .to_owned(),
            )
            .await?;

        // 添加 pool_recycle_reason 字段到 customers 表
        manager
            .alter_table(
                Table::alter()
                    .table(Customers::Table)
                    .add_column(ColumnDef::new(Customers::PoolRecycleReason).text().null())
                    .to_owned(),
            )
            .await?;
        // === m0115_add_crm_lead_custom_fields.rs ===
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
                    .add_column(ColumnDef::new(CrmLeads::CustomField4).text().null())
                    .add_column(ColumnDef::new(CrmLeads::CustomField5).text().null())
                    .to_owned(),
            )
            .await?;
        // === m0116_create_long_running_tasks.rs ===
// 创建 long_running_tasks 表
        manager
            .create_table(
                Table::create()
                    .table(LongRunningTasks::Table)
                    .if_not_exists()
                    .col(
                        ColumnDef::new(LongRunningTasks::Id)
                            .string_len(36)
                            .not_null()
                            .primary_key(),
                    )
                    .col(
                        ColumnDef::new(LongRunningTasks::TaskType)
                            .string_len(100)
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(LongRunningTasks::Status)
                            .string_len(20)
                            .not_null()
                            .default("pending"),
                    )
                    .col(
                        ColumnDef::new(LongRunningTasks::Progress)
                            .integer()
                            .not_null()
                            .default(0),
                    )
                    .col(
                        ColumnDef::new(LongRunningTasks::TotalSteps)
                            .integer()
                            .not_null()
                            .default(0),
                    )
                    .col(
                        ColumnDef::new(LongRunningTasks::CurrentStep)
                            .integer()
                            .not_null()
                            .default(0),
                    )
                    .col(
                        ColumnDef::new(LongRunningTasks::StepDescription)
                            .string_len(500)
                            .null(),
                    )
                    .col(
                        ColumnDef::new(LongRunningTasks::Result)
                            .json_binary()
                            .null(),
                    )
                    .col(ColumnDef::new(LongRunningTasks::ErrorMessage).text().null())
                    .col(ColumnDef::new(LongRunningTasks::StartedBy).integer().null())
                    .col(
                        ColumnDef::new(LongRunningTasks::StartedAt)
                            .timestamp_with_time_zone()
                            .null(),
                    )
                    .col(
                        ColumnDef::new(LongRunningTasks::CompletedAt)
                            .timestamp_with_time_zone()
                            .null(),
                    )
                    .col(
                        ColumnDef::new(LongRunningTasks::CreatedAt)
                            .timestamp_with_time_zone()
                            .not_null()
                            .default(Expr::cust("NOW()")),
                    )
                    .col(
                        ColumnDef::new(LongRunningTasks::UpdatedAt)
                            .timestamp_with_time_zone()
                            .not_null()
                            .default(Expr::cust("NOW()")),
                    )
                    .to_owned(),
            )
            .await?;

        // 创建索引
        manager
            .create_index(
                Index::create()
                    .name("idx_long_running_tasks_status")
                    .table(LongRunningTasks::Table)
                    .col(LongRunningTasks::Status)
                    .to_owned(),
            )
            .await?;

        manager
            .create_index(
                Index::create()
                    .name("idx_long_running_tasks_task_type")
                    .table(LongRunningTasks::Table)
                    .col(LongRunningTasks::TaskType)
                    .to_owned(),
            )
            .await?;

        manager
            .create_index(
                Index::create()
                    .name("idx_long_running_tasks_started_by")
                    .table(LongRunningTasks::Table)
                    .col(LongRunningTasks::StartedBy)
                    .to_owned(),
            )
            .await?;
        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        // === m0106_batch_dye_lot_unique_constraint.rs ===
// 回滚：删除组合唯一约束，恢复 batch_no 单字段 UNIQUE
        manager
            .drop_index(
                Index::drop()
                    .name("idx_batch_dye_lot_dye_lot_no_batch_no")
                    .table(BatchDyeLot::Table)
                    .to_owned(),
            )
            .await?;

        manager
            .drop_index(
                Index::drop()
                    .name("idx_dye_batch_dye_lot_no_batch_no")
                    .table(DyeBatch::Table)
                    .to_owned(),
            )
            .await?;

        manager
            .create_index(
                Index::create()
                    .name("batch_dye_lot_batch_no_key")
                    .table(BatchDyeLot::Table)
                    .col(BatchDyeLot::BatchNo)
                    .unique()
                    .to_owned(),
            )
            .await?;
        // === m0107_add_color_card_capability_fields.rs ===
// 删除字段
        manager
            .alter_table(
                Table::alter()
                    .table(ColorCards::Table)
                    .drop_column(ColorCards::DyeingCapability)
                    .drop_column(ColorCards::PrintingCapability)
                    .drop_column(ColorCards::ColorFastnessGrade)
                    .to_owned(),
            )
            .await?;
        // === m0108_create_customer_addresses.rs ===
// 删除索引
        manager
            .drop_index(
                Index::drop()
                    .name("idx_customer_addresses_is_default")
                    .table(CustomerAddresses::Table)
                    .to_owned(),
            )
            .await?;

        manager
            .drop_index(
                Index::drop()
                    .name("idx_customer_addresses_customer_id")
                    .table(CustomerAddresses::Table)
                    .to_owned(),
            )
            .await?;

        // 删除表
        manager
            .drop_table(Table::drop().table(CustomerAddresses::Table).to_owned())
            .await?;
        // === m0109_add_customer_special_process.rs ===
// 删除字段
        manager
            .alter_table(
                Table::alter()
                    .table(Customers::Table)
                    .drop_column(Customers::SpecialProcess)
                    .to_owned(),
            )
            .await?;
        // === m0110_create_aging_grade_configs.rs ===
// 删除表
        manager
            .drop_table(Table::drop().table(AgingGradeConfigs::Table).to_owned())
            .await?;
        // === m0111_create_industry_benchmark_configs.rs ===
// 删除索引
        manager
            .drop_index(
                Index::drop()
                    .name("idx_industry_benchmark_configs_industry")
                    .table(IndustryBenchmarkConfigs::Table)
                    .to_owned(),
            )
            .await?;

        // 删除表
        manager
            .drop_table(
                Table::drop()
                    .table(IndustryBenchmarkConfigs::Table)
                    .to_owned(),
            )
            .await?;
        // === m0112_add_accounting_period_close_fields.rs ===
// 删除字段
        manager
            .alter_table(
                Table::alter()
                    .table(AccountingPeriods::Table)
                    .drop_column(AccountingPeriods::ClosedBy)
                    .drop_column(AccountingPeriods::ClosedAt)
                    .drop_column(AccountingPeriods::CloseNotes)
                    .to_owned(),
            )
            .await?;
        // === m0113_add_fixed_asset_depreciation_start_date.rs ===
// 删除字段
        manager
            .alter_table(
                Table::alter()
                    .table(FixedAssets::Table)
                    .drop_column(FixedAssets::DepreciationStartDate)
                    .to_owned(),
            )
            .await?;
        // === m0114_add_customer_source_fields.rs ===
// 删除字段
        manager
            .alter_table(
                Table::alter()
                    .table(Customers::Table)
                    .drop_column(Customers::Source)
                    .drop_column(Customers::PoolRecycleReason)
                    .to_owned(),
            )
            .await?;
        // === m0115_add_crm_lead_custom_fields.rs ===
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
        // === m0116_create_long_running_tasks.rs ===
// 删除索引
        manager
            .drop_index(
                Index::drop()
                    .name("idx_long_running_tasks_started_by")
                    .table(LongRunningTasks::Table)
                    .to_owned(),
            )
            .await?;

        manager
            .drop_index(
                Index::drop()
                    .name("idx_long_running_tasks_task_type")
                    .table(LongRunningTasks::Table)
                    .to_owned(),
            )
            .await?;

        manager
            .drop_index(
                Index::drop()
                    .name("idx_long_running_tasks_status")
                    .table(LongRunningTasks::Table)
                    .to_owned(),
            )
            .await?;

        // 删除表
        manager
            .drop_table(Table::drop().table(LongRunningTasks::Table).to_owned())
            .await?;
        Ok(())
    }
}

// === m0106_batch_dye_lot_unique_constraint.rs ===
#[derive(Iden)]
enum BatchDyeLot {
    Table,
    DyeLotNo,
    BatchNo,
}

#[derive(Iden)]
enum DyeBatch {
    Table,
    DyeLotNo,
    BatchNo,
}
