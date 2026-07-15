#![allow(dead_code)]
// TODO(tech-debt): 业务接入或重评估后逐项移除；rustc 1.94+ 编译时由编译器报告具体死代码位置。

//! 验布记录模型（fabric_inspection_record 表）
//!
//! v14 批次 426：验布打卷流程贯通
//! 依据：面料行业真实业务调研文档 §12.4 验布打卷与成品入库
//! 真实业务流程：验布机对接码表/电子称 → 疵点采集 → 生成验布报告 → 卷唛标签打印 → PDA 扫码自动入库
//! 评分制式：四分制（AATCC/ASTM D5430，针织+梭织通用）/ 十分制（梭织布）

use rust_decimal::Decimal;
use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

/// 验布记录模型
///
/// 真实业务要点：
/// - 一缸一验布记录，记录评分制式/总扣分/每百平方码分数/等级判定
/// - 四分制等级：每百平方码分数 ≤40 = 首级(first)，>40 = 次级(second)
/// - 十分制等级：总扣分 < 总码数 = 首级(first)，≥ 总码数 = 次级(second)
/// - 联动 A/B/C 分级：A 级合格/B 级让步接收/C 级返工报废（基于合格率）
/// - 打卷汇总：total_rolls/total_roll_length/total_roll_weight 由打卷操作累加
#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Serialize, Deserialize, Default)]
#[sea_orm(table_name = "fabric_inspection_record")]
pub struct Model {
    #[sea_orm(primary_key)]
    pub id: i32,

    /// 验布单号（FIR-YYYYMMDDHHMMSS-NNN，唯一）
    pub inspection_no: String,

    /// 关联流转卡（可选，验布环节流转卡状态应为 inspecting）
    pub flow_card_id: Option<i32>,

    /// 缸号（面料行业追溯核心字段）
    pub dye_lot_no: Option<String>,

    /// 产品 ID
    pub product_id: Option<i32>,
    /// 产品名称（冗余存储便于直接查询）
    pub product_name: Option<String>,
    /// 色号
    pub color_no: Option<String>,

    /// 验布日期
    pub inspection_date: chrono::NaiveDate,
    /// 验布员 ID
    pub inspector_id: Option<i32>,
    /// 验布员姓名
    pub inspector_name: Option<String>,
    /// 验布机号
    pub machine_no: Option<String>,

    /// 评分制式：four_point(四分制) / ten_point(十分制)
    pub scoring_system: String,

    /// 受检码数（验布机码表读数）
    #[sea_orm(column_type = "Decimal(Some((12, 2)))")]
    pub inspected_yards: Decimal,

    /// 幅宽（英寸，用于四分制每百平方码分数计算）
    #[sea_orm(column_type = "Decimal(Some((8, 2)))")]
    pub fabric_width_inches: Option<Decimal>,

    /// 总扣分（所有疵点扣分之和）
    pub total_defect_points: i32,

    /// 每百平方码分数（四分制等级判定依据，计算字段）
    /// 计算公式：每百平方码分数 = (总扣分 × 36 × 100) / (受检码数 × 幅宽英寸)
    #[sea_orm(column_type = "Decimal(Some((10, 2)))")]
    pub points_per_100_sq_yards: Option<Decimal>,

    /// 验布等级：first(首级) / second(次级)
    pub grade: Option<String>,

    /// 合格率（百分比，用于联动 A/B/C 分级）
    #[sea_orm(column_type = "Decimal(Some((5, 2)))")]
    pub qualification_rate: Option<Decimal>,

    /// A/B/C 级（联动 determine_quality_grade：A 级合格/B 级让步接收/C 级返工报废）
    pub abc_grade: Option<String>,

    /// 打卷汇总：总卷数
    pub total_rolls: i32,
    /// 打卷汇总：总长度（米）
    #[sea_orm(column_type = "Decimal(Some((12, 2)))")]
    pub total_roll_length: Decimal,
    /// 打卷汇总：总重量（千克）
    #[sea_orm(column_type = "Decimal(Some((12, 2)))")]
    pub total_roll_weight: Decimal,

    /// 状态：pending/inspecting/graded/rolled/closed
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
    /// 一对多：验布记录下的疵点明细
    #[sea_orm(has_many = "super::fabric_defect_record::Entity")]
    Defects,

    /// 多对一：关联流转卡
    #[sea_orm(
        belongs_to = "super::production_flow_card::Entity",
        from = "Column::FlowCardId",
        to = "super::production_flow_card::Column::Id"
    )]
    FlowCard,
}

impl Related<super::fabric_defect_record::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::Defects.def()
    }
}

impl Related<super::production_flow_card::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::FlowCard.def()
    }
}

impl ActiveModelBehavior for ActiveModel {}
