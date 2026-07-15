//! 缸号回修记录模型（dye_batch_rework 表）
//!
//! v14 批次 432：缸号全生命周期状态机
//! 依据：面料行业真实业务调研文档 §12.7 缸号状态机 + §3.2 缸号全生命周期追踪
//! 真实业务：记录回修订单重新进缸，回修类型色差/疵点/规格不符/其他

use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

/// 缸号回修记录模型
///
/// 真实业务要点：
/// - 回修类型 4 种：color_difference 色差/defect 疵点/specification_unqualified 规格不符/other 其他
/// - 只有 inspecting/stored 状态可发起回修
/// - 回修单状态机：draft → approved → in_progress → completed / cancelled
/// - 同缸回修时 rework_batch_id 与 original_batch_id 相同
#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Serialize, Deserialize, Default)]
#[sea_orm(table_name = "dye_batch_rework")]
pub struct Model {
    #[sea_orm(primary_key)]
    pub id: i32,

    /// 原缸号 ID
    pub original_batch_id: i32,
    /// 原缸号
    pub original_batch_no: String,
    /// 回修缸号 ID（若同缸回修则为原缸号）
    pub rework_batch_id: Option<i32>,
    /// 回修缸号（若同缸回修则同原缸号）
    pub rework_batch_no: Option<String>,
    /// 回修类型：color_difference 色差/defect 疵点/specification_unqualified 规格不符/other 其他
    pub rework_type: String,
    /// 回修原因
    pub rework_reason: String,
    /// 回修前状态：inspecting 或 stored
    pub original_status: String,
    /// 审批人 ID
    pub approved_by: Option<i32>,
    /// 审批时间
    pub approved_at: Option<DateTimeWithTimeZone>,
    /// 状态：draft 草稿/approved 已审批/in_progress 回修中/completed 已完成/cancelled 已取消
    pub status: String,
    /// 回修开始时间
    pub started_at: Option<DateTimeWithTimeZone>,
    /// 回修完成时间
    pub completed_at: Option<DateTimeWithTimeZone>,
    /// 备注
    pub remarks: Option<String>,
    /// 软删除
    pub is_deleted: bool,

    pub created_at: DateTimeWithTimeZone,
    pub updated_at: DateTimeWithTimeZone,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {
    // 无 belongs_to 关系（不加外键约束，用应用层校验）
}

impl ActiveModelBehavior for ActiveModel {}
