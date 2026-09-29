#![allow(dead_code)]
use chrono::{DateTime, Utc};
use rust_decimal::Decimal;
use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Serialize, Deserialize)]
#[sea_orm(table_name = "budget_items")]
pub struct Model {
    #[sea_orm(primary_key)]
    pub id: i32,
    pub item_code: String,
    pub item_name: String,
    pub parent_id: Option<i32>,
    pub item_type: String,
    pub level: i32,
    pub status: String,
    /// 所属预算方案 ID（外键 → budget_plans.id，NOT NULL）；
    /// 并入方案主线，修复 budget_items 此前仅按 budget_year+account_subject_id 组织、
    /// 与 budget_plans/budget_versions/budget_executions 三表 plan 主线断裂的问题。
    pub plan_id: i32,
    /// v11 批次 145 P1-8：预算年度（可选，用于按年度筛选预算科目）
    pub budget_year: Option<i32>,
    /// v11 批次 145 P1-8：计划金额（该科目的年度计划预算总额 = Σ 各期间 planned_amount）
    #[sea_orm(column_type = "Decimal(Some((14, 2)))")]
    pub planned_amount: Decimal,
    /// v11 批次 145 P1-8：备注（最多 500 字符）
    #[sea_orm(column_type = "String(StringLen::N(500))")]
    pub remark: Option<String>,
    /// P2-14：预算科目-会计科目映射
    pub account_subject_id: Option<i32>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}

impl ActiveModelBehavior for ActiveModel {}
