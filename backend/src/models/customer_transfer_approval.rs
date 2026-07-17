#![allow(dead_code)]
// TODO(tech-debt): 业务接入或重评估后逐项移除；rustc 1.94+ 编译时由编译器报告具体死代码位置。

//! 客户转移审批 Model
//!
//! V15 P0-S08 修复：CRM 客户转移多级审批流
//! 流程：销售员申请 → 销售经理审批 → 总监审批（大客户额外触发）
//! 与 assignment_history 互补：assignment_history 是流水（仅记录），approvals 是流程（状态机）
//!
//! 对应迁移：20260717000001_add_crm_pool_rule_and_transfer_approval

use chrono::{DateTime, Utc};
use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

/// 审批状态：待审批
pub const STATUS_PENDING: &str = "pending";
/// 审批状态：已通过
pub const STATUS_APPROVED: &str = "approved";
/// 审批状态：已拒绝
pub const STATUS_REJECTED: &str = "rejected";
/// 审批状态：已取消
pub const STATUS_CANCELLED: &str = "cancelled";

/// 客户转移审批 Entity
#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Serialize, Deserialize)]
#[sea_orm(table_name = "customer_transfer_approvals")]
pub struct Model {
    /// 审批 ID（主键）
    #[sea_orm(primary_key)]
    pub id: i32,

    /// 审批单号（唯一，TA 前缀 + 时间戳）
    #[sea_orm(unique)]
    pub approval_no: String,

    /// 客户/线索 ID（lead_id，与 crm_lead.id 关联）
    pub lead_id: i32,

    /// 客户名称（冗余字段，避免 join 查询）
    pub company_name: Option<String>,

    /// 原归属人 ID
    pub from_user_id: i32,

    /// 原归属人姓名（冗余字段）
    pub from_user_name: Option<String>,

    /// 新归属人 ID
    pub to_user_id: i32,

    /// 新归属人姓名（冗余字段）
    pub to_user_name: Option<String>,

    /// 申请人 ID
    pub applicant_id: i32,

    /// 申请原因（必填）
    pub reason: String,

    /// 是否大客户转移（信用额度超过阈值时自动标记）
    pub is_large_customer: bool,

    /// 审批状态：pending / approved / rejected / cancelled
    pub approval_status: String,

    /// 当前审批层级（1=销售经理审批中，2=总监审批中）
    pub current_level: i32,

    /// 最大审批层级（普通客户 1，大客户 2）
    pub max_level: i32,

    /// 销售经理审批人 ID
    pub manager_approver_id: Option<i32>,

    /// 销售经理审批意见
    pub manager_comment: Option<String>,

    /// 销售经理审批时间
    pub manager_approved_at: Option<DateTime<Utc>>,

    /// 总监审批人 ID（仅大客户）
    pub director_approver_id: Option<i32>,

    /// 总监审批意见
    pub director_comment: Option<String>,

    /// 总监审批时间
    pub director_approved_at: Option<DateTime<Utc>>,

    /// 最终完成时间（审批通过且转移执行完成）
    pub completed_at: Option<DateTime<Utc>>,

    /// 创建时间
    pub created_at: DateTime<Utc>,

    /// 更新时间
    pub updated_at: DateTime<Utc>,
}

/// 客户转移审批关联关系
#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}

impl ActiveModelBehavior for ActiveModel {}
