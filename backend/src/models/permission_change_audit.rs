#![allow(dead_code)]
// TODO(tech-debt): 业务接入或重评估后逐项移除；rustc 1.94+ 编译时由编译器报告具体死代码位置。

use chrono::{DateTime, Utc};
use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

// V15 P0-S06：权限变更审计 model
// 记录角色权限变更和用户角色变更的审计日志，用于合规审查和安全追溯。
// 由 role_permission_service 和 user_service 在权限/角色变更时写入。

#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Eq, Serialize, Deserialize)]
#[sea_orm(table_name = "permission_change_audits")]
pub struct Model {
    #[sea_orm(primary_key)]
    pub id: i32,
    /// 变更类型：role_permission_assign / role_permission_remove / user_role_change
    pub change_type: String,
    /// 操作人 ID
    pub operator_id: i32,
    /// 受影响角色 ID
    pub role_id: Option<i32>,
    /// 受影响用户 ID（user_role_change 时有值）
    pub user_id: Option<i32>,
    /// 资源类型（role_permission 变更时有值）
    pub resource_type: Option<String>,
    /// 操作权限码（role_permission 变更时有值）
    pub action: Option<String>,
    /// 旧值（如旧 role_id / 旧 allowed）
    pub old_value: Option<String>,
    /// 新值（如新 role_id / 新 allowed）
    pub new_value: Option<String>,
    /// 变更时间
    pub changed_at: DateTime<Utc>,
    /// 客户端 IP
    pub client_ip: Option<String>,
    /// 备注
    pub remark: Option<String>,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}

impl ActiveModelBehavior for ActiveModel {}
