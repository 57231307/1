#![allow(dead_code)]
// TODO(tech-debt): 业务接入或重评估后逐项移除；rustc 1.94+ 编译时由编译器报告具体死代码位置。

//! 工序流转记录模型（process_step_record 表）
//!
//! v14 批次 425：流转卡条码与车间工序流转
//! 依据：面料行业真实业务调研文档 §12.3 车间工序流转 + §12.5 产量工资
//! 真实业务：扫描流转卡条码 → 登记工人 → 自动跟进该缸布所处的工序 → 自动统计工人产量
//! 状态机：pending(待开始) → in_progress(进行中) → completed(已完成) / abnormal(异常) / rework(回修)

use rust_decimal::Decimal;
use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

/// 工序流转记录模型
///
/// 真实业务要点：
/// - 每道工序扫码登记开始/结束，记录工人+产量+异常
/// - 工人产量按工序统计，自动汇总进入财务工资核算
/// - 异常情况登记后开工序质量反馈单
/// - 回修时关联原工序记录 ID，便于追溯
#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Serialize, Deserialize, Default)]
#[sea_orm(table_name = "process_step_record")]
pub struct Model {
    #[sea_orm(primary_key)]
    pub id: i32,

    /// 关联流转卡 ID
    pub flow_card_id: i32,
    /// 关联工序路线 ID
    pub process_route_id: Option<i32>,
    /// 工序序号
    pub step_seq: i32,
    /// 工序编码
    pub route_code: String,
    /// 工序名称
    pub route_name: String,
    /// 工序类型：pretreat/dye/print/finish/inspect/other
    pub process_type: String,

    // ===== 工人与设备 =====
    /// 操作工人 ID（可多个，逗号分隔）
    pub worker_ids: Option<String>,
    /// 工人姓名（冗余，便于报表）
    pub worker_names: Option<String>,
    /// 设备/机台 ID
    pub equipment_id: Option<i32>,
    /// 设备名称（冗余）
    pub equipment_name: Option<String>,

    // ===== 时间与工时 =====
    /// 开始时间
    pub start_at: DateTimeWithTimeZone,
    /// 结束时间
    pub end_at: Option<DateTimeWithTimeZone>,
    /// 工时（分钟，end_at - start_at 计算）
    pub duration_minutes: Option<i32>,

    // ===== 产量（kg/m） =====
    /// 计划产量（从流转卡计划配布数量继承）
    #[sea_orm(column_type = "Decimal(Some((12, 2)))")]
    pub planned_quantity: Option<Decimal>,
    /// 实际产量（工人上报）
    #[sea_orm(column_type = "Decimal(Some((12, 2)))")]
    pub actual_quantity: Option<Decimal>,
    /// 合格产量（扣除疵点）
    #[sea_orm(column_type = "Decimal(Some((12, 2)))")]
    pub qualified_quantity: Option<Decimal>,

    /// 状态：pending(待开始) → in_progress(进行中) → completed(已完成) / abnormal(异常) / rework(回修)
    pub status: String,

    // ===== 异常处理 =====
    /// 异常情况描述
    pub abnormal_description: Option<String>,
    /// 处理意见和方式
    pub handling_opinion: Option<String>,
    /// 回修关联原工序记录 ID（回修时指向原记录）
    pub rework_source_id: Option<i32>,

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

    /// 关联工序路线
    #[sea_orm(
        belongs_to = "super::process_route::Entity",
        from = "Column::ProcessRouteId",
        to = "super::process_route::Column::Id",
        on_update = "Cascade",
        on_delete = "SetNull"
    )]
    ProcessRoute,

    /// 自引用：回修关联原工序记录
    #[sea_orm(
        belongs_to = "Entity",
        from = "Column::ReworkSourceId",
        to = "Column::Id",
        on_update = "Cascade",
        on_delete = "SetNull"
    )]
    ReworkSource,

    /// 一对多：该工序记录的质量反馈单
    #[sea_orm(has_many = "super::process_quality_feedback::Entity")]
    QualityFeedbacks,
}

impl Related<super::production_flow_card::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::FlowCard.def()
    }
}

impl Related<super::process_route::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::ProcessRoute.def()
    }
}

impl ActiveModelBehavior for ActiveModel {}
