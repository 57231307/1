#![allow(dead_code)]
// TODO(tech-debt): 业务接入或重评估后逐项移除；rustc 1.94+ 编译时由编译器报告具体死代码位置。
// TODO(tech-debt): 业务接入后逐项移除此标注；rustc 1.94+ 编译时由编译器报告具体死代码位置。

//! 应收对账明细 Model
//!
//! 对账单明细行项目，记录每笔应收/收款明细

use chrono::{DateTime, Utc};
use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

/// 明细类型
#[derive(Debug, Clone, PartialEq, Eq, EnumIter, DeriveActiveEnum, Serialize, Deserialize)]
#[sea_orm(rs_type = "String", db_type = "String(StringLen::N(20))")]
pub enum ItemType {
    /// 期初余额
    #[sea_orm(string_value = "OPENING")]
    Opening,
    /// 销售发票
    #[sea_orm(string_value = "INVOICE")]
    Invoice,
    /// 收款
    #[sea_orm(string_value = "RECEIPT")]
    Receipt,
    /// 调整
    #[sea_orm(string_value = "ADJUSTMENT")]
    Adjustment,
    /// 争议
    #[sea_orm(string_value = "DISPUTE")]
    Dispute,
}

/// 匹配状态
#[derive(Debug, Clone, PartialEq, Eq, EnumIter, DeriveActiveEnum, Serialize, Deserialize)]
#[sea_orm(rs_type = "String", db_type = "String(StringLen::N(20))")]
pub enum MatchStatus {
    /// 未匹配
    #[sea_orm(string_value = "UNMATCHED")]
    Unmatched,
    /// 已匹配
    #[sea_orm(string_value = "MATCHED")]
    Matched,
    /// 部分匹配
    #[sea_orm(string_value = "PARTIAL")]
    Partial,
}

/// 应收对账明细 Entity
#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Serialize, Deserialize)]
#[sea_orm(table_name = "ar_reconciliation_items")]
pub struct Model {
    /// 明细 ID（主键）
    #[sea_orm(primary_key)]
    pub id: i32,

    /// 对账单 ID
    pub reconciliation_id: i32,

    /// 明细类型
    pub item_type: String,

    /// 业务单据类型（如 SALES_INVOICE, RECEIPT）
    pub document_type: Option<String>,

    /// 业务单据 ID
    pub document_id: Option<i32>,

    /// 业务单据编号
    pub document_no: Option<String>,

    /// 业务日期
    pub document_date: Option<chrono::NaiveDate>,

    /// 金额（应收为正，收款为负）
    pub amount: Decimal,

    /// 匹配金额
    pub matched_amount: Option<Decimal>,

    /// 匹配状态
    pub match_status: String,

    /// 匹配的对账单明细 ID（自关联）
    pub matched_item_id: Option<i32>,

    /// 备注
    pub remarks: Option<String>,

    /// 创建时间
    pub created_at: DateTime<Utc>,

    /// 更新时间
    pub updated_at: DateTime<Utc>,
}

/// 应收对账明细关联关系
#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {
    #[sea_orm(
        belongs_to = "super::ar_reconciliation::Entity",
        from = "Column::ReconciliationId",
        to = "super::ar_reconciliation::Column::Id"
    )]
    Reconciliation,
}

impl ActiveModelBehavior for ActiveModel {}
