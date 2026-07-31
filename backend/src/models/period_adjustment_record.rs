#![allow(dead_code)]
//! 期末调整记录模型（period_adjustment_record 表）
//!
//! V15 P2 B05-P2-10：期末调整机制（暂估 / 摊销 / 预提）
//! 依据：企业会计准则权责发生制——期末需对已发生尚未入账的业务做调整分录，
//!   保证收入与费用配比、资产与负债完整。
//! 三类调整：
//! - estimate（暂估）：已收货/已受益未取得发票，暂估入账，下月初红字冲销
//! - amortization（摊销）：待摊费用按受益期分摊（如保险费/租金）
//! - provision（预提）：已发生未支付的费用预提入账（如利息/水电）
//! 状态机：draft(草稿) → confirmed(已确认，生成凭证) → reversed(已冲销，生成红字凭证) / cancelled(已取消)

use rust_decimal::Decimal;
use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

/// 期末调整记录模型（按期末调整类型登记借贷科目与金额，确认生成凭证，暂估类下月初红字冲销）
#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Serialize, Deserialize, Default)]
#[sea_orm(table_name = "period_adjustment_record")]
pub struct Model {
    #[sea_orm(primary_key)]
    pub id: i32,

    /// 调整单号：PA-YYYYMMDDHHMMSS-NNN
    pub adjustment_no: String,
    /// 调整类型：estimate(暂估) / amortization(摊销) / provision(预提)
    pub adjustment_type: String,
    /// 所属会计期间（YYYY-MM）
    pub period: String,
    /// 摘要描述
    pub description: String,

    /// 借方科目编码
    pub debit_subject_code: String,
    /// 借方科目名称（冗余）
    pub debit_subject_name: String,
    /// 贷方科目编码
    pub credit_subject_code: String,
    /// 贷方科目名称（冗余）
    pub credit_subject_name: String,
    /// 调整金额
    #[sea_orm(column_type = "Decimal(Some((14, 2)))")]
    pub amount: Decimal,

    /// 来源类型（如 purchase_receipt / prepaid_expense / accrued_expense）
    pub source_type: Option<String>,
    /// 来源单据 ID
    pub source_bill_id: Option<i32>,
    /// 来源单据编号（冗余）
    pub source_bill_no: Option<String>,

    /// 确认时生成的凭证 ID
    pub voucher_id: Option<i32>,
    /// 冲销时生成的红字凭证 ID
    pub reverse_voucher_id: Option<i32>,

    /// 状态：draft(草稿) → confirmed(已确认) → reversed(已冲销) / cancelled(已取消)
    pub status: String,
    /// 确认人 ID
    pub confirmed_by: Option<i32>,
    /// 确认时间
    pub confirmed_at: Option<DateTimeWithTimeZone>,
    /// 冲销人 ID
    pub reversed_by: Option<i32>,
    /// 冲销时间
    pub reversed_at: Option<DateTimeWithTimeZone>,

    /// 备注
    pub remarks: Option<String>,

    // 软删除与审计
    pub is_deleted: bool,
    pub created_by: Option<i32>,
    pub created_at: DateTimeWithTimeZone,
    pub updated_at: DateTimeWithTimeZone,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}

impl ActiveModelBehavior for ActiveModel {}
