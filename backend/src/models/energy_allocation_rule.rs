//! 能耗分摊规则模型（energy_allocation_rule 表）
//!
//! v14 批次 428：能耗管理贯通
//! 依据：面料行业真实业务调研文档 §12.6 能耗管理
//! 真实业务：定义如何将车间总能耗分摊到缸号/工序/订单
//! 分摊基准：by_duration(按工时) / by_output(按产量) / by_equipment(按设备) / by_workshop(按车间)
//! 状态机：draft(草稿) → active(启用) → disabled(停用)

use rust_decimal::Decimal;
use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

/// 能耗分摊规则模型
///
/// 真实业务要点：
/// - 每道工序预设理论能耗基准（standard_consumption_per_unit）
/// - 支持生效/失效日期，便于规则调整
/// - 同车间同能源类型同工序同生效日期只能有一个规则
#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Serialize, Deserialize, Default)]
#[sea_orm(table_name = "energy_allocation_rule")]
pub struct Model {
    #[sea_orm(primary_key)]
    pub id: i32,

    /// 规则编号：EAR-YYYYMMDDHHMMSS-NNN
    pub rule_no: String,
    /// 规则名称
    pub rule_name: String,
    /// 能源类型
    pub meter_type: String,
    /// 分摊基准：by_duration(按工时) / by_output(按产量) / by_equipment(按设备) / by_workshop(按车间)
    pub allocation_basis: String,
    /// 所属车间
    pub workshop: Option<String>,
    /// 关联工序路线 ID（按工序归集时使用）
    pub process_route_id: Option<i32>,
    /// 工序编码（冗余）
    pub route_code: Option<String>,
    /// 生效日期
    pub effective_date: chrono::NaiveDate,
    /// 失效日期（NULL 表示长期有效）
    pub expiry_date: Option<chrono::NaiveDate>,
    /// 标准单位能耗（每米布/每缸/每公斤的理论消耗）
    #[sea_orm(column_type = "Decimal(Some((14, 4)))")]
    pub standard_consumption_per_unit: Decimal,
    /// 标准单位（米/缸/公斤/小时）
    pub standard_unit: Option<String>,
    /// 状态：draft(草稿) → active(启用) → disabled(停用)
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
    /// 关联工序路线
    #[sea_orm(
        belongs_to = "super::process_route::Entity",
        from = "Column::ProcessRouteId",
        to = "super::process_route::Column::Id",
        on_update = "Cascade",
        on_delete = "SetNull"
    )]
    ProcessRoute,
    /// 一对多：能耗分摊记录
    #[sea_orm(has_many = "super::energy_allocation_record::Entity")]
    AllocationRecords,
}

impl Related<super::process_route::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::ProcessRoute.def()
    }
}

impl Related<super::energy_allocation_record::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::AllocationRecords.def()
    }
}

impl ActiveModelBehavior for ActiveModel {}
