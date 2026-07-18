#![allow(dead_code)]
// TODO(tech-debt): 业务接入或重评估后逐项移除；rustc 1.94+ 编译时由编译器报告具体死代码位置。
use chrono::{DateTime, Utc};
use rust_decimal::Decimal;
use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Serialize, Deserialize)]
#[sea_orm(table_name = "logistics_waybills")]
pub struct Model {
    #[sea_orm(primary_key)]
    pub id: i32,
    pub order_id: i32,
    pub logistics_company: String,
    pub tracking_number: String,
    pub driver_name: Option<String>,
    pub driver_phone: Option<String>,
    pub freight_fee: Option<Decimal>,
    pub status: Option<String>,
    pub expected_arrival: Option<DateTime<Utc>>,
    pub actual_arrival: Option<DateTime<Utc>>,
    pub notes: Option<String>,
    /// V15 P0-B13：签收人 user_id（关联 users.id，sign_waybill handler 自动填入）
    pub signed_by: Option<i32>,
    /// V15 P0-B13：签收时间（客户实际签收的时间戳，触发 AR 应收确认）
    pub signed_at: Option<DateTime<Utc>>,
    /// V15 P0-B13：纸质回单扫描件 URL（上传到对象存储后返回）
    pub sign_receipt_url: Option<String>,
    /// V15 P0-B13：现场签收照片 URL（上传到对象存储后返回）
    pub sign_photo_url: Option<String>,
    /// V15 P0-B13：签收备注（异常情况说明，如包装破损/数量短缺）
    pub sign_remark: Option<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {
    #[sea_orm(
        belongs_to = "super::sales_order::Entity",
        from = "Column::OrderId",
        to = "super::sales_order::Column::Id"
    )]
    SalesOrder,
}

impl Related<super::sales_order::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::SalesOrder.def()
    }
}

impl ActiveModelBehavior for ActiveModel {}
