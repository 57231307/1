//! 工资记录模型（wage_record 表）
//!
//! v14 批次 427：产量工资核算贯通
//! 依据：面料行业真实业务调研文档 §12.5 产量工资（计件计时）
//! 真实业务：按周期/车间批量计算工人工资，生成工资单
//! 状态机：draft(草稿) → confirmed(已确认) → paid(已发放) → cancelled(已取消)

use rust_decimal::Decimal;
use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

/// 工资记录模型
///
/// 真实业务要点：
/// - 月末/旬末按车间汇总工人产量
/// - 按工价方案计算每个工人的应得工资
/// - 进入财务工资核算模块
#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Serialize, Deserialize, Default)]
#[sea_orm(table_name = "wage_record")]
pub struct Model {
    #[sea_orm(primary_key)]
    pub id: i32,

    /// 工资单号：WR-YYYYMM-NNN
    pub record_no: String,
    /// 统计周期开始日期
    pub period_start: chrono::NaiveDate,
    /// 统计周期结束日期
    pub period_end: chrono::NaiveDate,
    /// 车间（用于按车间汇总）
    pub workshop: Option<String>,

    /// 总人数（冗余统计字段）
    pub total_workers: i32,
    /// 总工序记录数
    pub total_step_records: i32,
    /// 总产量（kg/m，所有工序合格产量之和）
    #[sea_orm(column_type = "Decimal(Some((14, 2)))")]
    pub total_qualified_quantity: Decimal,
    /// 总工时（分钟）
    pub total_duration_minutes: i32,
    /// 工资总金额
    #[sea_orm(column_type = "Decimal(Some((14, 2)))")]
    pub total_amount: Decimal,

    /// 状态：draft(草稿) → confirmed(已确认) → paid(已发放) → cancelled(已取消)
    pub status: String,

    /// 确认人 ID
    pub confirmed_by: Option<i32>,
    /// 确认时间
    pub confirmed_at: Option<DateTimeWithTimeZone>,
    /// 发放人 ID
    pub paid_by: Option<i32>,
    /// 发放时间
    pub paid_at: Option<DateTimeWithTimeZone>,

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
    /// 一对多：工资明细
    #[sea_orm(has_many = "super::wage_record_detail::Entity")]
    Details,
}

impl Related<super::wage_record_detail::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::Details.def()
    }
}

impl ActiveModelBehavior for ActiveModel {}
