#![allow(dead_code)]
//! 出口退税申报表模型（export_refund_declarations 表）
//!
//! V15 P1 batch-08 缺陷 14：出口退税（免抵退）核算
//! 依据：财税[2012]39号 出口货物劳务增值税和消费税政策

use rust_decimal::Decimal;
use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

/// 出口退税申报表模型
///
/// 真实业务：按月汇总出口销售额 → 计算免抵退税额 → 生成退税申报表
#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Serialize, Deserialize, Default)]
#[sea_orm(table_name = "export_refund_declarations")]
pub struct Model {
    #[sea_orm(primary_key)]
    pub id: i32,
    /// 申报单号（唯一）
    pub declaration_no: String,
    /// 申报年度
    pub period_year: i32,
    /// 申报月份
    pub period_month: i32,
    /// 申报日期
    pub declaration_date: chrono::NaiveDate,
    /// 出口销售额（人民币）
    #[sea_orm(column_type = "Decimal(Some((14, 2)))")]
    pub export_sales_amount: Decimal,
    /// 应退税额（增值税）
    #[sea_orm(column_type = "Decimal(Some((14, 2)))")]
    pub refundable_vat_amount: Decimal,
    /// 免抵税额
    #[sea_orm(column_type = "Decimal(Some((14, 2)))")]
    pub exempt_vat_amount: Decimal,
    /// 应调增/调减税额
    #[sea_orm(column_type = "Decimal(Some((14, 2)))")]
    pub credit_vat_amount: Decimal,
    /// 实际退税额
    #[sea_orm(column_type = "Decimal(Some((14, 2)))")]
    pub actual_refund_amount: Decimal,
    /// 结转下期税额
    #[sea_orm(column_type = "Decimal(Some((14, 2)))")]
    pub carryforward_amount: Decimal,
    /// 退税率（小数，如 0.13）
    #[sea_orm(column_type = "Decimal(Some((6, 4)))")]
    pub refund_rate: Decimal,
    /// 单证是否齐全
    pub documents_complete: bool,
    /// 状态：draft(草稿) / submitted(已申报) / approved(已审批) / rejected(已退回)
    pub status: String,
    pub remarks: Option<String>,
    pub created_by: Option<i32>,
    pub created_at: DateTimeWithTimeZone,
    pub updated_at: DateTimeWithTimeZone,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}

impl ActiveModelBehavior for ActiveModel {}
