#![allow(dead_code)]
//! 出口报关单模型（export_customs_declarations 表）
//!
//! V15 P1 batch-08 缺陷 14：出口退税（免抵退）核算
//! 依据：财税[2012]39号 出口货物劳务增值税和消费税政策

use rust_decimal::Decimal;
use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

/// 出口报关单模型
///
/// 真实业务：出口面料报关登记，作为免抵退税核算的"单证齐全"基础数据
#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Serialize, Deserialize, Default)]
#[sea_orm(table_name = "export_customs_declarations")]
pub struct Model {
    #[sea_orm(primary_key)]
    pub id: i32,
    /// 报关单号（唯一）
    pub declaration_no: String,
    /// 关联销售订单
    pub sales_order_id: Option<i32>,
    /// 客户ID
    pub customer_id: Option<i32>,
    /// 产品ID
    pub product_id: Option<i32>,
    /// 出口日期
    pub export_date: chrono::NaiveDate,
    /// 目的国
    pub destination_country: Option<String>,
    /// 币种
    pub currency_code: Option<String>,
    /// 报关总金额（原币）
    #[sea_orm(column_type = "Decimal(Some((14, 2)))")]
    pub total_amount: Decimal,
    /// 汇率
    #[sea_orm(column_type = "Decimal(Some((10, 4)))")]
    pub exchange_rate: Decimal,
    /// 海关编码
    pub customs_code: Option<String>,
    /// 状态：pending(待核销) / verified(已核销) / cancelled(已撤销)
    pub status: String,
    /// 备注
    pub remarks: Option<String>,
    pub created_by: Option<i32>,
    pub created_at: DateTimeWithTimeZone,
    pub updated_at: DateTimeWithTimeZone,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}

impl ActiveModelBehavior for ActiveModel {}
