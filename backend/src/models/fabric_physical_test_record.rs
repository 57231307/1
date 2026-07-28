//! 面料物理指标检测记录 Model
//!
//! V15 P1-3：面料行业十项物理指标建模
//! 依据：V15 审计报告 类四 P1（batch-04 维度 7：验布打卷十项指标）
//! 业务背景：面料行业质检不仅看外观疵点（四分制/十分制），还需检验物理指标（十项）
//!   1. 纬斜（skewness）
//!   2. 缩水率（shrinkage）
//!   3. 起毛起球（pilling）
//!   4. 手感（handfeel）
//!   5. 拉伸强度（tensile_strength）
//!   6. 撕裂强度（tear_strength）
//!   7. 克重（weight_gsm）
//!   8. 色牢度（color_fastness）
//!   9. 门幅（width）
//!   10. 密度（density）
//! A 级判定需外观合格率 ≥95% 且 十项指标全部 pass

use rust_decimal::Decimal;
use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

/// 面料物理指标检测记录
#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Serialize, Deserialize)]
#[sea_orm(table_name = "fabric_physical_test_record")]
pub struct Model {
    #[sea_orm(primary_key)]
    pub id: i32,

    /// 关联验布记录 ID（fabric_inspection_record.id）
    pub inspection_id: i32,

    /// 检测项目：skewness/shrinkage/pilling/handfeel/tensile_strength/tear_strength/weight_gsm/color_fastness/width/density
    pub test_item: String,

    /// 实测值
    #[sea_orm(column_type = "Decimal(Some((12, 2)))")]
    pub test_value: Decimal,

    /// 标准值
    #[sea_orm(column_type = "Decimal(Some((12, 2)))")]
    pub standard_value: Option<Decimal>,

    /// 检测结果：pass(合格) / fail(不合格)
    pub test_result: String,

    /// 检测人 ID
    pub tested_by: Option<i32>,

    /// 检测时间
    pub tested_at: DateTimeWithTimeZone,

    /// 备注
    pub remarks: Option<String>,

    pub created_at: DateTimeWithTimeZone,
    pub updated_at: DateTimeWithTimeZone,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {
    /// 多对一：关联验布记录
    #[sea_orm(
        belongs_to = "super::fabric_inspection_record::Entity",
        from = "Column::InspectionId",
        to = "super::fabric_inspection_record::Column::Id"
    )]
    Inspection,
}

impl Related<super::fabric_inspection_record::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::Inspection.def()
    }
}

impl ActiveModelBehavior for ActiveModel {}
