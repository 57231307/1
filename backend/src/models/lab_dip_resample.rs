#![allow(dead_code)]
// TODO(tech-debt): 业务接入或重评估后逐项移除；rustc 1.94+ 编译时由编译器报告具体死代码位置。

//! 复样记录模型（lab_dip_resample 表）
//!
//! v14 批次 423B：化验室打样流程贯通
//! 依据：面料行业真实业务调研文档 §11.1 复样（大货前验证）
//! 真实业务：OK 样确认后，大货生产前必须复样——用车间半制品布+生产染化料模拟大生产，色差达 4-5 级方可投产

use rust_decimal::Decimal;
use sea_orm::entity::prelude::*;

/// 复样记录模型
///
/// 真实业务要点：
/// - 必须采用车间准备试产的半制品布（不可用化验室存布）
/// - 染料/助剂批号必须与生产一致
/// - 按生产计划安排的纱种、浴比、染化料模拟大生产
/// - 色差达 4-5 级方可投产
/// - 复样通过后由研发组长开染色技术卡
#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Serialize, Deserialize, Default)]
#[sea_orm(table_name = "lab_dip_resample")]
pub struct Model {
    #[sea_orm(primary_key)]
    pub id: i32,

    /// 关联打样通知单 ID
    pub request_id: i32,
    /// 关联 OK 样 ID（lab_dip_sample.id）
    pub source_sample_id: i32,

    /// 复样单号：RS-YYYYMMDDHHMMSS-NNN
    pub resample_no: String,

    // ===== 复样条件（真实业务：模拟大生产） =====
    /// 车间半制品布批号（不可用化验室存布）
    pub workshop_fabric_batch: Option<String>,
    /// 染料批号（与生产一致）
    pub dye_batch_no: Option<String>,
    /// 助剂批号（与生产一致）
    pub auxiliary_batch_no: Option<String>,
    /// 关联生产计划 ID（按生产计划安排的纱种/浴比/染化料）
    pub production_plan_id: Option<i32>,

    // ===== 复样处方（基于 OK 样处方+加成系数调整） =====
    /// 调整后处方（加成/冲减）
    pub adjusted_formula: Option<String>,
    /// 加成系数（小样→大货得色差异修正）
    #[sea_orm(column_type = "Decimal(Some((5, 2)))")]
    pub adjustment_factor: Option<Decimal>,
    /// 调整后温度
    #[sea_orm(column_type = "Decimal(Some((5, 2)))")]
    pub adjusted_temperature: Option<Decimal>,
    /// 调整后时间
    pub adjusted_time_minutes: Option<i32>,
    /// 调整后浴比（车间大货 1:8 以上）
    pub adjusted_liquor_ratio: Option<String>,

    // ===== 复样结果 =====
    /// 色差等级（4-5 级方可投产）
    pub color_difference_grade: Option<i32>,
    /// Delta E 色差值
    #[sea_orm(column_type = "Decimal(Some((5, 2)))")]
    pub color_difference_value: Option<Decimal>,
    /// 复样结果：pending → passed → failed → adjusted
    pub result: String,

    // ===== 审核信息（真实业务：落实审核制度） =====
    /// 审核人（研发组长）
    pub reviewed_by: Option<i32>,
    /// 审核时间
    pub reviewed_at: Option<DateTimeWithTimeZone>,
    /// 审核意见
    pub review_comment: Option<String>,

    // ===== 升级大货处方 =====
    /// 复样通过后升级的 dye_recipe ID（大货处方模板）
    pub production_recipe_id: Option<i32>,

    // ===== 染色技术卡（研发输出物） =====
    /// 染色技术卡编号
    pub tech_card_no: Option<String>,
    /// 开卡人（研发组长）
    pub tech_card_issued_by: Option<i32>,
    /// 开卡时间
    pub tech_card_issued_at: Option<DateTimeWithTimeZone>,

    /// 备注
    pub remarks: Option<String>,

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

    /// 关联 OK 样（源样）
    #[sea_orm(
        belongs_to = "super::lab_dip_sample::Entity",
        from = "Column::SourceSampleId",
        to = "super::lab_dip_sample::Column::Id",
        on_update = "Cascade",
        on_delete = "Restrict"
    )]
    SourceSample,

    /// 关联升级后的大货处方
    #[sea_orm(
        belongs_to = "super::dye_recipe::Entity",
        from = "Column::ProductionRecipeId",
        to = "super::dye_recipe::Column::Id",
        on_update = "Cascade",
        on_delete = "SetNull"
    )]
    ProductionRecipe,
}

impl Related<super::lab_dip_request::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::LabDipRequest.def()
    }
}

impl Related<super::lab_dip_sample::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::SourceSample.def()
    }
}

impl Related<super::dye_recipe::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::ProductionRecipe.def()
    }
}

impl ActiveModelBehavior for ActiveModel {}
