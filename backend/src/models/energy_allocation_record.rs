//! 能耗分摊记录模型（energy_allocation_record 表）
//!
//! v14 批次 428：能耗管理贯通
//! 依据：面料行业真实业务调研文档 §12.6 能耗管理
//! 真实业务：月末将总能耗按规则分摊到缸号/工序/订单，生成 cost_collection 记录
//! 状态机：draft(草稿) → confirmed(已确认) → cancelled(已取消)

use rust_decimal::Decimal;
use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

/// 能耗分摊记录模型
///
/// 真实业务要点：
/// - 月末将车间总能耗按规则分摊到缸号/工序/订单
/// - 记录分摊依据量（工时/产量）和分摊比例
/// - 计算单位能耗（allocated_consumption / output_quantity）
/// - 关联 cost_collection 实现成本归集
#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Serialize, Deserialize, Default)]
#[sea_orm(table_name = "energy_allocation_record")]
pub struct Model {
    #[sea_orm(primary_key)]
    pub id: i32,

    /// 分摊编号：EAR-YYYYMMDDHHMMSS-NNN
    pub allocation_no: String,
    /// 分摊时段开始
    pub period_start: DateTimeWithTimeZone,
    /// 分摊时段结束
    pub period_end: DateTimeWithTimeZone,
    /// 能源类型
    pub meter_type: String,
    /// 所属车间
    pub workshop: Option<String>,
    /// 关联分摊规则 ID
    pub allocation_rule_id: Option<i32>,
    /// 分摊基准（冗余）
    pub allocation_basis: String,
    /// 总消耗量
    #[sea_orm(column_type = "Decimal(Some((14, 2)))")]
    pub total_consumption: Decimal,
    /// 总成本
    #[sea_orm(column_type = "Decimal(Some((14, 2)))")]
    pub total_cost: Decimal,
    /// 关联缸号（分摊到具体缸号时使用）
    pub dye_lot_no: Option<String>,
    /// 关联生产订单 ID
    pub production_order_id: Option<i32>,
    /// 生产订单编号（冗余）
    pub production_order_no: Option<String>,
    /// 关联工序路线 ID
    pub process_route_id: Option<i32>,
    /// 工序编码（冗余）
    pub route_code: Option<String>,
    /// 关联流转卡 ID
    pub flow_card_id: Option<i32>,
    /// 分摊依据量（工时/产量/设备运行时长）
    #[sea_orm(column_type = "Decimal(Some((14, 2)))")]
    pub allocation_basis_value: Decimal,
    /// 分摊比例（0-1）
    #[sea_orm(column_type = "Decimal(Some((8, 4)))")]
    pub allocation_ratio: Decimal,
    /// 分摊消耗量
    #[sea_orm(column_type = "Decimal(Some((14, 2)))")]
    pub allocated_consumption: Decimal,
    /// 分摊成本
    #[sea_orm(column_type = "Decimal(Some((14, 2)))")]
    pub allocated_cost: Decimal,
    /// 单位产量（米/公斤，用于单位能耗分析）
    #[sea_orm(column_type = "Decimal(Some((14, 2)))")]
    pub output_quantity: Option<Decimal>,
    /// 单位能耗（allocated_consumption / output_quantity）
    #[sea_orm(column_type = "Decimal(Some((14, 4)))")]
    pub unit_consumption: Option<Decimal>,
    /// 关联成本归集 ID（月末分摊到成本时生成）
    pub cost_collection_id: Option<i32>,
    /// 状态：draft(草稿) → confirmed(已确认) → cancelled(已取消)
    pub status: String,
    /// 确认人
    pub confirmed_by: Option<i32>,
    /// 确认时间
    pub confirmed_at: Option<DateTimeWithTimeZone>,
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
    /// 关联分摊规则
    #[sea_orm(
        belongs_to = "super::energy_allocation_rule::Entity",
        from = "Column::AllocationRuleId",
        to = "super::energy_allocation_rule::Column::Id",
        on_update = "Cascade",
        on_delete = "SetNull"
    )]
    AllocationRule,
}

impl Related<super::energy_allocation_rule::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::AllocationRule.def()
    }
}

impl ActiveModelBehavior for ActiveModel {}
