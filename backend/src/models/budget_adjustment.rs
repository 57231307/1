#![allow(dead_code, unused_imports, unused_variables)]
//! 预算调整 Model
//!
//! 预算调整模块

use chrono::{DateTime, NaiveDate, Utc};
use rust_decimal::Decimal;
use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

/// 预算调整 Entity
#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Serialize, Deserialize)]
#[sea_orm(table_name = "budget_adjustments")]
pub struct Model {
    /// 调整 ID（主键）
    #[sea_orm(primary_key)]
    pub id: i32,

    /// 调整单号
    #[sea_orm(unique)]
    pub adjustment_no: String,

    /// 预算 ID（外键）
    pub budget_id: i32,

    /// 调整日期
    pub adjustment_date: NaiveDate,

    /// 调整类型：INCREASE=增加，DECREASE=减少
    pub adjustment_type: String,

    /// 调整金额
    pub amount: Decimal,

    /// 调整前预算
    pub budget_before: Decimal,

    /// 调整后预算
    pub budget_after: Decimal,

    /// 调整原因
    pub reason: String,

    /// 审批状态：DRAFT=草稿，PENDING=待审批，APPROVED=已审批，REJECTED=已拒绝
    pub approval_status: String,

    /// 备注
    pub remarks: Option<String>,

    /// 创建人 ID
    pub created_by: i32,

    /// 创建时间
    pub created_at: DateTime<Utc>,

    /// 更新时间
    pub updated_at: DateTime<Utc>,
}

/// 预算调整关联关系
#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}

impl ActiveModelBehavior for ActiveModel {}
