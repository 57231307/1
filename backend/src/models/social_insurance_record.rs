#![allow(dead_code)]
//! 社保公积金缴纳记录模型（social_insurance_records 表）
//!
//! V15 P1 batch-08 缺陷 23：社保公积金扣缴
//! 依据：《社会保险法》第58条 + 《住房公积金管理条例》第14条

use rust_decimal::Decimal;
use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

/// 社保公积金缴纳记录模型
///
/// 真实业务：按月扣缴五险一金，校验缴费基数合规性
#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Serialize, Deserialize, Default)]
#[sea_orm(table_name = "social_insurance_records")]
pub struct Model {
    #[sea_orm(primary_key)]
    pub id: i32,
    /// 工人ID
    pub worker_id: i32,
    /// 所属年度
    pub period_year: i32,
    /// 所属月份
    pub period_month: i32,
    /// 缴费基数（应为上年度月平均工资）
    #[sea_orm(column_type = "Decimal(Some((12, 2)))")]
    pub base_amount: Decimal,
    /// 养老保险单位部分
    #[sea_orm(column_type = "Decimal(Some((12, 2)))")]
    pub pension_employer: Decimal,
    /// 养老保险个人部分
    #[sea_orm(column_type = "Decimal(Some((12, 2)))")]
    pub pension_employee: Decimal,
    /// 医疗保险单位部分
    #[sea_orm(column_type = "Decimal(Some((12, 2)))")]
    pub medical_employer: Decimal,
    /// 医疗保险个人部分
    #[sea_orm(column_type = "Decimal(Some((12, 2)))")]
    pub medical_employee: Decimal,
    /// 失业保险单位部分
    #[sea_orm(column_type = "Decimal(Some((12, 2)))")]
    pub unemployment_employer: Decimal,
    /// 失业保险个人部分
    #[sea_orm(column_type = "Decimal(Some((12, 2)))")]
    pub unemployment_employee: Decimal,
    /// 工伤保险单位部分
    #[sea_orm(column_type = "Decimal(Some((12, 2)))")]
    pub work_injury_employer: Decimal,
    /// 生育保险单位部分
    #[sea_orm(column_type = "Decimal(Some((12, 2)))")]
    pub maternity_employer: Decimal,
    /// 公积金单位部分
    #[sea_orm(column_type = "Decimal(Some((12, 2)))")]
    pub housing_fund_employer: Decimal,
    /// 公积金个人部分
    #[sea_orm(column_type = "Decimal(Some((12, 2)))")]
    pub housing_fund_employee: Decimal,
    /// 单位缴纳合计
    #[sea_orm(column_type = "Decimal(Some((12, 2)))")]
    pub total_employer: Decimal,
    /// 个人缴纳合计
    #[sea_orm(column_type = "Decimal(Some((12, 2)))")]
    pub total_employee: Decimal,
    /// 状态：pending(待缴) / paid(已缴) / cancelled(已撤销)
    pub status: String,
    /// 缴纳日期
    pub payment_date: Option<chrono::NaiveDate>,
    pub remarks: Option<String>,
    pub created_by: Option<i32>,
    pub created_at: DateTimeWithTimeZone,
    pub updated_at: DateTimeWithTimeZone,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}

impl ActiveModelBehavior for ActiveModel {}
