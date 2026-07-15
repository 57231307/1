//! 能耗记录模型（energy_consumption_record 表）
//!
//! v14 批次 428：能耗管理贯通
//! 依据：面料行业真实业务调研文档 §12.6 能耗管理
//! 真实业务：按时间段登记能耗（手工或 IoT 自动采集），可关联缸号/工序/订单
//! 状态机：draft(草稿) → confirmed(已确认) → cancelled(已取消)

use rust_decimal::Decimal;
use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

/// 能耗记录模型
///
/// 真实业务要点：
/// - 按时间段（日/班次/月）登记能耗
/// - 支持手工录入和 IoT 自动采集
/// - 可直接关联缸号/工序/设备（直接归集场景）
#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Serialize, Deserialize, Default)]
#[sea_orm(table_name = "energy_consumption_record")]
pub struct Model {
    #[sea_orm(primary_key)]
    pub id: i32,

    /// 记录编号：EC-YYYYMMDDHHMMSS-NNN
    pub record_no: String,
    /// 关联计量设备 ID
    pub meter_id: Option<i32>,
    /// 能源类型（冗余）
    pub meter_type: String,
    /// 所属车间
    pub workshop: Option<String>,
    /// 计量单位
    pub unit: String,
    /// 上次读数
    #[sea_orm(column_type = "Decimal(Some((14, 2)))")]
    pub previous_reading: Decimal,
    /// 当前读数
    #[sea_orm(column_type = "Decimal(Some((14, 2)))")]
    pub current_reading: Decimal,
    /// 消耗量（current_reading - previous_reading）
    #[sea_orm(column_type = "Decimal(Some((14, 2)))")]
    pub consumption: Decimal,
    /// 单价
    #[sea_orm(column_type = "Decimal(Some((12, 4)))")]
    pub unit_price: Decimal,
    /// 总成本（consumption × unit_price）
    #[sea_orm(column_type = "Decimal(Some((14, 2)))")]
    pub total_cost: Decimal,
    /// 记录时段开始
    pub period_start: DateTimeWithTimeZone,
    /// 记录时段结束
    pub period_end: DateTimeWithTimeZone,
    /// 录入方式：manual(手工) / iot(IoT 自动) / auto_calc(自动计算)
    pub recording_method: String,
    /// 关联缸号（直接归集时使用）
    pub dye_lot_no: Option<String>,
    /// 关联工序路线 ID（按工序归集时使用）
    pub process_route_id: Option<i32>,
    /// 工序编码（冗余）
    pub route_code: Option<String>,
    /// 关联设备 ID
    pub equipment_id: Option<i32>,
    /// 设备名称（冗余）
    pub equipment_name: Option<String>,
    /// 操作员 ID
    pub operator_id: Option<i32>,
    /// 录入时间
    pub recorded_at: DateTimeWithTimeZone,
    /// 状态：draft(草稿) → confirmed(已确认) → cancelled(已取消)
    pub status: String,
    /// 备注
    pub remarks: Option<String>,

    // 软删除与审计
    pub is_deleted: bool,
    pub created_by: Option<i32>,
    pub created_at: DateTimeWithTimeZone,
    pub updated_at: DateTimeWithTimeZone,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {
    /// 关联计量设备
    #[sea_orm(
        belongs_to = "super::energy_meter::Entity",
        from = "Column::MeterId",
        to = "super::energy_meter::Column::Id",
        on_update = "Cascade",
        on_delete = "SetNull"
    )]
    Meter,
    /// 关联工序路线
    #[sea_orm(
        belongs_to = "super::process_route::Entity",
        from = "Column::ProcessRouteId",
        to = "super::process_route::Column::Id",
        on_update = "Cascade",
        on_delete = "SetNull"
    )]
    ProcessRoute,
}

impl Related<super::energy_meter::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::Meter.def()
    }
}

impl Related<super::process_route::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::ProcessRoute.def()
    }
}

impl ActiveModelBehavior for ActiveModel {}
