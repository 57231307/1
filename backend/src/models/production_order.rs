#![allow(dead_code)]
// TODO(tech-debt): 业务接入或重评估后逐项移除；rustc 1.94+ 编译时由编译器报告具体死代码位置。

//! 生产订单 Model
//!
//! 生产订单模块，用于管理面料行业的生产任务

use chrono::{DateTime, NaiveDate, Utc};
use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

/// 生产订单状态
#[derive(Debug, Clone, PartialEq, Eq, EnumIter, DeriveActiveEnum, Serialize, Deserialize)]
#[sea_orm(rs_type = "String", db_type = "String(StringLen::N(20))")]
pub enum ProductionOrderStatus {
    /// 草稿
    #[sea_orm(string_value = "DRAFT")]
    Draft,
    /// 已排产
    #[sea_orm(string_value = "SCHEDULED")]
    Scheduled,
    /// 生产中
    #[sea_orm(string_value = "IN_PROGRESS")]
    InProgress,
    /// 已完成
    #[sea_orm(string_value = "COMPLETED")]
    Completed,
    /// 已取消
    #[sea_orm(string_value = "CANCELLED")]
    Cancelled,
}

/// 生产订单 Entity
#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Serialize, Deserialize)]
#[sea_orm(table_name = "production_orders")]
pub struct Model {
    /// 生产订单 ID（主键）
    #[sea_orm(primary_key)]
    pub id: i32,

    /// 生产订单编号
    #[sea_orm(unique)]
    pub order_no: String,

    /// 关联销售订单 ID
    pub sales_order_id: Option<i32>,

    /// 产品 ID
    pub product_id: i32,

    /// 计划生产数量
    pub planned_quantity: Decimal,

    /// 实际完成数量
    pub actual_quantity: Option<Decimal>,

    /// 计划开始日期
    pub planned_start_date: Option<NaiveDate>,

    /// 计划完成日期
    pub planned_end_date: Option<NaiveDate>,

    /// 实际开始日期
    pub actual_start_date: Option<NaiveDate>,

    /// 实际完成日期
    pub actual_end_date: Option<NaiveDate>,

    /// 状态
    pub status: String,

    /// 优先级（1-10，1最高）
    pub priority: i32,

    /// 工作中心 ID
    pub work_center_id: Option<i32>,

    /// 备注
    pub remarks: Option<String>,

    /// 创建人 ID
    pub created_by: i32,

    /// 是否删除

    /// 创建时间
    pub created_at: DateTime<Utc>,

    /// 更新时间
    pub updated_at: DateTime<Utc>,
}

/// 生产订单关联关系
#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {
    /// 生产订单 - 销售订单（多对一）
    #[sea_orm(
        belongs_to = "super::sales_order::Entity",
        from = "Column::SalesOrderId",
        to = "super::sales_order::Column::Id",
        on_delete = "SetNull"
    )]
    SalesOrder,
}

impl Related<super::sales_order::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::SalesOrder.def()
    }
}

impl ActiveModelBehavior for ActiveModel {}
