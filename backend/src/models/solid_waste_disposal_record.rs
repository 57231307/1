#![allow(dead_code)]
//! 固废处置联单模型（solid_waste_disposal_records 表）
//!
//! V15 P1 batch-08 缺陷 19：固废处置联单制度
//! 依据：《固体废物污染环境防治法》危险废物转移联单制度

use rust_decimal::Decimal;
use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

/// 固废处置联单模型
///
/// 真实业务：印染污泥（危废）处置需填写转移联单，记录处置全流程
#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Serialize, Deserialize, Default)]
#[sea_orm(table_name = "solid_waste_disposal_records")]
pub struct Model {
    #[sea_orm(primary_key)]
    pub id: i32,
    /// 联单号（唯一）
    pub manifest_no: String,
    /// 废物类型：sludge(污泥) / waste_fabric(废布) / chemical_waste(废化学剂)
    pub waste_type: String,
    /// 废物类别：hazardous(危废) / general(一般固废)
    pub waste_category: String,
    /// 废物数量
    #[sea_orm(column_type = "Decimal(Some((14, 4)))")]
    pub waste_amount: Decimal,
    /// 数量单位
    pub waste_unit: String,
    /// 产生日期
    pub generation_date: chrono::NaiveDate,
    /// 处置日期
    pub disposal_date: Option<chrono::NaiveDate>,
    /// 处置方式：landfill(填埋) / incineration(焚烧) / reuse(综合利用) / storage(暂存)
    pub disposal_method: String,
    /// 处置 vendor ID
    pub disposal_vendor_id: Option<i32>,
    /// 处置 vendor 名称
    pub disposal_vendor_name: Option<String>,
    /// 运输许可证号
    pub transport_license_no: Option<String>,
    /// 处置许可证号
    pub disposal_license_no: Option<String>,
    /// 状态：pending(待处置) / transporting(运输中) / disposed(已处置) / cancelled(已撤销)
    pub status: String,
    pub remarks: Option<String>,
    pub created_by: Option<i32>,
    pub created_at: DateTimeWithTimeZone,
    pub updated_at: DateTimeWithTimeZone,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}

impl ActiveModelBehavior for ActiveModel {}
