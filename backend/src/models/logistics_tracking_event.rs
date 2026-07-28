#![allow(dead_code)]
use chrono::{DateTime, Utc};
use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

/// 物流跟踪事件实体（运单轨迹历史）
#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Serialize, Deserialize)]
#[sea_orm(table_name = "logistics_tracking_events")]
pub struct Model {
    #[sea_orm(primary_key)]
    pub id: i64,
    /// 运单 ID（关联 logistics_waybills.id）
    pub waybill_id: i32,
    /// 事件时间（快递公司上报时间或手工录入时间）
    pub event_time: DateTime<Utc>,
    /// 事件发生地点（如"上海转运中心"）
    pub location: Option<String>,
    /// 事件描述（如"已揽收"/"运输中"/"派送中"/"已签收"）
    pub description: String,
    /// 事件类型：picked_up / in_transit / arrived_at_hub / out_for_delivery / delivered / exception
    pub event_type: String,
    /// 数据来源：manual（手工录入）/ express_api（快递 API 同步）
    pub data_source: String,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {
    #[sea_orm(
        belongs_to = "super::logistics_waybill::Entity",
        from = "Column::WaybillId",
        to = "super::logistics_waybill::Column::Id"
    )]
    Waybill,
}

impl Related<super::logistics_waybill::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::Waybill.def()
    }
}

impl ActiveModelBehavior for ActiveModel {}
