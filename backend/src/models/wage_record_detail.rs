//! 工资明细模型（wage_record_detail 表）
//!
//! v14 批次 427：产量工资核算贯通
//! 依据：面料行业真实业务调研文档 §12.5 产量工资（计件计时）
//! 真实业务：工资计算生成的每个工人每道工序的明细记录
//! 三维度产量统计：工序产量 + 设备产量 + 工人产量工资

use rust_decimal::Decimal;
use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

/// 工资明细模型
///
/// 真实业务要点：
/// - 每条明细对应一个工人的一道工序记录
/// - 工资计算公式：wage_amount = piece_wage + time_wage
///   * piece_wage = qualified_quantity × piece_price × grade_ratio
///   * time_wage = duration_minutes × time_price × grade_ratio
/// - 等级系数依据合格率判定：A=grade_a_ratio / B=grade_b_ratio / C=grade_c_ratio
#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Serialize, Deserialize, Default)]
#[sea_orm(table_name = "wage_record_detail")]
pub struct Model {
    #[sea_orm(primary_key)]
    pub id: i32,

    /// 关联工资记录 ID
    pub wage_record_id: i32,
    /// 关联工序记录 ID（数据来源）
    pub step_record_id: i32,
    /// 关联流转卡 ID（冗余，便于追溯）
    pub flow_card_id: Option<i32>,
    /// 缸号（冗余）
    pub dye_lot_no: Option<String>,
    /// 关联工序路线 ID
    pub process_route_id: Option<i32>,
    /// 工序编码（冗余）
    pub route_code: Option<String>,
    /// 工序名称（冗余）
    pub route_name: Option<String>,
    /// 工序类型（冗余）
    pub process_type: Option<String>,

    /// 工人 ID
    pub worker_id: i32,
    /// 工人姓名（冗余，便于报表）
    pub worker_name: Option<String>,
    /// 设备 ID（冗余，设备产量统计维度）
    pub equipment_id: Option<i32>,
    /// 设备名称（冗余）
    pub equipment_name: Option<String>,

    /// 工价类型快照：piece/time/mixed
    pub wage_type: String,
    /// 质检等级：A/B/C（依据合格率判定）
    pub grade: String,

    /// 实际产量（kg/m，来自工序记录）
    #[sea_orm(column_type = "Decimal(Some((12, 2)))")]
    pub actual_quantity: Decimal,
    /// 合格产量（kg/m，来自工序记录）
    #[sea_orm(column_type = "Decimal(Some((12, 2)))")]
    pub qualified_quantity: Decimal,
    /// 合格率（百分比，0-100）
    #[sea_orm(column_type = "Decimal(Some((6, 2)))")]
    pub qualification_rate: Decimal,

    /// 计件单价快照（元/单位产量）
    #[sea_orm(column_type = "Decimal(Some((12, 4)))")]
    pub piece_price: Decimal,
    /// 计时单价快照（元/分钟）
    #[sea_orm(column_type = "Decimal(Some((12, 4)))")]
    pub time_price: Decimal,
    /// 等级系数快照（依据等级确定）
    #[sea_orm(column_type = "Decimal(Some((5, 4)))")]
    pub grade_ratio: Decimal,
    /// 工时（分钟，来自工序记录）
    pub duration_minutes: i32,

    /// 计件工资部分（合格产量 × 计件单价 × 等级系数）
    #[sea_orm(column_type = "Decimal(Some((12, 2)))")]
    pub piece_wage: Decimal,
    /// 计时工资部分（工时 × 计时单价 × 等级系数）
    #[sea_orm(column_type = "Decimal(Some((12, 2)))")]
    pub time_wage: Decimal,
    /// 应得工资（piece_wage + time_wage）
    #[sea_orm(column_type = "Decimal(Some((12, 2)))")]
    pub wage_amount: Decimal,

    /// 备注
    pub remarks: Option<String>,

    // 审计
    pub is_deleted: bool,
    pub created_at: DateTimeWithTimeZone,
    pub updated_at: DateTimeWithTimeZone,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {
    /// 关联工资记录
    #[sea_orm(
        belongs_to = "super::wage_record::Entity",
        from = "Column::WageRecordId",
        to = "super::wage_record::Column::Id",
        on_update = "Cascade",
        on_delete = "Cascade"
    )]
    WageRecord,

    /// 关联工序记录（数据来源）
    #[sea_orm(
        belongs_to = "super::process_step_record::Entity",
        from = "Column::StepRecordId",
        to = "super::process_step_record::Column::Id",
        on_update = "Cascade",
        on_delete = "Restrict"
    )]
    StepRecord,

    /// 关联流转卡
    #[sea_orm(
        belongs_to = "super::production_flow_card::Entity",
        from = "Column::FlowCardId",
        to = "super::production_flow_card::Column::Id",
        on_update = "Cascade",
        on_delete = "SetNull"
    )]
    FlowCard,

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

impl Related<super::wage_record::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::WageRecord.def()
    }
}

impl Related<super::process_step_record::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::StepRecord.def()
    }
}

impl Related<super::production_flow_card::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::FlowCard.def()
    }
}

impl Related<super::process_route::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::ProcessRoute.def()
    }
}

impl ActiveModelBehavior for ActiveModel {}
