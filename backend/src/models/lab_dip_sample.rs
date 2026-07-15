#![allow(dead_code)]
// TODO(tech-debt): 业务接入或重评估后逐项移除；rustc 1.94+ 编译时由编译器报告具体死代码位置。

//! 打样小样模型（lab_dip_sample 表）
//!
//! v14 批次 423B：化验室打样流程贯通
//! 依据：面料行业真实业务调研文档 §11.1 打样处方（ABCD 多版小样）
//! 真实业务：技术科打样员根据通知单打 ABCD 多版小样，每版含处方/工艺参数/对色结果，客户从中选 1 版作为 OK 样

use rust_decimal::Decimal;
use sea_orm::entity::prelude::*;

/// 处方明细项（染料组合+用量）
///
/// 真实业务：每个染料含名称、用量、单位、百分比（o.w.f 染料对织物重量百分比）
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, FromJsonQueryResult)]
pub struct FormulaDetailItem {
    /// 染料名称
    pub dye_name: String,
    /// 用量
    pub amount: Decimal,
    /// 单位：g/L、%(o.w.f)、g
    pub unit: String,
    /// 百分比（o.w.f 染料对织物重量百分比）
    pub percentage: Option<Decimal>,
}

/// 打样小样模型
#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Serialize, Deserialize, Default)]
#[sea_orm(table_name = "lab_dip_sample")]
pub struct Model {
    #[sea_orm(primary_key)]
    pub id: i32,

    /// 关联打样通知单 ID
    pub request_id: i32,

    // ===== 版本标识（真实业务 ABCD 四版） =====
    /// 版本标识：A/B/C/D/E...
    pub version_label: String,
    /// 版本序号：1/2/3/4...
    pub version_seq: i32,

    // ===== 处方信息 =====
    /// 配方编号（关联 dye_recipe 或独立小样配方）
    pub recipe_no: Option<String>,
    /// 关联 dye_recipe 表（如已建档）
    pub dye_recipe_id: Option<i32>,
    /// 处方详情（染料组合+用量，文本描述）
    pub formula: Option<Text>,
    /// 处方明细 JSON：[{dye_name, amount, unit, percentage}]
    pub formula_detail: Option<Vec<FormulaDetailItem>>,

    // ===== 工艺参数（真实业务关键参数） =====
    /// 染色温度（℃）
    #[sea_orm(column_type = "Decimal(Some((5, 2)))")]
    pub temperature: Option<Decimal>,
    /// 保温时间（分钟）
    pub time_minutes: Option<i32>,
    /// 浴比（如 1:5、1:8，小样标准 1:5）
    pub liquor_ratio: Option<String>,
    /// pH 值
    #[sea_orm(column_type = "Decimal(Some((5, 2)))")]
    pub ph_value: Option<Decimal>,
    /// 染色方法：dip(浸染)/pad(轧染)
    pub dyeing_method: Option<String>,

    // ===== 成本核算 =====
    /// 染料成本（元）
    #[sea_orm(column_type = "Decimal(Some((10, 4)))")]
    pub dye_cost: Option<Decimal>,
    /// 助剂成本（元）
    #[sea_orm(column_type = "Decimal(Some((10, 4)))")]
    pub auxiliary_cost: Option<Decimal>,
    /// 总成本（元，为报价依据）
    #[sea_orm(column_type = "Decimal(Some((10, 4)))")]
    pub total_cost: Option<Decimal>,

    // ===== 对色结果 =====
    /// 色差等级（4-5 级为 OK，<4 级为重打）
    pub color_difference_grade: Option<i32>,
    /// Delta E 色差值
    #[sea_orm(column_type = "Decimal(Some((5, 2)))")]
    pub color_difference_value: Option<Decimal>,
    /// 对色结果：pending(待对色) → matched(对色OK) → not_matched(不匹配) → selected(客户选中OK样)
    pub matching_result: String,

    // ===== 审核信息 =====
    /// 审核人（研发组长）
    pub approved_by: Option<i32>,
    /// 审核时间
    pub approved_at: Option<DateTimeWithTimeZone>,
    /// 审核意见
    pub approval_comment: Option<Text>,

    // ===== 复样关联（OK 样升级为复样） =====
    /// 复样状态：none(未复样) → resampling(复样中) → resampled(复样通过) → failed(复样失败)
    pub resample_status: Option<String>,
    /// 复样升级后的大货处方 ID（关联 dye_recipe）
    pub resample_recipe_id: Option<i32>,

    /// 备注
    pub remarks: Option<Text>,

    // ===== 软删除与审计 =====
    pub is_deleted: bool,
    pub created_by: Option<i32>,
    pub created_at: DateTimeWithTimeZone,
    pub updated_at: DateTimeWithTimeZone,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {
    /// 关联打样通知单
    #[sea_orm(
        belongs_to = "super::lab_dip_request::Entity",
        from = "Column::RequestId",
        to = "super::lab_dip_request::Column::Id",
        on_update = "Cascade",
        on_delete = "Cascade"
    )]
    LabDipRequest,

    /// 关联已建档的 dye_recipe
    #[sea_orm(
        belongs_to = "super::dye_recipe::Entity",
        from = "Column::DyeRecipeId",
        to = "super::dye_recipe::Column::Id",
        on_update = "Cascade",
        on_delete = "SetNull"
    )]
    DyeRecipe,

    /// 关联复样升级后的大货处方
    #[sea_orm(
        belongs_to = "super::dye_recipe::Entity",
        from = "Column::ResampleRecipeId",
        to = "super::dye_recipe::Column::Id",
        on_update = "Cascade",
        on_delete = "SetNull"
    )]
    ResampleRecipe,

    /// 一对多：基于此 OK 样的复样记录
    #[sea_orm(has_many = "super::lab_dip_resample::Entity")]
    LabDipResamples,
}

impl Related<super::lab_dip_request::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::LabDipRequest.def()
    }
}

impl Related<super::dye_recipe::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::DyeRecipe.def()
    }
}

impl Related<super::lab_dip_resample::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::LabDipResamples.def()
    }
}

impl ActiveModelBehavior for ActiveModel {}
