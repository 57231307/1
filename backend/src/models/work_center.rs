#![allow(dead_code)]
// TODO(tech-debt): 业务接入或重评估后逐项移除；rustc 1.94+ 编译时由编译器报告具体死代码位置。

//! 工作中心 Model
//!
//! 工作中心（设备/产线）信息维护

use chrono::{DateTime, Utc};
use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

/// 工作中心状态
#[derive(Debug, Clone, PartialEq, Eq, EnumIter, DeriveActiveEnum, Serialize, Deserialize)]
#[sea_orm(rs_type = "String", db_type = "String(StringLen::N(20))")]
pub enum WorkCenterStatus {
    /// 正常
    #[sea_orm(string_value = "ACTIVE")]
    Active,
    /// 维修中
    #[sea_orm(string_value = "MAINTENANCE")]
    Maintenance,
    /// 停用
    #[sea_orm(string_value = "INACTIVE")]
    Inactive,
}

/// 工作中心 Entity
#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Serialize, Deserialize)]
#[sea_orm(table_name = "work_centers")]
pub struct Model {
    /// 工作中心 ID（主键）
    #[sea_orm(primary_key)]
    pub id: i32,

    /// 工作中心编号
    #[sea_orm(unique)]
    pub code: String,

    /// 工作中心名称
    pub name: String,

    /// 工作中心类型
    pub work_center_type: Option<String>,

    /// 产能（每日）
    pub daily_capacity: Option<Decimal>,

    /// 产能单位
    pub capacity_unit: Option<String>,

    /// 状态
    pub status: String,

    /// 备注
    pub remarks: Option<String>,

    /// 创建时间
    pub created_at: DateTime<Utc>,

    /// 更新时间
    pub updated_at: DateTime<Utc>,

    // P1 batch-18 缺陷 10.1：产能模型精细字段
    /// 标准工时（小时/单位），用于派生计算 daily_capacity
    #[sea_orm(column_type = "Decimal(Some((10, 2)))", nullable)]
    pub standard_hours_per_unit: Option<Decimal>,
    /// 设备数（默认 1）
    pub equipment_count: Option<i32>,
    /// 人员数（默认 1）
    pub worker_count: Option<i32>,
    /// 班次工时（小时，默认 8）
    #[sea_orm(column_type = "Decimal(Some((6, 2)))", nullable)]
    pub shift_hours: Option<Decimal>,

    // P1 batch-18 缺陷 11.3：调度异常自动重排开关
    /// 工作中心状态异常时是否自动重排受影响订单（默认 true）
    #[sea_orm(default_value = true)]
    pub auto_reschedule_enabled: bool,
}

/// 工作中心关联关系
#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}

impl ActiveModelBehavior for ActiveModel {}
