//! 缸号状态流转规则模型（dye_batch_state_rule 表）
//!
//! v14 批次 432：缸号全生命周期状态机
//! 依据：面料行业真实业务调研文档 §12.7 缸号状态机
//! 真实业务：定义允许的状态转换，用于校验缸号状态流转合法性

use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

/// 缸号状态流转规则模型
///
/// 真实业务要点：定义 14 种状态之间允许的转换规则；
/// 终态（shipped/cancelled/terminated）不可流转；
/// 回修状态 rework 可回到 dyeing 重新进缸；
/// 校验逻辑用 JSONB 描述，支持复杂条件。
#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Serialize, Deserialize, Default)]
#[sea_orm(table_name = "dye_batch_state_rule")]
pub struct Model {
    #[sea_orm(primary_key)]
    pub id: i32,

    /// 流转前状态（pending_schedule/scheduled/preparing/dyeing/washing/fixing/dehydrating/drying/inspecting/stored/rework）
    pub from_status: String,
    /// 流转后状态（14 种状态之一）
    pub to_status: String,
    /// 流转操作代码：schedule/prepare/start_dyeing/wash/fix/dehydrate/dry/inspect/store/ship/cancel/rework/terminate
    pub transition_code: String,
    /// 流转操作名称
    pub transition_name: String,
    /// 是否允许
    pub is_allowed: bool,
    /// 是否必须记录操作人
    pub require_operator: bool,
    /// 是否必须记录设备
    pub require_equipment: bool,
    /// 是否必须填写备注
    pub require_remarks: bool,
    /// 额外校验逻辑描述（JSONB）
    #[sea_orm(column_type = "Json", nullable)]
    pub validation_logic: Option<serde_json::Value>,
    /// 规则描述
    pub description: Option<String>,
    /// 是否启用
    pub is_active: bool,

    pub created_at: DateTimeWithTimeZone,
    pub updated_at: DateTimeWithTimeZone,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {
    // 无关系（独立配置表）
}

impl ActiveModelBehavior for ActiveModel {}
