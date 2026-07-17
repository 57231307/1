#![allow(dead_code)]
// TODO(tech-debt): 业务接入或重评估后逐项移除；rustc 1.94+ 编译时由编译器报告具体死代码位置。

//! 公海规则配置 Model
//!
//! V15 P0-S08 修复：CRM 公海规则配置（保护期/领取上限/最大持有数）
//! 与 crm_recycle_rules 互补：recycle_rules 管"未跟进天数回收"，
//! pool_rules 管"保护期/领取上限/最大持有数"。
//!
//! 对应迁移：20260717000001_add_crm_pool_rule_and_transfer_approval

use chrono::{DateTime, Utc};
use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

/// 规则类型：保护期
pub const RULE_TYPE_PROTECTION_PERIOD: &str = "protection_period";
/// 规则类型：领取上限
pub const RULE_TYPE_CLAIM_LIMIT: &str = "claim_limit";
/// 规则类型：最大持有数
pub const RULE_TYPE_MAX_HOLDINGS: &str = "max_holdings";

/// 公海规则配置 Entity
#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Serialize, Deserialize)]
#[sea_orm(table_name = "customer_pool_rules")]
pub struct Model {
    /// 规则 ID（主键）
    #[sea_orm(primary_key)]
    pub id: i32,

    /// 规则名称（唯一）
    #[sea_orm(unique)]
    pub name: String,

    /// 规则类型：protection_period / claim_limit / max_holdings
    pub rule_type: String,

    /// 规则数值（天数/次数/数量，按 rule_type 解释）
    pub rule_value: i32,

    /// 适用客户类型：all / wholesale / retail / vip
    pub customer_type: String,

    /// 是否启用
    pub is_enabled: bool,

    /// 备注
    pub notes: Option<String>,

    /// 创建时间
    pub created_at: DateTime<Utc>,

    /// 更新时间
    pub updated_at: DateTime<Utc>,
}

/// 公海规则配置关联关系
#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}

impl ActiveModelBehavior for ActiveModel {}
