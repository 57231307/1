//! 排污许可证模型（pollution_permits 表）
//!
//! V15 P1 batch-08 缺陷 18：排污许可证登记
//! 依据：《环境保护法》第45条 + 《排污许可管理条例》第24条

use rust_decimal::Decimal;
use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

/// 排污许可证模型
///
/// 真实业务：登记排污许可证信息，到期前30日预警延续申请
#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Serialize, Deserialize, Default)]
#[sea_orm(table_name = "pollution_permits")]
pub struct Model {
    #[sea_orm(primary_key)]
    pub id: i32,
    /// 许可证编号（唯一）
    pub permit_no: String,
    /// 许可证类型：wastewater(废水) / exhaust(废气) / solid_waste(固废)
    pub permit_type: String,
    /// 许可证类别：general(通用) / special(专项)
    pub permit_category: Option<String>,
    /// 发证日期
    pub issue_date: chrono::NaiveDate,
    /// 到期日期
    pub expiry_date: chrono::NaiveDate,
    /// 发证机关
    pub issuing_authority: String,
    /// 许可排放量
    #[sea_orm(column_type = "Decimal(Some((14, 4)))")]
    pub permitted_capacity: Option<Decimal>,
    /// 容量单位
    pub capacity_unit: Option<String>,
    /// 许可排放污染物列表（JSON）
    pub permitted_pollutants: Option<serde_json::Value>,
    /// 状态：active(有效) / expired(过期) / revoked(吊销)
    pub status: String,
    pub remarks: Option<String>,
    pub created_by: Option<i32>,
    pub created_at: DateTimeWithTimeZone,
    pub updated_at: DateTimeWithTimeZone,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}

impl ActiveModelBehavior for ActiveModel {}
