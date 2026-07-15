#![allow(dead_code)]
// TODO(tech-debt): 业务接入或重评估后逐项移除；rustc 1.94+ 编译时由编译器报告具体死代码位置。

//! 大货处方模型（production_recipe 表）
//!
//! v14 批次 424：大货处方与加料处方流程
//! 依据：面料行业真实业务调研文档 §11.2 大货处方（染色配料单）
//! 真实业务：车间技术员扫描流转卡 → 依据备布数量与浴比 → 加载小样处方/历史大货处方
//!          → 填写物料明细 → 计算用量 → 开具大货处方单 → 审核后自动建立生产领用单据

use rust_decimal::Decimal;
use sea_orm::entity::prelude::*;
use sea_orm::FromJsonQueryResult;
use serde::{Deserialize, Serialize};

/// 大货处方物料明细项（recipe_detail JSON 数组元素）
///
/// 真实业务字段说明：
/// - concentration: 浓度百分比（owf%，对布重百分比），助剂可为空
/// - amount: 用量（kg/L，由浓度×布重×浴比/100 计算）
/// - category: dye(染料) / auxiliary(助剂)
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, FromJsonQueryResult)]
pub struct RecipeMaterialItem {
    /// 物料编码
    pub material_code: String,
    /// 物料名称
    pub material_name: String,
    /// 浓度百分比（owf%，对布重百分比），助剂可为空
    pub concentration: Option<Decimal>,
    /// 单位：kg/L/g/ml
    pub unit: String,
    /// 用量（由浓度×布重×浴比/100 计算）
    pub amount: Decimal,
    /// 类别：dye(染料) / auxiliary(助剂)
    pub category: String,
}

/// 大货处方模型
///
/// 真实业务要点：
/// - 同一工单号只能开一张大货处方单（业务约束，Service 层校验）
/// - 备布重量(fabric_weight)与浴比(liquor_ratio)为用量计算依据
/// - 浴量(bath_volume) = 布重 × 浴比
/// - 加成系数(adjustment_factor)用于修正小样→大货得色差异
/// - 审核后状态由 draft → approved，自动建立生产领用单据
#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Serialize, Deserialize, Default)]
#[sea_orm(table_name = "production_recipe")]
pub struct Model {
    #[sea_orm(primary_key)]
    pub id: i32,

    /// 大货处方单号：PR-YYYYMMDDHHMMSS-NNN
    pub recipe_no: String,

    // ===== 关联信息 =====
    /// 关联工单/生产订单（production_orders.id）
    pub work_order_id: Option<i32>,
    /// 关联缸号（dye_batch.id）
    pub dye_batch_id: Option<i32>,
    /// 引用的 dye_recipe 处方模板 id（可为空，表示手工录入）
    pub source_recipe_id: Option<i32>,
    /// 关联化验室复样记录（lab_dip_resample.id，可为空）
    pub lab_dip_resample_id: Option<i32>,
    /// 关联客户
    pub customer_id: Option<i32>,

    // ===== 布种与工艺信息 =====
    /// 色号
    pub color_no: Option<String>,
    /// 布种名称
    pub fabric_name: Option<String>,
    /// 规格：纱支/密度/成分
    pub fabric_spec: Option<String>,
    /// 门幅 cm
    #[sea_orm(column_type = "Decimal(Some((10, 2)))")]
    pub fabric_width: Option<Decimal>,
    /// 克重 g/m²
    #[sea_orm(column_type = "Decimal(Some((10, 2)))")]
    pub gram_weight: Option<Decimal>,
    /// 备布重量 kg（用量计算依据，真实业务必填）
    #[sea_orm(column_type = "Decimal(Some((12, 2)))")]
    pub fabric_weight: Decimal,
    /// 染缸设备编号
    pub equipment_no: Option<String>,
    /// 浴比如 1:8（真实业务必填）
    pub liquor_ratio: String,
    /// 浴量/升（= 布重 × 浴比）
    #[sea_orm(column_type = "Decimal(Some((12, 2)))")]
    pub bath_volume: Option<Decimal>,
    /// 加成系数（小样→大货得色差异修正）
    #[sea_orm(column_type = "Decimal(Some((5, 2)))")]
    pub adjustment_factor: Option<Decimal>,

    // ===== 处方明细与成本 =====
    /// 处方明细 JSON：[{material_code, material_name, concentration, unit, amount, category}]
    pub recipe_detail: Option<Vec<RecipeMaterialItem>>,
    /// 染料成本合计
    #[sea_orm(column_type = "Decimal(Some((12, 4)))")]
    pub total_dye_cost: Option<Decimal>,
    /// 助剂成本合计
    #[sea_orm(column_type = "Decimal(Some((12, 4)))")]
    pub total_auxiliary_cost: Option<Decimal>,

    // ===== 状态机 =====
    /// 状态：draft → approved → closed → cancelled
    pub status: String,

    // ===== 审核与开单信息 =====
    /// 审核人
    pub approved_by: Option<i32>,
    /// 审核时间
    pub approved_at: Option<DateTimeWithTimeZone>,
    /// 开单人
    pub issued_by: Option<i32>,
    /// 打印次数
    pub printed_count: Option<i32>,

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
    /// 关联工单/生产订单
    #[sea_orm(
        belongs_to = "super::production_order::Entity",
        from = "Column::WorkOrderId",
        to = "super::production_order::Column::Id",
        on_update = "Cascade",
        on_delete = "SetNull"
    )]
    WorkOrder,

    /// 关联缸号
    #[sea_orm(
        belongs_to = "super::dye_batch::Entity",
        from = "Column::DyeBatchId",
        to = "super::dye_batch::Column::Id",
        on_update = "Cascade",
        on_delete = "SetNull"
    )]
    DyeBatch,

    /// 关联源处方模板（dye_recipe）
    #[sea_orm(
        belongs_to = "super::dye_recipe::Entity",
        from = "Column::SourceRecipeId",
        to = "super::dye_recipe::Column::Id",
        on_update = "Cascade",
        on_delete = "SetNull"
    )]
    SourceRecipe,

    /// 关联客户
    #[sea_orm(
        belongs_to = "super::customer::Entity",
        from = "Column::CustomerId",
        to = "super::customer::Column::Id",
        on_update = "Cascade",
        on_delete = "SetNull"
    )]
    Customer,

    /// 一对多：大货处方下的加料处方
    #[sea_orm(has_many = "super::production_recipe_addition::Entity")]
    Additions,
}

impl Related<super::production_order::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::WorkOrder.def()
    }
}

impl Related<super::dye_batch::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::DyeBatch.def()
    }
}

impl Related<super::dye_recipe::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::SourceRecipe.def()
    }
}

impl Related<super::customer::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::Customer.def()
    }
}

impl Related<super::production_recipe_addition::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::Additions.def()
    }
}

impl ActiveModelBehavior for ActiveModel {}
