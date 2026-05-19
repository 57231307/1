#![allow(dead_code)]

//! 预算方案 Entity
use chrono::{DateTime, Utc};
use rust_decimal::Decimal;
use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

/// 预算方案模型
/// 用于管理年度预算方案的创建、审批和执行
#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Serialize, Deserialize)]
#[sea_orm(table_name = "budget_plans")]
pub struct Model {
    #[sea_orm(primary_key)]
    /// 方案ID
    pub id: i32,
    /// 方案编号
    pub plan_no: String,
    /// 方案名称
    pub plan_name: String,
    /// 预算年度
    pub budget_year: i32,
    /// 预算类型
    pub budget_type: String,
    /// 部门ID
    pub department_id: Option<i32>,
    /// 总金额
    pub total_amount: Decimal,
    /// 状态：draft-草稿、approved-已审批、rejected-已驳回、active-执行中、closed-已关闭
    pub status: Option<String>,
    /// 创建人
    pub prepared_by: Option<i32>,
    /// 审批人
    pub approved_by: Option<i32>,
    /// 审批时间
    pub approved_at: Option<DateTime<Utc>>,
    /// 备注
    pub remark: Option<String>,
    /// 创建时间
    pub created_at: Option<DateTime<Utc>>,
    /// 更新时间
    pub updated_at: Option<DateTime<Utc>>,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}

impl ActiveModelBehavior for ActiveModel {}
