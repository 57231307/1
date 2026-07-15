//! 业务模式规则模型（business_mode_rule 表）
//!
//! v14 批次 431：多业务模式支持
//! 依据：面料行业真实业务调研文档 §6 业务模式 6 种
//! 真实业务：每种业务模式定义一组规则（必需/可选/禁止），用于校验单据流转合法性

use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

/// 业务模式规则模型
///
/// 真实业务要点：
/// - 规则类型 3 种：required 必需/optional 可选/forbidden 禁止
/// - 用于校验单据流转合法性，例如染整加工模式禁止 sales 模块
/// - 校验逻辑用 JSONB 描述，支持复杂条件
#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Serialize, Deserialize, Default)]
#[sea_orm(table_name = "business_mode_rule")]
pub struct Model {
    #[sea_orm(primary_key)]
    pub id: i32,

    /// 业务模式 ID（外键 → business_mode_config）
    pub mode_id: i32,
    /// 规则代码
    pub rule_code: String,
    /// 规则名称
    pub rule_name: String,
    /// 规则类型：required 必需/optional 可选/forbidden 禁止
    pub rule_type: String,
    /// 模块名
    pub module_name: String,
    /// 校验逻辑描述（JSONB）
    #[sea_orm(column_type = "Json", nullable)]
    pub validation_logic: Option<serde_json::Value>,
    /// 规则描述
    pub description: Option<String>,
    /// 是否启用
    pub is_active: bool,

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
