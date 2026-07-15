#![allow(dead_code)]
// TODO(tech-debt): 业务接入或重评估后逐项移除；rustc 1.94+ 编译时由编译器报告具体死代码位置。

//! 工序质量反馈单模型（process_quality_feedback 表）
//!
//! v14 批次 425：流转卡条码与车间工序流转
//! 依据：面料行业真实业务调研文档 §12.3 工序质量反馈单
//! 真实业务：工序质量问题反馈登记，包括处理意见和方式，异常情况、回修情况登记
//! 状态机：pending(待处理) → processing(处理中) → resolved(已解决) → closed(已关闭)

use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

/// 工序质量反馈单模型
///
/// 真实业务要点：
/// - 工序质量问题反馈登记
/// - 包括处理意见和方式
/// - 异常情况、回修情况登记
/// - 各生产环节共同查找原因及处理办法
#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Serialize, Deserialize, Default)]
#[sea_orm(table_name = "process_quality_feedback")]
pub struct Model {
    #[sea_orm(primary_key)]
    pub id: i32,

    /// 反馈单号：QF-YYYYMMDDHHMMSS-NNN
    pub feedback_no: String,
    /// 关联流转卡 ID
    pub flow_card_id: i32,
    /// 关联工序记录 ID
    pub step_record_id: Option<i32>,

    /// 反馈类型：abnormal(异常) / rework(回修) / defect(疵点) / other(其他)
    pub feedback_type: String,
    /// 问题描述
    pub description: String,
    /// 严重等级：low(低) / medium(中) / high(高) / critical(严重)
    pub severity: String,

    /// 发现人 ID
    pub found_by: Option<i32>,
    /// 发现时间
    pub found_at: DateTimeWithTimeZone,

    /// 处理意见和方式
    pub handling_opinion: Option<String>,
    /// 处理人 ID
    pub handled_by: Option<i32>,
    /// 处理时间
    pub handled_at: Option<DateTimeWithTimeZone>,
    /// 处理结果
    pub handling_result: Option<String>,

    /// 状态：pending(待处理) → processing(处理中) → resolved(已解决) → closed(已关闭)
    pub status: String,

    /// 备注
    pub remarks: Option<String>,

    // 软删除与审计
    pub is_deleted: bool,
    pub created_by: Option<i32>,
    pub created_at: DateTimeWithTimeZone,
    pub updated_at: DateTimeWithTimeZone,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {
    /// 关联流转卡
    #[sea_orm(
        belongs_to = "super::production_flow_card::Entity",
        from = "Column::FlowCardId",
        to = "super::production_flow_card::Column::Id",
        on_update = "Cascade",
        on_delete = "Cascade"
    )]
    FlowCard,

    /// 关联工序记录
    #[sea_orm(
        belongs_to = "super::process_step_record::Entity",
        from = "Column::StepRecordId",
        to = "super::process_step_record::Column::Id",
        on_update = "Cascade",
        on_delete = "SetNull"
    )]
    StepRecord,
}

impl Related<super::production_flow_card::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::FlowCard.def()
    }
}

impl Related<super::process_step_record::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::StepRecord.def()
    }
}

impl ActiveModelBehavior for ActiveModel {}
