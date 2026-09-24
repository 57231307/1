#![allow(dead_code)]
//! 预算明细期间子表 Entity
//!
//! 一条预算明细（budget_items）按多期间（月 '2026-01' 或季 '2026-Q1'）多行金额分解，
//! 支持财务按月/季做期间预算。budget_items.planned_amount（年度总额）= Σ 本表各期间 planned_amount，
//! 由服务层在保存/审批时聚合校验，本表不单独约束该等式。
use chrono::{DateTime, Utc};
use rust_decimal::Decimal;
use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Serialize, Deserialize)]
#[sea_orm(table_name = "budget_item_periods")]
pub struct Model {
    #[sea_orm(primary_key)]
    /// 期间明细ID
    pub id: i32,
    /// 所属预算明细科目ID（外键 → budget_items.id）
    pub item_id: i32,
    /// 期间标识：月度 '2026-01'，季度 '2026-Q1'，年度聚合 '2026-FY'
    pub period: String,
    /// 该期间的计划金额
    #[sea_orm(column_type = "Decimal(Some((14, 2)))")]
    pub planned_amount: Decimal,
    /// 该期间的实际执行金额
    #[sea_orm(column_type = "Decimal(Some((14, 2)))")]
    pub actual_amount: Decimal,
    /// 创建时间
    pub created_at: DateTime<Utc>,
    /// 更新时间
    pub updated_at: DateTime<Utc>,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}

impl ActiveModelBehavior for ActiveModel {}
