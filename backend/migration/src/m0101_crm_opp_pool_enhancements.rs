use sea_orm_migration::prelude::*;
use sea_orm_migration::sea_query::Table;

/// V15 P2 18.2-D5/D6/D7 + 18.3-D5/D6/D7: CRM 商机+公海管理增强
///
/// - 18.2-D5: 阶段停留时长分析
/// - 18.2-D6: 商机竞争对手管理
/// - 18.2-D7: 商机跟进记录关联
/// - 18.3-D5: 回收规则区分跟进周期/成交周期
/// - 18.3-D6: 回收规则部门差异化
/// - 18.3-D7: 公海客户保护机制
#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        // 18.2-D5: 商机阶段变更历史表（用于计算阶段停留时长）
        manager
            .create_table(
                Table::create()
                    .table(OpportunityStageHistory::Table)
                    .if_not_exists()
                    .col(ColumnDef::new(OpportunityStageHistory::Id).integer().not_null().auto_increment().primary_key())
                    .col(ColumnDef::new(OpportunityStageHistory::OpportunityId).integer().not_null().comment("商机ID"))
                    .col(ColumnDef::new(OpportunityStageHistory::FromStage).string().null().comment("原阶段"))
                    .col(ColumnDef::new(OpportunityStageHistory::ToStage).string().not_null().comment("新阶段"))
                    .col(ColumnDef::new(OpportunityStageHistory::ChangedAt).timestamp_with_time_zone().not_null().comment("变更时间"))
                    .col(ColumnDef::new(OpportunityStageHistory::ChangedBy).integer().null().comment("变更人"))
                    .col(ColumnDef::new(OpportunityStageHistory::DurationDays).integer().null().comment("在原阶段停留天数"))
                    .to_owned(),
            )
            .await?;

        // 18.2-D6: 竞争对手表
        manager
            .create_table(
                Table::create()
                    .table(Competitor::Table)
                    .if_not_exists()
                    .col(ColumnDef::new(Competitor::Id).integer().not_null().auto_increment().primary_key())
                    .col(ColumnDef::new(Competitor::Name).string().not_null().comment("竞争对手名称"))
                    .col(ColumnDef::new(Competitor::Strengths).text().null().comment("优势"))
                    .col(ColumnDef::new(Competitor::Weaknesses).text().null().comment("劣势"))
                    .col(ColumnDef::new(Competitor::Website).string().null().comment("官网"))
                    .col(ColumnDef::new(Competitor::Notes).text().null().comment("备注"))
                    .col(ColumnDef::new(Competitor::CreatedAt).timestamp_with_time_zone().default(Expr::current_timestamp()))
                    .col(ColumnDef::new(Competitor::UpdatedAt).timestamp_with_time_zone().default(Expr::current_timestamp()))
                    .to_owned(),
            )
            .await?;

        // 18.2-D6: 商机-竞争对手关联表
        manager
            .create_table(
                Table::create()
                    .table(OpportunityCompetitor::Table)
                    .if_not_exists()
                    .col(ColumnDef::new(OpportunityCompetitor::Id).integer().not_null().auto_increment().primary_key())
                    .col(ColumnDef::new(OpportunityCompetitor::OpportunityId).integer().not_null().comment("商机ID"))
                    .col(ColumnDef::new(OpportunityCompetitor::CompetitorId).integer().not_null().comment("竞争对手ID"))
                    .col(ColumnDef::new(OpportunityCompetitor::ThreatLevel).string().null().comment("威胁级别：low/medium/high"))
                    .col(ColumnDef::new(OpportunityCompetitor::Notes).text().null().comment("备注"))
                    .col(ColumnDef::new(OpportunityCompetitor::CreatedAt).timestamp_with_time_zone().default(Expr::current_timestamp()))
                    .to_owned(),
            )
            .await?;

        // 18.2-D7: 商机跟进记录表
        manager
            .create_table(
                Table::create()
                    .table(OpportunityFollowUp::Table)
                    .if_not_exists()
                    .col(ColumnDef::new(OpportunityFollowUp::Id).integer().not_null().auto_increment().primary_key())
                    .col(ColumnDef::new(OpportunityFollowUp::OpportunityId).integer().not_null().comment("商机ID"))
                    .col(ColumnDef::new(OpportunityFollowUp::FollowUpType).string().not_null().comment("跟进方式：phone/email/visit/meeting/wechat"))
                    .col(ColumnDef::new(OpportunityFollowUp::Content).text().not_null().comment("跟进内容"))
                    .col(ColumnDef::new(OpportunityFollowUp::FollowUpTime).timestamp_with_time_zone().not_null().comment("跟进时间"))
                    .col(ColumnDef::new(OpportunityFollowUp::NextFollowUpDate).date().null().comment("下次跟进日期"))
                    .col(ColumnDef::new(OpportunityFollowUp::UserId).integer().not_null().comment("跟进人ID"))
                    .col(ColumnDef::new(OpportunityFollowUp::UserName).string().not_null().comment("跟进人姓名"))
                    .col(ColumnDef::new(OpportunityFollowUp::CreatedAt).timestamp_with_time_zone().default(Expr::current_timestamp()))
                    .to_owned(),
            )
            .await?;

        // 18.3-D5/D6/D7: 扩展回收规则表（添加字段）
        // 18.3-D5: 区分跟进周期/成交周期
        // 18.3-D6: 部门差异化
        // 18.3-D7: 公海客户保护机制
        manager
            .alter_table(
                Table::alter()
                    .table(CrmRecycleRule::Table)
                    .add_column(ColumnDef::new(CrmRecycleRule::FollowUpDays).integer().null().comment("跟进周期（天）"))
                    .add_column(ColumnDef::new(CrmRecycleRule::DealDays).integer().null().comment("成交周期（天）"))
                    .add_column(ColumnDef::new(CrmRecycleRule::DepartmentId).integer().null().comment("适用部门ID"))
                    .to_owned(),
            )
            .await?;

        // 18.3-D7: 扩展 crm_lead 表（添加保护机制字段）
        manager
            .alter_table(
                Table::alter()
                    .table(CrmLead::Table)
                    .add_column(ColumnDef::new(CrmLead::ProtectedUntil).timestamp_with_time_zone().null().comment("保护截止时间"))
                    .add_column(ColumnDef::new(CrmLead::ProtectedBy).integer().null().comment("保护人ID"))
                    .to_owned(),
            )
            .await?;

        // 创建索引
        manager
            .create_index(
                Index::create()
                    .name("idx_opp_stage_history_opp_id")
                    .table(OpportunityStageHistory::Table)
                    .col(OpportunityStageHistory::OpportunityId)
                    .to_owned(),
            )
            .await?;

        manager
            .create_index(
                Index::create()
                    .name("idx_opp_follow_up_opp_id")
                    .table(OpportunityFollowUp::Table)
                    .col(OpportunityFollowUp::OpportunityId)
                    .to_owned(),
            )
            .await?;

        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager.drop_table(Table::drop().table(OpportunityFollowUp::Table).to_owned()).await?;
        manager.drop_table(Table::drop().table(OpportunityCompetitor::Table).to_owned()).await?;
        manager.drop_table(Table::drop().table(Competitor::Table).to_owned()).await?;
        manager.drop_table(Table::drop().table(OpportunityStageHistory::Table).to_owned()).await?;
        Ok(())
    }
}

#[derive(Iden)]
enum OpportunityStageHistory {
    Table,
    Id,
    OpportunityId,
    FromStage,
    ToStage,
    ChangedAt,
    ChangedBy,
    DurationDays,
}

#[derive(Iden)]
enum Competitor {
    Table,
    Id,
    Name,
    Strengths,
    Weaknesses,
    Website,
    Notes,
    CreatedAt,
    UpdatedAt,
}

#[derive(Iden)]
enum OpportunityCompetitor {
    Table,
    Id,
    OpportunityId,
    CompetitorId,
    ThreatLevel,
    Notes,
    CreatedAt,
}

#[derive(Iden)]
enum OpportunityFollowUp {
    Table,
    Id,
    OpportunityId,
    FollowUpType,
    Content,
    FollowUpTime,
    NextFollowUpDate,
    UserId,
    UserName,
    CreatedAt,
}

#[derive(Iden)]
enum CrmRecycleRule {
    Table,
    FollowUpDays,
    DealDays,
    DepartmentId,
}

#[derive(Iden)]
enum CrmLead {
    Table,
    ProtectedUntil,
    ProtectedBy,
}
