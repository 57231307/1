#![allow(dead_code)]
// TODO(tech-debt): 业务接入或重评估后逐项移除；rustc 1.94+ 编译时由编译器报告具体死代码位置。

//! 加料处方模型（production_recipe_addition 表）
//!
//! v14 批次 424：大货处方与加料处方流程
//! 依据：面料行业真实业务调研文档 §11.2 加料处方（染色补料单）
//! 真实业务：大货处方审核后，生产过程中出现色差/助剂不足/工艺调整
//!          → 扫描流转卡 → 加载已审核大货处方 → 登记加料物料 → 生成加料处方单
//! 关键约束：关联的大货处方必须为 approved 状态

use rust_decimal::Decimal;
use sea_orm::entity::prelude::*;
use sea_orm::FromJsonQueryResult;
use serde::{Deserialize, Serialize};

/// 加料处方物料明细项（addition_detail JSON 数组元素）
///
/// 真实业务字段说明：
/// - amount: 加料用量（kg/L/g/ml）
/// - category: dye(染料) / auxiliary(助剂)
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, FromJsonQueryResult)]
pub struct AdditionMaterialItem {
    /// 物料编码
    pub material_code: String,
    /// 物料名称
    pub material_name: String,
    /// 加料用量
    pub amount: Decimal,
    /// 单位：kg/L/g/ml
    pub unit: String,
    /// 类别：dye(染料) / auxiliary(助剂)
    pub category: String,
}

/// 加料处方模型
///
/// 真实业务要点：
/// - 关联的大货处方必须为 approved 状态
/// - 加料原因：色差/助剂不足/工艺调整
/// - 状态机：draft → approved → closed
#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Serialize, Deserialize, Default)]
#[sea_orm(table_name = "production_recipe_addition")]
pub struct Model {
    #[sea_orm(primary_key)]
    pub id: i32,

    /// 加料处方单号：PA-YYYYMMDDHHMMSS-NNN
    pub addition_no: String,
    /// 关联大货处方（必填）
    pub production_recipe_id: i32,
    /// 关联工单
    pub work_order_id: Option<i32>,
    /// 关联缸号
    pub dye_batch_id: Option<i32>,

    /// 加料原因：色差/助剂不足/工艺调整
    pub addition_reason: Option<String>,
    /// 加料明细 JSON：[{material_code, material_name, amount, unit, category}]
    pub addition_detail: Option<Vec<AdditionMaterialItem>>,
    /// 加料成本合计
    #[sea_orm(column_type = "Decimal(Some((12, 4)))")]
    pub total_cost: Option<Decimal>,

    // ===== 状态机 =====
    /// 状态：draft → approved → closed
    pub status: String,

    // ===== 审核与开单信息 =====
    /// 审核人
    pub approved_by: Option<i32>,
    /// 审核时间
    pub approved_at: Option<DateTimeWithTimeZone>,
    /// 开单人
    pub issued_by: Option<i32>,

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
    /// 关联大货处方
    #[sea_orm(
        belongs_to = "super::production_recipe::Entity",
        from = "Column::ProductionRecipeId",
        to = "super::production_recipe::Column::Id",
        on_update = "Cascade",
        on_delete = "Cascade"
    )]
    ProductionRecipe,

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
}

impl Related<super::production_recipe::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::ProductionRecipe.def()
    }
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

impl ActiveModelBehavior for ActiveModel {}
