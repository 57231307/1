#![allow(dead_code)]

//! 应收对账 Model
//!
//! 客户应收对账单管理

use chrono::{DateTime, NaiveDate, Utc};
use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

/// 对账状态
#[derive(Debug, Clone, PartialEq, Eq, EnumIter, DeriveActiveEnum, Serialize, Deserialize)]
#[sea_orm(rs_type = "String", db_type = "String(StringLen::N(20))")]
pub enum ReconciliationStatus {
    /// 草稿
    #[sea_orm(string_value = "DRAFT")]
    Draft,
    /// 已发送
    #[sea_orm(string_value = "SENT")]
    Sent,
    /// 已确认
    #[sea_orm(string_value = "CONFIRMED")]
    Confirmed,
    /// 有争议
    #[sea_orm(string_value = "DISPUTED")]
    Disputed,
    /// 已关闭
    #[sea_orm(string_value = "CLOSED")]
    Closed,
}

/// 应收对账 Entity
#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Serialize, Deserialize)]
#[sea_orm(table_name = "ar_reconciliations")]
pub struct Model {
    /// 对账单 ID（主键）
    #[sea_orm(primary_key)]
    pub id: i32,

    /// 对账单编号
    #[sea_orm(unique)]
    pub reconciliation_no: String,

    /// 客户 ID
    pub customer_id: i32,

    /// 对账开始日期
    pub start_date: NaiveDate,

    /// 对账结束日期
    pub end_date: NaiveDate,

    /// 期初余额
    pub opening_balance: Decimal,

    /// 本期应收
    pub current_receivable: Decimal,

    /// 本期收款
    pub current_received: Decimal,

    /// 期末余额
    pub closing_balance: Decimal,

    /// 状态
    pub status: String,

    /// 客户确认日期
    pub confirmed_date: Option<NaiveDate>,

    /// 争议说明
    pub dispute_reason: Option<String>,

    /// 备注
    pub remarks: Option<String>,

    /// 是否删除
    pub is_deleted: bool,

    /// 创建人 ID
    pub created_by: Option<i32>,

    /// 客户确认人 ID
    pub confirmed_by: Option<i32>,

    /// 创建时间
    pub created_at: DateTime<Utc>,

    /// 更新时间
    pub updated_at: DateTime<Utc>,

    /// 客户确认时间
    pub confirmed_at: Option<DateTime<Utc>>,
}

/// 应收对账关联关系
#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}

impl ActiveModelBehavior for ActiveModel {}
