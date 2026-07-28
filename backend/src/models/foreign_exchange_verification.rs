//! 外汇核销单模型（foreign_exchange_verifications 表）
//!
//! V15 P1 batch-08 缺陷 14：出口退税（免抵退）核算
//! 依据：外汇管理局核销制度

use rust_decimal::Decimal;
use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

/// 外汇核销单模型
///
/// 真实业务：出口收汇核销，作为免抵退税核算的"单证齐全"基础数据
#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Serialize, Deserialize, Default)]
#[sea_orm(table_name = "foreign_exchange_verifications")]
pub struct Model {
    #[sea_orm(primary_key)]
    pub id: i32,
    /// 核销单号（唯一）
    pub verification_no: String,
    /// 关联报关单
    pub customs_declaration_id: Option<i32>,
    /// 关联销售订单
    pub sales_order_id: Option<i32>,
    /// 核销日期
    pub verification_date: chrono::NaiveDate,
    /// 外币金额
    #[sea_orm(column_type = "Decimal(Some((14, 2)))")]
    pub foreign_currency_amount: Decimal,
    /// 人民币金额
    #[sea_orm(column_type = "Decimal(Some((14, 2)))")]
    pub rmb_amount: Decimal,
    /// 汇率
    #[sea_orm(column_type = "Decimal(Some((10, 4)))")]
    pub exchange_rate: Decimal,
    /// 银行编码
    pub bank_code: Option<String>,
    /// 状态：pending(待核销) / verified(已核销) / cancelled(已撤销)
    pub status: String,
    pub remarks: Option<String>,
    pub created_by: Option<i32>,
    pub created_at: DateTimeWithTimeZone,
    pub updated_at: DateTimeWithTimeZone,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}

impl ActiveModelBehavior for ActiveModel {}
