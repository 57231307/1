#![allow(dead_code)]
// TODO(tech-debt): 业务接入或重评估后逐项移除；rustc 1.94+ 编译时由编译器报告具体死代码位置。
// TODO(tech-debt): 业务接入后逐项移除此标注；rustc 1.94+ 编译时由编译器报告具体死代码位置。

//! 应收账龄分析 Model
//!
//! 客户应收账款账龄分析数据

use chrono::{DateTime, NaiveDate, Utc};
use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

/// 应收账龄分析 Entity
#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Serialize, Deserialize)]
#[sea_orm(table_name = "ar_aging_analysis")]
pub struct Model {
    /// 分析 ID（主键）
    #[sea_orm(primary_key)]
    pub id: i32,

    /// 客户 ID
    pub customer_id: i32,

    /// 分析日期
    pub analysis_date: NaiveDate,

    /// 当前期金额（未逾期）
    pub current_amount: Decimal,

    /// 1-30天逾期金额
    pub days_1_30: Decimal,

    /// 31-60天逾期金额
    pub days_31_60: Decimal,

    /// 61-90天逾期金额
    pub days_61_90: Decimal,

    /// 90天以上逾期金额
    pub days_over_90: Decimal,

    /// 总金额
    pub total_amount: Decimal,

    /// 销售员 ID
    pub salesperson_id: Option<i32>,

    /// 创建时间
    pub created_at: DateTime<Utc>,

    /// 更新时间
    pub updated_at: DateTime<Utc>,
}

/// 应收账龄分析关联关系
#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}

impl ActiveModelBehavior for ActiveModel {}
