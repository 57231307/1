//! 角色变更审批模型（role_change_approvals 表）
//!
//! B12-P2-4：敏感角色变更双人审批

use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

/// 审批状态
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[derive(EnumIter, DeriveActiveEnum)]
#[sea_orm(rs_type = "String", db_type = "String(StringLen::N(20))")]
pub enum ApprovalStatus {
    #[sea_orm(string_value = "pending_l1")]
    PendingL1,
    #[sea_orm(string_value = "pending_l2")]
    PendingL2,
    #[sea_orm(string_value = "approved")]
    Approved,
    #[sea_orm(string_value = "rejected")]
    Rejected,
    #[sea_orm(string_value = "cancelled")]
    Cancelled,
}

/// 变更类型
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[derive(EnumIter, DeriveActiveEnum)]
#[sea_orm(rs_type = "String", db_type = "String(StringLen::N(20))")]
pub enum ChangeType {
    #[sea_orm(string_value = "assign_role")]
    AssignRole,
    #[sea_orm(string_value = "assign_permission")]
    AssignPermission,
    #[sea_orm(string_value = "remove_permission")]
    RemovePermission,
}

/// 角色变更审批模型
#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Serialize, Deserialize)]
#[sea_orm(table_name = "role_change_approvals")]
pub struct Model {
    #[sea_orm(primary_key)]
    pub id: i32,

    /// 审批单号
    #[sea_orm(unique)]
    pub approval_no: String,

    /// 变更类型
    pub change_type: String,

    /// 目标用户 ID（assign_role 时）
    pub target_user_id: Option<i32>,

    /// 目标角色 ID
    pub target_role_id: i32,

    /// 目标角色编码（冗余，便于判定）
    pub target_role_code: String,

    /// 提议的权限 ID（assign_permission 时）
    pub proposed_permission_id: Option<i32>,

    /// 提议的资源类型
    pub proposed_resource_type: Option<String>,

    /// 提议的操作
    pub proposed_action: Option<String>,

    /// 提议的允许/拒绝
    pub proposed_allowed: Option<bool>,

    /// 申请人 ID
    pub applicant_id: i32,

    /// 申请人用户名
    pub applicant_username: String,

    /// 一级审批人 ID
    pub approver1_id: Option<i32>,

    /// 一级审批意见
    pub approver1_comment: Option<String>,

    /// 一级审批时间
    pub approver1_at: Option<DateTimeWithTimeZone>,

    /// 二级审批人 ID
    pub approver2_id: Option<i32>,

    /// 二级审批意见
    pub approver2_comment: Option<String>,

    /// 二级审批时间
    pub approver2_at: Option<DateTimeWithTimeZone>,

    /// 审批状态
    pub status: String,

    /// 当前审批层级
    pub current_level: i32,

    /// 完成时间
    pub completed_at: Option<DateTimeWithTimeZone>,

    pub created_at: DateTimeWithTimeZone,
    pub updated_at: DateTimeWithTimeZone,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}

impl ActiveModelBehavior for ActiveModel {}
