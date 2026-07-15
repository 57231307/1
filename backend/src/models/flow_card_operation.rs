#![allow(dead_code)]
// TODO(tech-debt): 业务接入或重评估后逐项移除；rustc 1.94+ 编译时由编译器报告具体死代码位置。

//! 流转卡工序操作记录模型（flow_card_operation 表）
//!
//! v14 批次 425：流转卡工序流转模块
//! 依据：面料行业真实业务调研文档 §14.1 扫码签入签出流程
//! 真实业务：PDA/工控终端扫码 → 工人刷卡登记 → 记录工号、设备编号、开始/结束时间、实际产量、疵点数
//!          状态机：pending → in_progress → completed → transferred → stored
//!          分支：paused / rework

use rust_decimal::Decimal;
use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

/// 流转卡工序操作记录模型
///
/// 真实业务要点：
/// - 同一流转卡同一工序序号仅允许一条记录（业务约束，Service 层校验）
/// - 签入(pending → in_progress)记录操作员、设备、开始时间
/// - 签出(in_progress → completed)记录结束时间、实际产量、疵点数
/// - 流转(completed → transferred)转入下一道工序
/// - 入库(completed → stored / transferred → stored)末道工序完工入库
#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Serialize, Deserialize, Default)]
#[sea_orm(table_name = "flow_card_operation")]
pub struct Model {
    #[sea_orm(primary_key)]
    pub id: i32,

    /// 流转卡 ID
    pub flow_card_id: i32,
    /// 工序序号
    pub process_sequence: i32,
    /// 工序名称
    pub process_name: String,

    // ===== 操作信息 =====
    /// 操作员 ID
    pub operator_id: Option<i32>,
    /// 设备编号
    pub equipment_id: Option<String>,

    // ===== 状态机 =====
    /// 工序状态：pending → in_progress → completed → transferred → stored
    ///         分支：paused / rework
    pub status: String,

    // ===== 签入签出时间 =====
    /// 签入时间
    pub sign_in_at: Option<DateTimeWithTimeZone>,
    /// 签出时间
    pub sign_out_at: Option<DateTimeWithTimeZone>,

    // ===== 产量与质量 =====
    /// 实际产量
    #[sea_orm(column_type = "Decimal(Some((12, 2)))")]
    pub actual_quantity: Option<Decimal>,
    /// 实际匹数
    pub actual_pieces: Option<i32>,
    /// 疵点数
    pub defect_count: Option<i32>,

    /// 备注
    pub remarks: Option<String>,

    // ===== 审计 =====
    pub created_at: DateTimeWithTimeZone,
    pub updated_at: DateTimeWithTimeZone,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {
    /// 关联流转卡
    #[sea_orm(
        belongs_to = "super::flow_card::Entity",
        from = "Column::FlowCardId",
        to = "super::flow_card::Column::Id",
        on_update = "Cascade",
        on_delete = "Cascade"
    )]
    FlowCard,
}

impl Related<super::flow_card::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::FlowCard.def()
    }
}

impl ActiveModelBehavior for ActiveModel {}
