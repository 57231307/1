#![allow(dead_code, unused_imports, unused_variables)]
//! 预算方案 Entity
use chrono::{DateTime, NaiveDate, Utc};
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
    /// 部门ID
    pub department_id: i32,
    /// 总金额
    pub total_amount: Decimal,
    /// 开始日期
    pub start_date: NaiveDate,
    /// 结束日期
    pub end_date: NaiveDate,
    /// 状态：draft-草稿、approved-已审批、rejected-已驳回、active-执行中、closed-已关闭
    pub status: String,
    /// 备注
    pub remark: Option<String>,
    /// 创建人
    pub created_by: Option<i32>,
    #[sea_orm(column_type = "Timestamp")]
    /// 创建时间
    pub created_at: DateTime<Utc>,
    #[sea_orm(column_type = "Timestamp")]
    /// 更新时间
    pub updated_at: DateTime<Utc>,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}

impl ActiveModelBehavior for ActiveModel {}
