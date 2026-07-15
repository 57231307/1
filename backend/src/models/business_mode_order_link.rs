//! 单据-业务模式关联模型（business_mode_order_link 表）
//!
//! v14 批次 431：多业务模式支持
//! 依据：面料行业真实业务调研文档 §6 业务模式 6 种
//! 真实业务：销售订单/采购订单/生产订单/委外订单关联业务模式，含模式快照防止后续修改影响历史单据

use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

/// 单据-业务模式关联模型
///
/// 真实业务要点：
/// - 同一单据只能关联一个业务模式（UNIQUE(document_type, document_id)）
/// - 业务模式被引用后不可删除（ON DELETE RESTRICT）
/// - 模式快照防止后续模式修改影响历史单据
#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Serialize, Deserialize, Default)]
#[sea_orm(table_name = "business_mode_order_link")]
pub struct Model {
    #[sea_orm(primary_key)]
    pub id: i32,

    /// 业务模式 ID（外键 → business_mode_config，ON DELETE RESTRICT）
    pub mode_id: i32,
    /// 单据类型：sales_order/purchase_order/production_order/outsourcing_order
    pub document_type: String,
    /// 单据 ID
    pub document_id: i32,
    /// 单据号
    pub document_no: String,
    /// 业务模式快照（防止后续修改影响历史单据）
    #[sea_orm(column_type = "Json", nullable)]
    pub mode_snapshot: Option<serde_json::Value>,
    /// 备注
    pub remarks: Option<String>,

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
        on_delete = "Restrict"
    )]
    BusinessMode,
}

impl Related<super::business_mode_config::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::BusinessMode.def()
    }
}

impl ActiveModelBehavior for ActiveModel {}
