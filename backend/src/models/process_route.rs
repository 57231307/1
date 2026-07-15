#![allow(dead_code)]
// TODO(tech-debt): 业务接入或重评估后逐项移除；rustc 1.94+ 编译时由编译器报告具体死代码位置。

//! 工序路线模板模型（process_route 表）
//!
//! v14 批次 425：流转卡条码与车间工序流转
//! 依据：面料行业真实业务调研文档 §12.3 车间工序流转
//! 真实业务：后台自定义车间工序，根据实际车间布局配置关键工序（前处理→染色→印花→后整理→验布）

use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

/// 工序路线模板模型
///
/// 真实业务要点：
/// - 工序路线在后台自定义，可根据车间布局增删调整顺序
/// - 标准流程：前处理(PRE_TREAT) → 染色(DYE) → 印花(PRINT) → 后整理(FINISH) → 验布(INSPECT)
/// - 印花为非必经工序（require_scan=false 表示可跳过扫码确认）
/// - seq 决定流转顺序，扫码时按 seq 递增推进
#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Serialize, Deserialize, Default)]
#[sea_orm(table_name = "process_route")]
pub struct Model {
    #[sea_orm(primary_key)]
    pub id: i32,

    /// 工序编码（唯一，如 PRE_TREAT/DYE/PRINT/FINISH/INSPECT）
    pub route_code: String,
    /// 工序名称（如 前处理/染色/印花/后整理/验布）
    pub route_name: String,
    /// 工序序号（流转顺序，1=第一道工序）
    pub seq: i32,
    /// 工序类型：pretreat(前处理)/dye(染色)/print(印花)/finish(后整理)/inspect(验布)/other
    pub process_type: String,
    /// 默认工时（分钟）
    pub default_duration_minutes: Option<i32>,
    /// 是否需要扫码确认
    pub require_scan: bool,
    /// 是否启用
    pub is_active: bool,
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
    /// 一对多：流转卡引用的工序路线
    #[sea_orm(has_many = "super::production_flow_card::Entity")]
    FlowCards,

    /// 一对多：工序流转记录引用的工序路线
    #[sea_orm(has_many = "super::process_step_record::Entity")]
    StepRecords,
}

impl Related<super::production_flow_card::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::FlowCards.def()
    }
}

impl Related<super::process_step_record::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::StepRecords.def()
    }
}

impl ActiveModelBehavior for ActiveModel {}
