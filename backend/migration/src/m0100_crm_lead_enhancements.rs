use sea_orm::prelude::*;
use sea_orm::sea_query::Table;

/// V15 P2 18.1-D4/D5/D6: CRM 线索管理增强
///
/// - 18.1-D4: 线索来源 ROI 跟踪
/// - 18.1-D5: 线索分配规则
/// - 18.1-D6: 线索培育流程
#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        // 18.1-D4: 线索来源 ROI 跟踪表
        manager
            .create_table(
                Table::create()
                    .table(LeadSourceRoi::Table)
                    .if_not_exists()
                    .col(ColumnDef::new(LeadSourceRoi::Id).integer().not_null().auto_increment().primary_key())
                    .col(ColumnDef::new(LeadSourceRoi::Source).string().not_null().comment("渠道来源"))
                    .col(ColumnDef::new(LeadSourceRoi::PeriodStart).date().not_null().comment("统计周期开始"))
                    .col(ColumnDef::new(LeadSourceRoi::PeriodEnd).date().not_null().comment("统计周期结束"))
                    .col(ColumnDef::new(LeadSourceRoi::Cost).decimal_len(15, 2).default(0).comment("渠道投入成本"))
                    .col(ColumnDef::new(LeadSourceRoi::LeadCount).integer().default(0).comment("线索数量"))
                    .col(ColumnDef::new(LeadSourceRoi::ConvertedCount).integer().default(0).comment("转化客户数"))
                    .col(ColumnDef::new(LeadSourceRoi::OpportunityCount).integer().default(0).comment("商机数"))
                    .col(ColumnDef::new(LeadSourceRoi::OrderCount).integer().default(0).comment("成交订单数"))
                    .col(ColumnDef::new(LeadSourceRoi::Revenue).decimal_len(15, 2).default(0).comment("成交金额"))
                    .col(ColumnDef::new(LeadSourceRoi::ConversionRate).decimal_len(5, 2).default(0).comment("转化率"))
                    .col(ColumnDef::new(LeadSourceRoi::Roi).decimal_len(10, 2).default(0).comment("ROI = (收入-成本)/成本"))
                    .col(ColumnDef::new(LeadSourceRoi::CreatedAt).timestamp_with_time_zone().default(Expr::current_timestamp()))
                    .to_owned(),
            )
            .await?;

        // 18.1-D5: 线索分配规则表
        manager
            .create_table(
                Table::create()
                    .table(LeadAllocationRule::Table)
                    .if_not_exists()
                    .col(ColumnDef::new(LeadAllocationRule::Id).integer().not_null().auto_increment().primary_key())
                    .col(ColumnDef::new(LeadAllocationRule::RuleName).string().not_null().comment("规则名称"))
                    .col(ColumnDef::new(LeadAllocationRule::RuleType).string().not_null().comment("规则类型：round_robin/weighted/source_based/industry_based"))
                    .col(ColumnDef::new(LeadAllocationRule::SourceFilter).string().nullable().comment("适用来源过滤"))
                    .col(ColumnDef::new(LeadAllocationRule::IndustryFilter).string().nullable().comment("适用行业过滤"))
                    .col(ColumnDef::new(LeadAllocationRule::RegionFilter).string().nullable().comment("适用区域过滤"))
                    .col(ColumnDef::new(LeadAllocationRule::AssignedUserIds).json().nullable().comment("分配用户ID列表"))
                    .col(ColumnDef::new(LeadAllocationRule::Weights).json().nullable().comment("权重配置"))
                    .col(ColumnDef::new(LeadAllocationRule::DailyLimit).integer().default(0).comment("每日分配上限"))
                    .col(ColumnDef::new(LeadAllocationRule::Priority).integer().default(0).comment("规则优先级"))
                    .col(ColumnDef::new(LeadAllocationRule::IsActive).boolean().default(true).comment("是否启用"))
                    .col(ColumnDef::new(LeadAllocationRule::CreatedAt).timestamp_with_time_zone().default(Expr::current_timestamp()))
                    .col(ColumnDef::new(LeadAllocationRule::UpdatedAt).timestamp_with_time_zone().default(Expr::current_timestamp()))
                    .to_owned(),
            )
            .await?;

        // 18.1-D6: 线索培育计划表
        manager
            .create_table(
                Table::create()
                    .table(LeadNurturePlan::Table)
                    .if_not_exists()
                    .col(ColumnDef::new(LeadNurturePlan::Id).integer().not_null().auto_increment().primary_key())
                    .col(ColumnDef::new(LeadNurturePlan::LeadId).integer().not_null().comment("线索ID"))
                    .col(ColumnDef::new(LeadNurturePlan::PlanName).string().not_null().comment("培育计划名称"))
                    .col(ColumnDef::new(LeadNurturePlan::NurtureType).string().not_null().comment("培育类型：email/sms/visit/call"))
                    .col(ColumnDef::new(LeadNurturePlan::TriggerCondition).string().nullable().comment("触发条件"))
                    .col(ColumnDef::new(LeadNurturePlan::TemplateId).string().nullable().comment("模板ID"))
                    .col(ColumnDef::new(LeadNurturePlan::ScheduledAt).timestamp_with_time_zone().nullable().comment("计划执行时间"))
                    .col(ColumnDef::new(LeadNurturePlan::ExecutedAt).timestamp_with_time_zone().nullable().comment("实际执行时间"))
                    .col(ColumnDef::new(LeadNurturePlan::Status).string().default("pending").comment("状态：pending/executed/failed/cancelled"))
                    .col(ColumnDef::new(LeadNurturePlan::Result).string().nullable().comment("执行结果"))
                    .col(ColumnDef::new(LeadNurturePlan::CreatedBy).integer().nullable().comment("创建人"))
                    .col(ColumnDef::new(LeadNurturePlan::CreatedAt).timestamp_with_time_zone().default(Expr::current_timestamp()))
                    .to_owned(),
            )
            .await?;

        // 创建索引
        manager
            .create_index(
                sea_orm::Index::create()
                    .name("idx_lead_source_roi_source_period")
                    .table(LeadSourceRoi::Table)
                    .col(LeadSourceRoi::Source)
                    .col(LeadSourceRoi::PeriodStart)
                    .to_owned(),
            )
            .await?;

        manager
            .create_index(
                sea_orm::Index::create()
                    .name("idx_lead_nurture_plan_lead_id")
                    .table(LeadNurturePlan::Table)
                    .col(LeadNurturePlan::LeadId)
                    .to_owned(),
            )
            .await?;

        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager.drop_table(Table::drop().table(LeadNurturePlan::Table).to_owned()).await?;
        manager.drop_table(Table::drop().table(LeadAllocationRule::Table).to_owned()).await?;
        manager.drop_table(Table::drop().table(LeadSourceRoi::Table).to_owned()).await?;
        Ok(())
    }
}

#[derive(Iden)]
enum LeadSourceRoi {
    Table,
    Id,
    Source,
    PeriodStart,
    PeriodEnd,
    Cost,
    LeadCount,
    ConvertedCount,
    OpportunityCount,
    OrderCount,
    Revenue,
    ConversionRate,
    Roi,
    CreatedAt,
}

#[derive(Iden)]
enum LeadAllocationRule {
    Table,
    Id,
    RuleName,
    RuleType,
    SourceFilter,
    IndustryFilter,
    RegionFilter,
    AssignedUserIds,
    Weights,
    DailyLimit,
    Priority,
    IsActive,
    CreatedAt,
    UpdatedAt,
}

#[derive(Iden)]
enum LeadNurturePlan {
    Table,
    Id,
    LeadId,
    PlanName,
    NurtureType,
    TriggerCondition,
    TemplateId,
    ScheduledAt,
    ExecutedAt,
    Status,
    Result,
    CreatedBy,
    CreatedAt,
}
