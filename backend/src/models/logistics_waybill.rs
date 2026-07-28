#![allow(dead_code)]
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
    /// V15 P1 batch-19 缺陷 23.4.1：订单类型（sales_order/purchase_order/transfer_order）
    pub order_type: Option<String>,
    pub logistics_company: String,
    pub tracking_number: String,
    pub driver_name: Option<String>,
    pub driver_phone: Option<String>,
    pub freight_fee: Option<Decimal>,
    /// V15 P1 batch-19 缺陷 23.4.3：总重量（kg）
    pub total_weight: Option<Decimal>,
    /// V15 P1 batch-19 缺陷 23.4.3：总体积（m³）
    pub total_volume: Option<Decimal>,
    /// V15 P1 batch-19 缺陷 23.4.3：运输距离（km）
    pub distance_km: Option<Decimal>,
    /// V15 P1 batch-19 缺陷 23.4.3：运费费率（按重量/体积/距离核算的基准费率）
    pub freight_rate: Option<Decimal>,
    /// V15 P1 batch-19 缺陷 23.4.3：运费承担方（customer/company）
    pub freight_bearer: Option<String>,
    pub status: Option<String>,
    pub expected_arrival: Option<DateTime<Utc>>,
    pub actual_arrival: Option<DateTime<Utc>>,
    pub notes: Option<String>,
    /// V15 P0-B13：签收人 user_id，sign_waybill handler 自动填入
    pub signed_by: Option<i32>,
    /// V15 P0-B13：签收时间，触发 AR 应收确认
    pub signed_at: Option<DateTime<Utc>>,
    /// V15 P0-B13：纸质回单扫描件 URL
    pub sign_receipt_url: Option<String>,
    /// V15 P0-B13：现场签收照片 URL
    pub sign_photo_url: Option<String>,
    /// V15 P0-B13：签收备注（异常情况说明）
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
    /// V15 P1 batch-19 缺陷 23.4.2：关联跟踪事件历史（has_many）
    #[sea_orm(has_many = "super::logistics_tracking_event::Entity")]
    TrackingEvents,
}

impl Related<super::sales_order::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::SalesOrder.def()
    }
}

impl ActiveModelBehavior for ActiveModel {}
