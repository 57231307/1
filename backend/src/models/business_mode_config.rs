//! 业务模式配置主表模型（business_mode_config 表）
//!
//! v14 批次 431：多业务模式支持
//! 依据：面料行业真实业务调研文档 §6 业务模式 6 种
//! 真实业务：6 种典型业务模式贯穿采购/库存/生产/委外/销售/结算全链路

use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

/// 业务模式配置主表模型
///
/// 真实业务要点：
/// - 6 种典型业务模式：坯布经销/成品经销/染整加工/自织自染/委托加工/来料加工
/// - 物料来源 4 种：采购/客供/自制/来料
/// - 结算方式 2 种：销售结算/加工费结算
/// - 模式分类 3 种：贸易/加工/集成
#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Serialize, Deserialize, Default)]
#[sea_orm(table_name = "business_mode_config")]
pub struct Model {
    #[sea_orm(primary_key)]
    pub id: i32,

    /// 模式代码（唯一）：grey_trading/finished_trading/dyeing_processing/self_weave_dye/outsourcing/toll_processing
    pub mode_code: String,
    /// 模式名称
    pub mode_name: String,
    /// 模式描述
    pub description: Option<String>,
    /// 是否启用
    pub is_active: bool,
    /// 是否默认模式（同时只能有一个默认模式）
    pub is_default: bool,
    /// 业务流程链 JSONB 数组（例如 ["purchase","inventory_in","production","inventory_out","sales"]）
    #[sea_orm(column_type = "Json")]
    pub process_chain: serde_json::Value,
    /// 物料来源：purchase 采购/customer_provided 客供/self_made 自制/toll 来料
    pub material_source: String,
    /// 结算方式：sale_settlement 销售结算/processing_fee_settlement 加工费结算
    pub settlement_method: String,
    /// 库存类型：grey 坯布/finished 成品/both/none
    pub inventory_type: String,
    /// 成本核算方法：standard 标准/actual 实际/processing_fee 加工费
    pub cost_method: String,
    /// 是否需要采购模块
    pub require_purchase: bool,
    /// 是否需要生产模块
    pub require_production: bool,
    /// 是否需要委外模块
    pub require_outsourcing: bool,
    /// 是否需要销售模块
    pub require_sales: bool,
    /// 模式分类：trading 贸易/processing 加工/integrated 集成
    pub mode_category: String,
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
    /// 一对多：业务模式流程节点
    #[sea_orm(has_many = "super::business_mode_flow_step::Entity")]
    FlowSteps,
    /// 一对多：业务模式规则
    #[sea_orm(has_many = "super::business_mode_rule::Entity")]
    Rules,
    /// 一对多：单据-业务模式关联
    #[sea_orm(has_many = "super::business_mode_order_link::Entity")]
    OrderLinks,
}

impl Related<super::business_mode_flow_step::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::FlowSteps.def()
    }
}

impl Related<super::business_mode_rule::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::Rules.def()
    }
}

impl Related<super::business_mode_order_link::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::OrderLinks.def()
    }
}

impl ActiveModelBehavior for ActiveModel {}
