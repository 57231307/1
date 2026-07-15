//! 缸号生命周期日志模型（dye_batch_lifecycle_log 表）
//!
//! v14 批次 432：缸号全生命周期状态机
//! 依据：面料行业真实业务调研文档 §12.7 缸号状态机 + §3.2 缸号全生命周期追踪
//! 真实业务：记录缸号每次状态流转事件，PDA 扫码或工控终端确认时写入

use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

/// 缸号生命周期日志模型
///
/// 真实业务要点：记录缸号每次状态流转事件（14 种状态之间的转换）；
/// PDA 扫码或工控终端确认时自动捕获时间戳/操作人/设备 ID/采集参数；
/// batch_id 关联 dye_batch.id 但不加外键约束（dye_batch 是已有表，用应用层校验）。
#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Serialize, Deserialize, Default)]
#[sea_orm(table_name = "dye_batch_lifecycle_log")]
pub struct Model {
    #[sea_orm(primary_key)]
    pub id: i32,

    /// 缸号 ID（关联 dye_batch.id，应用层校验）
    pub batch_id: i32,
    /// 缸号（冗余便于查询）
    pub batch_no: String,
    /// 流转前状态（首次创建为 NULL）
    pub from_status: Option<String>,
    /// 流转后状态
    pub to_status: String,
    /// 流转操作代码：schedule/prepare/start_dyeing/wash/fix/dehydrate/dry/inspect/store/ship/cancel/rework/terminate
    pub transition_code: String,
    /// 流转操作名称：排缸/备布/进缸染色/皂洗/固色/脱水/烘干/验布/入库/发货/取消/回修/终止
    pub transition_name: String,
    /// 操作人 ID
    pub operator_id: Option<i32>,
    /// 操作人姓名
    pub operator_name: Option<String>,
    /// 设备 ID
    pub equipment_id: Option<i32>,
    /// 设备名称
    pub equipment_name: Option<String>,
    /// 班次：morning 早班/day 白班/night 夜班
    pub work_shift: Option<String>,
    /// PDA/工控终端采集参数（温度/色差ΔE/时间戳等）
    #[sea_orm(column_type = "Json", nullable)]
    pub captured_params: Option<serde_json::Value>,
    /// 备注
    pub remarks: Option<String>,
    /// 操作发生时间
    pub transition_at: DateTimeWithTimeZone,

    pub created_at: DateTimeWithTimeZone,
    pub updated_at: DateTimeWithTimeZone,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {
    // 无 belongs_to 关系（dye_batch 是已有表，不加外键约束，用应用层校验）
}

impl ActiveModelBehavior for ActiveModel {}
