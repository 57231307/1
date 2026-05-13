#![allow(dead_code)]

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

    /// 是否删除
    pub is_deleted: bool,

    /// 创建时间
    pub created_at: DateTime<Utc>,

    /// 更新时间
    pub updated_at: DateTime<Utc>,
}

/// 工作中心关联关系
#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}

impl ActiveModelBehavior for ActiveModel {}
