//! 劳动合同模型（labor_contracts 表）
//!
//! V15 P1 batch-08 缺陷 21：劳动合同电子化管理
//! 依据：《劳动法》《劳动合同法》第10/19/20条

use rust_decimal::Decimal;
use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

/// 劳动合同模型
///
/// 真实业务：管理劳动合同签订/续签/终止，校验试用期合规性，到期预警
#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Serialize, Deserialize, Default)]
#[sea_orm(table_name = "labor_contracts")]
pub struct Model {
    #[sea_orm(primary_key)]
    pub id: i32,
    /// 工人ID
    pub worker_id: i32,
    /// 合同编号（唯一）
    pub contract_no: String,
    /// 合同类型：fixed_term(固定期限) / permanent(无固定期限) / task_based(任务制)
    pub contract_type: String,
    /// 合同开始日期
    pub start_date: chrono::NaiveDate,
    /// 合同结束日期（无固定期限为 None）
    pub end_date: Option<chrono::NaiveDate>,
    /// 试用期结束日期
    pub probation_end_date: Option<chrono::NaiveDate>,
    /// 试用期工资（需 ≥ 转正工资 80%）
    #[sea_orm(column_type = "Decimal(Some((12, 2)))")]
    pub probation_salary: Decimal,
    /// 转正工资
    #[sea_orm(column_type = "Decimal(Some((12, 2)))")]
    pub regular_salary: Decimal,
    /// 岗位
    pub position: Option<String>,
    /// 部门
    pub department: Option<String>,
    /// 工作地点
    pub work_location: Option<String>,
    /// 工时制度：standard(标准) / comprehensive(综合) / flexible(不定)
    pub working_hours_system: String,
    /// 签订日期
    pub sign_date: chrono::NaiveDate,
    /// 状态：active(有效) / expired(过期) / terminated(已终止)
    pub status: String,
    /// 终止日期
    pub termination_date: Option<chrono::NaiveDate>,
    /// 终止原因
    pub termination_reason: Option<String>,
    pub remarks: Option<String>,
    pub created_by: Option<i32>,
    pub created_at: DateTimeWithTimeZone,
    pub updated_at: DateTimeWithTimeZone,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}

impl ActiveModelBehavior for ActiveModel {}
