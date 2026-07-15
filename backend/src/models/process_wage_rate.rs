//! 工序工价模型（process_wage_rate 表）
//!
//! v14 批次 427：产量工资核算贯通
//! 依据：面料行业真实业务调研文档 §12.5 产量工资（计件计时）
//! 真实业务：工价方案定义 → 每道工序的计件/计时单价 + A/B/C 等级系数
//! 等级系数业务规则：
//!   A 级（合格率≥95%）：全额 grade_a_ratio=1.0
//!   B 级（合格率 80-95%）：8 折 grade_b_ratio=0.8
//!   C 级（合格率<80%）：不计 grade_c_ratio=0.0

use rust_decimal::Decimal;
use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

/// 工序工价模型
///
/// 真实业务要点：
/// - 每道工序的计件/计时单价定义
/// - 等级系数联动质量分级（A/B/C）
/// - 支持生效日期与失效日期，便于工价调整
/// - 同一工序同一生效日期只能有一个工价（避免歧义）
#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Serialize, Deserialize, Default)]
#[sea_orm(table_name = "process_wage_rate")]
pub struct Model {
    #[sea_orm(primary_key)]
    pub id: i32,

    /// 工价单号：PWR-YYYYMMDDHHMMSS-NNN
    pub rate_no: String,
    /// 关联工序路线 ID
    pub process_route_id: i32,
    /// 工序编码（冗余）
    pub route_code: String,
    /// 工序名称（冗余）
    pub route_name: String,

    /// 工价类型：piece(计件) / time(计时) / mixed(混合)
    pub wage_type: String,

    /// 计件单价（元/单位产量，kg 或 m）
    #[sea_orm(column_type = "Decimal(Some((12, 4)))")]
    pub piece_price: Decimal,
    /// 计时单价（元/分钟）
    #[sea_orm(column_type = "Decimal(Some((12, 4)))")]
    pub time_price: Decimal,

    /// A 级等级系数（合格率≥95%，默认全额 1.0）
    #[sea_orm(column_type = "Decimal(Some((5, 4)))")]
    pub grade_a_ratio: Decimal,
    /// B 级等级系数（合格率 80-95%，默认 8 折 0.8）
    #[sea_orm(column_type = "Decimal(Some((5, 4)))")]
    pub grade_b_ratio: Decimal,
    /// C 级等级系数（合格率<80%，默认不计 0.0）
    #[sea_orm(column_type = "Decimal(Some((5, 4)))")]
    pub grade_c_ratio: Decimal,

    /// 生效日期
    pub effective_date: chrono::NaiveDate,
    /// 失效日期（NULL 表示长期有效）
    pub expiry_date: Option<chrono::NaiveDate>,
    /// 车间（用于按车间汇总）
    pub workshop: Option<String>,

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
        on_delete = "Restrict"
    )]
    ProcessRoute,
}

impl Related<super::process_route::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::ProcessRoute.def()
    }
}

impl ActiveModelBehavior for ActiveModel {}
