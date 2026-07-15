//! 业务模式流程节点模型（business_mode_flow_step 表）
//!
//! v14 批次 431：多业务模式支持
//! 依据：面料行业真实业务调研文档 §6 业务模式 6 种
//! 真实业务：每个业务模式对应若干流程节点，节点按 step_no 顺序流转

use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

/// 业务模式流程节点模型
///
/// 真实业务要点：
/// - 每个业务模式对应若干流程节点（3-5 个）
/// - 节点按 step_no 顺序流转，从 1 开始
/// - 步骤代码：purchase/inventory_in/production/outsourcing/inventory_out/sales/settlement
#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Serialize, Deserialize, Default)]
#[sea_orm(table_name = "business_mode_flow_step")]
pub struct Model {
    #[sea_orm(primary_key)]
    pub id: i32,

    /// 业务模式 ID（外键 → business_mode_config）
    pub mode_id: i32,
    /// 步骤序号（从 1 开始）
    pub step_no: i32,
    /// 步骤代码：purchase/inventory_in/production/outsourcing/inventory_out/sales/settlement
    pub step_code: String,
    /// 步骤名称
    pub step_name: String,
    /// 模块名：purchase/inventory/production/outsourcing/sales/cost
    pub module_name: String,
    /// 是否必需
    pub is_required: bool,
    /// 步骤描述
    pub description: Option<String>,

    pub created_at: DateTimeWithTimeZone,
    pub updated_at: DateTimeWithTimeZone,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {
    /// 关联业务模式配置
    #[sea_orm(
        belongs_to = "super::business_mode_config::Entity",
        from = "Column::ModeId",
        to = "super::business_mode_config::Column::Id",
        on_update = "Cascade",
        on_delete = "Cascade"
    )]
    BusinessMode,
}

impl Related<super::business_mode_config::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::BusinessMode.def()
    }
}

impl ActiveModelBehavior for ActiveModel {}
