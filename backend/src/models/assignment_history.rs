#![allow(dead_code)]
// TODO(tech-debt): 业务接入或重评估后逐项移除；rustc 1.94+ 编译时由编译器报告具体死代码位置。
// TODO(tech-debt): 业务接入后逐项移除此标注；rustc 1.94+ 编译时由编译器报告具体死代码位置。

//! 客户分配历史 Model
//!
//! 存储客户分配/回收历史记录

use chrono::{DateTime, Utc};
use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

/// 分配操作类型
#[derive(Debug, Clone, PartialEq, Eq, EnumIter, DeriveActiveEnum, Serialize, Deserialize)]
#[sea_orm(rs_type = "String", db_type = "String(StringLen::N(20))")]
pub enum AssignmentAction {
    /// 分配
    #[sea_orm(string_value = "ASSIGN")]
    Assign,
    /// 回收
    #[sea_orm(string_value = "RECYCLE")]
    Recycle,
    /// 转移
    #[sea_orm(string_value = "TRANSFER")]
    Transfer,
    /// 领取
    #[sea_orm(string_value = "CLAIM")]
    Claim,
}

/// 客户分配历史 Entity
#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Serialize, Deserialize)]
#[sea_orm(table_name = "assignment_histories")]
pub struct Model {
    /// 记录 ID（主键）
    #[sea_orm(primary_key)]
    pub id: i32,

    /// 租户 ID
    pub tenant_id: i32,

    /// 线索/客户 ID
    pub lead_id: i32,

    /// 线索编号
    pub lead_no: String,

    /// 公司名称
    pub company_name: Option<String>,

    /// 原归属人用户 ID
    pub from_user_id: Option<i32>,

    /// 原归属人姓名
    pub from_user_name: Option<String>,

    /// 新归属人用户 ID
    pub to_user_id: Option<i32>,

    /// 新归属人姓名
    pub to_user_name: Option<String>,

    /// 操作类型
    pub action: String,

    /// 操作原因
    pub reason: Option<String>,

    /// 备注
    pub notes: Option<String>,

    /// 操作人用户 ID
    pub operated_by: i32,

    /// 操作人姓名
    pub operated_by_name: String,

    /// 创建时间
    pub created_at: DateTime<Utc>,
}

/// 客户分配历史关联关系
#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}

impl ActiveModelBehavior for ActiveModel {}
