//! 缸号操作记录模型（dye_batch_operation 表）
//!
//! v14 批次 432：缸号全生命周期状态机
//! 依据：面料行业真实业务调研文档 §12.7 缸号状态机
//! 真实业务：记录合缸/分缸/优先级调整/缸变更等操作

use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

/// 缸号操作记录模型
///
/// 真实业务要点：操作类型 6 种，merge 合缸/split 分缸/priority_adjust 优先级调整/batch_change 缸变更/schedule_change 计划变更/terminate 终止；
/// 合缸/分缸时 source_batch_ids 记录源缸号 ID 列表；
/// operation_data 记录优先级值、变更前后信息等。
#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Serialize, Deserialize, Default)]
#[sea_orm(table_name = "dye_batch_operation")]
pub struct Model {
    #[sea_orm(primary_key)]
    pub id: i32,

    /// 操作类型：merge 合缸/split 分缸/priority_adjust 优先级调整/batch_change 缸变更/schedule_change 计划变更/terminate 终止
    pub operation_type: String,
    /// 操作名称
    pub operation_name: String,
    /// 目标缸号 ID（主操作缸号）
    pub target_batch_id: i32,
    /// 目标缸号
    pub target_batch_no: String,
    /// 源缸号 ID 列表（合缸/分缸时使用，JSONB 数组）
    #[sea_orm(column_type = "Json", nullable)]
    pub source_batch_ids: Option<serde_json::Value>,
    /// 源缸号列表（JSONB 数组）
    #[sea_orm(column_type = "Json", nullable)]
    pub source_batch_nos: Option<serde_json::Value>,
    /// 操作数据（优先级值、变更前后信息等）
    #[sea_orm(column_type = "Json", nullable)]
    pub operation_data: Option<serde_json::Value>,
    /// 操作人 ID
    pub operator_id: Option<i32>,
    /// 操作人姓名
    pub operator_name: Option<String>,
    /// 操作时间
    pub operation_at: DateTimeWithTimeZone,
    /// 备注
    pub remarks: Option<String>,

    pub created_at: DateTimeWithTimeZone,
    pub updated_at: DateTimeWithTimeZone,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {
    // 无关系（独立操作记录表）
}

impl ActiveModelBehavior for ActiveModel {}
