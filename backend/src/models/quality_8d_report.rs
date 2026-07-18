// V15 P0-F20 Batch 480：8D 质量管理流程报告实体
//
// 与 quality_issues 表一对一关联（uq_q8d_quality_issue_id 唯一索引）
// 11 态状态机：not_started → d0_plan → d1_team → d2_problem → d3_interim
//              → d4_root_cause → d5_permanent → d6_verify → d7_prevent
//              → d8_recognize → closed
//
// 详见 migration m0060_create_quality_8d_reports.rs

use chrono::{DateTime, NaiveDate, Utc};
use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

/// 8D 质量管理流程报告实体
#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Eq, Serialize, Deserialize)]
#[sea_orm(table_name = "quality_8d_reports")]
pub struct Model {
    #[sea_orm(primary_key)]
    pub id: i64,
    /// 关联质量异常（一对一）
    pub quality_issue_id: i64,
    /// 11 态状态机当前状态
    pub status: String,

    // D0 准备阶段
    pub d0_date: Option<DateTime<Utc>>,
    pub d0_prepared_by: Option<i32>,
    pub d0_plan: Option<String>,

    // D1 团队组建
    pub d1_date: Option<DateTime<Utc>>,
    pub d1_team_members: Option<String>,

    // D2 问题描述
    pub d2_date: Option<DateTime<Utc>>,
    pub d2_problem_description: Option<String>,

    // D3 临时措施
    pub d3_date: Option<DateTime<Utc>>,
    pub d3_interim_action: Option<String>,

    // D4 根本原因分析（5why / fishbone / other）
    pub d4_date: Option<DateTime<Utc>>,
    pub d4_root_cause_method: Option<String>,
    pub d4_root_cause_detail: Option<String>,
    pub d4_root_cause_summary: Option<String>,

    // D5 永久纠正措施（责任人 + 完成日期跟踪）
    pub d5_date: Option<DateTime<Utc>>,
    pub d5_permanent_action: Option<String>,
    pub d5_action_owner: Option<String>,
    pub d5_due_date: Option<NaiveDate>,
    pub d5_completed_at: Option<DateTime<Utc>>,

    // D6 实施验证
    pub d6_date: Option<DateTime<Utc>>,
    pub d6_verification_result: Option<String>,

    // D7 预防措施
    pub d7_date: Option<DateTime<Utc>>,
    pub d7_prevention_action: Option<String>,

    // D8 团队表彰与闭环
    pub d8_date: Option<DateTime<Utc>>,
    pub d8_closure_summary: Option<String>,

    // 关闭信息
    pub closed_at: Option<DateTime<Utc>>,
    pub closed_by: Option<i32>,

    // 元数据
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {
    #[sea_orm(
        belongs_to = "super::quality_issue::Entity",
        from = "Column::QualityIssueId",
        to = "super::quality_issue::Column::Id"
    )]
    QualityIssue,
    #[sea_orm(
        belongs_to = "super::user::Entity",
        from = "Column::D0PreparedBy",
        to = "super::user::Column::Id"
    )]
    D0PreparedByUser,
    #[sea_orm(
        belongs_to = "super::user::Entity",
        from = "Column::ClosedBy",
        to = "super::user::Column::Id"
    )]
    ClosedByUser,
}

impl Related<super::quality_issue::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::QualityIssue.def()
    }
}

impl ActiveModelBehavior for ActiveModel {}
