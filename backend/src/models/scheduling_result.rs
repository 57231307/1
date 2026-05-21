#![allow(dead_code)]

//! 排程结果 Model
//!
//! 存储自动排程的结果记录

use chrono::{DateTime, NaiveDate, Utc};
use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

/// 排程结果状态
#[derive(Debug, Clone, PartialEq, Eq, EnumIter, DeriveActiveEnum, Serialize, Deserialize)]
#[sea_orm(rs_type = "String", db_type = "String(StringLen::N(20))")]
pub enum ScheduleResultStatus {
    /// 草稿
    #[sea_orm(string_value = "DRAFT")]
    Draft,
    /// 已确认
    #[sea_orm(string_value = "CONFIRMED")]
    Confirmed,
    /// 已取消
    #[sea_orm(string_value = "CANCELLED")]
    Cancelled,
}

/// 排程结果 Entity
#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Serialize, Deserialize)]
#[sea_orm(table_name = "scheduling_result")]
pub struct Model {
    #[sea_orm(primary_key)]
    pub id: i32,

    /// 排程批次号
    pub batch_no: String,

    /// 排程策略
    pub strategy: String,

    /// 排程状态
    pub status: String,

    /// 总工单数
    pub total_orders: i32,

    /// 已排程工单数
    pub scheduled_orders: i32,

    /// 未排程工单数
    pub unscheduled_orders: i32,

    /// 冲突数量
    pub conflict_count: i32,

    /// 排程开始日期
    pub schedule_start_date: NaiveDate,

    /// 排程结束日期
    pub schedule_end_date: NaiveDate,

    /// 排程详情(JSON格式)
    pub schedule_details: Option<Json>,

    /// 甘特图数据(JSON格式)
    pub gantt_data: Option<Json>,

    /// 冲突信息(JSON格式)
    pub conflicts: Option<Json>,

    /// 创建人ID
    pub created_by: i32,

    /// 创建人名称
    pub created_by_name: Option<String>,

    /// 备注
    pub remarks: Option<String>,

    /// 创建时间
    pub created_at: DateTime<Utc>,

    /// 更新时间
    pub updated_at: DateTime<Utc>,
}

/// 排程结果关联关系
#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}

impl ActiveModelBehavior for ActiveModel {}
