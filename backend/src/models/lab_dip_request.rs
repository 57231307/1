#![allow(dead_code)]
// TODO(tech-debt): 业务接入或重评估后逐项移除；rustc 1.94+ 编译时由编译器报告具体死代码位置。

//! 化验室打样通知单模型
//!
//! v14 批次 423B：化验室打样流程贯通
//! 依据：面料行业真实业务调研文档 §11.1 化验室打样 5 步闭环
//! 真实业务：业务跟单接到客户打样需求，录入规范的技术要求（对色光源/色牢度/打样版数/坯布规格/交期）

use sea_orm::entity::prelude::*;

/// 打样通知单模型（lab_dip_request 表）
///
/// 真实业务字段说明：
/// - light_source: 对色光源（D65/TL84/U3000/CWF/A 等，多光源用逗号分隔）
/// - sample_versions: 打样版数（真实业务 ABCD 四版，每版可能是不同染料组合或浓度梯度）
/// - dye_category: 染料类别（来样分析后确定：棉麻→活性/还原/硫化/直接；毛丝→酸性；涤纶→分散；锦纶→酸性/活性）
#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Serialize, Deserialize, Default)]
#[sea_orm(table_name = "lab_dip_request")]
pub struct Model {
    #[sea_orm(primary_key)]
    pub id: i32,

    /// 打样通知单号：LD-YYYYMMDDHHMMSS-NNN
    pub request_no: String,

    // ===== 客户信息 =====
    pub customer_id: Option<i32>,
    /// 客户色号
    pub customer_color_no: Option<String>,
    /// 客户色名
    pub customer_color_name: Option<String>,

    // ===== 来样信息 =====
    /// 来样类型：fabric(布样)/yarn(纱样)/paper(纸板)/pantone(色卡)
    pub sample_type: Option<String>,
    /// 坯布规格（纱支/成分/组织）
    pub fabric_spec: Option<String>,
    /// 纤维成分（棉/麻/黏胶/毛/丝/涤纶/锦纶等，决定染料类别）
    pub fabric_component: Option<String>,
    /// 打样坯布大小（如 30x15cm）
    pub sample_size: Option<String>,

    // ===== 对色光源（真实业务多光源） =====
    /// 主对色光源：D65/TL84/U3000/CWF/A 等
    pub light_source: String,
    /// 副光源（检查跳灯现象）
    pub secondary_light_source: Option<String>,

    // ===== 色牢度与环保要求 =====
    /// 色牢度要求 JSON：{soaping, rubbing, daylight, chlorine, dry_cleaning}
    pub color_fastness_req: Option<String>,
    /// 环保标准：Oeko-Tex/GOTS/无
    pub eco_requirement: Option<String>,

    // ===== 打样版数（真实业务 ABCD 四版） =====
    /// 打样版数：默认 4（A/B/C/D）
    pub sample_versions: i32,

    /// 染料类别：reactive(活性)/disperse(分散)/acid(酸性)/vat(还原)/sulfur(硫化)/direct(直接)
    pub dye_category: Option<String>,

    // ===== 交期管理 =====
    /// 客户要求交期
    pub required_date: Date,
    /// 预期打样天数（行业惯例：染色烧杯样3天/印花样10天/色织样10天）
    pub expected_days: Option<i32>,

    // ===== 状态机 =====
    /// 状态：pending → sampling → submitted → approved/rejected → completed
    pub status: String,

    // ===== 客户确认信息 =====
    /// 客户确认时间
    pub customer_approved_at: Option<DateTimeWithTimeZone>,
    /// 客户确认意见
    pub customer_approval_comment: Option<String>,
    /// 客户选中的 OK 样 ID（关联 lab_dip_sample）
    pub approved_sample_id: Option<i32>,

    // ===== 关联生产 =====
    /// 复样通过后升级的 dye_recipe ID（大货处方模板）
    pub production_recipe_id: Option<i32>,

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
    /// 关联客户
    #[sea_orm(
        belongs_to = "super::customer::Entity",
        from = "Column::CustomerId",
        to = "super::customer::Column::Id",
        on_update = "Cascade",
        on_delete = "SetNull"
    )]
    Customer,

    /// 关联选中的 OK 样（lab_dip_sample）
    #[sea_orm(
        belongs_to = "super::lab_dip_sample::Entity",
        from = "Column::ApprovedSampleId",
        to = "super::lab_dip_sample::Column::Id",
        on_update = "Cascade",
        on_delete = "SetNull"
    )]
    ApprovedSample,

    /// 关联复样通过后升级的 dye_recipe（大货处方模板）
    #[sea_orm(
        belongs_to = "super::dye_recipe::Entity",
        from = "Column::ProductionRecipeId",
        to = "super::dye_recipe::Column::Id",
        on_update = "Cascade",
        on_delete = "SetNull"
    )]
    ProductionRecipe,

    /// 一对多：打样通知单下的 ABCD 多版小样
    #[sea_orm(has_many = "super::lab_dip_sample::Entity")]
    LabDipSamples,

    /// 一对多：打样通知单关联的复样记录
    #[sea_orm(has_many = "super::lab_dip_resample::Entity")]
    LabDipResamples,
}

impl Related<super::customer::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::Customer.def()
    }
}

impl Related<super::lab_dip_sample::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::LabDipSamples.def()
    }
}

impl Related<super::lab_dip_resample::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::LabDipResamples.def()
    }
}

impl Related<super::dye_recipe::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::ProductionRecipe.def()
    }
}

impl ActiveModelBehavior for ActiveModel {}
