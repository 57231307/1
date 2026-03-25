//! 预算执行明细 Entity
use sea_orm::entity::prelude::*;
use chrono::{DateTime, NaiveDate, Utc};
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};

/// 预算执行明细模型
/// 用于记录预算方案的实际执行情况
#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Serialize, Deserialize)]
#[sea_orm(table_name = "budget_executions")]
pub struct Model {
    #[sea_orm(primary_key)]
    /// 执行明细ID
    pub id: i32,
    /// 预算方案ID
    pub plan_id: i32,
    /// 执行类型：下达/调整/使用
    pub execution_type: String,
    /// 金额
    pub amount: Decimal,
    /// 费用类型
    pub expense_type: Option<String>,
    /// 费用日期
    pub expense_date: NaiveDate,
    /// 关联单据类型
    pub related_document_type: Option<String>,
    /// 关联单据ID
    pub related_document_id: Option<i32>,
    /// 备注
    pub remark: Option<String>,
    /// 创建人
    pub created_by: Option<i32>,
    #[sea_orm(column_type = "Timestamp")]
    /// 创建时间
    pub created_at: DateTime<Utc>,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}

impl ActiveModelBehavior for ActiveModel {}
